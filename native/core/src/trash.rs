use super::*;
use std::os::unix::fs::MetadataExt;
#[derive(Clone)]
pub struct PreparedTrash {
    pub root_id: i64,
    pub source: String,
    pub root_path: String,
    pub snapshot: Identity,
    pub epoch: u64,
    pub policy_revision: u64,
    pub authorization_epoch: u64,
}
impl Engine {
    pub fn prepare_trash(&self, root_id: i64, source: String) -> Result<PreparedTrash> {
        if !self.settings.trash_proposals {
            return Err("Trash proposals are disabled".into());
        }
        let root = self.roots.get(&root_id).ok_or("Folder access revoked")?;
        let (directory, name) = self.parent(root_id, &source).map_err(message)?;
        let snapshot = fs::identity(&directory, &name).map_err(message)?;
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(message)?
            .as_secs() as i64;
        if now.saturating_sub(snapshot.mtime) < 10 || snapshot.size > 64 * 1024 * 1024 {
            return Err("File stability or size limit".into());
        }
        Ok(PreparedTrash {
            root_id,
            source,
            root_path: root.path.clone(),
            snapshot,
            epoch: self.permission_epoch,
            policy_revision: self.settings.revision,
            authorization_epoch: self.authorization_epoch,
        })
    }
    pub fn trash_guarded<F: Fn() -> bool>(
        &mut self,
        prepared: PreparedTrash,
        trash_root: PathBuf,
        separately_confirmed: bool,
        current: F,
    ) -> Result<i64> {
        if !separately_confirmed
            || !self.settings.trash_proposals
            || self.permission_epoch != prepared.epoch
            || self.settings.revision != prepared.policy_revision
            || self.authorization_epoch != prepared.authorization_epoch
            || !current()
        {
            return Err("Separate current Trash confirmation required".into());
        }
        let root = self
            .roots
            .get(&prepared.root_id)
            .ok_or("Folder access revoked")?;
        if root.path != prepared.root_path {
            return Err("Root changed after Trash confirmation".into());
        }
        let unresolved:i64=self.db.query_row("SELECT count(*) FROM operations WHERE root_id=? AND state IN ('pending','undo_pending','needs_review')",[prepared.root_id],|r|r.get(0)).map_err(message)?;
        if unresolved > 0 {
            return Err("Resolve interrupted operations first".into());
        }
        let (from, name) = self
            .parent(prepared.root_id, &prepared.source)
            .map_err(message)?;
        let snapshot = fs::identity(&from, &name).map_err(message)?;
        if snapshot != prepared.snapshot {
            return Err("Source changed after Trash preview".into());
        }
        let target_root = fs::special_root(&trash_root, None).map_err(message)?;
        let metadata = target_root.metadata().map_err(message)?;
        if metadata.dev() != snapshot.dev {
            return Err("Cross volume Trash is not supported".into());
        }
        let digest = fs::digest(&from, &name, &snapshot).map_err(message)?;
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(message)?
            .as_nanos();
        let mut shortened = String::new();
        for ch in name.chars() {
            if shortened.len() + ch.len_utf8() > 120 {
                break;
            }
            shortened.push(ch);
        }
        let target = format!("FilePilot-{stamp}-{shortened}");
        let path = trash_root
            .to_str()
            .ok_or("Unsupported Trash root encoding")?;
        self.db.execute("INSERT INTO operations(root_id,source,destination,identity,state,digest,kind,trash_root,trash_device,trash_inode) VALUES(?,?,?,?,'pending',?,'trash',?,?,?)",params![prepared.root_id,prepared.source,target,serde_json::to_string(&snapshot).map_err(message)?,digest,path,metadata.dev().to_string(),metadata.ino().to_string()]).map_err(message)?;
        let id = self.db.last_insert_rowid();
        if !current() {
            self.db
                .execute("UPDATE operations SET state='cancelled' WHERE id=?", [id])
                .map_err(message)?;
            return Err("Permissions changed before Trash move".into());
        }
        fs::move_exclusive(&from, &name, &target_root, &target).map_err(message)?;
        let after = fs::identity(&target_root, &target).map_err(message)?;
        if !fs::same_file(&after, &snapshot)
            || fs::digest(&target_root, &target, &after).map_err(message)? != digest
        {
            return Err("Trash result requires recovery".into());
        }
        self.db
            .execute(
                "UPDATE operations SET state='applied',identity=? WHERE id=?",
                params![serde_json::to_string(&after).map_err(message)?, id],
            )
            .map_err(message)?;
        self.pending.clear();
        Ok(id)
    }
    fn trash_location(&self, id: i64) -> Result<(PathBuf, (u64, u64))> {
        let(path,device,inode):(String,String,String)=self.db.query_row("SELECT trash_root,trash_device,trash_inode FROM operations WHERE id=? AND kind='trash'",[id],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?))).map_err(message)?;
        Ok((
            PathBuf::from(path),
            (
                device.parse().map_err(message)?,
                inode.parse().map_err(message)?,
            ),
        ))
    }
    pub(crate) fn undo_trash_guarded<F: Fn() -> bool>(
        &mut self,
        id: i64,
        current: F,
    ) -> Result<()> {
        let(root,source,target,raw,digest,state):(i64,String,String,String,String,String)=self.db.query_row("SELECT root_id,source,destination,identity,digest,state FROM operations WHERE id=? AND kind='trash'",[id],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?,r.get(5)?))).map_err(message)?;
        if state != "applied" {
            return Err("Trash operation is not safely undoable".into());
        }
        let (to, name) = self.parent(root, &source).map_err(message)?;
        let (path, node) = self.trash_location(id)?;
        let from = fs::special_root(&path, Some(node)).map_err(message)?;
        let expected: Identity = serde_json::from_str(&raw).map_err(message)?;
        if fs::identity(&from, &target).map_err(message)? != expected
            || fs::digest(&from, &target, &expected).map_err(message)? != digest
        {
            return Err("Trashed file changed".into());
        }
        if fs::exists(&to, &name).map_err(message)? {
            return Err("Original destination is occupied".into());
        }
        self.db
            .execute(
                "UPDATE operations SET state='undo_pending' WHERE id=?",
                [id],
            )
            .map_err(message)?;
        if !current() {
            self.db
                .execute("UPDATE operations SET state='applied' WHERE id=?", [id])
                .map_err(message)?;
            return Err("Permission changed before Trash Undo".into());
        }
        fs::move_exclusive(&from, &target, &to, &name).map_err(message)?;
        self.db
            .execute("UPDATE operations SET state='undone' WHERE id=?", [id])
            .map_err(message)?;
        self.pending.clear();
        Ok(())
    }
    pub(crate) fn recover_trash(&mut self, operation: &Operation) -> Result<()> {
        if !self.roots.contains_key(&operation.root_id) {
            return Ok(());
        }
        let (raw, digest): (String, String) = self
            .db
            .query_row(
                "SELECT identity,digest FROM operations WHERE id=?",
                [operation.id],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .map_err(message)?;
        let expected: Identity = serde_json::from_str(&raw).map_err(message)?;
        let source = self.parent(operation.root_id, &operation.source);
        let trash = self
            .trash_location(operation.id)
            .and_then(|(path, node)| fs::special_root(&path, Some(node)).map_err(message));
        let mut new_state = "needs_review";
        if let (Ok((directory, name)), Ok(target)) = (source, trash) {
            let original = fs::identity(&directory, &name).ok();
            let moved = fs::identity(&target, &operation.destination).ok();
            let valid = |dir: &std::fs::File, name: &str, value: &Identity| {
                fs::same_file(value, &expected)
                    && fs::digest(dir, name, value).ok().as_deref() == Some(digest.as_str())
            };
            if original
                .as_ref()
                .is_some_and(|value| valid(&directory, &name, value))
                && !fs::exists(&target, &operation.destination).unwrap_or(true)
            {
                new_state = if operation.state == "undo_pending" {
                    "undone"
                } else {
                    "not_applied"
                };
            } else if moved
                .as_ref()
                .is_some_and(|value| valid(&target, &operation.destination, value))
                && !fs::exists(&directory, &name).unwrap_or(true)
            {
                new_state = "applied";
                self.db
                    .execute(
                        "UPDATE operations SET identity=? WHERE id=?",
                        params![
                            serde_json::to_string(&moved.unwrap()).map_err(message)?,
                            operation.id
                        ],
                    )
                    .map_err(message)?;
            }
        }
        self.db
            .execute(
                "UPDATE operations SET state=? WHERE id=?",
                params![new_state, operation.id],
            )
            .map_err(message)?;
        Ok(())
    }
}

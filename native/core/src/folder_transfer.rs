//! Explicit ordinary-folder transfers preserve the tree and reject protected or changing contents
use super::*;
use sha2::{Digest, Sha256};
use std::os::unix::fs::MetadataExt;
#[derive(Clone, Debug, Serialize, Deserialize)]
struct TreeEntry {
    path: String,
    directory: bool,
    dev: u64,
    ino: u64,
    mtime: i64,
    nanos: i64,
    identity: Option<Identity>,
    digest: Option<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
struct Tree {
    entries: Vec<TreeEntry>,
    digest: String,
    bytes: u64,
    files: usize,
    folders: usize,
}
#[derive(Clone, Debug, Serialize)]
pub struct FolderTransferPlan {
    pub token: u64,
    pub source_root_id: i64,
    pub target_root_id: i64,
    pub source: String,
    pub destination: String,
    pub folder_transfer: bool,
    pub files_count: usize,
    pub folders_count: usize,
    pub bytes: u64,
    tree: Tree,
    revision: u64,
    authorization_epoch: u64,
}
fn join(a: &str, b: &str) -> String {
    if a.is_empty() {
        b.into()
    } else {
        format!("{a}/{b}")
    }
}
fn tree(root: std::fs::File) -> Result<Tree> {
    let mut entries = Vec::new();
    let mut stack = vec![(root, String::new(), 0)];
    let (mut bytes, mut files, mut folders, mut visited) = (0u64, 0usize, 0usize, 0usize);
    while let Some((directory, path, depth)) = stack.pop() {
        if depth > 16
            || fs::project(directory.as_raw_fd())
            || fs::dataless(directory.as_raw_fd()).map_err(message)?
        {
            return Err(
                "Folder contains a protected project placeholder or exceeds the depth limit".into(),
            );
        }
        let meta = directory.metadata().map_err(message)?;
        entries.push(TreeEntry {
            path: path.clone(),
            directory: true,
            dev: meta.dev(),
            ino: meta.ino(),
            mtime: meta.mtime(),
            nanos: meta.mtime_nsec(),
            identity: None,
            digest: None,
        });
        folders += 1;
        for name in fs::names(&directory).map_err(message)? {
            visited += 1;
            if visited > 512 || fs::excluded(&name) {
                return Err(
                    "Folder contains a protected entry or exceeds the 512 entry transfer limit"
                        .into(),
                );
            }
            let relative = join(&path, &name);
            fs::parts(&relative).map_err(message)?;
            if let Ok(child) = fs::child_directory(&directory, &name) {
                stack.push((child, relative, depth + 1));
                continue;
            }
            let snapshot = fs::identity(&directory, &name).map_err(message)?;
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_err(message)?
                .as_secs() as i64;
            if now.saturating_sub(snapshot.mtime) < 10 || snapshot.size > 64 * 1024 * 1024 {
                return Err("Every file must be stable and at most 64 MiB".into());
            }
            bytes += snapshot.size;
            files += 1;
            if files > 100 || bytes > 256 * 1024 * 1024 {
                return Err("Folder transfers are limited to 100 files and 256 MiB".into());
            }
            let digest = fs::digest(&directory, &name, &snapshot).map_err(message)?;
            entries.push(TreeEntry {
                path: relative,
                directory: false,
                dev: snapshot.dev,
                ino: snapshot.ino,
                mtime: snapshot.mtime,
                nanos: snapshot.nanos,
                identity: Some(snapshot),
                digest: Some(digest),
            });
        }
    }
    entries.sort_by(|a, b| a.path.cmp(&b.path));
    let digest = format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(&entries).map_err(message)?)
    );
    Ok(Tree {
        entries,
        digest,
        bytes,
        files,
        folders,
    })
}
impl Engine {
    fn folder_tree(&self, id: i64, path: &str) -> Result<Tree> {
        let (folder, _) = self
            .parent(id, &format!("{path}/validation"))
            .map_err(message)?;
        tree(folder)
    }
    pub fn folder_is_held(&self, id: i64, source: &str) -> bool {
        self.folder_transfers.values().any(|p| {
            p.source_root_id == id
                && p.revision == self.settings.revision
                && p.authorization_epoch == self.authorization_epoch
                && (source == p.source || source.starts_with(&format!("{}/", p.source)))
        })
    }
    pub fn cancel_folder_transfer(&mut self, token: u64) {
        self.folder_transfers.remove(&token);
    }
    pub fn prepare_folder_transfer(
        &mut self,
        source_root_id: i64,
        source: String,
        target_root_id: i64,
        folder: String,
    ) -> Result<FolderTransferPlan> {
        for id in [source_root_id, target_root_id] {
            if !self.roots.get(&id).is_some_and(|r| r.initial_reviewed) {
                return Err("Both locations need active access and completed setup".into());
            }
        }
        let parts = fs::parts(&source).map_err(message)?;
        let basename = *parts.last().unwrap();
        let destination = join(&folder, basename);
        if source_root_id == target_root_id
            && (source == destination || destination.starts_with(&format!("{source}/")))
        {
            return Err("A folder cannot be moved into itself or its own descendants".into());
        }
        let (from, _) = self.parent(source_root_id, &source).map_err(message)?;
        let (to, name) = self.parent(target_root_id, &destination).map_err(message)?;
        if fs::exists(&to, &name).map_err(message)? {
            return Err(
                "Destination already exists folders are never merged or overwritten".into(),
            );
        }
        if from.metadata().map_err(message)?.dev() != to.metadata().map_err(message)?.dev() {
            return Err("Folder transfer requires the same volume".into());
        }
        let tree = self.folder_tree(source_root_id, &source)?;
        if tree.entries[0].dev != to.metadata().map_err(message)?.dev() {
            return Err("Folder transfer requires the same volume".into());
        }
        let token = self.next_id;
        self.next_id = self.next_id.checked_add(1).ok_or("Identifiers exhausted")?;
        self.folder_transfers
            .retain(|_, p| p.source_root_id != source_root_id || p.source != source);
        let plan = FolderTransferPlan {
            token,
            source_root_id,
            target_root_id,
            source,
            destination,
            folder_transfer: true,
            files_count: tree.files,
            folders_count: tree.folders,
            bytes: tree.bytes,
            tree,
            revision: self.settings.revision,
            authorization_epoch: self.authorization_epoch,
        };
        self.folder_transfers.insert(token, plan.clone());
        Ok(plan)
    }
    fn remember_tree(&self, id: i64, prefix: &str, tree: &Tree) -> Result<()> {
        for entry in tree.entries.iter().filter(|e| !e.directory) {
            self.db
                .execute(
                    "INSERT OR REPLACE INTO kept VALUES(?,?,?)",
                    params![
                        id,
                        join(prefix, &entry.path),
                        serde_json::to_string(entry.identity.as_ref().unwrap()).map_err(message)?
                    ],
                )
                .map_err(message)?;
        }
        Ok(())
    }
    pub fn apply_folder_transfer<F: Fn() -> bool>(
        &mut self,
        token: u64,
        current: F,
    ) -> Result<i64> {
        let p = self
            .folder_transfers
            .get(&token)
            .ok_or("Folder transfer preview expired")?
            .clone();
        if p.revision != self.settings.revision
            || p.authorization_epoch != self.authorization_epoch
            || !current()
        {
            return Err("Permissions changed review the folder again".into());
        }
        let unresolved:i64=self.db.query_row("SELECT count(*) FROM operations WHERE (root_id IN (?1,?2) OR destination_root_id IN (?1,?2)) AND state IN ('pending','undo_pending','needs_review')",params![p.source_root_id,p.target_root_id],|r|r.get(0)).map_err(message)?;
        if unresolved > 0 {
            return Err("Reconcile interrupted changes in both locations first".into());
        }
        let before = self.folder_tree(p.source_root_id, &p.source)?;
        if before.digest != p.tree.digest {
            return Err("Folder contents changed after preview".into());
        }
        let (from, name) = self.parent(p.source_root_id, &p.source).map_err(message)?;
        let (to, target) = self
            .parent(p.target_root_id, &p.destination)
            .map_err(message)?;
        if fs::exists(&to, &target).map_err(message)? {
            return Err("Destination collision".into());
        }
        if from.metadata().map_err(message)?.dev() != to.metadata().map_err(message)?.dev() {
            return Err("Folder transfer requires the same volume".into());
        }
        self.db.execute("INSERT INTO operations(root_id,source,destination,identity,state,digest,kind,destination_root_id) VALUES(?,?,?,?,'pending',?,'folder_transfer',?)",params![p.source_root_id,p.source,p.destination,serde_json::to_string(&p.tree).map_err(message)?,p.tree.digest,p.target_root_id]).map_err(message)?;
        let operation = self.db.last_insert_rowid();
        if !current() {
            self.db
                .execute(
                    "UPDATE operations SET state='cancelled' WHERE id=?",
                    [operation],
                )
                .map_err(message)?;
            return Err("Permissions changed before folder move".into());
        }
        fs::move_exclusive(&from, &name, &to, &target).map_err(message)?;
        let after = self.folder_tree(p.target_root_id, &p.destination)?;
        if after.digest != p.tree.digest {
            return Err(
                "Folder changed during transfer check interrupted changes in Activity".into(),
            );
        }
        let transaction = self.db.unchecked_transaction().map_err(message)?;
        transaction
            .execute(
                "UPDATE operations SET state='applied' WHERE id=?",
                [operation],
            )
            .map_err(message)?;
        self.remember_tree(p.target_root_id, &p.destination, &after)?;
        transaction.commit().map_err(message)?;
        self.pending.retain(|_, old| {
            old.root_id != p.source_root_id || !old.source.starts_with(&format!("{}/", p.source))
        });
        self.folder_transfers.remove(&token);
        Ok(operation)
    }
    pub(crate) fn undo_folder_transfer<F: Fn() -> bool>(
        &mut self,
        id: i64,
        current: F,
    ) -> Result<()> {
        let(a,b,source,destination,raw,state):(i64,i64,String,String,String,String)=self.db.query_row("SELECT root_id,destination_root_id,source,destination,identity,state FROM operations WHERE id=?",[id],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?,r.get(5)?))).map_err(message)?;
        if state != "applied" {
            return Err("Folder transfer is not safely undoable".into());
        }
        let expected: Tree = serde_json::from_str(&raw).map_err(message)?;
        let after = self.folder_tree(b, &destination)?;
        if after.digest != expected.digest {
            return Err("Folder contents changed after transfer".into());
        }
        let (from, name) = self.parent(b, &destination).map_err(message)?;
        let (to, original) = self.parent(a, &source).map_err(message)?;
        if fs::exists(&to, &original).map_err(message)? {
            return Err("Original folder location is occupied".into());
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
            return Err("Permissions changed before Undo".into());
        }
        fs::move_exclusive(&from, &name, &to, &original).map_err(message)?;
        let restored = self.folder_tree(a, &source)?;
        if restored.digest != expected.digest {
            return Err("Folder Undo needs reconciliation".into());
        }
        let transaction = self.db.unchecked_transaction().map_err(message)?;
        transaction
            .execute("UPDATE operations SET state='undone' WHERE id=?", [id])
            .map_err(message)?;
        self.remember_tree(a, &source, &restored)?;
        transaction.commit().map_err(message)?;
        self.pending.clear();
        Ok(())
    }
    pub(crate) fn recover_folder_transfer(&mut self, op: &Operation) -> Result<()> {
        let Some(b) = op.destination_root_id else {
            return Err("Folder journal has no destination authorization".into());
        };
        if !self.roots.contains_key(&op.root_id) || !self.roots.contains_key(&b) {
            return Ok(());
        }
        let raw: String = self
            .db
            .query_row("SELECT identity FROM operations WHERE id=?", [op.id], |r| {
                r.get(0)
            })
            .map_err(message)?;
        let expected: Tree = serde_json::from_str(&raw).map_err(message)?;
        let mut state = "needs_review";
        if let (Ok((a, name)), Ok((target, destination))) = (
            self.parent(op.root_id, &op.source),
            self.parent(b, &op.destination),
        ) {
            if self
                .folder_tree(op.root_id, &op.source)
                .is_ok_and(|tree| tree.digest == expected.digest)
                && !fs::exists(&target, &destination).unwrap_or(true)
            {
                state = if op.state == "undo_pending" {
                    "undone"
                } else {
                    "not_applied"
                };
                if state == "undone" {
                    self.remember_tree(op.root_id, &op.source, &expected)?;
                }
            } else if self
                .folder_tree(b, &op.destination)
                .is_ok_and(|tree| tree.digest == expected.digest)
                && !fs::exists(&a, &name).unwrap_or(true)
            {
                state = "applied";
                self.remember_tree(b, &op.destination, &expected)?;
            }
        }
        self.db
            .execute(
                "UPDATE operations SET state=? WHERE id=?",
                params![state, op.id],
            )
            .map_err(message)?;
        Ok(())
    }
}

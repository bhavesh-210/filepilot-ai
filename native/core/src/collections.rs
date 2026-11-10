//! Explicit user collections never infer permission to discard or move important files
use super::*;
#[derive(Clone, Debug, Serialize)]
pub struct CollectionPlan {
    pub token: u64,
    pub root_id: i64,
    pub folder: String,
    pub create_folder: bool,
    pub proposals: Vec<Proposal>,
    revision: u64,
    authorization_epoch: u64,
}
#[derive(Debug, Serialize)]
pub struct CollectionResult {
    pub operations: Vec<i64>,
    pub errors: Vec<String>,
}
fn stable(snapshot: &Identity) -> Result<()> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(message)?
        .as_secs() as i64;
    if now.saturating_sub(snapshot.mtime) < 10 || snapshot.size > 64 * 1024 * 1024 {
        return Err("Use stable files no larger than 64 MiB".into());
    }
    Ok(())
}
impl Engine {
    pub fn collection_title(&self, id: i64, sources: &[String]) -> Result<String> {
        if sources.is_empty() || sources.len() > 100 {
            return Err("Select between one and one hundred files".into());
        }
        let mut categories = HashSet::new();
        for source in sources {
            let (parent, name) = self.parent(id, source).map_err(message)?;
            fs::identity(&parent, &name).map_err(message)?;
            categories.insert(classify(&name));
        }
        let label = if categories.len() == 1 {
            match categories.iter().next().unwrap().as_str() {
                "Education" => "Important study files",
                "Development" => "Important code files",
                "Professional" => "Important work documents",
                "Finance" => "Important financial records",
                "Media" => "Important memories",
                "Software" => "Important installers",
                _ => "Important files",
            }
        } else {
            "Important files"
        };
        Ok(label.into())
    }
    pub fn cancel_collection(&mut self, id: i64, token: u64) -> Result<()> {
        let plan = self
            .collections
            .get(&id)
            .ok_or("Collection preview expired")?;
        if plan.token != token {
            return Err("Collection preview expired".into());
        }
        let plan = self.collections.remove(&id).unwrap();
        for p in plan.proposals {
            self.pending.remove(&p.id);
        }
        Ok(())
    }
    pub fn plan_collection(
        &mut self,
        id: i64,
        sources: Vec<String>,
        folder: String,
        create_folder: bool,
    ) -> Result<CollectionPlan> {
        if sources.is_empty() || sources.len() > 100 {
            return Err("Select between one and one hundred files".into());
        }
        let root = self.roots.get(&id).ok_or("Folder access required")?;
        if !root.initial_reviewed {
            return Err("Complete first cleanup setup before creating collections".into());
        }
        if folder.trim() != folder || folder.len() > 100 {
            return Err("Use a folder title up to one hundred bytes without outer spaces".into());
        }
        let parts = fs::parts(&folder).map_err(message)?;
        if create_folder {
            if parts.len() != 1 {
                return Err("New collections need a single folder title".into());
            }
            let root = self.opened_root(id)?;
            if fs::exists(&root, &folder).map_err(message)? {
                return Err(
                    "That name already exists choose the existing folder or another title".into(),
                );
            }
        } else {
            self.parent(id, &format!("{folder}/validation"))
                .map_err(message)?;
        }
        let mut proposals = Vec::new();
        let mut total_bytes = 0u64;
        let mut source_names = HashSet::new();
        let mut destinations = HashSet::new();
        for source in sources {
            if !source_names.insert(source.clone()) {
                return Err("The same file was selected twice".into());
            }
            let (parent, name) = self.parent(id, &source).map_err(message)?;
            let snapshot = fs::identity(&parent, &name).map_err(message)?;
            stable(&snapshot)?;
            total_bytes = total_bytes
                .checked_add(snapshot.size)
                .ok_or("Collection is too large")?;
            if total_bytes > 256 * 1024 * 1024 {
                return Err("Select at most 256 MiB per collection operation".into());
            }
            let destination = format!("{folder}/{name}");
            if destination == source {
                return Err(format!("{name} is already in that folder"));
            }
            if !destinations.insert(destination.to_lowercase()) {
                return Err("Selected files share a destination filename".into());
            }
            if !create_folder {
                let (parent, name) = self.parent(id, &destination).map_err(message)?;
                if fs::exists(&parent, &name).map_err(message)? {
                    return Err("Destination already contains a selected filename".into());
                }
            }
            let proposal = Proposal {
                id: self.next_id,
                root_id: id,
                source,
                destination: Some(destination),
                category: classify(&name),
                reason: "Selected explicitly for a named collection".into(),
                blocked: false,
                revision: self.settings.revision,
                rename: false,
                certain: false,
                manual_destination: true,
                snapshot,
            };
            self.next_id = self.next_id.checked_add(1).ok_or("Identifiers exhausted")?;
            proposals.push(proposal);
        }
        if let Some(old) = self.collections.remove(&id) {
            for p in old.proposals {
                self.pending.remove(&p.id);
            }
        }
        for p in &proposals {
            self.pending
                .retain(|_, old| old.root_id != id || old.source != p.source);
            self.pending.insert(p.id, p.clone());
        }
        let token = self.next_id;
        self.next_id = self.next_id.checked_add(1).ok_or("Identifiers exhausted")?;
        let plan = CollectionPlan {
            token,
            root_id: id,
            folder,
            create_folder,
            proposals,
            revision: self.settings.revision,
            authorization_epoch: self.authorization_epoch,
        };
        self.collections.insert(id, plan.clone());
        Ok(plan)
    }
    pub fn apply_collection<F: Fn() -> bool>(
        &mut self,
        id: i64,
        token: u64,
        current: F,
    ) -> Result<CollectionResult> {
        let plan = self
            .collections
            .get(&id)
            .ok_or("Collection preview expired")?
            .clone();
        if plan.token != token
            || plan.revision != self.settings.revision
            || plan.authorization_epoch != self.authorization_epoch
            || !current()
        {
            return Err("Permissions or preferences changed preview the collection again".into());
        }
        // Preflight the entire batch before creating a folder or moving the first file
        let _ = self.opened_root(id)?;
        for p in &plan.proposals {
            let (parent, name) = self.parent(id, &p.source).map_err(message)?;
            let identity = fs::identity(&parent, &name).map_err(message)?;
            if identity != p.snapshot {
                return Err("A selected file changed preview again".into());
            }
            stable(&identity)?;
            if !plan.create_folder {
                let (parent, name) = self
                    .parent(id, p.destination.as_deref().unwrap())
                    .map_err(message)?;
                if fs::exists(&parent, &name).map_err(message)? {
                    return Err("A destination is now occupied preview again".into());
                }
            }
        }
        if plan.create_folder {
            if fs::exists(&self.opened_root(id)?, &plan.folder).map_err(message)? {
                return Err("The new folder name is now occupied preview again".into());
            }
            if !current() {
                return Err("Permissions changed no folder created".into());
            }
            self.create_named_folder(id, &plan.folder)?;
        }
        self.collections.remove(&id);
        let mut result = CollectionResult {
            operations: Vec::new(),
            errors: Vec::new(),
        };
        for p in plan.proposals {
            self.pending.insert(p.id, p.clone());
            match self.apply_guarded(p.id, true, &current) {
                Ok(operation) => result.operations.push(operation),
                Err(error) => {
                    result.errors.push(format!("{} {}", p.source, error));
                    break;
                }
            }
        }
        Ok(result)
    }
}

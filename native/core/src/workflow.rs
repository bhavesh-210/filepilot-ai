//! One time setup and conservative maintenance planning
use super::*;
use rusqlite::OptionalExtension;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Preferences {
    pub reuse_existing: bool,
}
impl Default for Preferences {
    fn default() -> Self {
        Self {
            reuse_existing: true,
        }
    }
}
#[derive(Clone, Debug, Serialize)]
pub struct SetupPreview {
    pub token: u64,
    pub inventory: Inventory,
    pub create_folders: Vec<String>,
    revision: u64,
    authorization_epoch: u64,
}
pub(crate) fn inbox(relative: &str) -> bool {
    matches!(
        relative.to_lowercase().as_str(),
        "downloads" | "desktop" | "documents" | "inbox"
    )
}
pub(crate) fn destinations(directories: &[String], category: &str) -> Vec<String> {
    let aliases: &[&str] = match category {
        "Education" => &["education", "college", "school", "study", "university"],
        "Development" => &["development", "code", "programming"],
        "Professional" => &["professional", "work", "business"],
        "Finance" => &["finance", "finances", "receipts", "invoices"],
        "Media" => &["media", "photos", "pictures", "videos"],
        "Software" => &["software", "installers"],
        "Personal" => &["personal"],
        _ => &[],
    };
    // Exact category names win over aliases and all candidates are inside the grant
    let matches = directories
        .iter()
        .filter(|d| aliases.contains(&d.rsplit('/').next().unwrap_or("").to_lowercase().as_str()))
        .cloned()
        .collect::<Vec<_>>();
    let exact = matches
        .iter()
        .filter(|d| {
            d.rsplit('/')
                .next()
                .unwrap_or("")
                .eq_ignore_ascii_case(category)
        })
        .cloned()
        .collect::<Vec<_>>();
    if exact.is_empty() {
        matches
    } else {
        exact
    }
}
fn signature(i: &Inventory) -> Result<String> {
    let entries = i
        .proposals
        .iter()
        .map(|p| {
            (
                &p.source,
                &p.destination,
                &p.snapshot,
                &p.category,
                p.blocked,
                p.certain,
            )
        })
        .collect::<Vec<_>>();
    serde_json::to_string(&(i.files, i.protected, &i.folders, &i.errors, entries)).map_err(message)
}
impl Engine {
    pub fn preferences(&self) -> Result<Preferences> {
        self.db
            .query_row("SELECT value FROM settings WHERE id=2", [], |r| {
                r.get::<_, String>(0)
            })
            .optional()
            .map_err(message)?
            .map(|s| serde_json::from_str(&s).map_err(message))
            .transpose()
            .map(|p| p.unwrap_or_default())
    }
    pub fn set_preferences(&mut self, preferences: Preferences) -> Result<()> {
        self.db
            .execute(
                "INSERT OR REPLACE INTO settings VALUES(2,?)",
                [serde_json::to_string(&preferences).map_err(message)?],
            )
            .map_err(message)?;
        self.set_settings(self.settings.clone())?;
        self.setup_previews.clear();
        Ok(())
    }
    /// Snapshot totals come from the last scan while actionable rows always come from live pending state
    pub fn inventories(&self) -> Vec<Inventory> {
        let mut results = self
            .inventories
            .values()
            .filter(|i| self.roots.contains_key(&i.root_id))
            .map(|i| {
                let mut current = i.clone();
                current.proposals = self
                    .pending
                    .values()
                    .filter(|p| p.root_id == i.root_id)
                    .cloned()
                    .collect();
                current.proposals.sort_by(|a, b| a.source.cmp(&b.source));
                current
            })
            .collect::<Vec<_>>();
        results.sort_by_key(|i| i.root_id);
        results
    }
    fn missing_categories(&self, inventory: &Inventory) -> Result<Vec<String>> {
        if !inventory.errors.is_empty() {
            return Err("Incomplete scan cannot prepare folders".into());
        }
        let root = self.opened_root(inventory.root_id)?;
        let mut categories = Vec::new();
        for p in &inventory.proposals {
            // Only known stable category proposals may create a missing canonical folder
            if p.certain
                && p.reason == "Create or choose an authorized category folder"
                && CATEGORIES.contains(&p.category.as_str())
            {
                if let Some(destination) = &p.destination {
                    if destination.split('/').count() == 2
                        && destination.starts_with(&format!("{}/", p.category))
                        && !fs::exists(&root, &p.category).map_err(message)?
                    {
                        categories.push(p.category.clone());
                    }
                }
            }
        }
        categories.sort();
        categories.dedup();
        Ok(categories)
    }
    pub fn prepare_setup(&mut self, id: i64) -> Result<SetupPreview> {
        let inventory = self.scan_background(id)?;
        let create_folders = self.missing_categories(&inventory)?;
        let token = self.next_id;
        self.next_id = self
            .next_id
            .checked_add(1)
            .ok_or("Preview identifiers exhausted")?;
        let preview = SetupPreview {
            token,
            inventory,
            create_folders,
            revision: self.settings.revision,
            authorization_epoch: self.authorization_epoch,
        };
        self.setup_previews.insert(id, preview.clone());
        Ok(preview)
    }
    pub fn finish_setup(&mut self, id: i64, token: u64) -> Result<()> {
        self.finish_setup_guarded(id, token, || true)
    }
    pub fn finish_setup_guarded<F: Fn() -> bool>(
        &mut self,
        id: i64,
        token: u64,
        permission_current: F,
    ) -> Result<()> {
        let preview = self
            .setup_previews
            .get(&id)
            .ok_or("Review the first cleanup preview")?
            .clone();
        if preview.token != token
            || preview.revision != self.settings.revision
            || preview.authorization_epoch != self.authorization_epoch
        {
            return Err("Setup changed review a fresh preview".into());
        }
        let fresh = self.scan_background(id)?;
        if signature(&preview.inventory)? != signature(&fresh)? {
            return Err("Files changed review a fresh preview".into());
        }
        // Directory creation is authorized by the visible first cleanup plan
        for folder in &preview.create_folders {
            if !permission_current() {
                return Err("Permissions changed during setup".into());
            }
            self.create_category(id, folder)?;
        }
        self.scan_background(id)?;
        if !permission_current() {
            return Err("Permissions changed during setup".into());
        }
        self.mark_initial_reviewed(id)?;
        self.setup_previews.remove(&id);
        Ok(())
    }
    pub fn maintain(&mut self, id: i64) -> Result<Inventory> {
        self.maintain_guarded(id, || true)
    }
    pub fn maintain_guarded<F: Fn() -> bool>(
        &mut self,
        id: i64,
        permission_current: F,
    ) -> Result<Inventory> {
        let inventory = self.scan_background(id)?;
        if self.settings.mode != Mode::Full
            || !self.roots.get(&id).is_some_and(|r| r.initial_reviewed)
            || !self.settings.monitoring
        {
            return Ok(inventory);
        }
        let needed = self.missing_categories(&inventory)?;
        for category in &needed {
            if !permission_current() {
                return Err("Permissions changed during maintenance".into());
            }
            self.create_category(id, category)?;
        }
        if needed.is_empty() {
            Ok(inventory)
        } else {
            self.scan_background(id)
        }
    }
}

//! Several authorized locations with explicit routing and same-volume journaled transfers
use super::*;
use std::os::unix::fs::MetadataExt;
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Route {
    pub category: String,
    pub target_root_id: i64,
    pub folder: String,
    pub content: bool,
}
#[derive(Clone, Debug, Serialize)]
pub struct TransferPlan {
    pub token: u64,
    pub source_root_id: i64,
    pub target_root_id: i64,
    pub source: String,
    pub destination: String,
    pub category: String,
    pub reason: String,
    pub automatic: bool,
    pub(crate) snapshot: Identity,
    revision: u64,
    authorization_epoch: u64,
}
#[derive(Serialize)]
pub struct DirectoryStats {
    pub root_id: i64,
    pub directory: String,
    pub files: usize,
    pub folders: usize,
    pub pending: usize,
    pub protected: usize,
    pub received: i64,
    pub sent: i64,
    pub applied: i64,
    pub complete: bool,
}
fn directory(path: &str) -> String {
    let name = Path::new(path)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_lowercase();
    match name.as_str() {
        "desktop" => "Desktop",
        "downloads" => "Downloads",
        "documents" => "Documents",
        "pictures" | "photos" => "Pictures",
        "music" => "Music",
        "movies" | "videos" => "Movies",
        _ => "Other locations",
    }
    .into()
}
fn join(folder: &str, name: &str) -> String {
    if folder.is_empty() {
        name.into()
    } else {
        format!("{folder}/{name}")
    }
}
fn content_category(text: &str) -> Option<String> {
    let categories = super::summaries::categories(text);
    if categories.len() != 1 {
        return None;
    }
    let words = text
        .split(|c: char| !c.is_alphabetic())
        .map(str::to_lowercase)
        .collect::<HashSet<_>>();
    let terms: &[&str] = match categories[0].as_str() {
        "Finance" => &[
            "invoice",
            "receipt",
            "payment",
            "bank",
            "amount",
            "tax",
            "transaction",
            "salary",
        ],
        "Education" => &[
            "university",
            "course",
            "lecture",
            "assignment",
            "exam",
            "homework",
            "lesson",
            "students",
        ],
        "Professional" => &[
            "client",
            "business",
            "meeting",
            "contract",
            "proposal",
            "supplier",
            "employment",
            "resume",
        ],
        "Personal" => &["personal", "family", "travel", "holiday", "recipe", "diary"],
        "Development" => &[
            "function",
            "compiler",
            "programming",
            "javascript",
            "python",
            "rust",
        ],
        _ => &[],
    };
    (terms.iter().filter(|w| words.contains(**w)).count() >= 3).then(|| categories[0].clone())
}
impl Engine {
    pub fn routes(&self) -> Result<Vec<Route>> {
        let mut query = self
            .db
            .prepare("SELECT category,target_root_id,folder,content FROM routes ORDER BY category")
            .map_err(message)?;
        let values = query
            .query_map([], |r| {
                Ok(Route {
                    category: r.get(0)?,
                    target_root_id: r.get(1)?,
                    folder: r.get(2)?,
                    content: r.get(3)?,
                })
            })
            .map_err(message)?;
        values
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(message)
    }
    pub fn set_route<F: Fn() -> bool>(
        &mut self,
        route: Route,
        create: bool,
        current: F,
    ) -> Result<()> {
        if !CATEGORIES.contains(&route.category.as_str()) {
            return Err("Unknown category".into());
        }
        let root = self
            .roots
            .get(&route.target_root_id)
            .ok_or("Choose and authorize the destination location")?;
        if !root.initial_reviewed {
            return Err("Complete destination setup first".into());
        }
        if route.folder.is_empty() {
            self.opened_root(route.target_root_id)?;
        } else if create {
            if fs::parts(&route.folder).map_err(message)?.len() != 1 {
                return Err("Only a single new category folder can be created".into());
            }
            if !current() {
                return Err("Permission changed".into());
            }
            self.create_named_folder(route.target_root_id, &route.folder)?;
        } else {
            self.parent(route.target_root_id, &join(&route.folder, "validation"))
                .map_err(message)?;
        }
        if !current() {
            return Err("Permission changed".into());
        }
        let mut settings = self.settings.clone();
        settings.revision = settings
            .revision
            .checked_add(1)
            .ok_or("Revision exhausted")?;
        let transaction = self.db.unchecked_transaction().map_err(message)?;
        transaction
            .execute(
                "INSERT OR REPLACE INTO routes VALUES(?,?,?,?)",
                params![
                    route.category,
                    route.target_root_id,
                    route.folder,
                    route.content
                ],
            )
            .map_err(message)?;
        transaction
            .execute(
                "INSERT OR REPLACE INTO settings VALUES(1,?)",
                [serde_json::to_string(&settings).map_err(message)?],
            )
            .map_err(message)?;
        transaction.commit().map_err(message)?;
        self.settings = settings;
        self.pending.clear();
        self.transfers.clear();
        Ok(())
    }
    pub fn clear_routes(&mut self) -> Result<()> {
        let mut settings = self.settings.clone();
        settings.revision = settings
            .revision
            .checked_add(1)
            .ok_or("Revision exhausted")?;
        let transaction = self.db.unchecked_transaction().map_err(message)?;
        transaction
            .execute("DELETE FROM routes", [])
            .map_err(message)?;
        transaction
            .execute(
                "INSERT OR REPLACE INTO settings VALUES(1,?)",
                [serde_json::to_string(&settings).map_err(message)?],
            )
            .map_err(message)?;
        transaction.commit().map_err(message)?;
        self.settings = settings;
        self.pending.clear();
        self.transfers.clear();
        Ok(())
    }
    pub fn directory_stats(&self) -> Result<Vec<DirectoryStats>> {
        let inventories = self.inventories();
        let mut stats = Vec::new();
        for root in self.roots.values() {
            let inv = inventories.iter().find(|i| i.root_id == root.id);
            let count = |sql: &str| {
                self.db
                    .query_row(sql, [root.id], |r| r.get::<_, i64>(0))
                    .map_err(message)
            };
            stats.push(DirectoryStats{root_id:root.id,directory:directory(&root.path),files:inv.map_or(0,|i|i.files),folders:inv.map_or(0,|i|i.folders.len()),pending:inv.map_or(0,|i|i.proposals.len()),protected:inv.map_or(0,|i|i.protected),received:count("SELECT count(*) FROM operations WHERE destination_root_id=? AND root_id!=?1 AND state='applied'")?,sent:count("SELECT count(*) FROM operations WHERE root_id=? AND destination_root_id!=?1 AND kind IN ('transfer','folder_transfer') AND state='applied'")?,applied:count("SELECT count(*) FROM operations WHERE root_id=? AND kind IN ('move','transfer','folder_transfer','trash') AND state='applied'")?,complete:inv.is_some_and(|i|i.errors.is_empty())});
        }
        stats.sort_by(|a, b| {
            a.directory
                .cmp(&b.directory)
                .then_with(|| a.root_id.cmp(&b.root_id))
        });
        Ok(stats)
    }
    pub fn prepare_transfer(
        &mut self,
        source_root_id: i64,
        source: String,
        target_root_id: i64,
        folder: String,
    ) -> Result<TransferPlan> {
        let from = self
            .roots
            .get(&source_root_id)
            .ok_or("Source access is needed")?;
        let to = self
            .roots
            .get(&target_root_id)
            .ok_or("Destination access is needed")?;
        if !from.initial_reviewed || !to.initial_reviewed {
            return Err("Review the first cleanup for both locations first".into());
        }
        let (parent, name) = self.parent(source_root_id, &source).map_err(message)?;
        let snapshot = fs::identity(&parent, &name).map_err(message)?;
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(message)?
            .as_secs() as i64;
        if now.saturating_sub(snapshot.mtime) < 10 || snapshot.size > 64 * 1024 * 1024 {
            return Err("File stability or size limit".into());
        }
        let destination = join(&folder, &name);
        if source_root_id == target_root_id && source == destination {
            return Err("The file is already in that folder".into());
        }
        let (target, basename) = self.parent(target_root_id, &destination).map_err(message)?;
        if fs::exists(&target, &basename).map_err(message)? {
            return Err("Destination name already exists".into());
        }
        if parent.metadata().map_err(message)?.dev() != target.metadata().map_err(message)?.dev() {
            return Err("Cross volume transfers are not supported yet".into());
        }
        let token = self.next_id;
        self.next_id = self.next_id.checked_add(1).ok_or("Identifiers exhausted")?;
        let plan = TransferPlan {
            token,
            source_root_id,
            target_root_id,
            source,
            destination,
            category: classify(&name),
            reason: "Explicit move between approved locations".into(),
            automatic: false,
            snapshot,
            revision: self.settings.revision,
            authorization_epoch: self.authorization_epoch,
        };
        self.transfers
            .retain(|_, p| p.source_root_id != source_root_id || p.source != plan.source);
        self.transfers.insert(token, plan.clone());
        Ok(plan)
    }
    pub fn transfer_plans(&self) -> Vec<TransferPlan> {
        let mut plans = self
            .transfers
            .values()
            .filter(|p| {
                p.revision == self.settings.revision
                    && p.authorization_epoch == self.authorization_epoch
                    && self.roots.contains_key(&p.source_root_id)
                    && self.roots.contains_key(&p.target_root_id)
            })
            .cloned()
            .collect::<Vec<_>>();
        plans.sort_by(|a, b| a.source.cmp(&b.source));
        plans
    }
    pub fn route_for_proposal(&self, p: &Proposal) -> Result<Option<Route>> {
        let category = classify(
            Path::new(&p.source)
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or(""),
        );
        Ok(self.routes()?.into_iter().find(|r| r.category == category))
    }
    pub fn plan_routes(&mut self) -> Result<Vec<TransferPlan>> {
        let routes = self.routes()?;
        if routes.is_empty() {
            return Ok(Vec::new());
        }
        let mut proposals = self
            .inventories()
            .into_iter()
            .filter(|i| i.errors.is_empty())
            .flat_map(|i| i.proposals)
            .collect::<Vec<_>>();
        if !proposals.is_empty() {
            let length = proposals.len();
            proposals.rotate_left(self.route_cursor % length);
            self.route_cursor = (self.route_cursor + 64) % length;
        }
        let mut content_reads = 0usize;
        let mut results = Vec::new();
        let mut bytes = 0u64;
        for p in proposals {
            if results.len() >= 24 {
                break;
            }
            if self.folder_is_held(p.root_id, &p.source)
                || self.cleanup_is_held(p.root_id)
                || self.uses_cleanup_profile(p.root_id)
            {
                continue;
            }
            if p.manual_destination
                || !self
                    .roots
                    .get(&p.root_id)
                    .is_some_and(|r| r.initial_reviewed)
            {
                continue;
            }
            let name = Path::new(&p.source)
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("");
            let mut category = classify(name);
            let mut reason = "Approved filename category rule";
            if category == "Unknown" && routes.iter().any(|r| r.content) {
                let ext = Path::new(name)
                    .extension()
                    .and_then(|s| s.to_str())
                    .unwrap_or("");
                if !["txt", "md", "csv", "log"].contains(&ext)
                    || p.snapshot.size > 8000
                    || content_reads >= 64
                {
                    continue;
                }
                let Ok((parent, basename)) = self.parent(p.root_id, &p.source) else {
                    continue;
                };
                content_reads += 1;
                let Ok(text) = fs::text_sample(&parent, &basename, &p.snapshot) else {
                    continue;
                };
                if text.len() as u64 != p.snapshot.size || text.chars().count() >= 2000 {
                    continue;
                }
                let Some(inferred) = content_category(&text) else {
                    continue;
                };
                category = inferred;
                reason="Approved complete short text rule with three topic terms and no competing category";
            }
            let Some(route) = routes.iter().find(|r| {
                r.category == category && (reason == "Approved filename category rule" || r.content)
            }) else {
                continue;
            };
            if bytes + p.snapshot.size > 256 * 1024 * 1024 {
                continue;
            }
            let parent = p.source.rsplit_once('/').map(|x| x.0).unwrap_or("");
            if p.root_id == route.target_root_id
                && (route.folder.is_empty()
                    || parent == route.folder
                    || parent.starts_with(&format!("{}/", route.folder)))
            {
                self.db
                    .execute(
                        "INSERT OR REPLACE INTO kept VALUES(?,?,?)",
                        params![
                            p.root_id,
                            p.source,
                            serde_json::to_string(&p.snapshot).map_err(message)?
                        ],
                    )
                    .map_err(message)?;
                self.pending.remove(&p.id);
                continue;
            }
            let existing = self.transfer_plans().into_iter().find(|old| {
                old.source_root_id == p.root_id
                    && old.source == p.source
                    && old.target_root_id == route.target_root_id
                    && old.destination == join(&route.folder, name)
                    && old.snapshot == p.snapshot
                    && old.automatic
            });
            let mut plan = if let Some(old) = existing {
                old
            } else {
                let Ok(plan) = self.prepare_transfer(
                    p.root_id,
                    p.source.clone(),
                    route.target_root_id,
                    route.folder.clone(),
                ) else {
                    continue;
                };
                plan
            };
            if plan.snapshot != p.snapshot {
                self.transfers.remove(&plan.token);
                continue;
            }
            plan.automatic = true;
            plan.category = category;
            plan.reason = reason.into();
            self.transfers.insert(plan.token, plan.clone());
            bytes += p.snapshot.size;
            results.push(plan);
        }
        Ok(results)
    }
    pub fn pdf_route_candidates(&self) -> Result<Vec<u64>> {
        if !self.routes()?.iter().any(|r| r.content) {
            return Ok(Vec::new());
        }
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(message)?
            .as_secs() as i64;
        let mut proposals = self
            .inventories()
            .into_iter()
            .filter(|i| i.errors.is_empty())
            .flat_map(|i| i.proposals)
            .filter(|p| {
                !p.manual_destination
                    && p.category == "Unknown"
                    && p.source.to_lowercase().ends_with(".pdf")
                    && p.snapshot.size <= 2097152
                    && now.saturating_sub(p.snapshot.mtime) >= 10
                    && !(self.folder_is_held(p.root_id, &p.source)
                        || self.cleanup_is_held(p.root_id))
                    && self
                        .roots
                        .get(&p.root_id)
                        .is_some_and(|r| r.initial_reviewed)
                    && !self
                        .pdf_route_cache
                        .get(&(p.root_id, p.source.clone()))
                        .is_some_and(|(identity, revision)| {
                            identity == &p.snapshot && *revision == self.settings.revision
                        })
            })
            .collect::<Vec<_>>();
        proposals.sort_by(|a, b| a.source.cmp(&b.source));
        Ok(proposals.into_iter().take(2).map(|p| p.id).collect())
    }
    pub fn plan_pdf_route(
        &mut self,
        id: u64,
        parsed: &serde_json::Value,
    ) -> Result<Option<TransferPlan>> {
        let p = self.pending.get(&id).ok_or("PDF preview expired")?.clone();
        let (parent, name) = self.parent(p.root_id, &p.source).map_err(message)?;
        if fs::identity(&parent, &name).map_err(message)? != p.snapshot {
            return Err("PDF changed after inspection".into());
        }
        if !p.source.to_lowercase().ends_with(".pdf")
            || p.manual_destination
            || (self.folder_is_held(p.root_id, &p.source) || self.cleanup_is_held(p.root_id))
        {
            return Ok(None);
        }
        self.pdf_route_cache.insert(
            (p.root_id, p.source.clone()),
            (p.snapshot.clone(), self.settings.revision),
        );
        if parsed["status"] != "ok"
            || parsed["truncated"].as_bool() != Some(false)
            || parsed["pages_inspected"]
                .as_u64()
                .is_none_or(|n| n == 0 || n > 5)
        {
            return Ok(None);
        }
        let text = parsed["text"].as_str().unwrap_or("");
        if text.chars().count() > 2000 {
            return Ok(None);
        }
        let Some(category) = content_category(text) else {
            return Ok(None);
        };
        let Some(route) = self
            .routes()?
            .into_iter()
            .find(|r| r.category == category && r.content)
        else {
            return Ok(None);
        };
        let current = p.source.rsplit_once('/').map(|x| x.0).unwrap_or("");
        if p.root_id == route.target_root_id
            && (route.folder.is_empty()
                || current == route.folder
                || current.starts_with(&format!("{}/", route.folder)))
        {
            self.db
                .execute(
                    "INSERT OR REPLACE INTO kept VALUES(?,?,?)",
                    params![
                        p.root_id,
                        p.source,
                        serde_json::to_string(&p.snapshot).map_err(message)?
                    ],
                )
                .map_err(message)?;
            self.pending.remove(&id);
            return Ok(None);
        }
        let mut plan = self.prepare_transfer(
            p.root_id,
            p.source.clone(),
            route.target_root_id,
            route.folder,
        )?;
        if plan.snapshot != p.snapshot {
            self.transfers.remove(&plan.token);
            return Err("PDF changed before routing preview".into());
        }
        plan.category = category;
        plan.automatic = true;
        plan.reason = "Approved complete bounded PDF keyword rule".into();
        self.transfers.insert(plan.token, plan.clone());
        Ok(Some(plan))
    }
    pub fn apply_transfer<F: Fn() -> bool>(
        &mut self,
        token: u64,
        approved: bool,
        current: F,
    ) -> Result<i64> {
        let p = self
            .transfers
            .get(&token)
            .ok_or("Transfer preview expired")?
            .clone();
        if p.revision != self.settings.revision
            || p.authorization_epoch != self.authorization_epoch
            || !current()
        {
            return Err("Permissions changed preview again".into());
        }
        for root_id in [p.source_root_id, p.target_root_id] {
            let root = self
                .roots
                .get(&root_id)
                .ok_or("Both location permissions are required")?;
            if !root.initial_reviewed {
                return Err("Complete both location previews first".into());
            }
        }
        if !approved {
            if self.folder_is_held(p.source_root_id, &p.source)
                || self.cleanup_is_held(p.source_root_id)
                || self.uses_cleanup_profile(p.source_root_id)
            {
                return Err("A folder transfer preview is holding this collection".into());
            }
            if !self.settings.monitoring || self.settings.mode == Mode::Smart || !p.automatic {
                return Err("Explicit transfer approval required".into());
            }
            let folder = p.destination.rsplit_once('/').map(|x| x.0).unwrap_or("");
            if !self.routes()?.iter().any(|r| {
                r.category == p.category
                    && r.target_root_id == p.target_root_id
                    && r.folder == folder
            }) {
                return Err("Routing permission expired".into());
            }
        }
        let unresolved:i64=self.db.query_row("SELECT count(*) FROM operations WHERE (root_id IN (?1,?2) OR destination_root_id IN (?1,?2)) AND state IN ('pending','undo_pending','needs_review')",params![p.source_root_id,p.target_root_id],|r|r.get(0)).map_err(message)?;
        if unresolved > 0 {
            return Err("Reconcile interrupted changes in both locations first".into());
        }
        let (from, source) = self.parent(p.source_root_id, &p.source).map_err(message)?;
        let (to, destination) = self
            .parent(p.target_root_id, &p.destination)
            .map_err(message)?;
        let snapshot = fs::identity(&from, &source).map_err(message)?;
        if snapshot != p.snapshot {
            return Err("Source changed after preview".into());
        }
        if fs::exists(&to, &destination).map_err(message)? {
            return Err("Destination collision".into());
        }
        if from.metadata().map_err(message)?.dev() != to.metadata().map_err(message)?.dev() {
            return Err("Cross volume transfers are not supported".into());
        }
        let digest = fs::digest(&from, &source, &snapshot).map_err(message)?;
        self.db.execute("INSERT INTO operations(root_id,source,destination,identity,state,digest,kind,destination_root_id) VALUES(?,?,?,?,'pending',?,'transfer',?)",params![p.source_root_id,p.source,p.destination,serde_json::to_string(&snapshot).map_err(message)?,digest,p.target_root_id]).map_err(message)?;
        let operation = self.db.last_insert_rowid();
        if let Some(run) = self.active_cleanup_run {
            self.db
                .execute(
                    "INSERT INTO cleanup_operations VALUES(?,?)",
                    params![run, operation],
                )
                .map_err(message)?;
        }
        if !current() {
            self.db
                .execute(
                    "UPDATE operations SET state='cancelled' WHERE id=?",
                    [operation],
                )
                .map_err(message)?;
            return Err("Permissions changed before transfer".into());
        }
        fs::move_exclusive(&from, &source, &to, &destination).map_err(message)?;
        let after = fs::identity(&to, &destination).map_err(message)?;
        if !fs::same_file(&after, &snapshot)
            || fs::digest(&to, &destination, &after).map_err(message)? != digest
        {
            return Err("Transfer verification needs reconciliation".into());
        }
        let transaction = self.db.unchecked_transaction().map_err(message)?;
        transaction
            .execute(
                "UPDATE operations SET state='applied',identity=? WHERE id=?",
                params![serde_json::to_string(&after).map_err(message)?, operation],
            )
            .map_err(message)?;
        transaction
            .execute(
                "INSERT OR REPLACE INTO kept VALUES(?,?,?)",
                params![
                    p.target_root_id,
                    p.destination,
                    serde_json::to_string(&after).map_err(message)?
                ],
            )
            .map_err(message)?;
        transaction.commit().map_err(message)?;
        self.pending
            .retain(|_, old| old.root_id != p.source_root_id || old.source != p.source);
        self.transfers.remove(&token);
        Ok(operation)
    }
    pub(crate) fn undo_transfer<F: Fn() -> bool>(&mut self, id: i64, current: F) -> Result<()> {
        let(src,dst,source,destination,raw,digest,state):(i64,i64,String,String,String,String,String)=self.db.query_row("SELECT root_id,destination_root_id,source,destination,identity,digest,state FROM operations WHERE id=?",[id],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?,r.get(5)?,r.get(6)?))).map_err(message)?;
        if state != "applied" {
            return Err("Transfer is not safely undoable".into());
        }
        let (from, name) = self.parent(dst, &destination).map_err(message)?;
        let (to, original) = self.parent(src, &source).map_err(message)?;
        let expected: Identity = serde_json::from_str(&raw).map_err(message)?;
        if fs::identity(&from, &name).map_err(message)? != expected
            || fs::digest(&from, &name, &expected).map_err(message)? != digest
        {
            return Err("Transferred file has changed".into());
        }
        if fs::exists(&to, &original).map_err(message)? {
            return Err("Original location is occupied".into());
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
        let restored = fs::identity(&to, &original).map_err(message)?;
        if fs::digest(&to, &original, &restored).map_err(message)? != digest {
            return Err("Undo verification needs reconciliation".into());
        }
        let transaction = self.db.unchecked_transaction().map_err(message)?;
        transaction
            .execute("UPDATE operations SET state='undone' WHERE id=?", [id])
            .map_err(message)?;
        transaction
            .execute(
                "INSERT OR REPLACE INTO kept VALUES(?,?,?)",
                params![
                    src,
                    source,
                    serde_json::to_string(&restored).map_err(message)?
                ],
            )
            .map_err(message)?;
        transaction.commit().map_err(message)?;
        self.pending.clear();
        Ok(())
    }
    pub(crate) fn recover_transfer(&mut self, op: &Operation) -> Result<()> {
        let Some(target) = op.destination_root_id else {
            return Err("Missing transfer destination authorization".into());
        };
        if !self.roots.contains_key(&op.root_id) || !self.roots.contains_key(&target) {
            return Ok(());
        }
        let (raw, digest): (String, String) = self
            .db
            .query_row(
                "SELECT identity,digest FROM operations WHERE id=?",
                [op.id],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .map_err(message)?;
        let expected: Identity = serde_json::from_str(&raw).map_err(message)?;
        let a = self.parent(op.root_id, &op.source);
        let b = self.parent(target, &op.destination);
        let mut state = "needs_review";
        if let (Ok((from, name)), Ok((to, destination))) = (a, b) {
            let source = fs::identity(&from, &name).ok();
            let dest = fs::identity(&to, &destination).ok();
            if let Some(identity) = source.filter(|identity| {
                fs::same_file(identity, &expected)
                    && fs::digest(&from, &name, identity).ok().as_deref() == Some(digest.as_str())
            }) {
                if !fs::exists(&to, &destination).unwrap_or(true) {
                    state = if op.state == "undo_pending" {
                        "undone"
                    } else {
                        "not_applied"
                    };
                    if state == "undone" {
                        self.db
                            .execute(
                                "INSERT OR REPLACE INTO kept VALUES(?,?,?)",
                                params![
                                    op.root_id,
                                    op.source,
                                    serde_json::to_string(&identity).map_err(message)?
                                ],
                            )
                            .map_err(message)?;
                    }
                }
            } else if let Some(identity) = dest.filter(|identity| {
                fs::same_file(identity, &expected)
                    && fs::digest(&to, &destination, identity).ok().as_deref()
                        == Some(digest.as_str())
            }) {
                if !fs::exists(&from, &name).unwrap_or(true) {
                    state = "applied";
                    self.db
                        .execute(
                            "UPDATE operations SET identity=? WHERE id=?",
                            params![serde_json::to_string(&identity).map_err(message)?, op.id],
                        )
                        .map_err(message)?;
                    self.db
                        .execute(
                            "INSERT OR REPLACE INTO kept VALUES(?,?,?)",
                            params![
                                target,
                                op.destination,
                                serde_json::to_string(&identity).map_err(message)?
                            ],
                        )
                        .map_err(message)?;
                }
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

#[cfg(test)]
mod faults {
    use super::*;
    fn setup() -> (tempfile::TempDir, Engine, i64, i64, TransferPlan) {
        let d = tempfile::tempdir().unwrap();
        let root = d.path().canonicalize().unwrap();
        for name in ["Downloads", "Documents"] {
            std::fs::create_dir(root.join(name)).unwrap();
        }
        let mut e = Engine::new(Connection::open_in_memory().unwrap()).unwrap();
        let a = e.authorize(root.join("Downloads"), "a".into()).unwrap();
        let b = e.authorize(root.join("Documents"), "b".into()).unwrap();
        e.mark_initial_reviewed(a).unwrap();
        e.mark_initial_reviewed(b).unwrap();
        std::fs::write(
            root.join("Downloads/receipt.txt"),
            "invoice payment bank amount",
        )
        .unwrap();
        std::fs::File::options()
            .write(true)
            .open(root.join("Downloads/receipt.txt"))
            .unwrap()
            .set_times(
                std::fs::FileTimes::new()
                    .set_modified(SystemTime::now() - std::time::Duration::from_secs(60)),
            )
            .unwrap();
        let plan = e
            .prepare_transfer(a, "receipt.txt".into(), b, "".into())
            .unwrap();
        (d, e, a, b, plan)
    }
    #[test]
    fn journal_intent_failure_never_moves_source() {
        let (d, mut e, _, _, p) = setup();
        e.db.execute_batch("CREATE TRIGGER fail_transfer BEFORE INSERT ON operations WHEN NEW.kind='transfer' BEGIN SELECT RAISE(FAIL,'synthetic failure'); END;").unwrap();
        assert!(e.apply_transfer(p.token, true, || true).is_err());
        assert!(d.path().join("Downloads/receipt.txt").exists());
        assert!(!d.path().join("Documents/receipt.txt").exists());
    }
    #[test]
    fn commit_failure_recovers_verified_transfer() {
        let (d, mut e, _, _, p) = setup();
        e.db.execute_batch("CREATE TRIGGER fail_transfer BEFORE UPDATE OF state ON operations WHEN NEW.state='applied' AND NEW.kind='transfer' BEGIN SELECT RAISE(FAIL,'synthetic failure'); END;").unwrap();
        assert!(e.apply_transfer(p.token, true, || true).is_err());
        assert!(d.path().join("Documents/receipt.txt").exists());
        assert_eq!(e.history().unwrap()[0].state, "pending");
        e.db.execute_batch("DROP TRIGGER fail_transfer").unwrap();
        e.recover().unwrap();
        let op = e.history().unwrap()[0].id;
        assert_eq!(e.history().unwrap()[0].state, "applied");
        e.undo(op).unwrap();
        assert!(d.path().join("Downloads/receipt.txt").exists());
    }
    #[test]
    fn interrupted_undo_recovers_and_stays_put() {
        let (d, mut e, a, _, p) = setup();
        let op = e.apply_transfer(p.token, true, || true).unwrap();
        e.db.execute_batch("CREATE TRIGGER fail_undo BEFORE UPDATE OF state ON operations WHEN NEW.state='undone' BEGIN SELECT RAISE(FAIL,'synthetic failure'); END;").unwrap();
        assert!(e.undo(op).is_err());
        assert!(d.path().join("Downloads/receipt.txt").exists());
        assert_eq!(e.history().unwrap()[0].state, "undo_pending");
        e.db.execute_batch("DROP TRIGGER fail_undo").unwrap();
        e.recover().unwrap();
        assert_eq!(e.history().unwrap()[0].state, "undone");
        assert!(e.scan(a).unwrap().proposals.is_empty());
    }
    #[test]
    fn ambiguous_recovery_preserves_both_files() {
        let (d, mut e, _, _, p) = setup();
        let op = e.apply_transfer(p.token, true, || true).unwrap();
        std::fs::write(
            d.path().join("Downloads/receipt.txt"),
            "occupied by another file",
        )
        .unwrap();
        e.db.execute("UPDATE operations SET state='pending' WHERE id=?", [op])
            .unwrap();
        e.recover().unwrap();
        assert_eq!(e.history().unwrap()[0].state, "needs_review");
        assert_eq!(
            std::fs::read_to_string(d.path().join("Downloads/receipt.txt")).unwrap(),
            "occupied by another file"
        );
        assert!(d.path().join("Documents/receipt.txt").exists());
    }
    #[test]
    fn route_policy_and_saved_rule_commit_together() {
        let (_, mut e, _, b, _) = setup();
        e.db.execute_batch("CREATE TRIGGER fail_revision BEFORE INSERT ON settings WHEN NEW.id=1 BEGIN SELECT RAISE(FAIL,'synthetic failure'); END;").unwrap();
        assert!(e
            .set_route(
                Route {
                    category: "Finance".into(),
                    target_root_id: b,
                    folder: "".into(),
                    content: true
                },
                false,
                || true
            )
            .is_err());
        assert!(e.routes().unwrap().is_empty());
        assert_eq!(e.settings.revision, 0);
    }
}

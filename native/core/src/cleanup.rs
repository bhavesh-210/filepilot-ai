//! Structured cleanup plans built from safe scanner proposals and executed through the journal.
use super::*;
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CleanupProfile {
    pub root_ids: Vec<i64>,
    pub by_year: bool,
    pub date_prefix: bool,
}
#[derive(Clone, Debug, Serialize)]
pub struct CleanupRow {
    pub root_id: i64,
    pub source: String,
    pub destination: String,
    pub selected: bool,
    pub reason: String,
    snapshot: Identity,
}
#[derive(Clone, Debug, Serialize)]
pub struct CleanupPlan {
    pub token: u64,
    pub rows: Vec<CleanupRow>,
    pub folders: Vec<(i64, String)>,
    pub protected: usize,
    pub unchanged: usize,
    profile: CleanupProfile,
    revision: u64,
    epoch: u64,
}
#[derive(Serialize)]
pub struct CleanupResult {
    pub run_id: i64,
    pub applied: usize,
    pub error: Option<String>,
}
#[derive(Serialize)]
pub struct CleanupRun {
    pub id: i64,
    pub label: String,
    pub operations: i64,
    pub remaining: i64,
}
fn date(seconds: i64) -> Result<String> {
    let time = seconds as libc::time_t;
    let mut tm = unsafe { std::mem::zeroed::<libc::tm>() };
    if unsafe { libc::gmtime_r(&time, &mut tm) }.is_null() {
        return Err("Invalid modification date".into());
    }
    Ok(format!(
        "{:04}-{:02}-{:02}",
        tm.tm_year + 1900,
        tm.tm_mon + 1,
        tm.tm_mday
    ))
}
fn kind(source: &str) -> &'static str {
    let ext = Path::new(source)
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_lowercase();
    match ext.as_str() {
        "png" | "jpg" | "jpeg" | "heic" | "gif" | "webp" => "Images",
        "mp4" | "mov" | "mkv" => "Video",
        "mp3" | "wav" | "m4a" => "Audio",
        "pdf" => "PDFs",
        "txt" | "md" | "docx" | "odt" | "rtf" => "Documents",
        "csv" | "xlsx" => "Spreadsheets",
        "zip" | "tar" | "gz" | "7z" => "Archives",
        "dmg" | "pkg" => "Installers",
        _ => "Other",
    }
}
impl Engine {
    pub fn saved_cleanup_profile(&self) -> Result<Option<CleanupProfile>> {
        use rusqlite::OptionalExtension;
        let value: Option<String> = self
            .db
            .query_row("SELECT value FROM cleanup_profiles WHERE id=1", [], |r| {
                r.get(0)
            })
            .optional()
            .map_err(message)?;
        value
            .map(|s| serde_json::from_str(&s).map_err(message))
            .transpose()
    }
    pub fn save_cleanup_profile(&mut self, token: u64) -> Result<()> {
        let p = self
            .cleanup_plans
            .get(&token)
            .ok_or("Preview this layout before enabling automation")?;
        if p.revision != self.settings.revision || p.epoch != self.authorization_epoch {
            return Err("Preview expired".into());
        }
        let profile = p.profile.clone();
        self.db
            .execute(
                "INSERT OR REPLACE INTO cleanup_profiles VALUES(1,?)",
                [serde_json::to_string(&profile).map_err(message)?],
            )
            .map_err(message)?;
        Ok(())
    }
    pub fn clear_cleanup_profile(&mut self) -> Result<()> {
        self.db
            .execute("DELETE FROM cleanup_profiles", [])
            .map_err(message)?;
        Ok(())
    }
    pub fn uses_cleanup_profile(&self, id: i64) -> bool {
        self.saved_cleanup_profile()
            .ok()
            .flatten()
            .is_some_and(|p| p.root_ids.contains(&id))
    }
    pub fn maintain_cleanup_profile<F: Fn() -> bool>(
        &mut self,
        current: F,
    ) -> Result<Option<CleanupResult>> {
        if !self.settings.monitoring || self.settings.mode == Mode::Smart {
            return Ok(None);
        }
        let Some(profile) = self.saved_cleanup_profile()? else {
            return Ok(None);
        };
        if profile.root_ids.iter().any(|id| self.cleanup_is_held(*id)) {
            return Ok(None);
        }
        let plan = self.prepare_cleanup(profile)?;
        let choices = plan
            .rows
            .iter()
            .enumerate()
            .filter(|(_, r)| r.selected)
            .map(|(i, r)| (i, r.destination.clone()))
            .collect::<Vec<_>>();
        if choices.is_empty() {
            self.cancel_cleanup(plan.token);
            return Ok(None);
        }
        self.apply_cleanup(plan.token, choices, current).map(Some)
    }

    pub fn apply_cleanup_with_profile<F: Fn() -> bool>(
        &mut self,
        token: u64,
        choices: Vec<(usize, String)>,
        save: bool,
        current: F,
    ) -> Result<CleanupResult> {
        let profile = self
            .cleanup_plans
            .get(&token)
            .ok_or("Create a fresh preview")?
            .profile
            .clone();
        let result = self.apply_cleanup(token, choices, &current)?;
        if save && result.error.is_none() && current() {
            self.db
                .execute(
                    "INSERT OR REPLACE INTO cleanup_profiles VALUES(1,?)",
                    [serde_json::to_string(&profile).map_err(message)?],
                )
                .map_err(message)?;
        }
        Ok(result)
    }
    pub fn cancel_cleanup(&mut self, token: u64) {
        self.cleanup_plans.remove(&token);
    }
    pub fn cleanup_is_held(&self, id: i64) -> bool {
        self.cleanup_plans.values().any(|p| {
            p.revision == self.settings.revision
                && p.epoch == self.authorization_epoch
                && p.profile.root_ids.contains(&id)
        })
    }

    pub fn cleanup_runs(&self) -> Result<Vec<CleanupRun>> {
        let mut q=self.db.prepare("SELECT r.id,r.label,count(o.operation_id),sum(CASE WHEN p.state='applied' THEN 1 ELSE 0 END) FROM cleanup_runs r LEFT JOIN cleanup_operations o ON o.run_id=r.id LEFT JOIN operations p ON p.id=o.operation_id GROUP BY r.id ORDER BY r.id DESC").map_err(message)?;
        let rows = q
            .query_map([], |r| {
                Ok(CleanupRun {
                    id: r.get(0)?,
                    label: r.get(1)?,
                    operations: r.get(2)?,
                    remaining: r.get(3)?,
                })
            })
            .map_err(message)?
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(message)?;
        Ok(rows)
    }
    pub fn prepare_cleanup(&mut self, profile: CleanupProfile) -> Result<CleanupPlan> {
        if profile.root_ids.is_empty() || profile.root_ids.len() > 16 {
            return Err("Choose one to sixteen authorized locations".into());
        }
        let mut ids = profile.root_ids.clone();
        ids.sort();
        ids.dedup();
        if ids.len() != profile.root_ids.len() {
            return Err("Duplicate location".into());
        }
        let (mut rows, mut folders, mut protected, mut unchanged) = (Vec::new(), Vec::new(), 0, 0);
        for id in ids {
            if !self.roots.get(&id).is_some_and(|r| r.initial_reviewed) {
                return Err("Complete location setup first".into());
            }
            let inv = self.scan_background(id)?;
            if !inv.errors.is_empty() {
                return Err("Resolve incomplete scans before planning".into());
            }
            protected += inv.protected;
            unchanged += inv.files.saturating_sub(inv.proposals.len());
            for p in inv.proposals {
                if rows.len() >= 100 {
                    return Err("Use a smaller location selection this preview supports 100 candidate files".into());
                }
                if self.folder_is_held(id, &p.source) {
                    return Err("Finish the folder transfer preview first".into());
                }
                let date = date(p.snapshot.mtime)?;
                let folder = if profile.by_year {
                    format!("{}/{}", kind(&p.source), &date[..4])
                } else {
                    kind(&p.source).into()
                };
                let name = Path::new(&p.source)
                    .file_name()
                    .and_then(|s| s.to_str())
                    .ok_or("Invalid filename")?;
                let name = if profile.date_prefix && !name.starts_with(&format!("{date} - ")) {
                    format!("{date} - {name}")
                } else {
                    name.into()
                };
                let destination = format!("{folder}/{name}");
                if destination == p.source {
                    unchanged += 1;
                    continue;
                }
                let mut blocked = if p.manual_destination {
                    Some("A manually selected destination requires approval")
                } else if kind(&p.source) == "Other" {
                    Some("Unrecognized file kind keep in place unless selected")
                } else {
                    None
                };
                for prefix in [
                    folder.split('/').next().unwrap().to_string(),
                    folder.clone(),
                ] {
                    if !inv.folders.contains(&prefix) && !folders.contains(&(id, prefix.clone())) {
                        if let Ok((parent, name)) = self.parent(id, &prefix) {
                            if fs::exists(&parent, &name).map_err(message)? {
                                blocked = Some("A destination entry already exists")
                            }
                        }
                        folders.push((id, prefix));
                    }
                }
                if let Ok((parent, name)) = self.parent(id, &destination) {
                    if fs::exists(&parent, &name).map_err(message)? {
                        blocked = Some("Destination collision")
                    }
                }
                if rows
                    .iter()
                    .any(|r: &CleanupRow| r.root_id == id && r.destination == destination)
                {
                    blocked = Some("Two incoming files have the same destination")
                }
                rows.push(CleanupRow {
                    root_id: id,
                    source: p.source,
                    destination,
                    selected: blocked.is_none(),
                    reason: blocked
                        .unwrap_or("Group by file kind using modification date when requested")
                        .into(),
                    snapshot: p.snapshot,
                });
            }
        }
        self.cleanup_plans.clear();
        let token = self.next_id;
        self.next_id = self.next_id.checked_add(1).ok_or("Identifiers exhausted")?;
        let plan = CleanupPlan {
            token,
            rows,
            folders,
            protected,
            unchanged,
            profile,
            revision: self.settings.revision,
            epoch: self.authorization_epoch,
        };
        self.cleanup_plans.insert(token, plan.clone());
        Ok(plan)
    }
    fn cleanup_directory(&self, id: i64, path: &str) -> Result<()> {
        let (parent, name) = self.parent(id, path).map_err(message)?;
        if fs::exists(&parent, &name).map_err(message)? {
            fs::child_directory(&parent, &name).map_err(message)?;
            return Ok(());
        }
        self.db.execute("INSERT INTO operations(root_id,source,destination,identity,state,digest,kind) VALUES(?,'',?,'{}','directory_pending','','mkdir')",params![id,path]).map_err(message)?;
        let op = self.db.last_insert_rowid();
        fs::mkdir(&parent, &name).map_err(message)?;
        self.db
            .execute(
                "UPDATE operations SET state='created_directory' WHERE id=?",
                [op],
            )
            .map_err(message)?;
        Ok(())
    }
    pub fn apply_cleanup<F: Fn() -> bool>(
        &mut self,
        token: u64,
        choices: Vec<(usize, String)>,
        current: F,
    ) -> Result<CleanupResult> {
        let plan = self
            .cleanup_plans
            .remove(&token)
            .ok_or("Create a fresh cleanup preview")?;
        if plan.revision != self.settings.revision
            || plan.epoch != self.authorization_epoch
            || !current()
        {
            return Err("Cleanup permissions changed preview again".into());
        }
        if choices.is_empty() {
            return Err("Select at least one file".into());
        }
        let mut seen = HashSet::new();
        let mut destinations = HashSet::new();
        for (index, dest) in &choices {
            let row = plan.rows.get(*index).ok_or("Unknown preview row")?;
            if !seen.insert(index) || !destinations.insert((row.root_id, dest.clone())) {
                return Err("Duplicate selection or destination".into());
            }
            fs::parts(dest).map_err(message)?;
            if !dest.contains('/') {
                return Err("Choose a destination folder and filename".into());
            }
            if Path::new(dest).file_name() != Path::new(&row.source).file_name()
                && !self.settings.rename
            {
                return Err("Enable renaming before adding date prefixes".into());
            }
            let (parent, name) = self.parent(row.root_id, &row.source).map_err(message)?;
            if fs::identity(&parent, &name).map_err(message)? != row.snapshot {
                return Err("A source changed preview again".into());
            }
            // Edits may reuse existing directories or only directories already shown in the preview.
            let folder = dest.rsplit_once('/').unwrap().0;
            if !plan.folders.contains(&(row.root_id, folder.into())) {
                self.parent(row.root_id, &format!("{folder}/validation"))
                    .map_err(message)?;
            }
            if let Ok((p, n)) = self.parent(row.root_id, dest) {
                if fs::exists(&p, &n).map_err(message)? {
                    return Err("Destination collision preview again".into());
                }
            }
        }
        self.db
            .execute(
                "INSERT INTO cleanup_runs(label) VALUES(?)",
                [if plan.profile.by_year {
                    "File kind and year"
                } else {
                    "File kind"
                }],
            )
            .map_err(message)?;
        let run_id = self.db.last_insert_rowid();
        let mut applied = 0;
        let mut error = None;
        for (index, destination) in choices {
            let row = &plan.rows[index];
            let result = (|| {
                if !current() {
                    return Err("Cleanup paused because permissions changed".into());
                }
                let folder = destination.rsplit_once('/').unwrap().0;
                let mut prefix = String::new();
                for part in folder.split('/') {
                    if !prefix.is_empty() {
                        prefix.push('/')
                    }
                    prefix.push_str(part);
                    if !current() {
                        return Err("Cleanup cancelled".into());
                    }
                    self.cleanup_directory(row.root_id, &prefix)?;
                }
                let mut transfer = self.prepare_transfer(
                    row.root_id,
                    row.source.clone(),
                    row.root_id,
                    folder.into(),
                )?;
                if transfer.snapshot != row.snapshot {
                    self.transfers.remove(&transfer.token);
                    return Err("Source changed during cleanup".into());
                }
                transfer.destination = destination;
                self.transfers.insert(transfer.token, transfer.clone());
                self.active_cleanup_run = Some(run_id);
                let outcome = self.apply_transfer(transfer.token, true, &current);
                self.active_cleanup_run = None;
                outcome?;
                Ok(())
            })();
            match result {
                Ok(()) => applied += 1,
                Err(e) => {
                    error = Some(e);
                    break;
                }
            }
        }
        Ok(CleanupResult {
            run_id,
            applied,
            error,
        })
    }
    pub fn undo_cleanup<F: Fn() -> bool>(&mut self, id: i64, current: F) -> Result<CleanupResult> {
        let ops = {
            let mut q=self.db.prepare("SELECT p.id FROM operations p JOIN cleanup_operations c ON c.operation_id=p.id WHERE c.run_id=? AND p.state='applied' ORDER BY p.id DESC").map_err(message)?;
            let ids = q
                .query_map([id], |r| r.get::<_, i64>(0))
                .map_err(message)?
                .collect::<std::result::Result<Vec<_>, _>>()
                .map_err(message)?;
            ids
        };
        let mut applied = 0;
        let mut error = None;
        for op in ops {
            match self.undo_guarded(op, &current) {
                Ok(()) => applied += 1,
                Err(e) => {
                    error = Some(e);
                    break;
                }
            }
        }
        Ok(CleanupResult {
            run_id: id,
            applied,
            error,
        })
    }
}

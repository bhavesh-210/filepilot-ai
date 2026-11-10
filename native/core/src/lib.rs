//! Trusted native core with no network client and no arbitrary command execution
pub mod fs;
use fs::Identity;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    os::fd::AsRawFd,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};
pub type Result<T> = std::result::Result<T, String>;
fn message<E: std::fmt::Display>(e: E) -> String {
    e.to_string()
}
pub const CATEGORIES: [&str; 7] = [
    "Education",
    "Development",
    "Professional",
    "Finance",
    "Media",
    "Software",
    "Personal",
];
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Mode {
    Full,
    Smart,
    Limited,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Settings {
    pub mode: Mode,
    pub rename: bool,
    pub monitoring: bool,
    pub personalization: bool,
    pub revision: u64,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            mode: Mode::Full,
            rename: true,
            monitoring: true,
            personalization: true,
            revision: 0,
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Root {
    pub id: i64,
    pub path: String,
    pub bookmark: String,
    pub initial_reviewed: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Proposal {
    pub id: u64,
    pub root_id: i64,
    pub source: String,
    pub destination: Option<String>,
    pub category: String,
    pub reason: String,
    pub blocked: bool,
    pub revision: u64,
    pub rename: bool,
    pub certain: bool,
    pub snapshot: Identity,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Inventory {
    pub root_id: i64,
    pub files: usize,
    pub protected: usize,
    pub errors: Vec<String>,
    pub proposals: Vec<Proposal>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Operation {
    pub id: i64,
    pub root_id: i64,
    pub source: String,
    pub destination: String,
    pub state: String,
}
pub struct Engine {
    db: Connection,
    pub settings: Settings,
    pub roots: HashMap<i64, Root>,
    pending: HashMap<u64, Proposal>,
    next_id: u64,
}
impl Engine {
    pub fn new(db: Connection) -> Result<Self> {
        db.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=FULL; PRAGMA secure_delete=ON;
        CREATE TABLE IF NOT EXISTS roots(id INTEGER PRIMARY KEY AUTOINCREMENT,path TEXT NOT NULL UNIQUE,bookmark TEXT NOT NULL,reviewed INTEGER NOT NULL DEFAULT 0);
        CREATE TABLE IF NOT EXISTS settings(id INTEGER PRIMARY KEY,value TEXT NOT NULL);
        CREATE TABLE IF NOT EXISTS operations(id INTEGER PRIMARY KEY,root_id INTEGER NOT NULL,source TEXT NOT NULL,destination TEXT NOT NULL,identity TEXT NOT NULL,state TEXT NOT NULL,digest TEXT NOT NULL);
        CREATE TABLE IF NOT EXISTS corrections(root_id INTEGER NOT NULL,category TEXT NOT NULL,destination TEXT NOT NULL,PRIMARY KEY(root_id,category));
        CREATE TABLE IF NOT EXISTS rules(root_id INTEGER NOT NULL,category TEXT NOT NULL,destination TEXT NOT NULL,PRIMARY KEY(root_id,category));").map_err(message)?;
        let settings = db
            .query_row("SELECT value FROM settings WHERE id=1", [], |r| {
                r.get::<_, String>(0)
            })
            .ok()
            .map(|x| serde_json::from_str(&x))
            .transpose()
            .map_err(message)?
            .unwrap_or_default();
        Ok(Self {
            db,
            settings,
            roots: HashMap::new(),
            pending: HashMap::new(),
            next_id: 1,
        })
    }
    pub fn saved_roots(&self) -> Result<Vec<Root>> {
        let mut query = self
            .db
            .prepare("SELECT id,path,bookmark,reviewed FROM roots")
            .map_err(message)?;
        let rows = query
            .query_map([], |r| {
                Ok(Root {
                    id: r.get(0)?,
                    path: r.get(1)?,
                    bookmark: r.get(2)?,
                    initial_reviewed: r.get(3)?,
                })
            })
            .map_err(message)?;
        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(message)
    }
    pub fn authorize(&mut self, path: PathBuf, bookmark: String) -> Result<i64> {
        let fd = fs::root(&path).map_err(message)?;
        if fs::project(fd.as_raw_fd()) {
            return Err("Do not authorize a software project root".into());
        }
        let path = path
            .to_str()
            .ok_or("Unsupported root encoding")?
            .to_string();
        for root in self.roots.values() {
            if root.path != path
                && (Path::new(&path).starts_with(&root.path)
                    || Path::new(&root.path).starts_with(&path))
            {
                return Err("Overlapping authorized folders are not supported".into());
            }
        }
        self.db.execute("INSERT INTO roots(path,bookmark) VALUES(?,?) ON CONFLICT(path) DO UPDATE SET bookmark=excluded.bookmark",params![path,bookmark]).map_err(message)?;
        let root = self
            .saved_roots()?
            .into_iter()
            .find(|r| r.path == path)
            .ok_or("Missing root")?;
        let id = root.id;
        self.roots.insert(id, root);
        Ok(id)
    }
    pub fn revoke(&mut self, id: i64) -> Result<()> {
        self.roots.remove(&id);
        self.pending.retain(|_, p| p.root_id != id);
        self.db
            .execute("DELETE FROM roots WHERE id=?", [id])
            .map_err(message)?;
        Ok(())
    }
    pub fn set_settings(&mut self, mut settings: Settings) -> Result<()> {
        settings.revision = self
            .settings
            .revision
            .checked_add(1)
            .ok_or("Revision exhausted")?;
        self.db
            .execute(
                "INSERT OR REPLACE INTO settings VALUES(1,?)",
                [serde_json::to_string(&settings).map_err(message)?],
            )
            .map_err(message)?;
        self.settings = settings;
        self.pending.clear();
        Ok(())
    }
    pub fn create_categories(&self, id: i64) -> Result<()> {
        let root = self.roots.get(&id).ok_or("Folder is not authorized")?;
        let fd = fs::root(Path::new(&root.path)).map_err(message)?;
        if fs::project(fd.as_raw_fd()) {
            return Err("Project protected".into());
        }
        for category in CATEGORIES {
            fs::mkdir(&fd, category).map_err(message)?;
        }
        Ok(())
    }
    pub fn mark_initial_reviewed(&mut self, id: i64) -> Result<()> {
        let root = self.roots.get_mut(&id).ok_or("Folder is not authorized")?;
        self.db
            .execute("UPDATE roots SET reviewed=1 WHERE id=?", [id])
            .map_err(message)?;
        root.initial_reviewed = true;
        Ok(())
    }
    pub fn correct(
        &mut self,
        id: i64,
        category: String,
        destination: String,
        rule: bool,
    ) -> Result<()> {
        if !CATEGORIES.contains(&category.as_str()) {
            return Err("Unknown category".into());
        }
        let root = self.roots.get(&id).ok_or("Folder is not authorized")?;
        fs::parent(Path::new(&root.path), &format!("{destination}/validation")).map_err(message)?;
        if rule {
            self.db
                .execute(
                    "INSERT OR REPLACE INTO rules VALUES(?,?,?)",
                    params![id, category, destination],
                )
                .map_err(message)?;
        } else if self.settings.personalization {
            self.db
                .execute(
                    "INSERT OR REPLACE INTO corrections VALUES(?,?,?)",
                    params![id, category, destination],
                )
                .map_err(message)?;
        } else {
            return Err("Personalization is disabled".into());
        }
        self.pending.clear();
        Ok(())
    }
    pub fn clear_learning(&mut self) -> Result<()> {
        self.db
            .execute("DELETE FROM corrections", [])
            .map_err(message)?;
        self.pending.clear();
        Ok(())
    }
    pub fn clear_rules(&mut self) -> Result<()> {
        self.db.execute("DELETE FROM rules", []).map_err(message)?;
        self.pending.clear();
        Ok(())
    }
    pub fn scan(&mut self, id: i64) -> Result<Inventory> {
        let root = self
            .roots
            .get(&id)
            .ok_or("Folder is not authorized")?
            .clone();
        self.pending.retain(|_, p| p.root_id != id);
        let base = Path::new(&root.path);
        let fd = fs::root(base).map_err(message)?;
        let mut inventory = Inventory {
            root_id: id,
            files: 0,
            protected: 0,
            errors: Vec::new(),
            proposals: Vec::new(),
        };
        let mut stack = vec![(fd, String::new(), 0)];
        let mut targets = HashSet::new();
        let mut visited = 0;
        'walk: while let Some((directory, relative, depth)) = stack.pop() {
            if fs::project(directory.as_raw_fd())
                || fs::dataless(directory.as_raw_fd()).map_err(message)?
            {
                inventory.protected += 1;
                continue;
            }
            let names = match fs::names(&directory) {
                Ok(x) => x,
                Err(e) => {
                    inventory.errors.push(e.to_string());
                    continue;
                }
            };
            for name in names {
                visited += 1;
                if visited > 10000 {
                    inventory.errors.push("Entry limit reached".into());
                    break 'walk;
                }
                if fs::excluded(&name) {
                    inventory.protected += 1;
                    continue;
                }
                let source = if relative.is_empty() {
                    name.clone()
                } else {
                    format!("{relative}/{name}")
                };
                if let Ok(child) = fs::child_directory(&directory, &name) {
                    if depth >= 32 {
                        inventory.errors.push("Depth limit reached".into());
                    } else {
                        stack.push((child, source, depth + 1));
                    }
                    continue;
                }
                let snapshot = match fs::identity(&directory, &name) {
                    Ok(x) => x,
                    Err(_) => {
                        inventory.protected += 1;
                        continue;
                    }
                };
                inventory.files += 1;
                let category = classify(&name);
                let mut target_folder = category.clone();
                if self.settings.personalization {
                    if let Ok(value) = self.db.query_row(
                        "SELECT destination FROM corrections WHERE root_id=? AND category=?",
                        params![id, category],
                        |r| r.get::<_, String>(0),
                    ) {
                        target_folder = value;
                    }
                }
                let proposed_name = if self.settings.rename {
                    normalized_name(&name)
                } else {
                    name.clone()
                };
                let destination = if category == "Unknown" {
                    None
                } else {
                    Some(format!("{target_folder}/{proposed_name}"))
                };
                if destination.as_deref() == Some(&source) {
                    continue;
                }
                let mut reason = if destination.is_none() {
                    "Needs classification review"
                } else {
                    "Category destination rule"
                }
                .to_string();
                let mut blocked = destination.is_none();
                if let Some(dest) = &destination {
                    match fs::parent(base, dest) {
                        Ok((parent, basename)) => {
                            if fs::exists(&parent, &basename).unwrap_or(true) {
                                blocked = true;
                                reason = "Destination already exists".into();
                            }
                        }
                        Err(_) => {
                            blocked = true;
                            reason = "Create or choose an authorized category folder".into();
                        }
                    }
                    // Kernel exclusive rename remains the final collision authority
                    if !targets.insert(dest.to_lowercase()) {
                        blocked = true;
                        reason = "Two proposals share a destination".into();
                    }
                }
                if snapshot.size > 64 * 1024 * 1024 {
                    blocked = true;
                    reason = "File exceeds the 64 MiB execution limit".into();
                }
                let now = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .map_err(message)?
                    .as_secs() as i64;
                if now.saturating_sub(snapshot.mtime) < 10 {
                    blocked = true;
                    reason = "File must be stable for at least ten seconds".into();
                }
                let proposal = Proposal {
                    id: self.next_id,
                    root_id: id,
                    source,
                    destination,
                    category: category.clone(),
                    reason,
                    blocked,
                    revision: self.settings.revision,
                    rename: proposed_name != name,
                    certain: category != "Unknown",
                    snapshot,
                };
                self.next_id = self
                    .next_id
                    .checked_add(1)
                    .ok_or("Proposal identifiers exhausted")?;
                inventory.proposals.push(proposal);
            }
        }
        if !inventory.errors.is_empty() {
            for p in &mut inventory.proposals {
                p.blocked = true;
                p.reason = "Incomplete scan requires review and reconciliation".into();
            }
        }
        for p in &inventory.proposals {
            self.pending.insert(p.id, p.clone());
        }
        Ok(inventory)
    }
    pub fn apply(&mut self, id: u64, approved: bool) -> Result<i64> {
        let p = self
            .pending
            .get(&id)
            .ok_or("Proposal expired please scan again")?
            .clone();
        let root = self
            .roots
            .get(&p.root_id)
            .ok_or("Folder access revoked")?
            .clone();
        let unresolved:i64=self.db.query_row("SELECT count(*) FROM operations WHERE root_id=? AND state IN ('pending','undo_pending','needs_review')",[root.id],|r|r.get(0)).map_err(message)?;
        if unresolved > 0 {
            return Err("Resolve interrupted operations before more file changes".into());
        }
        if p.revision != self.settings.revision || p.blocked || (!p.certain && !approved) {
            return Err("Policy or safety review required".into());
        }
        if p.rename && !self.settings.rename {
            return Err("Renaming disabled".into());
        }
        if !approved {
            if !root.initial_reviewed || self.settings.mode == Mode::Smart {
                return Err("Explicit preview approval required".into());
            }
            if self.settings.mode == Mode::Limited {
                let allowed = self
                    .db
                    .query_row(
                        "SELECT destination FROM rules WHERE root_id=? AND category=?",
                        params![root.id, p.category],
                        |r| r.get::<_, String>(0),
                    )
                    .ok();
                let dest = p
                    .destination
                    .as_deref()
                    .and_then(|x| x.rsplit_once('/').map(|(a, _)| a));
                if allowed.as_deref() != dest {
                    return Err("Outside approved rules".into());
                }
            }
        }
        let destination = p.destination.ok_or("Missing destination")?;
        let base = Path::new(&root.path);
        let (from, source) = fs::parent(base, &p.source).map_err(message)?;
        let (to, target) = fs::parent(base, &destination).map_err(message)?;
        let current = fs::identity(&from, &source).map_err(message)?;
        if current != p.snapshot {
            return Err("File changed since preview".into());
        }
        if fs::exists(&to, &target).map_err(message)? {
            return Err("Destination collision".into());
        }
        let digest = fs::digest(&from, &source, &current).map_err(message)?;
        self.db.execute("INSERT INTO operations(root_id,source,destination,identity,state,digest) VALUES(?,?,?,?,'pending',?)",params![root.id,p.source,destination,serde_json::to_string(&current).map_err(message)?,digest]).map_err(message)?;
        let operation = self.db.last_insert_rowid();
        // On any uncertain filesystem result leave pending for reconciliation
        fs::move_exclusive(&from, &source, &to, &target).map_err(message)?;
        let after = fs::identity(&to, &target).map_err(message)?;
        if !fs::same_file(&after, &current)
            || fs::digest(&to, &target, &after).map_err(message)? != digest
        {
            return Err("Unexpected file identity requires recovery".into());
        }
        self.db
            .execute(
                "UPDATE operations SET state='applied',identity=? WHERE id=?",
                params![serde_json::to_string(&after).map_err(message)?, operation],
            )
            .map_err(message)?;
        self.pending.remove(&id);
        Ok(operation)
    }
    pub fn duplicates(&self, id: i64) -> Result<serde_json::Value> {
        let root = self.roots.get(&id).ok_or("Folder is not authorized")?;
        let base = Path::new(&root.path);
        let mut stack = vec![(fs::root(base).map_err(message)?, String::new(), 0)];
        let mut sizes: HashMap<u64, Vec<(String, Identity)>> = HashMap::new();
        let mut entries = 0;
        let mut partial = false;
        'walk: while let Some((directory, relative, depth)) = stack.pop() {
            if fs::project(directory.as_raw_fd())
                || fs::dataless(directory.as_raw_fd()).map_err(message)?
            {
                continue;
            }
            for name in fs::names(&directory).map_err(message)? {
                entries += 1;
                if entries > 10000 {
                    partial = true;
                    break 'walk;
                }
                if fs::excluded(&name) {
                    continue;
                }
                let path = if relative.is_empty() {
                    name.clone()
                } else {
                    format!("{relative}/{name}")
                };
                if let Ok(child) = fs::child_directory(&directory, &name) {
                    if depth < 32 {
                        stack.push((child, path, depth + 1));
                    } else {
                        partial = true;
                    }
                    continue;
                }
                if let Ok(identity) = fs::identity(&directory, &name) {
                    if identity.size <= 64 * 1024 * 1024 {
                        sizes
                            .entry(identity.size)
                            .or_default()
                            .push((path, identity));
                    } else {
                        partial = true;
                    }
                }
            }
        }
        let mut hashes: HashMap<String, Vec<String>> = HashMap::new();
        let mut bytes = 0u64;
        for (_, files) in sizes {
            if files.len() < 2 {
                continue;
            }
            for (path, identity) in files {
                if bytes + identity.size > 256 * 1024 * 1024 {
                    partial = true;
                    continue;
                }
                let (directory, name) = fs::parent(base, &path).map_err(message)?;
                match fs::digest(&directory, &name, &identity) {
                    Ok(hash) => {
                        bytes += identity.size;
                        hashes.entry(hash).or_default().push(path);
                    }
                    Err(_) => partial = true,
                }
            }
        }
        let mut groups = hashes
            .into_values()
            .filter(|paths| paths.len() > 1)
            .collect::<Vec<_>>();
        for group in &mut groups {
            group.sort();
        }
        groups.sort();
        Ok(
            serde_json::json!({"groups":groups,"bytes_hashed":bytes,"partial":partial,"read_only":true,"deletion_authorized":false}),
        )
    }
    pub fn history(&self) -> Result<Vec<Operation>> {
        let mut q=self.db.prepare("SELECT id,root_id,source,destination,state FROM operations ORDER BY id DESC LIMIT 500").map_err(message)?;
        let rows = q
            .query_map([], |r| {
                Ok(Operation {
                    id: r.get(0)?,
                    root_id: r.get(1)?,
                    source: r.get(2)?,
                    destination: r.get(3)?,
                    state: r.get(4)?,
                })
            })
            .map_err(message)?;
        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(message)
    }
    pub fn undo(&mut self, id: i64) -> Result<()> {
        let (root_id, source, destination, identity, state): (i64, String, String, String, String) =
            self.db
                .query_row(
                    "SELECT root_id,source,destination,identity,state FROM operations WHERE id=?",
                    [id],
                    |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)),
                )
                .map_err(message)?;
        if state != "applied" {
            return Err("Operation is not safely undoable".into());
        }
        let root = self
            .roots
            .get(&root_id)
            .ok_or("Authorize the original folder before Undo")?;
        let base = Path::new(&root.path);
        let (from, current_name) = fs::parent(base, &destination).map_err(message)?;
        let (to, original_name) = fs::parent(base, &source).map_err(message)?;
        let expected: Identity = serde_json::from_str(&identity).map_err(message)?;
        if fs::identity(&from, &current_name).map_err(message)? != expected {
            return Err("File changed after operation".into());
        }
        if fs::exists(&to, &original_name).map_err(message)? {
            return Err("Original location is occupied".into());
        }
        let digest: String = self
            .db
            .query_row("SELECT digest FROM operations WHERE id=?", [id], |r| {
                r.get(0)
            })
            .map_err(message)?;
        if fs::digest(&from, &current_name, &expected).map_err(message)? != digest {
            return Err("File content changed after operation".into());
        }
        self.db
            .execute(
                "UPDATE operations SET state='undo_pending' WHERE id=?",
                [id],
            )
            .map_err(message)?;
        fs::move_exclusive(&from, &current_name, &to, &original_name).map_err(message)?;
        self.db
            .execute("UPDATE operations SET state='undone' WHERE id=?", [id])
            .map_err(message)?;
        self.pending.clear();
        Ok(())
    }
    pub fn recover(&mut self) -> Result<()> {
        let mut query=self.db.prepare("SELECT id,root_id,source,destination,state FROM operations WHERE state IN ('pending','undo_pending')").map_err(message)?;
        let operations = query
            .query_map([], |r| {
                Ok(Operation {
                    id: r.get(0)?,
                    root_id: r.get(1)?,
                    source: r.get(2)?,
                    destination: r.get(3)?,
                    state: r.get(4)?,
                })
            })
            .map_err(message)?
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(message)?;
        for op in operations {
            let Some(root) = self.roots.get(&op.root_id) else {
                continue;
            };
            let base = Path::new(&root.path);
            let raw: String = self
                .db
                .query_row("SELECT identity FROM operations WHERE id=?", [op.id], |r| {
                    r.get(0)
                })
                .map_err(message)?;
            let expected: Identity = serde_json::from_str(&raw).map_err(message)?;
            let digest: String = self
                .db
                .query_row("SELECT digest FROM operations WHERE id=?", [op.id], |r| {
                    r.get(0)
                })
                .map_err(message)?;
            let from = fs::parent(base, &op.source);
            let to = fs::parent(base, &op.destination);
            let state = if let (Ok((a, s)), Ok((b, d))) = (from, to) {
                let src = fs::identity(&a, &s).ok();
                let dst = fs::identity(&b, &d).ok();
                if src.as_ref().is_some_and(|x| {
                    fs::same_file(x, &expected)
                        && fs::digest(&a, &s, x).ok().as_deref() == Some(digest.as_str())
                }) && !fs::exists(&b, &d).unwrap_or(true)
                {
                    Some(if op.state == "undo_pending" {
                        "undone"
                    } else {
                        "not_applied"
                    })
                } else if dst.as_ref().is_some_and(|x| {
                    fs::same_file(x, &expected)
                        && fs::digest(&b, &d, x).ok().as_deref() == Some(digest.as_str())
                }) && !fs::exists(&a, &s).unwrap_or(true)
                {
                    self.db
                        .execute(
                            "UPDATE operations SET identity=? WHERE id=?",
                            params![
                                serde_json::to_string(&dst.unwrap()).map_err(message)?,
                                op.id
                            ],
                        )
                        .map_err(message)?;
                    Some("applied")
                } else {
                    None
                }
            } else {
                None
            };
            self.db
                .execute(
                    "UPDATE operations SET state=? WHERE id=?",
                    params![state.unwrap_or("needs_review"), op.id],
                )
                .map_err(message)?;
        }
        Ok(())
    }
}
pub fn normalized_name(name: &str) -> String {
    let words: Vec<_> = name.split_whitespace().collect();
    words.join("_")
}
pub fn classify(name: &str) -> String {
    let ext = Path::new(name)
        .extension()
        .and_then(|x| x.to_str())
        .unwrap_or("")
        .to_lowercase();
    if [
        "py", "rs", "js", "ts", "tsx", "jsx", "java", "c", "cpp", "swift", "go",
    ]
    .contains(&ext.as_str())
    {
        return "Development".into();
    }
    let words: HashSet<_> = name
        .split(|c: char| !c.is_alphabetic())
        .map(|s| s.to_lowercase())
        .collect();
    let mut categories = Vec::new();
    for (category, terms) in [
        (
            "Education",
            vec!["lecture", "assignment", "exam", "college", "syllabus"],
        ),
        (
            "Finance",
            vec!["invoice", "receipt", "tax", "bank", "salary"],
        ),
        (
            "Professional",
            vec!["resume", "cv", "contract", "meeting", "proposal"],
        ),
        ("Personal", vec!["diary", "personal", "journal"]),
    ] {
        if terms.iter().any(|x| words.contains(*x)) {
            categories.push(category);
        }
    }
    if categories.len() > 1 {
        return "Unknown".into();
    }
    if let Some(category) = categories.first() {
        return category.to_string();
    }
    match ext.as_str() {
        "png" | "jpg" | "jpeg" | "gif" | "heic" | "mp3" | "wav" | "m4a" | "mp4" | "mov" => "Media",
        "dmg" | "pkg" | "exe" => "Software",
        _ => "Unknown",
    }
    .into()
}
#[cfg(test)]
mod recovery_tests {
    use super::*;
    #[test]
    fn interrupted_move_and_undo_reconcile() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().canonicalize().unwrap();
        let mut engine = Engine::new(Connection::open_in_memory().unwrap()).unwrap();
        let id = engine.authorize(path.clone(), "test".into()).unwrap();
        engine.create_categories(id).unwrap();
        std::fs::write(path.join("lecture.txt"), "synthetic").unwrap();
        let (from, name) = fs::parent(&path, "lecture.txt").unwrap();
        let (to, dest) = fs::parent(&path, "Education/lecture.txt").unwrap();
        let identity = fs::identity(&from, &name).unwrap();
        engine.db.execute("INSERT INTO operations(root_id,source,destination,identity,state,digest) VALUES(?,'lecture.txt','Education/lecture.txt',?,'pending',?)",params![id,serde_json::to_string(&identity).unwrap(),fs::digest(&from,&name,&identity).unwrap()]).unwrap();
        fs::move_exclusive(&from, &name, &to, &dest).unwrap();
        engine.recover().unwrap();
        assert_eq!(engine.history().unwrap()[0].state, "applied");
        engine
            .db
            .execute("UPDATE operations SET state='undo_pending'", [])
            .unwrap();
        fs::move_exclusive(&to, &dest, &from, &name).unwrap();
        engine.recover().unwrap();
        assert_eq!(engine.history().unwrap()[0].state, "undone");
    }
}

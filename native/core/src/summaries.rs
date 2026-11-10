//! Bounded local extractive summaries and review decisions with no generative or execution authority
use super::*;
#[derive(Clone, Debug, Serialize)]
pub struct FolderSuggestion {
    pub folder: String,
    pub reason: String,
    pub similar_files: Vec<String>,
}
#[derive(Clone, Debug, Serialize)]
pub struct FileSummary {
    pub supported: bool,
    pub excerpts: Vec<String>,
    pub scope: String,
    pub partial: bool,
    pub method: String,
    pub categories: Vec<String>,
    pub destinations: Vec<FolderSuggestion>,
    pub suggested_title: String,
    pub recommendation: String,
}
pub(crate) fn folder_fits(folder: &str, category: &str) -> bool {
    let prefixes = folder
        .split('/')
        .scan(String::new(), |path, part| {
            if !path.is_empty() {
                path.push('/');
            }
            path.push_str(part);
            Some(path.clone())
        })
        .collect::<Vec<_>>();
    !super::workflow::destinations(&prefixes, category).is_empty()
}
fn words(text: &str) -> HashSet<String> {
    text.split(|c: char| !c.is_alphanumeric())
        .map(str::to_lowercase)
        .filter(|w| {
            w.len() > 2
                && !matches!(
                    w.as_str(),
                    "the"
                        | "and"
                        | "for"
                        | "this"
                        | "that"
                        | "with"
                        | "from"
                        | "are"
                        | "was"
                        | "have"
                        | "will"
                        | "your"
                        | "file"
                        | "files"
                        | "document"
                        | "synthetic"
                        | "test"
                        | "fixture"
                        | "only"
                        | "txt"
                        | "pdf"
                        | "notes"
                )
        })
        .take(512)
        .collect()
}
fn categories(text: &str) -> Vec<String> {
    let w = words(text);
    let mut scores = Vec::new();
    for (c, terms) in [
        (
            "Education",
            &[
                "lecture",
                "assignment",
                "exam",
                "university",
                "syllabus",
                "lesson",
                "homework",
                "course",
                "students",
                "study",
            ][..],
        ),
        (
            "Finance",
            &[
                "invoice",
                "receipt",
                "payment",
                "bank",
                "salary",
                "tax",
                "balance",
                "transaction",
                "amount",
                "budget",
            ][..],
        ),
        (
            "Professional",
            &[
                "contract",
                "employment",
                "meeting",
                "client",
                "resume",
                "proposal",
                "supplier",
                "business",
                "project",
            ][..],
        ),
        (
            "Personal",
            &["family", "travel", "recipe", "diary", "holiday", "personal"][..],
        ),
        (
            "Development",
            &[
                "function",
                "class",
                "compiler",
                "programming",
                "source",
                "javascript",
                "python",
                "rust",
            ][..],
        ),
    ] {
        let score = terms.iter().filter(|word| w.contains(**word)).count();
        if score >= 2 {
            scores.push((score, c.to_string()));
        }
    }
    scores.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.cmp(&b.1)));
    scores.into_iter().map(|(_, c)| c).take(3).collect()
}
fn excerpts(text: &str) -> Vec<String> {
    let mut candidates = Vec::new();
    let mut seen = HashSet::new();
    let separated = text
        .replace(". ", ".\n")
        .replace("? ", "?\n")
        .replace("! ", "!\n");
    for part in separated.lines() {
        let value = part.split_whitespace().collect::<Vec<_>>().join(" ");
        if value.len() < 12 || words(&value).is_empty() || !seen.insert(value.clone()) {
            continue;
        }
        let index = candidates.len();
        let score = categories(&value).len() * 3 + words(&value).len().min(15);
        candidates.push((index, score, value.chars().take(240).collect::<String>()));
    }
    if candidates.len() > 3 {
        let first = candidates.remove(0);
        candidates.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
        candidates.truncate(2);
        candidates.push(first);
        candidates.sort_by_key(|c| c.0);
    }
    candidates.into_iter().map(|(_, _, s)| s).collect()
}
impl Engine {
    pub(crate) fn is_kept(&self, id: i64, source: &str, snapshot: &Identity) -> Result<bool> {
        let previous = self
            .db
            .query_row(
                "SELECT identity FROM kept WHERE root_id=? AND source=?",
                params![id, source],
                |r| r.get::<_, String>(0),
            )
            .ok();
        Ok(previous.as_deref() == Some(serde_json::to_string(snapshot).map_err(message)?.as_str()))
    }
    pub fn keep_here(&mut self, proposal_id: u64) -> Result<()> {
        let p = self
            .pending
            .get(&proposal_id)
            .ok_or("Proposal expired")?
            .clone();
        if p.revision != self.settings.revision {
            return Err("Preferences changed scan again".into());
        }
        let (parent, name) = self.parent(p.root_id, &p.source).map_err(message)?;
        if fs::identity(&parent, &name).map_err(message)? != p.snapshot {
            return Err("File changed summarise again".into());
        }
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
        self.pending.remove(&proposal_id);
        Ok(())
    }
    pub fn clear_kept(&mut self) -> Result<()> {
        self.db.execute("DELETE FROM kept", []).map_err(message)?;
        Ok(())
    }
    pub fn review_folder(&mut self, proposal_id: u64, folder: String) -> Result<Proposal> {
        let p = self
            .pending
            .get(&proposal_id)
            .ok_or("Proposal expired")?
            .clone();
        if p.revision != self.settings.revision || p.reason.contains("Incomplete scan") {
            return Err("Scan again before choosing a folder".into());
        }
        let (parent, name) = self.parent(p.root_id, &p.source).map_err(message)?;
        if fs::identity(&parent, &name).map_err(message)? != p.snapshot {
            return Err("Source changed".into());
        }
        let destination = format!("{folder}/{name}");
        if destination == p.source {
            return Err("File is already there choose Keep here".into());
        }
        let (to, target) = self.parent(p.root_id, &destination).map_err(message)?;
        if fs::exists(&to, &target).map_err(message)? {
            return Err("Destination collision".into());
        }
        let current = self.pending.get_mut(&proposal_id).unwrap();
        current.destination = Some(destination);
        current.rename = false;
        current.manual_destination = true;
        current.certain = false;
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(message)?
            .as_secs() as i64;
        current.blocked = now.saturating_sub(current.snapshot.mtime) < 10
            || current.snapshot.size > 64 * 1024 * 1024;
        current.reason = "User reviewed a similar folder still requires explicit approval".into();
        Ok(current.clone())
    }
    pub fn summary(
        &self,
        proposal_id: u64,
        pdf: Option<&serde_json::Value>,
    ) -> Result<FileSummary> {
        let p = self.pending.get(&proposal_id).ok_or("Proposal expired")?;
        if p.revision != self.settings.revision {
            return Err("Preferences changed scan again".into());
        }
        let (parent, name) = self.parent(p.root_id, &p.source).map_err(message)?;
        if fs::identity(&parent, &name).map_err(message)? != p.snapshot {
            return Err("File changed summarise again".into());
        }
        let extension = Path::new(&name)
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_lowercase();
        let (mut supported, mut partial, mut text, mut scope) = (
            false,
            false,
            String::new(),
            "No content summary for this format yet".to_string(),
        );
        if ["txt", "md", "csv", "log"].contains(&extension.as_str()) {
            match fs::text_sample(&parent, &name, &p.snapshot) {
                Ok(value) => {
                    partial = value.chars().count() >= 2000 || p.snapshot.size > value.len() as u64;
                    text = value;
                    supported = true;
                    scope =
                        "UTF8 text sample up to 2000 characters from a file no larger than 64 KiB"
                            .into();
                }
                Err(error) => {
                    scope = error.to_string();
                }
            }
        } else if extension == "pdf" {
            if let Some(parsed) = pdf {
                if parsed["status"] == "ok" {
                    text = parsed["text"]
                        .as_str()
                        .unwrap_or("")
                        .chars()
                        .take(2000)
                        .collect();
                    supported = true;
                    partial = parsed["truncated"].as_bool().unwrap_or(true);
                    scope = format!(
                        "Text from {} PDF pages up to 2000 characters without OCR",
                        parsed["pages_inspected"].as_u64().unwrap_or(0)
                    );
                } else {
                    scope = parsed["reason"]
                        .as_str()
                        .unwrap_or("PDF contains no extractable text OCR is not available")
                        .into();
                }
            }
        }
        if text.trim().is_empty() {
            supported = false;
        }
        let inferred = if supported {
            categories(&text)
        } else {
            Vec::new()
        };
        let query = words(&text);
        let current = p.source.rsplit_once('/').map(|x| x.0).unwrap_or("");
        let inventory = self.inventories.get(&p.root_id);
        let mut destinations = Vec::new();
        if let Some(inventory) = inventory {
            let mut grouped: HashMap<&str, Vec<&String>> = HashMap::new();
            for source in &inventory.items {
                if let Some((folder, _)) = source.rsplit_once('/') {
                    grouped.entry(folder).or_default().push(source);
                }
            }
            let mut ranked = Vec::new();
            for folder in &inventory.folders {
                if folder == current {
                    continue;
                }
                let mut score = 0usize;
                let mut reason = String::new();
                let mut similar = Vec::new();
                if inferred
                    .iter()
                    .any(|category| folder_fits(folder, category))
                {
                    score += 8;
                    reason =
                        "Existing folder name matches topics found in the content sample".into();
                }
                for source in grouped
                    .get(folder.as_str())
                    .into_iter()
                    .flat_map(|items| items.iter().copied())
                    .take(12)
                {
                    if source == &p.source {
                        continue;
                    }
                    let shared = query.intersection(&words(source)).count();
                    if shared >= 2 {
                        score += shared.min(6);
                        similar.push(source.clone());
                        reason = "Related filename terms inside this folder".into();
                    }
                }
                if score > 0 {
                    ranked.push((
                        score,
                        FolderSuggestion {
                            folder: folder.clone(),
                            reason,
                            similar_files: similar,
                        },
                    ));
                }
            }
            // Compare a small bounded selection of actual neighbouring text samples too
            let mut sampled = 0;
            let preferred = ranked
                .iter()
                .map(|(_, s)| s.folder.clone())
                .collect::<HashSet<_>>();
            let mut peers = inventory.items.iter().collect::<Vec<_>>();
            peers.sort_by_key(|path| {
                if path
                    .rsplit_once('/')
                    .is_some_and(|(folder, _)| preferred.contains(folder))
                {
                    0
                } else {
                    1
                }
            });
            let mut sampled_folders = HashSet::new();
            for source in peers {
                if sampled >= 8 {
                    break;
                }
                if source == &p.source {
                    continue;
                }
                let Some((folder, _)) = source.rsplit_once('/') else {
                    continue;
                };
                if folder == current {
                    continue;
                }
                let extension = Path::new(source)
                    .extension()
                    .and_then(|s| s.to_str())
                    .unwrap_or("");
                if !["txt", "md", "csv", "log"].contains(&extension) {
                    continue;
                }
                let Ok((dir, name)) = self.parent(p.root_id, source) else {
                    continue;
                };
                let Ok(identity) = fs::identity(&dir, &name) else {
                    continue;
                };
                if identity.size > 65536 {
                    continue;
                }
                if !sampled_folders.insert(folder.to_string()) {
                    continue;
                }
                sampled += 1;
                let Ok(other) = fs::text_sample(&dir, &name, &identity) else {
                    continue;
                };
                let shared = query.intersection(&words(&other)).count();
                if shared < 3 {
                    continue;
                }
                if let Some((score, suggestion)) =
                    ranked.iter_mut().find(|(_, s)| s.folder == folder)
                {
                    *score += shared.min(12);
                    suggestion.similar_files.push(source.clone());
                    suggestion.reason =
                        "Shared terms in sampled contents of a neighbouring file".into();
                } else {
                    ranked.push((
                        shared.min(12),
                        FolderSuggestion {
                            folder: folder.into(),
                            reason: "Shared terms in sampled contents of a neighbouring file"
                                .into(),
                            similar_files: vec![source.clone()],
                        },
                    ));
                }
            }
            ranked.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.folder.cmp(&b.1.folder)));
            for (_, candidate) in &mut ranked {
                candidate.similar_files.sort();
                candidate.similar_files.dedup();
            }
            destinations = ranked.into_iter().take(3).map(|(_, s)| s).collect();
        }
        let title = match inferred.first().map(String::as_str) {
            Some("Education") => "Study documents",
            Some("Finance") => "Financial records",
            Some("Professional") => "Work documents",
            Some("Personal") => "Personal documents",
            Some("Development") => "Programming references",
            _ => "Reviewed documents",
        };
        let recommendation = if !supported {
            "Choose Keep here or review a destination manually no content based decision is available"
        } else if inferred.len() == 1 && folder_fits(current, &inferred[0]) {
            "The current folder matches the main topic in the content sample Keep here is a reasonable choice"
        } else if destinations.is_empty() {
            "Keep here or create a titled collection after reviewing the sampled content"
        } else {
            "Compare the related folders below or Keep here these suggestions never grant automatic move permission"
        };
        Ok(FileSummary{supported,excerpts:excerpts(&text),scope,partial,method:"Local extractive overview using quoted passages and keyword overlap no generative model".into(),categories:inferred,destinations,suggested_title:title.into(),recommendation:recommendation.into()})
    }
}

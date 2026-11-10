use super::*;
fn words(value: &str) -> HashSet<String> {
    value
        .split(|c: char| !c.is_alphanumeric())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_lowercase())
        .collect()
}
impl Engine {
    pub fn remember_example(&mut self, proposal_id: u64) -> Result<()> {
        if !self.settings.personalization {
            return Err("Personalization is disabled".into());
        }
        let p = self.pending.get(&proposal_id).ok_or("Proposal expired")?;
        if p.blocked || p.category == "Unknown" {
            return Err("Select a safe destination before saving a correction".into());
        }
        let destination = p
            .destination
            .as_deref()
            .and_then(|s| s.rsplit_once('/').map(|(folder, _)| folder))
            .ok_or("Destination missing")?;
        self.parent(p.root_id, &format!("{destination}/validation"))
            .map_err(message)?;
        let filename = Path::new(&p.source)
            .file_name()
            .and_then(|x| x.to_str())
            .ok_or("Filename missing")?;
        self.db.execute("INSERT INTO examples(root_id,filename,category,destination) VALUES(?,?,?,?) ON CONFLICT(root_id,filename) DO UPDATE SET category=excluded.category,destination=excluded.destination",params![p.root_id,filename,p.category,destination]).map_err(message)?;
        self.pending.clear();
        Ok(())
    }
    pub(crate) fn nearest_example(
        &self,
        root_id: i64,
        filename: &str,
    ) -> Result<Option<(String, String)>> {
        if !self.settings.personalization {
            return Ok(None);
        }
        let query = words(filename);
        if query.is_empty() {
            return Ok(None);
        }
        let mut statement=self.db.prepare("SELECT filename,category,destination FROM examples WHERE root_id=? ORDER BY filename LIMIT 1000").map_err(message)?;
        let rows = statement
            .query_map([root_id], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            })
            .map_err(message)?;
        let mut best: Option<(f64, String, String)> = None;
        for row in rows {
            let (example, category, destination) = row.map_err(message)?;
            let reference = words(&example);
            let score = query.intersection(&reference).count() as f64
                / query.union(&reference).count() as f64;
            // The extension alone is not sufficient evidence for transferring a correction
            let extension = Path::new(filename)
                .extension()
                .and_then(|x| x.to_str())
                .unwrap_or("")
                .to_lowercase();
            let shared_meaning = query
                .intersection(&reference)
                .any(|word| word.as_str() != extension);
            if score > 0.0
                && shared_meaning
                && best.as_ref().is_none_or(|(value, _, _)| score > *value)
            {
                best = Some((score, category, destination));
            }
        }
        Ok(best.map(|(_, category, destination)| (category, destination)))
    }
}

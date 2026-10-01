use crate::{crypto, model::*};
use rusqlite::{params, Connection, OptionalExtension};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{collections::HashMap, fs, path::PathBuf};

fn err(e: impl std::fmt::Display) -> String {
    format!("Magazyn danych: {e}")
}
pub struct Store {
    db: Connection,
    root: PathBuf,
    fingerprints: HashMap<String, String>,
    pub settings: Settings,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Stats {
    pub history: i64,
    pub snippets: i64,
    pub pinned: i64,
    pub bytes: i64,
    pub disk_bytes: u64,
}
#[derive(Serialize)]
pub struct Page {
    pub items: Vec<Summary>,
    pub total: usize,
}
#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct LibraryItem {
    pub content: Content,
    pub pinned: bool,
}
#[derive(Serialize)]
pub struct Revision {
    pub id: String,
    pub saved: i64,
}
impl Store {
    pub fn open(root: PathBuf) -> Result<Self> {
        fs::create_dir_all(root.join("images")).map_err(err)?;
        let db = Connection::open(root.join("clipboard.db")).map_err(err)?;
        db.busy_timeout(std::time::Duration::from_secs(3))
            .map_err(err)?;
        db.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=FULL; PRAGMA secure_delete=ON; PRAGMA temp_store=MEMORY; PRAGMA auto_vacuum=INCREMENTAL;
            CREATE TABLE IF NOT EXISTS migrations(version INTEGER PRIMARY KEY);").map_err(err)?;
        let version: i64 = db
            .query_row("SELECT COALESCE(MAX(version),0) FROM migrations", [], |r| {
                r.get(0)
            })
            .map_err(err)?;
        if version > 2 {
            return Err(
                "Baza pochodzi z nowszej wersji aplikacji. Zainstaluj nowszą wersję.".into(),
            );
        }
        if version < 1 {
            db.execute_batch("BEGIN IMMEDIATE; CREATE TABLE entries(id TEXT PRIMARY KEY, kind TEXT NOT NULL, created INTEGER NOT NULL, updated INTEGER NOT NULL, bytes INTEGER NOT NULL, pinned INTEGER NOT NULL, library INTEGER NOT NULL, payload BLOB NOT NULL);
                CREATE INDEX entries_history ON entries(library,pinned,updated DESC); CREATE INDEX entries_kind ON entries(kind,updated DESC);
                CREATE TABLE settings(id INTEGER PRIMARY KEY CHECK(id=1),payload BLOB NOT NULL); INSERT INTO migrations VALUES(1); COMMIT;").map_err(err)?;
        }
        if version < 2 {
            if version == 1 {
                let recovery = root.join("migration-v1.sqlite");
                if !recovery.exists() {
                    // SQLite copies a consistent snapshot, including committed WAL data.
                    db.execute("VACUUM INTO ?1", [recovery.to_string_lossy().as_ref()])
                        .map_err(err)?;
                }
            }
            db.execute_batch("BEGIN IMMEDIATE;
                CREATE TABLE revisions(id TEXT PRIMARY KEY, entry_id TEXT NOT NULL REFERENCES entries(id) ON DELETE CASCADE, saved INTEGER NOT NULL, payload BLOB NOT NULL);
                CREATE INDEX revisions_entry ON revisions(entry_id,saved DESC);
                INSERT INTO migrations VALUES(2); COMMIT;").map_err(err)?;
        }
        db.execute_batch("PRAGMA foreign_keys=ON;").map_err(err)?;
        db.execute_batch("ATTACH DATABASE ':memory:' AS search; CREATE VIRTUAL TABLE search.fts USING fts5(id UNINDEXED, text, tokenize='unicode61 remove_diacritics 2');").map_err(err)?;
        let settings = match db
            .query_row("SELECT payload FROM settings WHERE id=1", [], |r| {
                r.get::<_, Vec<u8>>(0)
            })
            .optional()
            .map_err(err)?
        {
            Some(v) => serde_json::from_slice(&crypto::unprotect(&v)?).map_err(err)?,
            None => Settings::default(),
        };
        let mut s = Self {
            db,
            root,
            fingerprints: HashMap::new(),
            settings,
        };
        s.settings.validate()?;
        s.cleanup()?;
        let ids =
            s.db.prepare("SELECT id FROM entries")
                .map_err(err)?
                .query_map([], |r| r.get::<_, String>(0))
                .map_err(err)?
                .collect::<std::result::Result<Vec<_>, _>>()
                .map_err(err)?;
        for id in ids {
            let e = s.get(&id)?;
            s.index(&e)?;
            if !e.library {
                s.fingerprints.insert(s.fingerprint_entry(&e)?, e.id);
            }
        }
        s.remove_orphans()?;
        Ok(s)
    }
    pub fn is_empty(&self) -> Result<bool> {
        self.db
            .query_row("SELECT count(*)=0 FROM entries", [], |r| r.get(0))
            .map_err(err)
    }
    fn decode(row: &rusqlite::Row<'_>) -> rusqlite::Result<Entry> {
        let bytes: Vec<u8> = row.get(7)?;
        let content = crypto::unprotect(&bytes)
            .and_then(|b| serde_json::from_slice(&b).map_err(err))
            .map_err(|e| {
                rusqlite::Error::FromSqlConversionFailure(
                    7,
                    rusqlite::types::Type::Blob,
                    Box::new(std::io::Error::other(e)),
                )
            })?;
        Ok(Entry {
            id: row.get(0)?,
            kind: row.get(1)?,
            created: row.get(2)?,
            updated: row.get(3)?,
            bytes: row.get(4)?,
            pinned: row.get(5)?,
            library: row.get(6)?,
            content,
        })
    }
    pub fn get(&self, id: &str) -> Result<Entry> {
        self.db
            .query_row("SELECT * FROM entries WHERE id=?1", [id], Self::decode)
            .map_err(|_| {
                "Nie można odczytać elementu. Mógł zostać usunięty lub jest uszkodzony.".into()
            })
    }
    #[cfg(test)]
    pub fn all(&self) -> Result<Vec<Entry>> {
        self.db
            .prepare("SELECT * FROM entries")
            .map_err(err)?
            .query_map([], Self::decode)
            .map_err(err)?
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(err)
    }
    pub fn snippets(&self) -> Result<Vec<Entry>> {
        self.db
            .prepare("SELECT * FROM entries WHERE library=1")
            .map_err(err)?
            .query_map([], Self::decode)
            .map_err(err)?
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(err)
    }
    fn index(&self, e: &Entry) -> Result<()> {
        self.db
            .execute("DELETE FROM search.fts WHERE id=?1", [&e.id])
            .map_err(err)?;
        let text = format!(
            "{} {} {} {} {} {}",
            e.content.title,
            e.content.text,
            e.content.description,
            e.content.category,
            e.content.tags.join(" "),
            e.content.source
        )
        .replace('ł', "l")
        .replace('Ł', "L");
        self.db
            .execute(
                "INSERT INTO search.fts(id,text) VALUES(?1,?2)",
                params![e.id, text],
            )
            .map_err(err)?;
        Ok(())
    }
    fn write(&self, e: &Entry) -> Result<()> {
        let encrypted = crypto::protect(&serde_json::to_vec(&e.content).map_err(err)?)?;
        self.db
            .execute_batch("SAVEPOINT entry_write")
            .map_err(err)?;
        let result = (|| {
            self.db.execute("INSERT INTO entries VALUES(?1,?2,?3,?4,?5,?6,?7,?8) ON CONFLICT(id) DO UPDATE SET kind=excluded.kind,updated=excluded.updated,bytes=excluded.bytes,pinned=excluded.pinned,payload=excluded.payload",params![e.id,e.kind,e.created,e.updated,e.bytes,e.pinned,e.library,encrypted]).map_err(err)?;
            self.index(e)
        })();
        match result {
            Ok(()) => self.db.execute_batch("RELEASE entry_write").map_err(err),
            Err(e) => {
                let _ = self
                    .db
                    .execute_batch("ROLLBACK TO entry_write; RELEASE entry_write;");
                Err(e)
            }
        }
    }
    pub fn query(
        &self,
        query: &str,
        view: &str,
        kind: &str,
        sort: &str,
        offset: usize,
    ) -> Result<Page> {
        if query.len() > 500 {
            return Err("Wyszukiwanie jest za długie.".into());
        }
        let normalized = query.replace('ł', "l").replace('Ł', "L");
        let words: Vec<String> = normalized
            .split_whitespace()
            .filter(|w| w.chars().any(char::is_alphanumeric))
            .take(30)
            .map(|w| format!("\"{}\"*", w.replace('"', "\"\"")))
            .collect();
        if !query.trim().is_empty() && words.is_empty() {
            return Ok(Page {
                items: vec![],
                total: 0,
            });
        }
        let matching = if words.is_empty() {
            None
        } else {
            Some(
                self.db
                    .prepare("SELECT id FROM search.fts WHERE fts MATCH ?1")
                    .map_err(err)?
                    .query_map([words.join(" AND ")], |r| r.get::<_, String>(0))
                    .map_err(err)?
                    .collect::<std::result::Result<std::collections::HashSet<_>, _>>()
                    .map_err(err)?,
            )
        };
        let clause = match view {
            "snippets" => "library=1",
            "favorites" => "pinned=1",
            "images" => "library=0 AND kind='image'",
            "code" => "library=0 AND kind IN ('code','json')",
            "links" => "library=0 AND kind='link'",
            "quick" => "1=1",
            _ => "library=0",
        };
        let ordering = match sort {
            "oldest" => "updated ASC",
            "size" => "bytes DESC",
            _ => "pinned DESC,updated DESC",
        };
        let sql = format!(
            "SELECT id FROM entries WHERE {clause} AND (?1='' OR kind=?1) ORDER BY {ordering},id"
        );
        let ids = self
            .db
            .prepare(&sql)
            .map_err(err)?
            .query_map([kind], |r| r.get::<_, String>(0))
            .map_err(err)?
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(err)?;
        let ids: Vec<_> = ids
            .into_iter()
            .filter(|id| matching.as_ref().is_none_or(|m| m.contains(id)))
            .collect();
        let total = ids.len();
        let entries: Vec<Summary> = if sort == "title" {
            let mut all = ids
                .iter()
                .map(|id| self.get(id).map(Summary::from))
                .collect::<Result<Vec<_>>>()?;
            all.sort_by_key(|e| e.title.to_lowercase());
            all.into_iter().skip(offset).take(80).collect()
        } else {
            ids.iter()
                .skip(offset)
                .take(80)
                .map(|id| self.get(id).map(Summary::from))
                .collect::<Result<Vec<_>>>()?
        };
        Ok(Page {
            items: entries,
            total,
        })
    }
    fn hash(c: &Content, image: Option<&[u8]>) -> String {
        let mut h = Sha256::new();
        h.update(c.text.as_bytes());
        h.update(&c.html);
        h.update(&c.rtf);
        for f in &c.files {
            h.update(f.as_bytes());
            h.update([0]);
        }
        if let Some(b) = image {
            h.update(b);
        }
        format!("{:x}", h.finalize())
    }
    fn fingerprint_entry(&self, e: &Entry) -> Result<String> {
        let image = if e.kind == "image" {
            Some(self.image(&e.id)?)
        } else {
            None
        };
        Ok(Self::hash(&e.content, image.as_deref()))
    }
    pub fn capture(
        &mut self,
        content: Content,
        kind: &str,
        image: Option<Vec<u8>>,
    ) -> Result<bool> {
        let fingerprint = Self::hash(&content, image.as_deref());
        if let Some(id) = self.fingerprints.get(&fingerprint).cloned() {
            self.db
                .execute(
                    "UPDATE entries SET updated=?1 WHERE id=?2",
                    params![now(), id],
                )
                .map_err(err)?;
            return Ok(true);
        }
        let mut e = Entry {
            id: uuid::Uuid::new_v4().to_string(),
            kind: kind.into(),
            created: now(),
            updated: now(),
            bytes: 0,
            pinned: false,
            library: false,
            content,
        };
        e.bytes = (serde_json::to_vec(&e.content).map_err(err)?.len()
            + image.as_ref().map_or(0, Vec::len)) as i64;
        if let Some(b) = image {
            let encrypted = crypto::protect(&b)?;
            use std::io::Write;
            let mut file = fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(self.image_path(&e.id)?)
                .map_err(err)?;
            file.write_all(&encrypted).map_err(err)?;
            file.sync_all().map_err(err)?;
        }
        self.write(&e)?;
        self.fingerprints.insert(fingerprint, e.id);
        self.cleanup()?;
        Ok(true)
    }
    pub fn save_snippet(
        &mut self,
        id: Option<String>,
        mut content: Content,
        pinned: bool,
    ) -> Result<Entry> {
        validate_content(&content)?;
        if content.title.trim().is_empty() || content.text.trim().is_empty() {
            return Err("Nazwa i treść snippetu są wymagane.".into());
        }
        content.html.clear();
        content.rtf.clear();
        content.files.clear();
        content.source.clear();
        content.width = 0;
        content.height = 0;
        let existing = id.as_deref().map(|id| self.get(id)).transpose()?;
        if existing.as_ref().is_some_and(|e| !e.library) {
            return Err("Ten element nie jest snippetem.".into());
        }
        if existing.is_none() && self.stats()?.snippets >= 1000 {
            return Err("Biblioteka może zawierać do 1000 snippetów.".into());
        }
        let library_bytes: i64 = self
            .db
            .query_row(
                "SELECT COALESCE(SUM(bytes),0) FROM entries WHERE library=1",
                [],
                |r| r.get(0),
            )
            .map_err(err)?;
        if library_bytes - existing.as_ref().map_or(0, |e| e.bytes) + content.text.len() as i64
            > 64 * 1024 * 1024
        {
            return Err(
                "Biblioteka snippetów przekroczyłaby 64 MB. Usuń niepotrzebne fragmenty.".into(),
            );
        }
        let e = Entry {
            id: id.unwrap_or_else(|| uuid::Uuid::new_v4().to_string()),
            kind: classify(&content.text, false).into(),
            created: existing.as_ref().map_or_else(now, |e| e.created),
            updated: now(),
            bytes: content.text.len() as i64,
            pinned,
            library: true,
            content,
        };
        self.db
            .execute_batch("SAVEPOINT snippet_edit")
            .map_err(err)?;
        let result = (|| {
            if let Some(old) = existing.filter(|old| old.content != e.content) {
                let encrypted = crypto::protect(&serde_json::to_vec(&old.content).map_err(err)?)?;
                self.db
                    .execute(
                        "INSERT INTO revisions VALUES(?1,?2,?3,?4)",
                        params![uuid::Uuid::new_v4().to_string(), e.id, now(), encrypted],
                    )
                    .map_err(err)?;
                self.db.execute("DELETE FROM revisions WHERE entry_id=?1 AND id NOT IN (SELECT id FROM revisions WHERE entry_id=?1 ORDER BY saved DESC,rowid DESC LIMIT 10)", [&e.id]).map_err(err)?;
                self.db.execute_batch("DELETE FROM revisions WHERE id IN (SELECT id FROM (SELECT id,SUM(length(payload)) OVER (ORDER BY saved DESC,rowid DESC) AS total FROM revisions) WHERE total>67108864);").map_err(err)?;
            }
            self.write(&e)
        })();
        match result {
            Ok(()) => self.db.execute_batch("RELEASE snippet_edit").map_err(err)?,
            Err(error) => {
                let _ = self
                    .db
                    .execute_batch("ROLLBACK TO snippet_edit; RELEASE snippet_edit;");
                return Err(error);
            }
        }
        Ok(e)
    }
    pub fn revisions(&self, id: &str) -> Result<Vec<Revision>> {
        self.db
            .prepare(
                "SELECT id,saved FROM revisions WHERE entry_id=?1 ORDER BY saved DESC,rowid DESC",
            )
            .map_err(err)?
            .query_map([id], |r| {
                Ok(Revision {
                    id: r.get(0)?,
                    saved: r.get(1)?,
                })
            })
            .map_err(err)?
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(err)
    }
    pub fn revision(&self, id: &str, revision: &str) -> Result<Content> {
        let data: Vec<u8> = self
            .db
            .query_row(
                "SELECT payload FROM revisions WHERE id=?1 AND entry_id=?2",
                params![revision, id],
                |r| r.get(0),
            )
            .map_err(|_| "Nie można odczytać tej wersji.")?;
        serde_json::from_slice(&crypto::unprotect(&data)?).map_err(err)
    }
    pub fn library(&self) -> Result<Vec<LibraryItem>> {
        Ok(self
            .snippets()?
            .into_iter()
            .map(|e| LibraryItem {
                content: e.content,
                pinned: e.pinned,
            })
            .collect())
    }
    pub fn restore_library(&mut self, items: Vec<LibraryItem>) -> Result<usize> {
        if items.len() > 1000 {
            return Err("Kopia zawiera ponad 1000 snippetów.".into());
        }
        for item in &items {
            validate_content(&item.content)?;
            if item.content.title.trim().is_empty() || item.content.text.trim().is_empty() {
                return Err("Kopia zawiera pusty snippet.".into());
            }
        }
        let mut known = self.library()?;
        for item in &mut known {
            item.content.shortcut.clear();
        }
        self.db
            .execute_batch("SAVEPOINT library_restore")
            .map_err(err)?;
        let result = (|| {
            let mut added = 0;
            for mut item in items {
                item.content.shortcut.clear();
                if known.iter().any(|old| old.content == item.content) {
                    continue;
                }
                self.save_snippet(None, item.content.clone(), item.pinned)?;
                known.push(item);
                added += 1;
            }
            Ok(added)
        })();
        match result {
            Ok(n) => {
                self.db
                    .execute_batch("RELEASE library_restore")
                    .map_err(err)?;
                Ok(n)
            }
            Err(e) => {
                let _ = self
                    .db
                    .execute_batch("ROLLBACK TO library_restore; RELEASE library_restore;");
                Err(e)
            }
        }
    }
    pub fn pin(&mut self, id: &str) -> Result<()> {
        let mut e = self.get(id)?;
        e.pinned = !e.pinned;
        self.write(&e)?;
        self.cleanup()?;
        Ok(())
    }
    pub fn tags(&self, id: &str, tags: Vec<String>) -> Result<()> {
        let mut e = self.get(id)?;
        e.content.tags = tags;
        validate_content(&e.content)?;
        self.write(&e)
    }
    pub fn delete(&mut self, id: &str) -> Result<()> {
        let e = self.get(id)?;
        self.db
            .execute_batch("SAVEPOINT entry_delete")
            .map_err(err)?;
        let result = (|| {
            self.db
                .execute("DELETE FROM entries WHERE id=?1", [id])
                .map_err(err)?;
            self.db
                .execute("DELETE FROM search.fts WHERE id=?1", [id])
                .map_err(err)?;
            Ok(())
        })();
        match result {
            Ok(()) => self.db.execute_batch("RELEASE entry_delete").map_err(err)?,
            Err(e) => {
                let _ = self
                    .db
                    .execute_batch("ROLLBACK TO entry_delete; RELEASE entry_delete;");
                return Err(e);
            }
        }
        self.fingerprints.retain(|_, v| v != id);
        if e.kind == "image" {
            match fs::remove_file(self.image_path(id)?) {
                Ok(_) => {}
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                Err(_) => log::warn!("Orphan image cleanup deferred"),
            }
        }
        Ok(())
    }
    pub fn cleanup(&mut self) -> Result<usize> {
        let cutoff = now() - self.settings.max_days as i64 * 86_400_000;
        let rows=self.db.prepare("SELECT id,updated,bytes FROM entries WHERE library=0 AND pinned=0 ORDER BY updated DESC,id").map_err(err)?.query_map([],|r|Ok((r.get::<_,String>(0)?,r.get::<_,i64>(1)?,r.get::<_,i64>(2)?))).map_err(err)?.collect::<std::result::Result<Vec<_>,_>>().map_err(err)?;
        let mut bytes = 0;
        let mut removed = 0;
        for (i, (id, updated, size)) in rows.into_iter().enumerate() {
            bytes += size;
            if i >= self.settings.max_items as usize
                || updated < cutoff
                || bytes > self.settings.max_mb as i64 * 1024 * 1024
            {
                self.delete(&id)?;
                removed += 1;
            }
        }
        if removed > 0 {
            self.db
                .execute_batch("PRAGMA wal_checkpoint(TRUNCATE); PRAGMA incremental_vacuum(256);")
                .map_err(err)?;
        }
        Ok(removed)
    }
    pub fn clear_history(&mut self) -> Result<usize> {
        let ids = self
            .db
            .prepare("SELECT id FROM entries WHERE library=0 AND pinned=0")
            .map_err(err)?
            .query_map([], |r| r.get::<_, String>(0))
            .map_err(err)?
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(err)?;
        for id in &ids {
            self.delete(id)?;
        }
        self.db
            .execute_batch("PRAGMA wal_checkpoint(TRUNCATE); VACUUM;")
            .map_err(err)?;
        Ok(ids.len())
    }
    pub fn save_settings(&mut self, s: Settings) -> Result<()> {
        s.validate()?;
        self.db.execute("INSERT INTO settings VALUES(1,?1) ON CONFLICT(id) DO UPDATE SET payload=excluded.payload",[crypto::protect(&serde_json::to_vec(&s).map_err(err)?)?]).map_err(err)?;
        self.settings = s;
        Ok(())
    }
    fn image_path(&self, id: &str) -> Result<PathBuf> {
        uuid::Uuid::parse_str(id).map_err(|_| "Nieprawidłowy identyfikator obrazu.")?;
        Ok(self.root.join("images").join(format!("{id}.bin")))
    }
    pub fn image(&self, id: &str) -> Result<Vec<u8>> {
        crypto::unprotect(&fs::read(self.image_path(id)?).map_err(err)?)
    }
    pub fn stats(&self) -> Result<Stats> {
        let mut s=self.db.query_row("SELECT COALESCE(SUM(library=0),0),COALESCE(SUM(library=1),0),COALESCE(SUM(pinned=1),0),COALESCE(SUM(bytes),0) FROM entries",[],|r|Ok(Stats {history:r.get(0)?,snippets:r.get(1)?,pinned:r.get(2)?,bytes:r.get(3)?,disk_bytes:0})).map_err(err)?;
        for dir in [&self.root, &self.root.join("images")] {
            for item in fs::read_dir(dir).map_err(err)?.flatten() {
                if let Ok(m) = item.metadata() {
                    if m.is_file() {
                        s.disk_bytes += m.len();
                    }
                }
            }
        }
        Ok(s)
    }
    fn remove_orphans(&self) -> Result<()> {
        for file in fs::read_dir(self.root.join("images"))
            .map_err(err)?
            .flatten()
        {
            let p = file.path();
            if p.extension().is_some_and(|e| e == "bin") {
                if let Some(id) = p.file_stem().and_then(|s| s.to_str()) {
                    let exists: bool = self
                        .db
                        .query_row(
                            "SELECT EXISTS(SELECT 1 FROM entries WHERE id=?1)",
                            [id],
                            |r| r.get(0),
                        )
                        .map_err(err)?;
                    if !exists {
                        fs::remove_file(p).map_err(err)?;
                    }
                }
            }
        }
        Ok(())
    }
    pub fn import(&mut self, contents: Vec<Content>) -> Result<usize> {
        if contents.len() > 1000 || self.stats()?.snippets + contents.len() as i64 > 1000 {
            return Err("Import przekroczyłby limit 1000 snippetów.".into());
        }
        for c in &contents {
            validate_content(c)?;
            if c.title.trim().is_empty() || c.text.trim().is_empty() {
                return Err("Import zawiera snippet bez nazwy lub treści.".into());
            }
        }
        self.db.execute_batch("BEGIN IMMEDIATE").map_err(err)?;
        let result = (|| {
            for mut c in contents.iter().cloned() {
                c.shortcut.clear();
                self.save_snippet(None, c, false)?;
            }
            Ok(contents.len())
        })();
        match result {
            Ok(n) => {
                self.db.execute_batch("COMMIT").map_err(err)?;
                Ok(n)
            }
            Err(e) => {
                let _ = self.db.execute_batch("ROLLBACK");
                Err(e)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn revisions_are_bounded_encrypted_and_survive_restart() {
        let (root, mut s) = fixture();
        let original = Content {
            title: "Template".into(),
            text: "Hello {{name}}".into(),
            is_template: true,
            ..Default::default()
        };
        let e = s.save_snippet(None, original.clone(), true).unwrap();
        for i in 0..14 {
            let mut c = original.clone();
            c.text = format!("Revision {i}: {{{{name}}}}");
            s.save_snippet(Some(e.id.clone()), c, true).unwrap();
        }
        let revisions = s.revisions(&e.id).unwrap();
        assert_eq!(revisions.len(), 10);
        let previous = s.revision(&e.id, &revisions[0].id).unwrap();
        assert!(previous.text.starts_with("Revision 12:"));
        assert!(s
            .revision(&uuid::Uuid::new_v4().to_string(), &revisions[0].id)
            .is_err());
        drop(s);
        let mut s = Store::open(root.clone()).unwrap();
        assert_eq!(s.revisions(&e.id).unwrap().len(), 10);
        s.delete(&e.id).unwrap();
        assert!(s.revisions(&e.id).unwrap().is_empty());
        drop(s);
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn restore_preserves_pins_deduplicates_and_rolls_back() {
        let (root, mut s) = fixture();
        let item = LibraryItem {
            content: Content {
                title: "Backup".into(),
                text: "Hello {{name}}".into(),
                is_template: true,
                shortcut: "Ctrl+Alt+1".into(),
                ..Default::default()
            },
            pinned: true,
        };
        assert_eq!(
            s.restore_library(vec![item.clone(), item.clone()]).unwrap(),
            1
        );
        let restored = s.snippets().unwrap().pop().unwrap();
        assert!(restored.pinned);
        assert!(restored.content.is_template);
        assert!(restored.content.shortcut.is_empty());
        assert_eq!(s.restore_library(vec![item]).unwrap(), 0);
        let valid = LibraryItem {
            content: Content {
                title: "New".into(),
                text: "New".into(),
                ..Default::default()
            },
            pinned: false,
        };
        let invalid = LibraryItem {
            content: Content {
                title: "Bad".into(),
                text: "{{broken".into(),
                is_template: true,
                ..Default::default()
            },
            pinned: false,
        };
        assert!(s.restore_library(vec![valid, invalid]).is_err());
        assert_eq!(s.stats().unwrap().snippets, 1);
        drop(s);
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn v1_database_migrates_without_losing_data() {
        let (root, mut s) = fixture();
        let e = s
            .save_snippet(
                None,
                Content {
                    title: "Old".into(),
                    text: "Existing data".into(),
                    ..Default::default()
                },
                true,
            )
            .unwrap();
        s.db.execute_batch("DROP TABLE revisions; DELETE FROM migrations WHERE version=2;")
            .unwrap();
        drop(s);
        let s = Store::open(root.clone()).unwrap();
        assert_eq!(s.get(&e.id).unwrap().content.text, "Existing data");
        assert!(s.revisions(&e.id).unwrap().is_empty());
        assert!(root.join("migration-v1.sqlite").exists());
        drop(s);
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn failed_save_rolls_back_revision_and_search() {
        let (root, mut s) = fixture();
        let old = Content {
            title: "Stable".into(),
            text: "original marker".into(),
            ..Default::default()
        };
        let e = s.save_snippet(None, old.clone(), false).unwrap();
        s.db.execute_batch("CREATE TRIGGER simulate_disk_failure BEFORE INSERT ON entries BEGIN SELECT RAISE(ABORT,'simulated write failure'); END;").unwrap();
        let mut changed = old.clone();
        changed.text = "new marker".into();
        assert!(s.save_snippet(Some(e.id.clone()), changed, false).is_err());
        assert_eq!(s.get(&e.id).unwrap().content, old);
        assert!(s.revisions(&e.id).unwrap().is_empty());
        assert_eq!(
            s.query("original", "snippets", "", "recent", 0)
                .unwrap()
                .total,
            1
        );
        assert_eq!(
            s.query("new", "snippets", "", "recent", 0).unwrap().total,
            0
        );
        drop(s);
        fs::remove_dir_all(root).unwrap();
    }
    fn fixture() -> (PathBuf, Store) {
        let root =
            std::env::temp_dir().join(format!("super-clipboard-test-{}", uuid::Uuid::new_v4()));
        let store = Store::open(root.clone()).unwrap();
        (root, store)
    }
    #[test]
    fn item_and_byte_limits_preserve_pins_and_library() {
        let (root, mut s) = fixture();
        s.settings.max_items = 10;
        s.save_snippet(
            None,
            Content {
                title: "Stay".into(),
                text: "Permanent".into(),
                ..Default::default()
            },
            false,
        )
        .unwrap();
        for i in 0..30 {
            s.capture(
                Content {
                    text: format!("Unique {i}"),
                    ..Default::default()
                },
                "text",
                None,
            )
            .unwrap();
        }
        assert_eq!(s.stats().unwrap().history, 10);
        assert_eq!(s.stats().unwrap().snippets, 1);
        let id = s
            .all()
            .unwrap()
            .into_iter()
            .find(|e| !e.library)
            .unwrap()
            .id;
        s.pin(&id).unwrap();
        s.settings.max_mb = 0;
        s.cleanup().unwrap();
        assert_eq!(s.stats().unwrap().history, 1);
        assert!(s.get(&id).unwrap().pinned);
        drop(s);
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn image_encryption_orphans_and_restart() {
        let (root, mut s) = fixture();
        let image = b"synthetic-image-private-payload".to_vec();
        s.capture(
            Content {
                title: "Image".into(),
                ..Default::default()
            },
            "image",
            Some(image.clone()),
        )
        .unwrap();
        let id = s.all().unwrap()[0].id.clone();
        assert_eq!(s.image(&id).unwrap(), image);
        assert!(!fs::read(s.image_path(&id).unwrap())
            .unwrap()
            .windows(image.len())
            .any(|w| w == image));
        assert!(s.image("../clipboard.db").is_err());
        drop(s);
        let mut s = Store::open(root.clone()).unwrap();
        assert_eq!(s.image(&id).unwrap(), image);
        s.delete(&id).unwrap();
        assert!(!s.image_path(&id).unwrap().exists());
        drop(s);
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn imports_validate_atomically_and_strip_shortcuts() {
        let (root, mut s) = fixture();
        let c = Content {
            title: "Valid".into(),
            text: "Persist this".into(),
            shortcut: "Ctrl+Alt+1".into(),
            ..Default::default()
        };
        assert!(s.import(vec![c.clone(), Content::default()]).is_err());
        assert_eq!(s.stats().unwrap().snippets, 0);
        assert_eq!(s.import(vec![c]).unwrap(), 1);
        assert!(s.all().unwrap()[0].content.shortcut.is_empty());
        drop(s);
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn long_text_fts_and_encrypted_database() {
        let (root, mut s) = fixture();
        let text = format!(
            "{} searchabletailtoken",
            "plain-secret-marker ".repeat(100000)
        );
        s.capture(
            Content {
                text,
                source: "notepad.exe".into(),
                ..Default::default()
            },
            "text",
            None,
        )
        .unwrap();
        assert_eq!(
            s.query("searchabletailtoken", "history", "", "recent", 0)
                .unwrap()
                .total,
            1
        );
        assert_eq!(
            s.query("notepad", "history", "", "recent", 0)
                .unwrap()
                .total,
            1
        );
        drop(s);
        let db = fs::read(root.join("clipboard.db")).unwrap();
        assert!(!db.windows(19).any(|w| w == b"plain-secret-marker"));
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn persistence_dedupe_search_retention() {
        let root =
            std::env::temp_dir().join(format!("super-clipboard-test-{}", uuid::Uuid::new_v4()));
        {
            let mut s = Store::open(root.clone()).unwrap();
            let c = Content {
                text: "zażółć gęślą secret-independent".into(),
                ..Default::default()
            };
            s.capture(c.clone(), "text", None).unwrap();
            s.capture(c, "text", None).unwrap();
            assert_eq!(s.stats().unwrap().history, 1);
            assert_eq!(
                s.query("zazolc", "history", "", "recent", 0).unwrap().total,
                1
            );
            assert_eq!(
                s.query("\" OR *", "history", "", "recent", 0)
                    .unwrap()
                    .total,
                0
            );
            let id = s.all().unwrap()[0].id.clone();
            s.pin(&id).unwrap();
            s.clear_history().unwrap();
            assert_eq!(s.stats().unwrap().history, 1);
            s.pin(&id).unwrap();
            s.db.execute("UPDATE entries SET updated=1", []).unwrap();
            assert_eq!(s.cleanup().unwrap(), 1);
            s.save_snippet(
                None,
                Content {
                    title: "Prompt".into(),
                    text: "Trwały tekst".into(),
                    ..Default::default()
                },
                true,
            )
            .unwrap();
        }
        {
            let s = Store::open(root.clone()).unwrap();
            assert_eq!(s.stats().unwrap().snippets, 1);
            assert_eq!(
                s.query("Trwały", "snippets", "", "recent", 0)
                    .unwrap()
                    .total,
                1
            );
        }
        fs::remove_dir_all(root).unwrap();
    }
}

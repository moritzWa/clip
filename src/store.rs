use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use rusqlite::{Connection, params};
use sha2::{Digest, Sha256};

#[derive(Clone)]
pub struct Hit {
    pub id: String,
    pub title: String,
    pub created_at: String,
    pub source: String,
    pub kind: String,
}

pub struct Preview {
    pub text: String,
    pub image_path: Option<PathBuf>,
    pub file_path: Option<PathBuf>,
    pub source: String,
    pub created_at: String,
    pub kind: String,
    pub bytes: Option<u64>,
}

pub struct Store {
    conn: Connection,
    files: PathBuf,
}

pub fn data_dir() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    PathBuf::from(home).join("Library/Application Support/Clip")
}

impl Store {
    pub fn open() -> Result<Self, String> {
        let dir = data_dir();
        Self::open_in(&dir)
    }

    fn open_in(dir: &Path) -> Result<Self, String> {
        std::fs::create_dir_all(dir.join("files")).map_err(|e| e.to_string())?;
        let imported = dir.join("import/history.db");
        let live = dir.join("history.db");
        let path = if live.exists() {
            live
        } else if imported.exists() {
            imported
        } else {
            live
        };
        let conn = Connection::open(&path).map_err(|e| e.to_string())?;
        conn.busy_timeout(std::time::Duration::from_millis(500))
            .map_err(|e| e.to_string())?;
        conn.execute_batch(
            "
            PRAGMA journal_mode = WAL;
            PRAGMA synchronous = NORMAL;
            CREATE TABLE IF NOT EXISTS entries (
                id TEXT PRIMARY KEY,
                created_at TEXT NOT NULL,
                updated_at TEXT,
                title TEXT,
                categories TEXT,
                source_name TEXT,
                source_path TEXT,
                copy_count INTEGER,
                content_hash TEXT
            );
            CREATE TABLE IF NOT EXISTS parts (
                id TEXT PRIMARY KEY,
                entry_id TEXT NOT NULL,
                kind TEXT NOT NULL,
                mime TEXT,
                text TEXT,
                file_path TEXT
            );
            CREATE INDEX IF NOT EXISTS parts_entry ON parts(entry_id);
            CREATE INDEX IF NOT EXISTS entries_created ON entries(created_at);
            CREATE INDEX IF NOT EXISTS entries_hash ON entries(content_hash);
            CREATE INDEX IF NOT EXISTS entries_recency ON entries(COALESCE(updated_at, created_at) DESC, id DESC);
            CREATE VIRTUAL TABLE IF NOT EXISTS parts_fts USING fts5(text, entry_id UNINDEXED, tokenize='unicode61');
            ",
        )
        .map_err(|e| e.to_string())?;
        Ok(Self {
            conn,
            files: dir.join("files"),
        })
    }

    pub fn recent(&self, limit: usize) -> Vec<Hit> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, COALESCE(title, ''), created_at, COALESCE(source_name, ''), COALESCE(categories, '')
                 FROM entries
                 ORDER BY COALESCE(updated_at, created_at) DESC, id DESC
                 LIMIT ?1",
            )
            .expect("recent");
        stmt.query_map([limit as i64], row_hit)
            .expect("recent")
            .filter_map(|r| r.ok())
            .collect()
    }

    pub fn search(&self, query: &str, limit: usize) -> Vec<Hit> {
        let fts = fts_query(query);
        if fts.is_empty() {
            return self.recent(limit);
        }
        let like = format!(
            "%{}%",
            query
                .trim()
                .replace('\\', "\\\\")
                .replace('%', "\\%")
                .replace('_', "\\_")
        );
        let mut stmt = self
            .conn
            .prepare(
                "SELECT e.id, COALESCE(e.title, ''), e.created_at, COALESCE(e.source_name, ''), COALESCE(e.categories, '')
                 FROM entries e
                 WHERE e.id IN (
                     SELECT entry_id FROM parts_fts WHERE parts_fts MATCH ?1
                     UNION
                     SELECT id FROM entries WHERE title LIKE ?2 ESCAPE '\\'
                 )
                 ORDER BY COALESCE(e.updated_at, e.created_at) DESC
                 LIMIT ?3",
            )
            .expect("search");
        stmt.query_map(params![fts, like, limit as i64], row_hit)
            .map(|rows| rows.filter_map(|r| r.ok()).collect())
            .unwrap_or_else(|_| self.recent(limit))
    }

    pub fn preview(&self, id: &str) -> Option<Preview> {
        let (source, created_at, categories): (String, String, String) = self
            .conn
            .query_row(
                "SELECT COALESCE(source_name, ''), created_at, COALESCE(categories, '') FROM entries WHERE id = ?1",
                [id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .ok()?;
        let mut stmt = self
            .conn
            .prepare("SELECT kind, COALESCE(text, ''), COALESCE(file_path, '') FROM parts WHERE entry_id = ?1")
            .ok()?;
        let parts: Vec<(String, String, String)> = stmt
            .query_map([id], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
            .ok()?
            .filter_map(|r| r.ok())
            .collect();
        let text = parts
            .iter()
            .find(|p| (p.0 == "text" || p.0 == "link") && !p.1.is_empty())
            .or_else(|| parts.iter().find(|p| p.0 == "ocr" && !p.1.is_empty()))
            .map(|p| p.1.clone())
            .unwrap_or_default();
        let image_path = parts
            .iter()
            .find(|p| p.0 == "image" && !p.2.is_empty())
            .map(|p| recovered_asset_path(&p.2));
        let file_path = parts
            .iter()
            .find(|p| p.0 == "file" && !p.2.is_empty())
            .map(|p| PathBuf::from(&p.2));
        let bytes = image_path
            .as_ref()
            .or(file_path.as_ref())
            .and_then(|p| std::fs::metadata(p).ok())
            .map(|m| m.len());
        let kind = if image_path.is_some() {
            "image"
        } else if file_path.is_some() {
            "file"
        } else if categories.contains("link") {
            "link"
        } else {
            "text"
        };
        Some(Preview {
            text,
            image_path,
            file_path,
            source,
            created_at,
            kind: kind.into(),
            bytes,
        })
    }

    /// Returns true if this was a new row. A repeat of the latest same text bumps it to the top.
    pub fn insert_text(&mut self, text: &str, source: Option<&str>) -> bool {
        if text.trim().is_empty() || text.len() > 2_000_000 {
            return false;
        }
        let hash = hex_hash(text.as_bytes());
        if self.bump_hash(&hash) {
            return false;
        }
        let id = new_id();
        let now = now_rfc3339();
        let title: String = text
            .lines()
            .next()
            .unwrap_or("")
            .chars()
            .take(180)
            .collect();
        let kind = if text.starts_with("http://") || text.starts_with("https://") {
            "link"
        } else {
            "text"
        };
        let Ok(tx) = self.conn.transaction() else {
            return false;
        };
        let result = (|| -> rusqlite::Result<()> {
            tx.execute(
                "INSERT INTO entries (id, created_at, updated_at, title, categories, source_name, source_path, copy_count, content_hash)
                 VALUES (?1, ?2, ?2, ?3, ?4, ?5, '', 1, ?6)",
                params![id, now, title, format!("[\"{kind}\"]"), source.unwrap_or(""), hash],
            )?;
            tx.execute(
                "INSERT INTO parts (id, entry_id, kind, mime, text, file_path) VALUES (?1, ?2, ?3, 'text/plain', ?4, '')",
                params![format!("{id}:t"), id, kind, text],
            )?;
            tx.execute(
                "INSERT INTO parts_fts (text, entry_id) VALUES (?1, ?2)",
                params![text, id],
            )?;
            Ok(())
        })();
        result.is_ok() && tx.commit().is_ok()
    }

    pub fn insert_image(&mut self, bytes: &[u8], ext: &str, source: Option<&str>) -> bool {
        if bytes.is_empty() || bytes.len() > 40_000_000 {
            return false;
        }
        let hash = hex_hash(bytes);
        if self.bump_hash(&hash) {
            return false;
        }
        let id = new_id();
        let now = now_rfc3339();
        if !matches!(ext, "png" | "tiff") {
            return false;
        }
        let name = format!("{hash}.{ext}");
        let path = self.files.join(&name);
        if std::fs::write(&path, bytes).is_err() {
            return false;
        }
        let title = name.clone();
        let Ok(tx) = self.conn.transaction() else {
            return false;
        };
        let result = (|| -> rusqlite::Result<()> {
            tx.execute(
                "INSERT INTO entries (id, created_at, updated_at, title, categories, source_name, source_path, copy_count, content_hash)
                 VALUES (?1, ?2, ?2, ?3, '[\"image\"]', ?4, '', 1, ?5)",
                params![id, now, title, source.unwrap_or(""), hash],
            )?;
            tx.execute(
                "INSERT INTO parts (id, entry_id, kind, mime, text, file_path) VALUES (?1, ?2, 'image', ?3, '', ?4)",
                params![format!("{id}:i"), id, format!("image/{ext}"), path.display().to_string()],
            )?;
            Ok(())
        })();
        result.is_ok() && tx.commit().is_ok()
    }

    pub fn insert_file(&mut self, path: &Path, source: Option<&str>) -> bool {
        if !path.exists() {
            return false;
        }
        let hash = hex_hash(path.to_string_lossy().as_bytes());
        if self.bump_hash(&hash) {
            return false;
        }
        let id = new_id();
        let now = now_rfc3339();
        let title = path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("file")
            .to_string();
        let Ok(tx) = self.conn.transaction() else {
            return false;
        };
        let result = (|| -> rusqlite::Result<()> {
            tx.execute(
                "INSERT INTO entries (id, created_at, updated_at, title, categories, source_name, source_path, copy_count, content_hash)
                 VALUES (?1, ?2, ?2, ?3, '[\"file\"]', ?4, '', 1, ?5)",
                params![id, now, title, source.unwrap_or(""), hash],
            )?;
            tx.execute(
                "INSERT INTO parts (id, entry_id, kind, mime, text, file_path) VALUES (?1, ?2, 'file', '', '', ?3)",
                params![format!("{id}:f"), id, path.display().to_string()],
            )?;
            tx.execute(
                "INSERT INTO parts_fts (text, entry_id) VALUES (?1, ?2)",
                params![title, id],
            )?;
            Ok(())
        })();
        result.is_ok() && tx.commit().is_ok()
    }

    fn bump_hash(&self, hash: &str) -> bool {
        let now = now_rfc3339();
        let n = self
            .conn
            .execute(
                "UPDATE entries SET updated_at = ?1, copy_count = COALESCE(copy_count, 1) + 1
                 WHERE content_hash = ?2 AND id = (
                     SELECT id FROM entries WHERE content_hash = ?2 ORDER BY COALESCE(updated_at, created_at) DESC LIMIT 1
                 )",
                params![now, hash],
            )
            .unwrap_or(0);
        n > 0
    }

    pub fn pending_ocr(&self) -> rusqlite::Result<Vec<crate::ocr::Job>> {
        let mut stmt = self.conn.prepare(
            "SELECT e.id, p.file_path FROM entries e JOIN parts p ON p.entry_id = e.id
             WHERE p.kind = 'image' AND p.file_path != ''
             AND NOT EXISTS (SELECT 1 FROM parts done WHERE done.entry_id = e.id AND done.kind IN ('ocr', 'ocr_error'))
             GROUP BY e.id ORDER BY COALESCE(e.updated_at, e.created_at) DESC LIMIT 8"
        )?;
        stmt.query_map([], |r| {
            let raw: String = r.get(1)?;
            Ok(crate::ocr::Job {
                id: r.get(0)?,
                path: recovered_asset_path(&raw),
            })
        })?
        .collect()
    }

    pub fn finish_ocr(&mut self, result: &crate::ocr::Completed) -> rusqlite::Result<()> {
        let tx = self.conn.transaction()?;
        let (kind, text) = match &result.text {
            Ok(text) => ("ocr", text),
            Err(error) => ("ocr_error", error),
        };
        let inserted = tx.execute(
            "INSERT OR IGNORE INTO parts (id, entry_id, kind, mime, text, file_path) VALUES (?1, ?2, ?3, 'text/plain', ?4, '')",
            params![format!("{}:ocr", result.id), result.id, kind, text],
        )?;
        if let Ok(text) = &result.text && inserted > 0 {
            tx.execute(
                "INSERT INTO parts_fts (text, entry_id) VALUES (?1, ?2)",
                params![text, result.id],
            )?;
        }
        tx.commit()
    }
}

fn row_hit(r: &rusqlite::Row<'_>) -> rusqlite::Result<Hit> {
    let categories: String = r.get(4)?;
    let kind = if categories.contains("image") {
        "image"
    } else if categories.contains("file") {
        "file"
    } else if categories.contains("link") {
        "link"
    } else {
        "text"
    };
    Ok(Hit {
        id: r.get(0)?,
        title: r.get(1)?,
        created_at: r.get(2)?,
        source: r.get(3)?,
        kind: kind.into(),
    })
}

fn fts_query(query: &str) -> String {
    query
        .split_whitespace()
        .filter_map(|token| {
            let clean: String = token
                .chars()
                .filter(|c| c.is_alphanumeric() || *c == '_' || *c == '-' || *c == '.' || *c == '@')
                .collect();
            if clean.is_empty() {
                None
            } else {
                Some(format!("\"{clean}\"*"))
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn hex_hash(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hasher
        .finalize()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

fn now_rfc3339() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let days = (secs / 86_400) as i64;
    let tod = secs % 86_400;
    let (y, m, d) = civil_from_days(days);
    format!(
        "{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}Z",
        tod / 3600,
        (tod / 60) % 60,
        tod % 60
    )
}

fn civil_from_days(z: i64) -> (i32, u32, u32) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y as i32, m as u32, d as u32)
}

fn new_id() -> String {
    static SEQUENCE: AtomicU64 = AtomicU64::new(0);
    let ns = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let sequence = SEQUENCE.fetch_add(1, Ordering::Relaxed);
    format!("c{ns:032x}-{:08x}-{sequence:016x}", std::process::id())
}

fn recovered_asset_path(raw: &str) -> PathBuf {
    let path = PathBuf::from(raw);
    if path.exists() {
        return path;
    }
    // Resolve assets moved after the first export, without editing the database
    // currently used by the running app.
    if raw.contains("/Library/Application%20Support/Clip/files/") {
        if let Some(name) = path.file_name() {
            let recovered = data_dir().join("import/files").join(name);
            if recovered.exists() {
                return recovered;
            }
        }
    }
    path
}

pub fn format_when(raw: &str) -> String {
    if raw.len() >= 16 && raw.as_bytes().get(10) == Some(&b'T') {
        return format!("{} {}", &raw[0..10], &raw[11..16]);
    }
    raw.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Fixture {
        store: Store,
        dir: PathBuf,
    }

    impl Fixture {
        fn new() -> Self {
            let dir = std::env::temp_dir().join(format!("clip-test-{}", new_id()));
            let store = Store::open_in(&dir).unwrap();
            Self { store, dir }
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            // Only this fixture's uniquely named directory is removed.
            let _ = std::fs::remove_dir_all(&self.dir);
        }
    }

    #[test]
    fn preserves_whitespace_and_link_content() {
        let mut fixture = Fixture::new();
        let text = "  indented code\n\n";
        assert!(fixture.store.insert_text(text, Some("Editor")));
        let hits = fixture.store.search("indented", 10);
        assert_eq!(hits.len(), 1);
        assert_eq!(fixture.store.preview(&hits[0].id).unwrap().text, text);
        assert!(fixture.store.insert_text("https://example.com/path", None));
        let hits = fixture.store.search("example", 10);
        assert_eq!(hits.len(), 1);
        assert_eq!(
            fixture.store.preview(&hits[0].id).unwrap().text,
            "https://example.com/path"
        );
    }

    #[test]
    fn rapid_inserts_never_overwrite_each_other() {
        let mut fixture = Fixture::new();
        for i in 0..500 {
            assert!(fixture.store.insert_text(&format!("burst item {i}"), None));
        }
        assert_eq!(fixture.store.recent(1000).len(), 500);
        let count: i64 = fixture
            .store
            .conn
            .query_row("SELECT count(*) FROM parts", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 500);
    }

    #[test]
    fn repeats_deduplicate_without_losing_content() {
        let mut fixture = Fixture::new();
        assert!(fixture.store.insert_text("repeated", None));
        assert!(!fixture.store.insert_text("repeated", None));
        assert_eq!(fixture.store.recent(10).len(), 1);
        let copies: i64 = fixture
            .store
            .conn
            .query_row("SELECT copy_count FROM entries", [], |r| r.get(0))
            .unwrap();
        assert_eq!(copies, 2);
    }

    #[test]
    fn failed_part_write_rolls_back_entry_and_search_index() {
        let mut fixture = Fixture::new();
        fixture.store.conn.execute_batch(
            "CREATE TRIGGER reject_part BEFORE INSERT ON parts BEGIN SELECT RAISE(ABORT, 'disk-write-fault'); END;"
        ).unwrap();
        assert!(
            !fixture
                .store
                .insert_text("must not survive partially", None)
        );
        assert!(fixture.store.recent(10).is_empty());
        let count: i64 = fixture
            .store
            .conn
            .query_row("SELECT count(*) FROM parts_fts", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn literal_underscore_does_not_match_arbitrary_titles() {
        let mut fixture = Fixture::new();
        fixture.store.insert_text("alphaXbeta", None);
        fixture.store.insert_text("alpha_beta", None);
        let hits = fixture.store.search("alpha_beta", 10);
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].title, "alpha_beta");
        assert!(fixture.store.search("no_such_value", 10).is_empty());
    }

    #[test]
    fn punctuation_cannot_inject_fts_syntax() {
        let mut fixture = Fixture::new();
        fixture.store.insert_text("needle document", None);
        for query in ["needle\"", "needle*", "needle)", "needle:"] {
            assert_eq!(fixture.store.search(query, 10).len(), 1, "{query}");
        }
    }

    #[test]
    fn ocr_is_searchable_without_reading_image_data() {
        let fixture = Fixture::new();
        fixture.store.conn.execute(
            "INSERT INTO entries (id, created_at, title, categories) VALUES ('scan', '2026-01-01T00:00:00Z', 'screenshot.png', '[\"image\"]')", [],
        ).unwrap();
        fixture
            .store
            .conn
            .execute(
                "INSERT INTO parts_fts(text, entry_id) VALUES ('Invoice asparagus', 'scan')",
                [],
            )
            .unwrap();
        let hits = fixture.store.search("aspar", 10);
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].id, "scan");
    }

    #[test]
    fn recent_uses_recency_index() {
        let fixture = Fixture::new();
        let plan: String = fixture.store.conn.query_row(
            "EXPLAIN QUERY PLAN SELECT id FROM entries ORDER BY COALESCE(updated_at, created_at) DESC, id DESC LIMIT 80", [], |r| r.get(3),
        ).unwrap();
        assert!(plan.contains("entries_recency"), "{plan}");
    }

    #[test]
    fn ocr_completion_indexes_once_and_failed_jobs_do_not_loop() {
        let mut fixture = Fixture::new();
        assert!(fixture.store.insert_image(b"synthetic fixture, not decoded in this test", "png", None));
        let job = fixture.store.pending_ocr().unwrap().remove(0);
        let completed = crate::ocr::Completed { id: job.id, text: Ok("screenshot bananas".into()) };
        fixture.store.finish_ocr(&completed).unwrap();
        fixture.store.finish_ocr(&completed).unwrap();
        assert!(fixture.store.pending_ocr().unwrap().is_empty());
        assert_eq!(fixture.store.search("bananas", 10).len(), 1);
        let count: i64 = fixture.store.conn.query_row("SELECT count(*) FROM parts_fts", [], |r| r.get(0)).unwrap();
        assert_eq!(count, 1);

        assert!(fixture.store.insert_image(b"second synthetic fixture", "png", None));
        let job = fixture.store.pending_ocr().unwrap().remove(0);
        fixture.store.finish_ocr(&crate::ocr::Completed { id: job.id, text: Err("not an image".into()) }).unwrap();
        assert!(fixture.store.pending_ocr().unwrap().is_empty());
    }

    #[test]
    fn civil_dates_include_epoch_and_leap_day() {
        assert_eq!(civil_from_days(0), (1970, 1, 1));
        assert_eq!(civil_from_days(19_782), (2024, 2, 29));
    }
}

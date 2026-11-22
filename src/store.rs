use std::os::unix::fs::PermissionsExt;
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

const IMAGE_EXTENSIONS: &[&str] = &[
    "png", "jpg", "jpeg", "gif", "tiff", "tif", "heic", "heif", "webp", "bmp", "ico", "avif",
];

pub fn data_dir() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    PathBuf::from(home).join("Library/Application Support/Clip")
}

impl Store {
    pub fn thumbnail_path(&self, id: &str) -> Option<PathBuf> {
        let mut statement = self.conn.prepare(
            "SELECT kind, file_path FROM parts WHERE entry_id=?1 AND kind IN ('image', 'file') AND file_path IS NOT NULL AND file_path<>'' ORDER BY CASE kind WHEN 'image' THEN 0 ELSE 1 END, rowid",
        ).ok()?;
        let parts = statement
            .query_map([id], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })
            .ok()?;
        parts.filter_map(Result::ok).find_map(|(kind, path)| {
            if kind == "image" {
                return Some(recovered_asset_path(&path));
            }
            let path = PathBuf::from(path);
            let extension = path.extension()?.to_str()?.to_ascii_lowercase();
            IMAGE_EXTENSIONS
                .contains(&extension.as_str())
                .then_some(path)
        })
    }

    pub fn list_title(&self, hit: &Hit) -> String {
        let title = display_title(&hit.title);
        if !title.is_empty() {
            return title;
        }
        let title = self
            .preview(&hit.id)
            .map(|p| display_title(&p.text))
            .unwrap_or_default();
        if !title.is_empty() {
            title
        } else {
            match hit.kind.as_str() {
                "image" => "Image",
                "file" => "File",
                "link" => "Link",
                _ => "Empty clip",
            }
            .into()
        }
    }
    pub fn selected_previews(
        &self,
        ids: &std::collections::HashSet<String>,
    ) -> Result<Vec<Preview>, String> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let placeholders = std::iter::repeat_n("?", ids.len())
            .collect::<Vec<_>>()
            .join(",");
        let sql = format!(
            "SELECT id FROM entries WHERE id IN ({placeholders}) ORDER BY COALESCE(updated_at, created_at), rowid"
        );
        let mut statement = self.conn.prepare(&sql).map_err(|e| e.to_string())?;
        let ordered = statement
            .query_map(rusqlite::params_from_iter(ids.iter()), |row| {
                row.get::<_, String>(0)
            })
            .map_err(|e| e.to_string())?;
        let mut previews = Vec::new();
        for id in ordered {
            let id = id.map_err(|e| e.to_string())?;
            previews.push(self.preview(&id).ok_or("A selected clip is unavailable")?);
        }
        if previews.len() != ids.len() {
            return Err("A selected clip was removed".into());
        }
        Ok(previews)
    }

    pub fn open() -> Result<Self, String> {
        let dir = data_dir();
        Self::open_in(&dir)
    }

    fn open_in(dir: &Path) -> Result<Self, String> {
        std::fs::create_dir_all(dir.join("files")).map_err(|e| e.to_string())?;
        std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o700))
            .map_err(|e| e.to_string())?;
        let imported = dir.join("import/history.db");
        let live = dir.join("history.db");
        if !live.exists() && imported.exists() {
            let source =
                Connection::open_with_flags(&imported, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
                    .map_err(|e| e.to_string())?;
            let staging = dir.join(format!("{}.db", new_id()));
            source
                .backup("main", &staging, None::<fn(rusqlite::backup::Progress)>)
                .map_err(|e| e.to_string())?;
            std::fs::rename(&staging, &live).map_err(|e| e.to_string())?;
        }
        let path = live;
        let conn = Connection::open(&path).map_err(|e| e.to_string())?;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))
            .map_err(|e| e.to_string())?;
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
            CREATE VIRTUAL TABLE IF NOT EXISTS entry_search USING fts5(title, text, entry_id UNINDEXED, tokenize='unicode61', prefix='2 3');
            CREATE TABLE IF NOT EXISTS entry_search_map(entry_id TEXT PRIMARY KEY, seq INTEGER NOT NULL UNIQUE);
            ",
        )
        .map_err(|e| e.to_string())?;
        let indexed: i64 = conn
            .query_row("SELECT count(*) FROM entry_search_map", [], |r| r.get(0))
            .map_err(|e| e.to_string())?;
        if indexed == 0 {
            let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
            {
                let mut stmt = tx
                    .prepare("SELECT id FROM entries ORDER BY COALESCE(updated_at, created_at), id")
                    .map_err(|e| e.to_string())?;
                let rows = stmt
                    .query_map([], |r| r.get::<_, String>(0))
                    .map_err(|e| e.to_string())?;
                for row in rows {
                    refresh_search_entry(&tx, &row.map_err(|e| e.to_string())?)
                        .map_err(|e| e.to_string())?;
                }
            }
            tx.commit().map_err(|e| e.to_string())?;
        }
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
        let mut stmt = self
            .conn
            .prepare(
                "SELECT e.id, COALESCE(e.title, ''), e.created_at, COALESCE(e.source_name, ''), COALESCE(e.categories, '')
                 FROM entry_search s JOIN entries e ON e.id = s.entry_id
                 WHERE entry_search MATCH ?1
                 ORDER BY s.rowid DESC LIMIT ?2",
            )
            .expect("search");
        stmt.query_map(params![fts, limit as i64], row_hit)
            .map(|rows| rows.filter_map(|r| r.ok()).collect())
            .unwrap_or_else(|error| {
                eprintln!("clip: search failed: {error}");
                Vec::new()
            })
    }

    /// Apply type constraints before LIMIT, across the database rather than
    /// filtering the cached recent/search page. Image files count as images
    /// without changing their native clipboard payload type.
    pub fn filtered(&self, query: &str, limit: usize, kind: Option<&str>) -> Vec<Hit> {
        let image_files = IMAGE_EXTENSIONS
            .iter()
            .map(|extension| format!("lower(p.file_path) LIKE '%.{extension}'"))
            .collect::<Vec<_>>()
            .join(" OR ");
        let predicate = match kind {
            Some("image") => format!(
                "EXISTS (SELECT 1 FROM parts p WHERE p.entry_id=e.id AND (p.kind='image' OR (p.kind='file' AND ({image_files}))))"
            ),
            Some("file") => {
                "EXISTS (SELECT 1 FROM parts p WHERE p.entry_id=e.id AND p.kind='file')".into()
            }
            Some("link") => {
                "EXISTS (SELECT 1 FROM parts p WHERE p.entry_id=e.id AND p.kind='link')".into()
            }
            Some("text") => {
                "EXISTS (SELECT 1 FROM parts p WHERE p.entry_id=e.id AND p.kind='text')".into()
            }
            _ => "1".into(),
        };
        let fts = fts_query(query);
        let (join, search, order) = if fts.is_empty() {
            (
                "",
                "?1 = ''",
                "COALESCE(e.updated_at,e.created_at) DESC,e.id DESC",
            )
        } else {
            (
                "JOIN entry_search s ON s.entry_id=e.id",
                "entry_search MATCH ?1",
                "s.rowid DESC",
            )
        };
        let sql = format!(
            "SELECT e.id,COALESCE(e.title,''),e.created_at,COALESCE(e.source_name,''),COALESCE(e.categories,'') FROM entries e {join} WHERE {search} AND ({predicate}) ORDER BY {order} LIMIT ?2"
        );
        let mut stmt = self.conn.prepare(&sql).expect("filtered history");
        stmt.query_map(params![fts, limit as i64], row_hit)
            .map(|rows| rows.filter_map(Result::ok).collect())
            .unwrap_or_default()
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
        let title = display_title(text);
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
            refresh_search_entry(&tx, &id)?;
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
            refresh_search_entry(&tx, &id)?;
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
            refresh_search_entry(&tx, &id)?;
            Ok(())
        })();
        result.is_ok() && tx.commit().is_ok()
    }

    fn bump_hash(&self, hash: &str) -> bool {
        let Ok(tx) = self.conn.unchecked_transaction() else {
            return false;
        };
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
        if n == 0 {
            return false;
        }
        let id: rusqlite::Result<String> = tx.query_row("SELECT id FROM entries WHERE content_hash = ?1 ORDER BY COALESCE(updated_at, created_at) DESC LIMIT 1", [hash], |r| r.get(0));
        let Ok(id) = id else {
            return false;
        };
        let updated = (|| -> rusqlite::Result<()> {
            tx.execute("DELETE FROM entry_search WHERE rowid = (SELECT seq FROM entry_search_map WHERE entry_id = ?1)", [&id])?;
            tx.execute("DELETE FROM entry_search_map WHERE entry_id = ?1", [&id])?;
            refresh_search_entry(&tx, &id)
        })();
        updated.is_ok() && tx.commit().is_ok()
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
        if let Ok(text) = &result.text
            && inserted > 0
        {
            tx.execute(
                "INSERT INTO parts_fts (text, entry_id) VALUES (?1, ?2)",
                params![text, result.id],
            )?;
        }
        refresh_search_entry(&tx, &result.id)?;
        tx.commit()
    }
}

fn refresh_search_entry(conn: &Connection, id: &str) -> rusqlite::Result<()> {
    let previous_seq = conn
        .query_row(
            "SELECT seq FROM entry_search_map WHERE entry_id = ?1",
            [id],
            |r| r.get::<_, i64>(0),
        )
        .ok();
    conn.execute("DELETE FROM entry_search WHERE rowid = (SELECT seq FROM entry_search_map WHERE entry_id = ?1)", [id])?;
    conn.execute("DELETE FROM entry_search_map WHERE entry_id = ?1", [id])?;
    conn.execute(
        "INSERT INTO entry_search(rowid, title, text, entry_id)
         SELECT ?2, COALESCE(title, ''), COALESCE((SELECT group_concat(text, char(10)) FROM parts WHERE entry_id = e.id AND kind IN ('text', 'link', 'ocr')), ''), id
         FROM entries e WHERE id = ?1", params![id, previous_seq],
    )?;
    let seq = conn.last_insert_rowid();
    conn.execute(
        "INSERT INTO entry_search_map(entry_id, seq) VALUES (?1, ?2)",
        params![id, seq],
    )?;
    Ok(())
}

fn display_title(text: &str) -> String {
    text.trim_start()
        .lines()
        .next()
        .unwrap_or("")
        .trim_end()
        .chars()
        .take(180)
        .collect()
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
    fn image_filter_includes_image_files_and_searches_beyond_the_recent_page() {
        let mut fixture = Fixture::new();
        let path = fixture.store.files.join("needle-screenshot.PNG");
        std::fs::write(&path, b"synthetic file").unwrap();
        fixture.store.insert_file(&path, None);
        for i in 0..220 {
            fixture.store.insert_text(&format!("needle text {i}"), None);
        }
        assert!(
            fixture
                .store
                .recent(200)
                .iter()
                .all(|hit| hit.kind == "text")
        );
        let images = fixture.store.filtered("", 80, Some("image"));
        assert_eq!(images.len(), 1);
        assert_eq!(images[0].title, "needle-screenshot.PNG");
        assert_eq!(images[0].kind, "file");
        assert_eq!(fixture.store.filtered("needle", 80, Some("image")).len(), 1);
        assert!(
            fixture
                .store
                .filtered("absent", 80, Some("image"))
                .is_empty()
        );
    }

    #[test]
    fn image_files_get_thumbnails_without_changing_their_paste_type() {
        let mut fixture = Fixture::new();
        for name in [
            "screenshot.png",
            "IMG_8176.PNG",
            "photo.jpeg",
            "photo.HEIC",
            "notes.txt",
        ] {
            let path = fixture.store.files.join(name);
            std::fs::write(&path, b"synthetic path fixture").unwrap();
            assert!(fixture.store.insert_file(&path, None));
            let hit = fixture
                .store
                .search(name.split('.').next().unwrap(), 10)
                .into_iter()
                .find(|hit| hit.title == name)
                .unwrap();
            let thumbnail = fixture.store.thumbnail_path(&hit.id);
            if name.ends_with(".txt") {
                assert!(thumbnail.is_none());
            } else {
                assert_eq!(thumbnail, Some(path.clone()));
            }
            let preview = fixture.store.preview(&hit.id).unwrap();
            assert_eq!(hit.kind, "file");
            assert_eq!(preview.kind, "file");
            assert_eq!(preview.file_path, Some(path));
            assert!(preview.image_path.is_none());
        }
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
                "INSERT INTO parts(id, entry_id, kind, text) VALUES ('scan:ocr', 'scan', 'ocr', 'Invoice asparagus')",
                [],
            )
            .unwrap();
        refresh_search_entry(&fixture.store.conn, "scan").unwrap();
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
    fn display_titles_skip_whitespace_without_changing_pasted_text() {
        let mut fixture = Fixture::new();
        let text = " \t\n\r\n\u{2003}    first real line  \n    keep indentation\n\n";
        fixture.store.insert_text(text, None);
        let hit = fixture.store.recent(1).remove(0);
        assert_eq!(hit.title, "first real line");
        assert_eq!(fixture.store.preview(&hit.id).unwrap().text, text);
        // Old imported entries may already have an empty/whitespace title.
        fixture
            .store
            .conn
            .execute("UPDATE entries SET title=' \t\n' WHERE id=?1", [&hit.id])
            .unwrap();
        let imported = fixture.store.recent(1).remove(0);
        assert_eq!(fixture.store.list_title(&imported), "first real line");
        assert_eq!(fixture.store.preview(&hit.id).unwrap().text, text);
        assert_eq!(display_title("\t  code\nsecond line"), "code");
        assert_eq!(display_title("\n\t "), "");
        assert_eq!(display_title(&"ü".repeat(300)).chars().count(), 180);
    }

    #[test]
    fn batch_paste_orders_by_last_copy_not_selection_or_search_order() {
        let mut fixture = Fixture::new();
        for text in ["first", "second", "third"] {
            fixture.store.insert_text(text, None);
        }
        let hits = fixture.store.recent(10);
        let ids = hits.iter().map(|h| h.id.clone()).collect();
        for (i, word) in ["first", "second", "third"].into_iter().enumerate() {
            let id = &hits.iter().find(|h| h.title == word).unwrap().id;
            fixture
                .store
                .conn
                .execute(
                    "UPDATE entries SET created_at=?1, updated_at=NULL WHERE id=?2",
                    params![format!("2026-01-01T00:00:0{i}Z"), id],
                )
                .unwrap();
        }
        let texts = || {
            fixture
                .store
                .selected_previews(&ids)
                .unwrap()
                .into_iter()
                .map(|p| p.text)
                .collect::<Vec<_>>()
        };
        assert_eq!(texts(), ["first", "second", "third"]);
        let first_id = &hits.iter().find(|h| h.title == "first").unwrap().id;
        fixture
            .store
            .conn
            .execute(
                "UPDATE entries SET updated_at='2026-01-02T00:00:00Z' WHERE id=?1",
                [first_id],
            )
            .unwrap();
        assert_eq!(texts(), ["second", "third", "first"]);
        let missing = ["does-not-exist".into()].into_iter().collect();
        assert!(fixture.store.selected_previews(&missing).is_err());
    }

    #[test]
    fn opening_recovered_history_does_not_modify_the_archive() {
        let mut fixture = Fixture::new();
        fixture.store.insert_text("recovered original", None);
        let dest = fixture.dir.join("fresh-app");
        std::fs::create_dir_all(dest.join("import")).unwrap();
        let archive = dest.join("import/history.db");
        fixture
            .store
            .conn
            .backup("main", &archive, None::<fn(rusqlite::backup::Progress)>)
            .unwrap();
        let mut live = Store::open_in(&dest).unwrap();
        assert!(dest.join("history.db").exists());
        assert_eq!(live.search("recovered", 10).len(), 1);
        live.insert_text("newly captured", None);
        let source = Connection::open(&archive).unwrap();
        let count: i64 = source
            .query_row("SELECT count(*) FROM entries", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 1);
        assert_eq!(live.recent(10).len(), 2);
    }

    #[test]
    fn ocr_completion_indexes_once_and_failed_jobs_do_not_loop() {
        let mut fixture = Fixture::new();
        assert!(fixture.store.insert_image(
            b"synthetic fixture, not decoded in this test",
            "png",
            None
        ));
        let job = fixture.store.pending_ocr().unwrap().remove(0);
        let completed = crate::ocr::Completed {
            id: job.id,
            text: Ok("screenshot bananas".into()),
        };
        fixture.store.finish_ocr(&completed).unwrap();
        fixture.store.finish_ocr(&completed).unwrap();
        assert!(fixture.store.pending_ocr().unwrap().is_empty());
        assert_eq!(fixture.store.search("bananas", 10).len(), 1);
        let count: i64 = fixture
            .store
            .conn
            .query_row("SELECT count(*) FROM parts_fts", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 1);

        assert!(
            fixture
                .store
                .insert_image(b"second synthetic fixture", "png", None)
        );
        let job = fixture.store.pending_ocr().unwrap().remove(0);
        fixture
            .store
            .finish_ocr(&crate::ocr::Completed {
                id: job.id,
                text: Err("not an image".into()),
            })
            .unwrap();
        assert!(fixture.store.pending_ocr().unwrap().is_empty());
    }

    #[test]
    fn recopy_updates_search_order_but_background_ocr_does_not() {
        let mut fixture = Fixture::new();
        fixture.store.insert_text("common first", None);
        fixture.store.insert_text("common second", None);
        assert_eq!(fixture.store.search("common", 10)[0].title, "common second");
        fixture.store.insert_text("common first", None);
        let ordered = fixture.store.search("common", 10);
        assert_eq!(ordered[0].title, "common first");
        let old_id = ordered[1].id.clone();
        fixture
            .store
            .finish_ocr(&crate::ocr::Completed {
                id: old_id,
                text: Ok("common scanned content".into()),
            })
            .unwrap();
        assert_eq!(fixture.store.search("common", 10)[0].title, "common first");
    }

    #[test]
    fn civil_dates_include_epoch_and_leap_day() {
        assert_eq!(civil_from_days(0), (1970, 1, 1));
        assert_eq!(civil_from_days(19_782), (2024, 2, 29));
    }

    #[test]
    #[ignore = "explicit synthetic performance run"]
    fn benchmark_100k_synthetic_entries() {
        let mut fixture = Fixture::new();
        let tx = fixture.store.conn.transaction().unwrap();
        {
            let mut entry = tx.prepare_cached(
                "INSERT INTO entries(id, created_at, updated_at, title, categories) VALUES (?1, '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z', ?2, '[\"text\"]')"
            ).unwrap();
            let mut index = tx
                .prepare_cached("INSERT INTO parts_fts(text, entry_id) VALUES (?1, ?2)")
                .unwrap();
            for n in 0..100_000 {
                let id = format!("synthetic-{n:06}");
                let text = format!("Synthetic clipboard record {n} project bucket{}", n % 1000);
                entry.execute(params![id, text]).unwrap();
                index.execute(params![text, id]).unwrap();
                refresh_search_entry(&tx, &id).unwrap();
            }
        }
        tx.commit().unwrap();
        for (name, query) in [
            ("recent", ""),
            ("rare", "bucket999"),
            ("common", "clipboard"),
            ("miss", "missingvalue"),
        ] {
            let mut times = Vec::new();
            for _ in 0..40 {
                let start = std::time::Instant::now();
                let hits = fixture.store.search(query, 80);
                times.push(start.elapsed().as_micros());
                if name == "miss" {
                    assert!(hits.is_empty());
                } else {
                    assert_eq!(hits.len(), 80);
                }
            }
            times.sort_unstable();
            println!(
                "100k synthetic {name}: p50={}us p95={}us",
                times[20], times[38]
            );
        }
    }
}

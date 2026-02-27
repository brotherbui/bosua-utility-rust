//! Multi-source search engine with SQLite cache and background refresh.
//!
//! Supports searching across media, news, and software sources.
//! Results are cached in SQLite at the configured `SheetsCacheFile` path.
//! Background cache refresh runs via tokio tasks and can be stopped
//! on HTTP server shutdown using a `CancellationToken`.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use chrono::{DateTime, Utc};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;

use crate::errors::BosuaError;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// The kind of search source.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SearchSource {
    Media,
    News,
    Software,
}

impl std::fmt::Display for SearchSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Media => write!(f, "media"),
            Self::News => write!(f, "news"),
            Self::Software => write!(f, "software"),
        }
    }
}

/// A single search result.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SearchResult {
    pub title: String,
    pub url: String,
    pub source: SearchSource,
    pub description: String,
    pub timestamp: DateTime<Utc>,
}

// ---------------------------------------------------------------------------
// SearchEngine
// ---------------------------------------------------------------------------

/// Multi-source search engine backed by a SQLite cache.
pub struct SearchEngine {
    db_path: PathBuf,
    db: Arc<Mutex<Connection>>,
    cancel: CancellationToken,
}

impl SearchEngine {
    /// Create a new `SearchEngine` with the given SQLite database path.
    ///
    /// Initialises the schema if the database does not yet exist.
    pub fn new(db_path: impl Into<PathBuf>) -> Result<Self, BosuaError> {
        let db_path = db_path.into();
        let conn = Connection::open(&db_path)?;
        Self::init_schema(&conn)?;
        Ok(Self {
            db_path,
            db: Arc::new(Mutex::new(conn)),
            cancel: CancellationToken::new(),
        })
    }

    /// Open with an explicit `CancellationToken` (useful for sharing the
    /// token with the HTTP server shutdown logic).
    pub fn with_cancel_token(
        db_path: impl Into<PathBuf>,
        cancel: CancellationToken,
    ) -> Result<Self, BosuaError> {
        let db_path = db_path.into();
        let conn = Connection::open(&db_path)?;
        Self::init_schema(&conn)?;
        Ok(Self {
            db_path,
            db: Arc::new(Mutex::new(conn)),
            cancel,
        })
    }

    /// Return a clone of the cancellation token so callers can trigger
    /// shutdown externally.
    pub fn cancel_token(&self) -> CancellationToken {
        self.cancel.clone()
    }

    /// The path to the backing SQLite database.
    pub fn db_path(&self) -> &Path {
        &self.db_path
    }

    // -- Schema --------------------------------------------------------------

    fn init_schema(conn: &Connection) -> Result<(), BosuaError> {
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS search_cache (
                id          INTEGER PRIMARY KEY AUTOINCREMENT,
                title       TEXT    NOT NULL,
                url         TEXT    NOT NULL,
                source      TEXT    NOT NULL,
                description TEXT    NOT NULL DEFAULT '',
                timestamp   TEXT    NOT NULL,
                created_at  TEXT    NOT NULL DEFAULT (datetime('now'))
            );
            CREATE INDEX IF NOT EXISTS idx_search_cache_source
                ON search_cache(source);",
        )?;
        Ok(())
    }

    // -- Cache operations ----------------------------------------------------

    /// Insert a search result into the cache.
    pub async fn cache_result(&self, result: &SearchResult) -> Result<(), BosuaError> {
        let db = self.db.lock().await;
        db.execute(
            "INSERT INTO search_cache (title, url, source, description, timestamp)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                result.title,
                result.url,
                result.source.to_string(),
                result.description,
                result.timestamp.to_rfc3339(),
            ],
        )?;
        Ok(())
    }

    /// Insert many results in a single transaction.
    pub async fn cache_results(&self, results: &[SearchResult]) -> Result<(), BosuaError> {
        let db = self.db.lock().await;
        let tx = db.unchecked_transaction()?;
        {
            let mut stmt = tx.prepare(
                "INSERT INTO search_cache (title, url, source, description, timestamp)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
            )?;
            for r in results {
                stmt.execute(params![
                    r.title,
                    r.url,
                    r.source.to_string(),
                    r.description,
                    r.timestamp.to_rfc3339(),
                ])?;
            }
        }
        tx.commit()?;
        Ok(())
    }

    /// Query cached results for a given source, optionally filtering by a
    /// case-insensitive substring match on `title` or `description`.
    pub async fn search(
        &self,
        source: SearchSource,
        query: Option<&str>,
    ) -> Result<Vec<SearchResult>, BosuaError> {
        let db = self.db.lock().await;
        let rows = if let Some(q) = query {
            let pattern = format!("%{}%", q);
            let mut stmt = db.prepare(
                "SELECT title, url, source, description, timestamp
                 FROM search_cache
                 WHERE source = ?1
                   AND (title LIKE ?2 OR description LIKE ?2)
                 ORDER BY timestamp DESC",
            )?;
            Self::collect_rows(&mut stmt, params![source.to_string(), pattern])?
        } else {
            let mut stmt = db.prepare(
                "SELECT title, url, source, description, timestamp
                 FROM search_cache
                 WHERE source = ?1
                 ORDER BY timestamp DESC",
            )?;
            Self::collect_rows(&mut stmt, params![source.to_string()])?
        };
        Ok(rows)
    }

    /// Clear all cached results for a given source.
    pub async fn clear_source(&self, source: SearchSource) -> Result<(), BosuaError> {
        let db = self.db.lock().await;
        db.execute(
            "DELETE FROM search_cache WHERE source = ?1",
            params![source.to_string()],
        )?;
        Ok(())
    }

    // -- Background refresh --------------------------------------------------

    /// Spawn a background task that periodically refreshes the cache.
    ///
    /// The task runs until the cancellation token is triggered (e.g. on HTTP
    /// server shutdown). `interval` controls how often the refresh fires.
    pub fn start_background_refresh(
        &self,
        interval: std::time::Duration,
    ) -> tokio::task::JoinHandle<()> {
        let cancel = self.cancel.clone();
        let db = Arc::clone(&self.db);
        let db_path = self.db_path.clone();

        tokio::spawn(async move {
            tracing::info!(
                path = %db_path.display(),
                "search cache background refresh started"
            );
            loop {
                tokio::select! {
                    _ = cancel.cancelled() => {
                        tracing::info!("search cache background refresh stopped");
                        break;
                    }
                    _ = tokio::time::sleep(interval) => {
                        tracing::debug!("search cache refresh tick");
                        // Actual refresh logic (fetching from external sources)
                        // will be wired in when the real search providers are
                        // implemented. For now this is a no-op tick.
                        let _db = db.lock().await;
                        // placeholder: future tasks will add real refresh logic
                    }
                }
            }
        })
    }

    /// Stop background refresh by cancelling the token.
    pub fn stop_background_refresh(&self) {
        self.cancel.cancel();
    }

    // -- Helpers -------------------------------------------------------------

    fn collect_rows(
        stmt: &mut rusqlite::Statement<'_>,
        params: impl rusqlite::Params,
    ) -> Result<Vec<SearchResult>, BosuaError> {
        let rows = stmt.query_map(params, |row| {
            let source_str: String = row.get(2)?;
            let ts_str: String = row.get(4)?;
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                source_str,
                row.get::<_, String>(3)?,
                ts_str,
            ))
        })?;

        let mut results = Vec::new();
        for row in rows {
            let (title, url, source_str, description, ts_str) = row?;
            let source = match source_str.as_str() {
                "media" => SearchSource::Media,
                "news" => SearchSource::News,
                "software" => SearchSource::Software,
                other => {
                    tracing::warn!(source = other, "unknown search source in cache, skipping");
                    continue;
                }
            };
            let timestamp = DateTime::parse_from_rfc3339(&ts_str)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now());
            results.push(SearchResult {
                title,
                url,
                source,
                description,
                timestamp,
            });
        }
        Ok(results)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn temp_db_path() -> PathBuf {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let id = COUNTER.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir();
        dir.join(format!(
            "bosua_search_test_{}_{}.db",
            std::process::id(),
            id
        ))
    }

    fn make_result(title: &str, source: SearchSource) -> SearchResult {
        SearchResult {
            title: title.to_string(),
            url: format!("https://example.com/{}", title.to_lowercase().replace(' ', "-")),
            source,
            description: format!("Description for {title}"),
            timestamp: Utc::now(),
        }
    }

    #[test]
    fn test_search_engine_creation() {
        let path = temp_db_path();
        let engine = SearchEngine::new(&path).expect("should create engine");
        assert_eq!(engine.db_path(), path);
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn test_search_engine_with_cancel_token() {
        let path = temp_db_path();
        let token = CancellationToken::new();
        let engine =
            SearchEngine::with_cancel_token(&path, token.clone()).expect("should create engine");
        assert!(!engine.cancel_token().is_cancelled());
        token.cancel();
        assert!(engine.cancel_token().is_cancelled());
        std::fs::remove_file(&path).ok();
    }

    #[tokio::test]
    async fn test_cache_and_search() {
        let path = temp_db_path();
        let engine = SearchEngine::new(&path).unwrap();

        let r1 = make_result("Rust Tutorial", SearchSource::Media);
        let r2 = make_result("Rust News", SearchSource::News);
        let r3 = make_result("Cargo Tool", SearchSource::Software);

        engine.cache_result(&r1).await.unwrap();
        engine.cache_result(&r2).await.unwrap();
        engine.cache_result(&r3).await.unwrap();

        let media = engine.search(SearchSource::Media, None).await.unwrap();
        assert_eq!(media.len(), 1);
        assert_eq!(media[0].title, "Rust Tutorial");

        let news = engine.search(SearchSource::News, None).await.unwrap();
        assert_eq!(news.len(), 1);

        let sw = engine.search(SearchSource::Software, None).await.unwrap();
        assert_eq!(sw.len(), 1);
        assert_eq!(sw[0].title, "Cargo Tool");

        std::fs::remove_file(&path).ok();
    }

    #[tokio::test]
    async fn test_search_with_query_filter() {
        let path = temp_db_path();
        let engine = SearchEngine::new(&path).unwrap();

        engine
            .cache_result(&make_result("Rust Tutorial", SearchSource::Media))
            .await
            .unwrap();
        engine
            .cache_result(&make_result("Go Tutorial", SearchSource::Media))
            .await
            .unwrap();

        let results = engine
            .search(SearchSource::Media, Some("Rust"))
            .await
            .unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].title, "Rust Tutorial");

        std::fs::remove_file(&path).ok();
    }

    #[tokio::test]
    async fn test_cache_results_batch() {
        let path = temp_db_path();
        let engine = SearchEngine::new(&path).unwrap();

        let batch = vec![
            make_result("Item 1", SearchSource::News),
            make_result("Item 2", SearchSource::News),
            make_result("Item 3", SearchSource::News),
        ];
        engine.cache_results(&batch).await.unwrap();

        let results = engine.search(SearchSource::News, None).await.unwrap();
        assert_eq!(results.len(), 3);

        std::fs::remove_file(&path).ok();
    }

    #[tokio::test]
    async fn test_clear_source() {
        let path = temp_db_path();
        let engine = SearchEngine::new(&path).unwrap();

        engine
            .cache_result(&make_result("A", SearchSource::Media))
            .await
            .unwrap();
        engine
            .cache_result(&make_result("B", SearchSource::News))
            .await
            .unwrap();

        engine.clear_source(SearchSource::Media).await.unwrap();

        let media = engine.search(SearchSource::Media, None).await.unwrap();
        assert!(media.is_empty());

        // News should be untouched
        let news = engine.search(SearchSource::News, None).await.unwrap();
        assert_eq!(news.len(), 1);

        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn test_search_result_serialization() {
        let result = make_result("Test", SearchSource::Software);
        let json = serde_json::to_string(&result).unwrap();
        let deserialized: SearchResult = serde_json::from_str(&json).unwrap();
        assert_eq!(result.title, deserialized.title);
        assert_eq!(result.url, deserialized.url);
        assert_eq!(result.source, deserialized.source);
        assert_eq!(result.description, deserialized.description);
    }

    #[test]
    fn test_search_source_display() {
        assert_eq!(SearchSource::Media.to_string(), "media");
        assert_eq!(SearchSource::News.to_string(), "news");
        assert_eq!(SearchSource::Software.to_string(), "software");
    }

    #[tokio::test]
    async fn test_stop_background_refresh() {
        let path = temp_db_path();
        let engine = SearchEngine::new(&path).unwrap();

        let handle = engine.start_background_refresh(std::time::Duration::from_secs(3600));
        engine.stop_background_refresh();

        // The task should complete quickly after cancellation
        tokio::time::timeout(std::time::Duration::from_secs(2), handle)
            .await
            .expect("background task should stop within timeout")
            .expect("task should not panic");

        std::fs::remove_file(&path).ok();
    }
}

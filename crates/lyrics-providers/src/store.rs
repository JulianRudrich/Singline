//! Everything the app keeps about songs, in one SQLite file: your own lyrics,
//! per-song timing corrections, the cache of online results and the listening
//! history. Nothing in it leaves the device.

use std::path::Path;

use lyrics_core::listening::ListenSession;
use lyrics_core::{lrc, LyricLine};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{de::DeserializeOwned, Serialize};
use thiserror::Error;

/// The largest lyrics file accepted for one track, in UTF-8 bytes.
pub const MAX_LYRICS_BYTES: usize = 512 * 1_024;

/// How many online results are kept; the least recently used go first.
const CACHE_CAPACITY: i64 = 2_000;

/// Each step brings the database from the version before it to its own
/// version, the step's position counted from one. Steps only add, so what an
/// older version wrote stays.
const MIGRATIONS: [&str; 2] = [
    // 1: your own lyrics, timing corrections and the cache of online results.
    "CREATE TABLE IF NOT EXISTS track_settings (
         track_key TEXT PRIMARY KEY,
         timing_offset REAL NOT NULL,
         updated_at INTEGER NOT NULL
     );
     CREATE TABLE IF NOT EXISTS local_lyrics (
         track_key TEXT PRIMARY KEY,
         lrc TEXT NOT NULL,
         updated_at INTEGER NOT NULL
     );
     CREATE TABLE IF NOT EXISTS lyrics_cache (
         query_key TEXT PRIMARY KEY,
         result TEXT NOT NULL,
         expires_at INTEGER NOT NULL,
         last_used INTEGER NOT NULL
     );
     CREATE INDEX IF NOT EXISTS lyrics_cache_last_used ON lyrics_cache (last_used);",
    // 2: the listening history, one row per song heard.
    "CREATE TABLE IF NOT EXISTS listens (
         id INTEGER PRIMARY KEY,
         track_key TEXT NOT NULL,
         title TEXT NOT NULL,
         artist TEXT NOT NULL,
         album TEXT,
         service TEXT NOT NULL,
         started_at INTEGER NOT NULL,
         duration REAL,
         listened REAL NOT NULL,
         synced_lyrics INTEGER NOT NULL
     );
     CREATE INDEX IF NOT EXISTS listens_started_at ON listens (started_at);",
];

/// The schema version this code writes.
const SCHEMA_VERSION: i32 = MIGRATIONS.len() as i32;

#[derive(Debug, Error)]
pub enum StoreError {
    #[error("the song database is not available: {0}")]
    Database(#[from] rusqlite::Error),
    #[error("the song database was written by a newer version of Singline")]
    NewerSchema,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ImportError {
    #[error("the lyrics contain no line with an LRC timestamp")]
    MissingTimedLyrics,
    #[error("the lyrics are larger than 512 KB")]
    TooLarge,
    #[error("the lyrics could not be saved: {0}")]
    Storage(String),
}

pub struct Store {
    connection: Connection,
}

impl Store {
    pub fn open(path: &Path) -> Result<Self, StoreError> {
        if let Some(directory) = path.parent() {
            // A missing directory surfaces as an open error below.
            let _ = std::fs::create_dir_all(directory);
        }
        Self::prepare(Connection::open(path)?)
    }

    pub fn open_in_memory() -> Result<Self, StoreError> {
        Self::prepare(Connection::open_in_memory()?)
    }

    fn prepare(mut connection: Connection) -> Result<Self, StoreError> {
        connection.busy_timeout(std::time::Duration::from_secs(2))?;
        connection.pragma_update(None, "journal_mode", "WAL")?;
        let version: i32 = connection.pragma_query_value(None, "user_version", |row| row.get(0))?;
        if version > SCHEMA_VERSION {
            return Err(StoreError::NewerSchema);
        }
        for (step, target) in MIGRATIONS.iter().zip(1..) {
            if version < target {
                // One transaction per step, so a failed update leaves the
                // database as the version before it.
                let transaction = connection.transaction()?;
                transaction.execute_batch(step)?;
                transaction.pragma_update(None, "user_version", target)?;
                transaction.commit()?;
            }
        }
        Ok(Self { connection })
    }

    /// Seconds the lyrics of this track are moved; positive shows them earlier.
    pub fn timing_offset(&self, track_key: &str) -> f64 {
        self.connection
            .query_row(
                "SELECT timing_offset FROM track_settings WHERE track_key = ?1",
                params![track_key],
                |row| row.get::<_, f64>(0),
            )
            .optional()
            .ok()
            .flatten()
            .filter(|offset| offset.is_finite())
            .unwrap_or(0.0)
    }

    /// Stores a correction for one track. Zero removes it.
    pub fn set_timing_offset(&self, track_key: &str, offset: f64) -> Result<(), StoreError> {
        if !offset.is_finite() {
            return Ok(());
        }
        if offset == 0.0 {
            self.connection.execute(
                "DELETE FROM track_settings WHERE track_key = ?1",
                params![track_key],
            )?;
        } else {
            self.connection.execute(
                "INSERT INTO track_settings (track_key, timing_offset, updated_at) VALUES (?1, ?2, ?3)
                 ON CONFLICT (track_key) DO UPDATE SET timing_offset = excluded.timing_offset, updated_at = excluded.updated_at",
                params![track_key, offset, unix_now()],
            )?;
        }
        Ok(())
    }

    /// Your own lyrics for this track, as LRC text.
    pub fn local_lyrics(&self, track_key: &str) -> Option<String> {
        self.connection
            .query_row(
                "SELECT lrc FROM local_lyrics WHERE track_key = ?1",
                params![track_key],
                |row| row.get(0),
            )
            .optional()
            .ok()
            .flatten()
    }

    /// Saves your own LRC lyrics for a track. A failed import keeps the lyrics
    /// the track had before.
    pub fn import_lyrics(
        &self,
        track_key: &str,
        text: &str,
    ) -> Result<Vec<LyricLine>, ImportError> {
        if text.len() > MAX_LYRICS_BYTES {
            return Err(ImportError::TooLarge);
        }
        let lines = lrc::parse(text);
        if !lines.iter().any(|line| !line.is_break()) {
            return Err(ImportError::MissingTimedLyrics);
        }
        self.connection
            .execute(
                "INSERT INTO local_lyrics (track_key, lrc, updated_at) VALUES (?1, ?2, ?3)
                 ON CONFLICT (track_key) DO UPDATE SET lrc = excluded.lrc, updated_at = excluded.updated_at",
                params![track_key, text, unix_now()],
            )
            .map_err(|error| ImportError::Storage(error.to_string()))?;
        Ok(lines)
    }

    /// Removes your own lyrics for a track. Removing nothing is not an error.
    pub fn remove_local_lyrics(&self, track_key: &str) -> Result<(), StoreError> {
        self.connection.execute(
            "DELETE FROM local_lyrics WHERE track_key = ?1",
            params![track_key],
        )?;
        Ok(())
    }

    /// A cached online result that has not expired at `now` (Unix seconds).
    pub fn cached<T: DeserializeOwned>(&self, query_key: &str, now: i64) -> Option<T> {
        let json: String = self
            .connection
            .query_row(
                "SELECT result FROM lyrics_cache WHERE query_key = ?1 AND expires_at > ?2",
                params![query_key, now],
                |row| row.get(0),
            )
            .optional()
            .ok()
            .flatten()?;
        let _ = self.connection.execute(
            "UPDATE lyrics_cache SET last_used = ?2 WHERE query_key = ?1",
            params![query_key, now],
        );
        serde_json::from_str(&json).ok()
    }

    /// A cached result, also one that is out of date: an old answer still
    /// holds the song's lyrics, as Sing to find needs without a connection.
    pub fn cached_any<T: DeserializeOwned>(&self, query_key: &str) -> Option<T> {
        let json: String = self
            .connection
            .query_row(
                "SELECT result FROM lyrics_cache WHERE query_key = ?1",
                params![query_key],
                |row| row.get(0),
            )
            .optional()
            .ok()
            .flatten()?;
        serde_json::from_str(&json).ok()
    }

    /// Caches an online result until `expires_at` and trims the cache to its capacity.
    pub fn store_cached<T: Serialize>(
        &self,
        query_key: &str,
        result: &T,
        now: i64,
        expires_at: i64,
    ) -> Result<(), StoreError> {
        let json = serde_json::to_string(result)
            .map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))?;
        self.connection.execute(
            "INSERT INTO lyrics_cache (query_key, result, expires_at, last_used) VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT (query_key) DO UPDATE SET result = excluded.result, expires_at = excluded.expires_at, last_used = excluded.last_used",
            params![query_key, json, expires_at, now],
        )?;
        self.connection.execute(
            "DELETE FROM lyrics_cache WHERE expires_at <= ?1 OR query_key IN (
                 SELECT query_key FROM lyrics_cache ORDER BY last_used DESC, query_key LIMIT -1 OFFSET ?2
             )",
            params![now, CACHE_CAPACITY],
        )?;
        Ok(())
    }

    /// Forgets one cached result, for "search again".
    pub fn forget_cached(&self, query_key: &str) -> Result<(), StoreError> {
        self.connection.execute(
            "DELETE FROM lyrics_cache WHERE query_key = ?1",
            params![query_key],
        )?;
        Ok(())
    }

    pub fn clear_cache(&self) -> Result<(), StoreError> {
        self.connection.execute("DELETE FROM lyrics_cache", [])?;
        Ok(())
    }

    pub fn cache_len(&self) -> usize {
        self.connection
            .query_row("SELECT COUNT(*) FROM lyrics_cache", [], |row| {
                row.get::<_, i64>(0)
            })
            .map_or(0, |count| count.max(0) as usize)
    }

    /// Keeps one song of the listening history.
    pub fn add_listen(&self, session: &ListenSession) -> Result<(), StoreError> {
        let listened = if session.listened.is_finite() {
            session.listened.max(0.0)
        } else {
            0.0
        };
        self.connection.execute(
            "INSERT INTO listens (track_key, title, artist, album, service, started_at, duration, listened, synced_lyrics)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                session.track_key,
                session.title,
                session.artist,
                session.album,
                session.service,
                session.started_at,
                session.duration.filter(|duration| duration.is_finite() && *duration > 0.0),
                listened,
                session.synced_lyrics,
            ],
        )?;
        Ok(())
    }

    /// The listening history from `since` on, in Unix seconds, oldest first;
    /// all of it for `None`.
    pub fn listens(&self, since: Option<i64>) -> Result<Vec<ListenSession>, StoreError> {
        let mut statement = self.connection.prepare_cached(
            "SELECT track_key, title, artist, album, service, started_at, duration, listened, synced_lyrics
             FROM listens WHERE started_at >= ?1 ORDER BY started_at, id",
        )?;
        let rows = statement.query_map(params![since.unwrap_or(i64::MIN)], |row| {
            Ok(ListenSession {
                track_key: row.get(0)?,
                title: row.get(1)?,
                artist: row.get(2)?,
                album: row.get(3)?,
                service: row.get(4)?,
                started_at: row.get(5)?,
                duration: row.get(6)?,
                listened: row.get(7)?,
                synced_lyrics: row.get(8)?,
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    /// Deletes the whole listening history. The rows are overwritten on disk,
    /// not only marked as free, and the write-ahead log is emptied.
    pub fn clear_listens(&self) -> Result<(), StoreError> {
        self.connection.pragma_update(None, "secure_delete", true)?;
        let deleted = self.connection.execute("DELETE FROM listens", []);
        let _ = self.connection.pragma_update(None, "secure_delete", false);
        deleted?;
        // An in-memory database has no log to empty.
        let _ = self
            .connection
            .query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |_| Ok(()));
        Ok(())
    }

    /// Whether you have any lyrics of your own.
    pub fn has_local_lyrics(&self) -> bool {
        self.connection
            .query_row("SELECT EXISTS (SELECT 1 FROM local_lyrics)", [], |row| {
                row.get::<_, bool>(0)
            })
            .unwrap_or(false)
    }

    pub fn listen_count(&self) -> usize {
        self.connection
            .query_row("SELECT COUNT(*) FROM listens", [], |row| {
                row.get::<_, i64>(0)
            })
            .map_or(0, |count| count.max(0) as usize)
    }
}

pub(crate) fn unix_now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_secs() as i64)
}

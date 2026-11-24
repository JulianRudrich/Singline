//! Finding lyrics for a track: your own lyrics first, then the local cache,
//! then LRCLIB. For Sing to find, [`search`] finds songs by words of their
//! lyrics, through [`genius`].

pub mod genius;
pub mod lrclib;
pub mod search;
pub mod store;

use std::sync::{Arc, Mutex, MutexGuard};

use lyrics_core::{lrc, Lyrics, SyncLevel};
use serde::de::DeserializeOwned;
use serde::Serialize;

pub use genius::GeniusClient;
pub use lrclib::{LookupError, LookupResult, LrclibClient, Query, SongLookup};
pub use search::{
    find_sung, find_sung_locally, SearchError, SongCandidate, SongSearch, SungMatch, SungSearch,
};
pub use store::{ImportError, Store, StoreError, MAX_LYRICS_BYTES};

const DAY: i64 = 24 * 60 * 60;

/// Where the lyrics for a track came from, or why there are none.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum LyricsOutcome {
    /// Your own lyrics, which always win.
    Local {
        lyrics: Lyrics,
    },
    Online {
        lyrics: Lyrics,
    },
    Instrumental,
    NotFound,
    /// Online search is turned off and the track has no own lyrics.
    LocalOnly,
    Offline,
    RateLimited {
        retry_after_secs: u64,
    },
    Failed {
        message: String,
    },
}

impl LyricsOutcome {
    /// True when trying again later may give a different answer.
    pub fn can_retry(&self) -> bool {
        matches!(
            self,
            LyricsOutcome::NotFound
                | LyricsOutcome::Offline
                | LyricsOutcome::RateLimited { .. }
                | LyricsOutcome::Failed { .. }
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LookupOptions {
    /// Search LRCLIB when the track has no own lyrics.
    pub online: bool,
    /// Skip the cache, for "search again".
    pub refresh: bool,
}

impl Default for LookupOptions {
    fn default() -> Self {
        Self {
            online: true,
            refresh: false,
        }
    }
}

/// The lyrics for a title and artist, with the length and album LRCLIB knows.
#[derive(Debug, Clone, PartialEq)]
pub struct SongLyrics {
    pub outcome: LyricsOutcome,
    pub duration: Option<f64>,
    pub album: Option<String>,
}

pub struct LyricsService {
    client: LrclibClient,
    store: Arc<Mutex<Store>>,
}

impl LyricsService {
    pub fn new(client: LrclibClient, store: Arc<Mutex<Store>>) -> Self {
        Self { client, store }
    }

    fn store(&self) -> MutexGuard<'_, Store> {
        self.store
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// Whether Sing to find can look for songs on this device: among your own
    /// lyrics, and with `kept` among the songs you heard whose lyrics Singline
    /// keeps from LRCLIB.
    pub fn knows_songs(&self, kept: bool) -> bool {
        let store = self.store();
        store.has_local_lyrics() || (kept && store.listen_count() > 0)
    }

    /// Your own lyrics for the track, when it has some with timed text.
    pub fn local(&self, track_key: &str) -> Option<Lyrics> {
        let text = self.store().local_lyrics(track_key)?;
        let lines = lrc::parse(&text);
        lines
            .iter()
            .any(|line| !line.is_break())
            .then(|| Lyrics::timed(lines))
    }

    pub async fn lyrics_for(
        &self,
        track_key: &str,
        query: &Query,
        options: LookupOptions,
    ) -> LyricsOutcome {
        if let Some(lyrics) = self.local(track_key) {
            return LyricsOutcome::Local { lyrics };
        }
        if !options.online {
            return LyricsOutcome::LocalOnly;
        }

        let key = query.cache_key();
        let now = store::unix_now();
        if !options.refresh {
            let cached = self.store().cached::<LookupResult>(&key, now);
            if let Some(result) = cached {
                return outcome(result);
            }
        }
        match self.client.find(query).await {
            Ok(result) => {
                let expires_at = now + lifetime(&result);
                let _ = self.store().store_cached(&key, &result, now, expires_at);
                outcome(result)
            }
            Err(error) => match self.stale::<LookupResult>(&key, options, |result| result) {
                Some(result) => outcome(result),
                None => failure(error),
            },
        }
    }

    /// The answer kept for `key` after its time, for when LRCLIB cannot
    /// answer: lyrics or "instrumental" still hold. "Not found" does not, as
    /// the song may have lyrics by now, and a search the listener asked for
    /// shows why it failed.
    fn stale<T: DeserializeOwned>(
        &self,
        key: &str,
        options: LookupOptions,
        result: fn(&T) -> &LookupResult,
    ) -> Option<T> {
        if options.refresh {
            return None;
        }
        self.store()
            .cached_any::<T>(key)
            .filter(|answer| !matches!(result(answer), LookupResult::NotFound))
    }
}

impl LyricsService {
    /// The lyrics for a song known only by title and artist, such as one
    /// someone sang: your own lyrics under `track_key` first, then the cache,
    /// then LRCLIB.
    pub async fn song_lyrics(
        &self,
        track_key: &str,
        title: &str,
        artist: &str,
        options: LookupOptions,
    ) -> SongLyrics {
        let answer = |outcome| SongLyrics {
            outcome,
            duration: None,
            album: None,
        };
        if let Some(lyrics) = self.local(track_key) {
            return answer(LyricsOutcome::Local { lyrics });
        }
        if !options.online {
            return answer(LyricsOutcome::LocalOnly);
        }
        let key = song_cache_key(title, artist);
        let now = store::unix_now();
        let cached = if options.refresh {
            None
        } else {
            self.store().cached::<SongLookup>(&key, now)
        };
        let found = match cached {
            Some(found) => found,
            None => match self.client.find_song(title, artist).await {
                Ok(found) => {
                    let expires_at = now + lifetime(&found.result);
                    let _ = self.store().store_cached(&key, &found, now, expires_at);
                    found
                }
                Err(error) => {
                    match self.stale::<SongLookup>(&key, options, |found| &found.result) {
                        Some(found) => found,
                        None => return answer(failure(error)),
                    }
                }
            },
        };
        SongLyrics {
            outcome: outcome(found.result),
            duration: found.duration,
            album: found.album,
        }
    }
}

fn failure(error: LookupError) -> LyricsOutcome {
    match error {
        LookupError::Offline => LyricsOutcome::Offline,
        LookupError::RateLimited { retry_after } => LyricsOutcome::RateLimited {
            retry_after_secs: retry_after.as_secs().max(1),
        },
        error => LyricsOutcome::Failed {
            message: error.to_string(),
        },
    }
}

fn outcome(result: LookupResult) -> LyricsOutcome {
    match result {
        LookupResult::Found { lyrics } => LyricsOutcome::Online { lyrics },
        LookupResult::Instrumental => LyricsOutcome::Instrumental,
        LookupResult::NotFound => LyricsOutcome::NotFound,
    }
}

/// How long an online answer is reused. Answers that may improve, such as
/// "not found" or plain text, are asked again sooner.
fn lifetime(result: &LookupResult) -> i64 {
    match result {
        LookupResult::Found { lyrics } if lyrics.sync == SyncLevel::Unsynced => 3 * DAY,
        LookupResult::Found { .. } | LookupResult::Instrumental => 30 * DAY,
        LookupResult::NotFound => DAY,
    }
}

/// The cache key of a song looked up by its title and artist alone.
pub(crate) fn song_cache_key(title: &str, artist: &str) -> String {
    format!(
        "lrclib-song:{}\u{1f}{}",
        lyrics_core::matching::normalized(title),
        lyrics_core::matching::normalized(artist)
    )
}

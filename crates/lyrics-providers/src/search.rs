//! Finding a song from a few of its words, for Sing to find.
//!
//! LRCLIB searches titles and artists, not lyrics, so a [`SongSearch`] provider
//! turns the words someone sang into candidate songs, such as
//! [`crate::genius::GeniusClient`]. [`find_sung`] loads every candidate's
//! lyrics and ranks the candidates by how well they explain what was heard.

use std::collections::HashSet;
use std::future::Future;
use std::pin::Pin;
use std::time::Duration;

use lyrics_core::alignment::{self, Alignment, HeardWord};
use lyrics_core::listening::ListenSession;
use lyrics_core::{Lyrics, SyncLevel};
use serde::Serialize;
use thiserror::Error;

use crate::lrclib::{LookupResult, Query, SongLookup};
use crate::{song_cache_key, LookupOptions, LyricsOutcome, LyricsService};

/// A song a search provider found for some words of its lyrics.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SongCandidate {
    pub title: String,
    pub artist: String,
    pub artwork_url: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum SearchError {
    #[error("the song search needs an access token")]
    NoToken,
    #[error("the song search refused the access token")]
    Unauthorized,
    #[error("no internet connection")]
    Offline,
    #[error("the song search is busy; try again in {} s", .retry_after.as_secs())]
    RateLimited { retry_after: Duration },
    #[error("the song search is not available right now (HTTP {0})")]
    Http(u16),
    #[error("the song search sent an answer that could not be read")]
    InvalidResponse,
    #[error("the request failed: {0}")]
    Network(String),
}

pub type SearchFuture<'a> =
    Pin<Box<dyn Future<Output = Result<Vec<SongCandidate>, SearchError>> + Send + 'a>>;

/// Turns words of a song's lyrics into the songs they may come from.
pub trait SongSearch: Send + Sync {
    /// Short name for diagnostics, such as `genius`.
    fn name(&self) -> &'static str;

    /// Songs whose lyrics may contain `words`, the most likely first. Only the
    /// words are sent, nothing else about the listener.
    fn search<'a>(&'a self, words: &'a str) -> SearchFuture<'a>;
}

/// A candidate with its lyrics and where the heard words stand in them.
#[derive(Debug, Clone, PartialEq)]
pub struct SungMatch {
    pub candidate: SongCandidate,
    pub lyrics: Lyrics,
    /// True for your own lyrics, false for LRCLIB.
    pub own: bool,
    /// Seconds, when LRCLIB knows the length of the song.
    pub duration: Option<f64>,
    pub album: Option<String>,
    pub alignment: Alignment,
}

/// What [`find_sung`] found.
#[derive(Debug, Clone, PartialEq)]
pub struct SungSearch {
    /// Candidates with synced lyrics, the best match first.
    pub matches: Vec<SungMatch>,
    /// How many songs the provider suggested.
    pub candidates: usize,
    /// A lyrics lookup that failed for a reason worth telling, such as being offline.
    pub lookup_problem: Option<LyricsOutcome>,
}

/// Asks `search` for songs with `query` in their lyrics, loads the synced
/// lyrics of the first `limit` of them and ranks them by how well they explain
/// `heard`. `track_key` gives the key of a title and artist, for your own lyrics.
pub async fn find_sung(
    search: &dyn SongSearch,
    service: &LyricsService,
    query: &str,
    heard: &[HeardWord],
    limit: usize,
    options: LookupOptions,
    track_key: impl Fn(&str, &str) -> String,
) -> Result<SungSearch, SearchError> {
    let candidates = search.search(query).await?;
    let suggested = candidates.len();
    let mut loaded = Vec::new();
    let mut lookup_problem = None;
    for candidate in candidates.into_iter().take(limit) {
        let key = track_key(&candidate.title, &candidate.artist);
        let found = service
            .song_lyrics(&key, &candidate.title, &candidate.artist, options)
            .await;
        let (lyrics, own) = match found.outcome {
            LyricsOutcome::Local { lyrics } => (lyrics, true),
            LyricsOutcome::Online { lyrics } => (lyrics, false),
            other @ (LyricsOutcome::Offline
            | LyricsOutcome::RateLimited { .. }
            | LyricsOutcome::Failed { .. }) => {
                lookup_problem.get_or_insert(other);
                continue;
            }
            _ => continue,
        };
        // Without timing the lyrics cannot follow the singer.
        if lyrics.sync == SyncLevel::Unsynced || lyrics.instrumental {
            continue;
        }
        loaded.push(Loaded {
            candidate,
            lyrics,
            own,
            duration: found.duration,
            album: found.album,
        });
    }
    Ok(SungSearch {
        matches: ranked(heard, loaded),
        candidates: suggested,
        lookup_problem,
    })
}

/// A song with synced lyrics, ready to be ranked.
struct Loaded {
    candidate: SongCandidate,
    lyrics: Lyrics,
    own: bool,
    duration: Option<f64>,
    album: Option<String>,
}

/// `loaded` ranked by how well each song explains `heard`, the best first.
fn ranked(heard: &[HeardWord], loaded: Vec<Loaded>) -> Vec<SungMatch> {
    let words: Vec<_> = loaded
        .iter()
        .map(|song| alignment::song_words(&song.lyrics.lines))
        .collect();
    let mut matches = Vec::new();
    let mut entries: Vec<Option<Loaded>> = loaded.into_iter().map(Some).collect();
    for (index, alignment) in alignment::rank(heard, &words) {
        if let Some(song) = entries[index].take() {
            matches.push(SungMatch {
                candidate: song.candidate,
                lyrics: song.lyrics,
                own: song.own,
                duration: song.duration,
                album: song.album,
                alignment,
            });
        }
    }
    matches
}

/// At most this many songs from the history are looked at, the most
/// recently heard first.
const HEARD_SONGS: usize = 2000;
/// Of those, the ones sharing the most words with what was heard are aligned.
const ALIGNED_SONGS: usize = 40;

/// Finds the sung song on this device, without asking anyone: among the songs
/// you heard, in your own lyrics and, with `kept`, in the lyrics Singline
/// keeps from LRCLIB, also when they are out of date. Only songs with synced
/// lyrics count, ranked like [`find_sung`] ranks the songs of a search.
pub fn find_sung_locally(service: &LyricsService, heard: &[HeardWord], kept: bool) -> SungSearch {
    let listens = service.store().listens(None).unwrap_or_default();
    let mut seen = HashSet::new();
    let songs: Vec<&ListenSession> = listens
        .iter()
        .rev()
        .filter(|listen| seen.insert(listen.track_key.as_str()))
        .take(HEARD_SONGS)
        .collect();
    let sung: HashSet<String> = heard
        .iter()
        .flat_map(|word| alignment::words(&word.text))
        .collect();
    let mut shared: Vec<(usize, Loaded)> = songs
        .into_iter()
        .filter_map(|song| {
            let (lyrics, own) = match service.local(&song.track_key) {
                Some(lyrics) => (lyrics, true),
                None if kept => (kept_lyrics(service, song)?, false),
                None => return None,
            };
            if lyrics.sync == SyncLevel::Unsynced || lyrics.instrumental {
                return None;
            }
            let words: HashSet<String> = lyrics
                .lines
                .iter()
                .flat_map(|line| alignment::words(&line.text))
                .collect();
            let common = sung.intersection(&words).count();
            (common >= 2).then(|| {
                (
                    common,
                    Loaded {
                        candidate: SongCandidate {
                            title: song.title.clone(),
                            artist: song.artist.clone(),
                            artwork_url: None,
                        },
                        lyrics,
                        own,
                        duration: song.duration,
                        album: song.album.clone(),
                    },
                )
            })
        })
        .collect();
    // The songs with the most words in common first; the sort keeps the most
    // recently heard first among equals.
    shared.sort_by(|a, b| b.0.cmp(&a.0));
    let loaded: Vec<Loaded> = shared
        .into_iter()
        .take(ALIGNED_SONGS)
        .map(|(_, song)| song)
        .collect();
    let candidates = loaded.len();
    SungSearch {
        matches: ranked(heard, loaded),
        candidates,
        lookup_problem: None,
    }
}

/// The lyrics Singline keeps from LRCLIB for a song you heard: from the
/// lookup while it played, or from an earlier Sing to find.
fn kept_lyrics(service: &LyricsService, song: &ListenSession) -> Option<Lyrics> {
    let store = service.store();
    let played = song.duration.and_then(|duration| {
        let query = Query {
            title: song.title.clone(),
            artist: song.artist.clone(),
            album: song.album.clone(),
            duration,
        };
        store.cached_any::<LookupResult>(&query.cache_key())
    });
    let result = played.or_else(|| {
        store
            .cached_any::<SongLookup>(&song_cache_key(&song.title, &song.artist))
            .map(|found| found.result)
    })?;
    match result {
        LookupResult::Found { lyrics } => Some(lyrics),
        LookupResult::Instrumental | LookupResult::NotFound => None,
    }
}

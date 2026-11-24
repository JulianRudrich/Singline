//! Sing to find on the device: finding a sung song among the songs you heard,
//! without asking Genius or LRCLIB.

use std::sync::{Arc, Mutex};

use lyrics_core::alignment::HeardWord;
use lyrics_core::listening::ListenSession;
use lyrics_core::{lrc, Lyrics};
use lyrics_providers::{
    find_sung_locally, LookupResult, LrclibClient, LyricsService, Query, SongLookup, Store,
};

const AGENT: &str = "Singline tests";

/// A service whose LRCLIB cannot be reached, so every answer comes from the store.
fn service() -> (LyricsService, Arc<Mutex<Store>>) {
    let store = Arc::new(Mutex::new(Store::open_in_memory().expect("store opens")));
    let client = LrclibClient::with_base_url("http://127.0.0.1:9", AGENT);
    (LyricsService::new(client, Arc::clone(&store)), store)
}

fn listen(title: &str, artist: &str, duration: Option<f64>, started_at: i64) -> ListenSession {
    ListenSession {
        track_key: format!("key:{title}:{artist}"),
        title: title.into(),
        artist: artist.into(),
        album: Some("Album".into()),
        service: "spotify".into(),
        started_at,
        duration,
        listened: 60.0,
        synced_lyrics: true,
    }
}

fn lyrics(lrc_text: &str) -> Lyrics {
    Lyrics::timed(lrc::parse(lrc_text))
}

fn sung(text: &str, start: f64) -> Vec<HeardWord> {
    text.split_whitespace()
        .enumerate()
        .map(|(index, word)| HeardWord::new(word, start + index as f64 * 0.4))
        .collect()
}

const MORNING: &str = "[00:05.00]The night is over\n[00:12.00]I can hear the morning calling from the mountain\n[00:18.00]And the river answers";
const CITY: &str = "[00:04.00]Neon lights and city sounds\n[00:09.00]Nobody sleeps in this town";

/// Keeps the lyrics of a song as the playback lookup does, until `expires_at`.
fn keep(store: &Arc<Mutex<Store>>, song: &ListenSession, text: &str, expires_at: i64) {
    let query = Query {
        title: song.title.clone(),
        artist: song.artist.clone(),
        album: song.album.clone(),
        duration: song.duration.unwrap_or(0.0),
    };
    let result = LookupResult::Found {
        lyrics: lyrics(text),
    };
    let store = store.lock().expect("store");
    store
        .store_cached(&query.cache_key(), &result, 1, expires_at)
        .expect("cached");
}

#[test]
fn finds_a_heard_song_by_its_kept_lyrics() {
    let (service, store) = service();
    let morning = listen("Morning Song", "Example Artist", Some(120.0), 10);
    let city = listen("City Song", "Decoy Band", Some(200.0), 20);
    {
        let store = store.lock().expect("store");
        store.add_listen(&morning).expect("listen");
        store.add_listen(&city).expect("listen");
    }
    // Out of date long ago: an old answer still holds the lyrics.
    keep(&store, &morning, MORNING, 2);
    keep(&store, &city, CITY, i64::MAX);

    let heard = sung("i can hear the morning calling from the mountain", 30.0);
    let found = find_sung_locally(&service, &heard, true);
    let best = found.matches.first().expect("a match");
    assert_eq!(best.candidate.title, "Morning Song");
    assert_eq!(best.candidate.artist, "Example Artist");
    assert!(best.alignment.is_confident());
    assert_eq!(best.alignment.line, 1);
    assert!(!best.own);
    assert_eq!(
        (best.duration, best.album.as_deref()),
        (Some(120.0), Some("Album"))
    );
    assert_eq!(found.matches.len(), 1, "the city song shares no words");

    // Without the kept lyrics, as when online lookups are off, nothing is found.
    assert!(find_sung_locally(&service, &heard, false)
        .matches
        .is_empty());
}

#[test]
fn your_own_lyrics_count_also_when_online_lookups_are_off() {
    let (service, store) = service();
    let song = listen("City Song", "Decoy Band", Some(200.0), 10);
    {
        let store = store.lock().expect("store");
        store.add_listen(&song).expect("listen");
        store
            .import_lyrics(&song.track_key, MORNING)
            .expect("imported");
    }
    let heard = sung("i can hear the morning calling", 30.0);
    let found = find_sung_locally(&service, &heard, false);
    let best = found.matches.first().expect("a match");
    assert_eq!(best.candidate.title, "City Song");
    assert!(best.own);
}

#[test]
fn finds_a_song_from_an_earlier_sing_to_find() {
    let (service, store) = service();
    // Sing to find looks songs up by title and artist and records no length.
    let song = listen("Morning Song", "Example Artist", None, 10);
    {
        let store = store.lock().expect("store");
        store.add_listen(&song).expect("listen");
        let found = SongLookup {
            result: LookupResult::Found {
                lyrics: lyrics(MORNING),
            },
            duration: Some(120.0),
            album: None,
        };
        let key = format!("lrclib-song:{}\u{1f}{}", "morning song", "example artist");
        store
            .store_cached(&key, &found, 1, i64::MAX)
            .expect("cached");
    }
    let heard = sung("i can hear the morning calling from the mountain", 30.0);
    let found = find_sung_locally(&service, &heard, true);
    assert_eq!(
        found
            .matches
            .first()
            .map(|found| found.candidate.title.as_str()),
        Some("Morning Song")
    );
}

#[test]
fn knows_songs_only_with_a_history_or_lyrics_of_your_own() {
    let (service, store) = service();
    assert!(!service.knows_songs(true));
    store
        .lock()
        .expect("store")
        .add_listen(&listen("Morning Song", "Example Artist", Some(120.0), 10))
        .expect("listen");
    assert!(service.knows_songs(true));
    assert!(
        !service.knows_songs(false),
        "the history alone needs kept lyrics"
    );
    store
        .lock()
        .expect("store")
        .import_lyrics("key:Mine:Me", MORNING)
        .expect("imported");
    assert!(service.knows_songs(false));
}

#[test]
fn plain_lyrics_cannot_be_followed() {
    let (service, store) = service();
    let song = listen("Morning Song", "Example Artist", Some(120.0), 10);
    store
        .lock()
        .expect("store")
        .add_listen(&song)
        .expect("listen");
    let query = Query {
        title: song.title.clone(),
        artist: song.artist.clone(),
        album: song.album.clone(),
        duration: 120.0,
    };
    let plain = LookupResult::Found {
        // Plain lyrics with estimated times, as LRCLIB gives for songs without timing.
        lyrics: Lyrics::estimated(lrc::parse(MORNING)),
    };
    store
        .lock()
        .expect("store")
        .store_cached(&query.cache_key(), &plain, 1, i64::MAX)
        .expect("cached");
    let heard = sung("i can hear the morning calling from the mountain", 30.0);
    assert!(find_sung_locally(&service, &heard, true).matches.is_empty());
}

//! The LRCLIB client and the lyrics service against a local pretend server.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use lyrics_core::{lrc, Lyrics, SyncLevel};
use lyrics_providers::{
    LookupError, LookupOptions, LookupResult, LrclibClient, LyricsOutcome, LyricsService, Query,
    SongLookup, Store,
};
use serde_json::json;
use wiremock::matchers::{header, method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

const AGENT: &str = "Singline/test (https://github.com/JulianRudrich/Singline)";

fn client(server: &MockServer) -> LrclibClient {
    LrclibClient::with_base_url(&server.uri(), AGENT).with_pause(Duration::ZERO)
}

fn query(title: &str, album: Option<&str>, duration: f64) -> Query {
    Query {
        title: title.into(),
        artist: "Example Artist".into(),
        album: album.map(str::to_owned),
        duration,
    }
}

fn record(
    title: &str,
    album: &str,
    duration: f64,
    synced: Option<&str>,
    plain: Option<&str>,
) -> serde_json::Value {
    json!({
        "id": 1, "trackName": title, "artistName": "Example Artist", "albumName": album,
        "duration": duration, "instrumental": false, "plainLyrics": plain, "syncedLyrics": synced
    })
}

async fn not_found(server: &MockServer) {
    Mock::given(method("GET"))
        .and(path("/api/get"))
        .respond_with(
            ResponseTemplate::new(404).set_body_json(json!({"code": 404, "name": "TrackNotFound"})),
        )
        .mount(server)
        .await;
}

#[tokio::test]
async fn exact_match_with_synced_lyrics() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/get"))
        .and(query_param("track_name", "Paper Lanterns"))
        .and(query_param("artist_name", "Example Artist"))
        .and(query_param("album_name", "Lights"))
        .and(query_param("duration", "201"))
        .and(header("user-agent", AGENT))
        .respond_with(ResponseTemplate::new(200).set_body_json(record(
            "Paper Lanterns",
            "Lights",
            201.0,
            Some("[00:01.00] Glow"),
            Some("Glow"),
        )))
        .expect(1)
        .mount(&server)
        .await;

    let result = client(&server)
        .find(&query("Paper Lanterns", Some("Lights"), 200.6))
        .await
        .expect("lookup works");
    let LookupResult::Found { lyrics } = result else {
        panic!("expected lyrics, got {result:?}");
    };
    assert_eq!(lyrics.sync, SyncLevel::Line);
    assert_eq!(lyrics.lines[0].text, "Glow");
}

#[tokio::test]
async fn search_prefers_the_same_album_and_word_timing() {
    let server = MockServer::start().await;
    not_found(&server).await;
    let words = "version: '1.0'\nlines:\n  - text: 'Glow on'\n    start_ms: 1000\n    words:\n      - {text: 'Glow ', start_ms: 1000}\n      - {text: 'on', start_ms: 1500}\n";
    let mut word_record = record(
        "Paper Lanterns",
        "Lights",
        203.0,
        Some("[00:01.00] Glow on"),
        None,
    );
    word_record["lyricsfile"] = json!(words);
    Mock::given(method("GET"))
        .and(path("/api/search"))
        .and(query_param("track_name", "Paper Lanterns"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([
            record(
                "Paper Lanterns",
                "Live in Berlin",
                200.0,
                Some("[00:01.00] Live"),
                None
            ),
            record(
                "Paper Lanterns",
                "Lights",
                202.5,
                Some("[00:01.00] Album"),
                None
            ),
            word_record,
            record(
                "Paper Lanterns (Remix)",
                "Lights",
                200.0,
                Some("[00:01.00] Remix"),
                None
            ),
        ])))
        .mount(&server)
        .await;

    let result = client(&server)
        .find(&query("Paper Lanterns", Some("Lights"), 202.0))
        .await
        .expect("lookup works");
    let LookupResult::Found { lyrics } = result else {
        panic!("expected lyrics, got {result:?}");
    };
    assert_eq!(
        lyrics.sync,
        SyncLevel::Word,
        "word timing on the same album wins: {lyrics:?}"
    );
    assert_eq!(lyrics.lines[0].words[1].start, 1.5);
}

#[tokio::test]
async fn plain_lyrics_are_the_last_resort() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/get"))
        .respond_with(ResponseTemplate::new(200).set_body_json(record(
            "Quiet Song",
            "",
            180.0,
            None,
            Some("One line\nAnother line"),
        )))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/api/search"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([])))
        .expect(1)
        .mount(&server)
        .await;

    let result = client(&server)
        .find(&query("Quiet Song", None, 180.0))
        .await
        .expect("lookup works");
    let LookupResult::Found { lyrics } = result else {
        panic!("expected plain lyrics, got {result:?}");
    };
    assert_eq!(lyrics.sync, SyncLevel::Unsynced);
    assert_eq!(lyrics.lines.len(), 2);
}

#[tokio::test]
async fn instrumental_needs_the_same_artist_and_album() {
    let server = MockServer::start().await;
    let mut instrumental = record("Interlude", "Lights", 90.0, None, None);
    instrumental["instrumental"] = json!(true);
    Mock::given(method("GET"))
        .and(path("/api/get"))
        .respond_with(ResponseTemplate::new(200).set_body_json(instrumental))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/api/search"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([])))
        .mount(&server)
        .await;

    let lookup = client(&server);
    assert_eq!(
        lookup.find(&query("Interlude", Some("Lights"), 90.0)).await,
        Ok(LookupResult::Instrumental)
    );
    assert_eq!(
        lookup
            .find(&query("Interlude", Some("Other Album"), 90.0))
            .await,
        Ok(LookupResult::NotFound),
        "different album"
    );
}

#[tokio::test]
async fn wrong_versions_are_not_shown() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/get"))
        .respond_with(ResponseTemplate::new(200).set_body_json(record(
            "Paper Lanterns",
            "Lights",
            260.0,
            Some("[00:01.00] Extended"),
            None,
        )))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/api/search"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([record(
            "Paper Lanterns",
            "Lights",
            240.0,
            Some("[00:01.00] Long"),
            None
        )])))
        .mount(&server)
        .await;
    assert_eq!(
        client(&server)
            .find(&query("Paper Lanterns", None, 200.0))
            .await,
        Ok(LookupResult::NotFound)
    );
}

#[tokio::test]
async fn plus_signs_survive_the_query_string() {
    let server = MockServer::start().await;
    not_found(&server).await;
    Mock::given(method("GET"))
        .and(path("/api/search"))
        .and(query_param("track_name", "1+1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([record(
            "1+1",
            "",
            200.0,
            Some("[00:01.00] Plus"),
            None
        )])))
        .expect(1)
        .mount(&server)
        .await;
    let result = client(&server)
        .find(&query("1+1", None, 200.0))
        .await
        .expect("lookup works");
    assert!(matches!(result, LookupResult::Found { .. }), "{result:?}");
}

#[tokio::test]
async fn busy_server_pauses_further_requests() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(429).insert_header("Retry-After", "5"))
        .expect(1)
        .mount(&server)
        .await;
    let lookup = client(&server);
    let first = lookup.find(&query("Song", None, 200.0)).await;
    assert!(
        matches!(first, Err(LookupError::RateLimited { retry_after }) if retry_after >= Duration::from_secs(5)),
        "{first:?}"
    );
    let second = lookup.find(&query("Other", None, 200.0)).await;
    assert!(
        matches!(second, Err(LookupError::RateLimited { .. })),
        "no second request while waiting: {second:?}"
    );
}

#[tokio::test]
async fn server_errors_and_missing_servers() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(500))
        .mount(&server)
        .await;
    assert_eq!(
        client(&server).find(&query("Song", None, 200.0)).await,
        Err(LookupError::Http(500))
    );

    let unreachable =
        LrclibClient::with_base_url("http://127.0.0.1:9", AGENT).with_pause(Duration::ZERO);
    assert_eq!(
        unreachable.find(&query("Song", None, 200.0)).await,
        Err(LookupError::Offline)
    );

    assert_eq!(
        client(&server).find(&query("", None, 200.0)).await,
        Ok(LookupResult::NotFound),
        "nothing to search for"
    );
}

#[tokio::test]
async fn failed_requests_do_not_name_the_song() {
    // A server that hangs up without answering.
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("a free port");
    let address = listener.local_addr().expect("a bound port");
    std::thread::spawn(move || {
        for connection in listener.incoming() {
            drop(connection);
        }
    });
    let silent =
        LrclibClient::with_base_url(&format!("http://{address}"), AGENT).with_pause(Duration::ZERO);
    let error = silent
        .find(&query("Paper Lanterns", None, 200.0))
        .await
        .expect_err("no answer");
    assert!(matches!(error, LookupError::Network(_)), "{error:?}");
    let message = error.to_string();
    assert!(
        !message.contains("Lanterns") && !message.contains("Example"),
        "the message ends up in the log: {message}"
    );
}

#[tokio::test]
async fn service_prefers_own_lyrics_and_caches_online_results() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/get"))
        .respond_with(ResponseTemplate::new(200).set_body_json(record(
            "Paper Lanterns",
            "",
            200.0,
            Some("[00:01.00] Online"),
            None,
        )))
        .expect(2)
        .mount(&server)
        .await;

    let store = Arc::new(Mutex::new(Store::open_in_memory().expect("store opens")));
    let service = LyricsService::new(client(&server), Arc::clone(&store));
    let song = query("Paper Lanterns", None, 200.0);

    let first = service
        .lyrics_for("track:a", &song, LookupOptions::default())
        .await;
    assert!(
        matches!(&first, LyricsOutcome::Online { lyrics } if lyrics.lines[0].text == "Online"),
        "{first:?}"
    );
    let cached = service
        .lyrics_for("track:a", &song, LookupOptions::default())
        .await;
    assert_eq!(cached, first, "the second lookup comes from the cache");
    let refreshed = service
        .lyrics_for(
            "track:a",
            &song,
            LookupOptions {
                online: true,
                refresh: true,
            },
        )
        .await;
    assert_eq!(refreshed, first, "search again asks the server");

    store
        .lock()
        .expect("store")
        .import_lyrics("track:a", "[00:02.00] Mine")
        .expect("valid LRC");
    let own = service
        .lyrics_for("track:a", &song, LookupOptions::default())
        .await;
    assert!(
        matches!(&own, LyricsOutcome::Local { lyrics } if lyrics.lines[0].text == "Mine"),
        "{own:?}"
    );

    let offline = service
        .lyrics_for(
            "track:b",
            &song,
            LookupOptions {
                online: false,
                refresh: false,
            },
        )
        .await;
    assert_eq!(offline, LyricsOutcome::LocalOnly);
}

#[tokio::test]
async fn word_timing_wins_a_tie() {
    // Found on the real LRCLIB: a version without word timing 4 s off is listed
    // before one with word timing 5 s off. With the bonus both score 4.
    let server = MockServer::start().await;
    not_found(&server).await;
    let words = "version: '1.0'\nlines:\n  - text: 'Glow'\n    start_ms: 1000\n    words:\n      - {text: 'Glow', start_ms: 1000}\n";
    let mut word_record = record(
        "Paper Lanterns",
        "Compilation",
        217.0,
        Some("[00:01.00] Glow"),
        None,
    );
    word_record["lyricsfile"] = json!(words);
    Mock::given(method("GET"))
        .and(path("/api/search"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([
            record(
                "Paper Lanterns",
                "Bonus Edition",
                218.0,
                Some("[00:01.00] Glow"),
                None
            ),
            word_record,
        ])))
        .mount(&server)
        .await;

    let result = client(&server)
        .find(&query("Paper Lanterns", None, 222.0))
        .await
        .expect("lookup works");
    let LookupResult::Found { lyrics } = result else {
        panic!("expected lyrics, got {result:?}");
    };
    assert_eq!(lyrics.sync, SyncLevel::Word);
}

#[tokio::test]
async fn lyrics_found_earlier_stay_when_lrclib_cannot_answer() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(500))
        .mount(&server)
        .await;
    let store = Arc::new(Mutex::new(Store::open_in_memory().expect("store opens")));
    let broken = LyricsService::new(client(&server), Arc::clone(&store));
    let unreachable = LyricsService::new(
        LrclibClient::with_base_url("http://127.0.0.1:9", AGENT).with_pause(Duration::ZERO),
        Arc::clone(&store),
    );
    let song = query("Paper Lanterns", None, 200.0);
    let missing = query("Quiet Rooms", None, 180.0);
    let kept = |text: &str| LookupResult::Found {
        lyrics: Lyrics::timed(lrc::parse(text)),
    };
    {
        // Found long ago: the answers are past their time.
        let store = store.lock().expect("store");
        store
            .store_cached(&song.cache_key(), &kept("[00:01.00] Kept"), 1, 2)
            .expect("cached");
        store
            .store_cached(&missing.cache_key(), &LookupResult::NotFound, 1, 2)
            .expect("cached");
        let sung = SongLookup {
            result: kept("[00:01.00] Sung"),
            duration: Some(200.0),
            album: None,
        };
        let key = format!("lrclib-song:{}\u{1f}{}", "paper lanterns", "example artist");
        store.store_cached(&key, &sung, 1, 2).expect("cached");
    }
    let shows = |outcome: &LyricsOutcome, line: &str| matches!(outcome, LyricsOutcome::Online { lyrics } if lyrics.lines[0].text == line);

    for service in [&unreachable, &broken] {
        let found = service
            .lyrics_for("track:a", &song, LookupOptions::default())
            .await;
        assert!(shows(&found, "Kept"), "{found:?}");
        let sung = service
            .song_lyrics(
                "track:s",
                "Paper Lanterns",
                "Example Artist",
                LookupOptions::default(),
            )
            .await;
        assert!(shows(&sung.outcome, "Sung"), "{:?}", sung.outcome);
        assert_eq!(sung.duration, Some(200.0));
    }
    // The song may have lyrics by now, so "not found" makes way for the reason.
    assert_eq!(
        unreachable
            .lyrics_for("track:b", &missing, LookupOptions::default())
            .await,
        LyricsOutcome::Offline
    );
    // Search again shows that the search failed.
    let again = LookupOptions {
        online: true,
        refresh: true,
    };
    assert_eq!(
        unreachable.lyrics_for("track:a", &song, again).await,
        LyricsOutcome::Offline
    );
}

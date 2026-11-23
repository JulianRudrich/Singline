//! The loop that reads the players, follows one of them and loads its lyrics.

use std::sync::atomic::Ordering;
use std::sync::mpsc::{Receiver, RecvTimeoutError};
use std::thread;
use std::time::{Duration, Instant};

use lyrics_core::{chorus, lrc, Lyrics, PlaybackClock, SyncLevel};
use lyrics_providers::{LookupOptions, LyricsOutcome, Query};
use media_session::{
    default_backends, Backend, Capabilities, Command, CommandError, DemoBackend, Engine,
    MicrophoneBackend, NowPlaying, PlayerInfo, Problem, SourcePreference, Track, DEMO_LRC,
    DEMO_PLAYER_ID,
};
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};

use crate::AppState;

pub enum Request {
    Command(Command),
    Preference {
        source: SourcePreference,
        include_other: bool,
        ignored: Vec<String>,
    },
    ReloadLyrics {
        refresh: bool,
    },
}

/// What the overlay shows about the player, as sent to the web interface.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaybackView {
    pub state: &'static str,
    pub player: Option<PlayerInfo>,
    pub problem: Option<Problem>,
    pub track: Option<Track>,
    pub playing: bool,
    /// Seconds into the track at the moment this view is sent.
    pub position: Option<f64>,
    pub rate: f64,
    pub capabilities: Capabilities,
    /// When `position` was read, so it can be moved on to a later moment.
    #[serde(skip)]
    pub sampled_at: Instant,
}

impl PlaybackView {
    pub fn idle() -> Self {
        Self {
            state: "idle",
            player: None,
            problem: None,
            track: None,
            playing: false,
            position: None,
            rate: 1.0,
            capabilities: Capabilities::default(),
            sampled_at: Instant::now(),
        }
    }

    /// The position at `now`: the reading moved on while the track plays, never
    /// past its end. `None` when the player reports no position.
    pub fn position_at(&self, now: Instant) -> Option<f64> {
        let position = self.position?;
        let elapsed = now.saturating_duration_since(self.sampled_at).as_secs_f64();
        let duration = self
            .track
            .as_ref()
            .and_then(|track| track.duration)
            .unwrap_or(0.0);
        let clock = PlaybackClock::new(position, 0.0, self.playing).with_rate(self.rate);
        Some(clock.position_at(elapsed, duration))
    }

    fn new(now_playing: &NowPlaying, now: Instant) -> Self {
        match now_playing {
            NowPlaying::Idle => Self::idle(),
            NowPlaying::Problem { player, problem } => Self {
                state: "problem",
                player: Some(player.clone()),
                problem: Some(problem.clone()),
                ..Self::idle()
            },
            NowPlaying::Track(snapshot) => Self {
                state: "track",
                player: Some(snapshot.player.clone()),
                problem: None,
                track: snapshot.track.clone(),
                playing: snapshot.status == media_session::PlaybackStatus::Playing,
                position: snapshot.position_at(now),
                rate: snapshot.rate,
                capabilities: snapshot.capabilities,
                sampled_at: now,
            },
        }
    }
}

/// The lyrics of the followed track, as sent to the web interface.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LyricsView {
    pub track_key: Option<String>,
    pub status: &'static str,
    pub origin: Option<&'static str>,
    pub lyrics: Option<Lyrics>,
    /// Lyric times where each chorus starts. Empty when nothing repeats, and
    /// for plain lyrics, whose estimated times are too rough to jump to.
    pub chorus: Vec<f64>,
    pub song_offset: f64,
    pub can_retry: bool,
}

impl LyricsView {
    pub fn idle() -> Self {
        Self {
            track_key: None,
            status: "idle",
            origin: None,
            lyrics: None,
            chorus: Vec::new(),
            song_offset: 0.0,
            can_retry: false,
        }
    }

    fn loading(track_key: &str, song_offset: f64) -> Self {
        Self {
            track_key: Some(track_key.to_owned()),
            status: "loading",
            song_offset,
            ..Self::idle()
        }
    }

    fn ready(track_key: &str, lyrics: Lyrics, origin: &'static str, song_offset: f64) -> Self {
        let chorus = if lyrics.sync == SyncLevel::Unsynced {
            Vec::new()
        } else {
            chorus::starts(&lyrics.lines)
        };
        Self {
            track_key: Some(track_key.to_owned()),
            status: "ready",
            origin: Some(origin),
            lyrics: Some(lyrics),
            chorus,
            song_offset,
            can_retry: false,
        }
    }

    fn from_outcome(track_key: &str, outcome: LyricsOutcome, song_offset: f64) -> Self {
        let can_retry = outcome.can_retry();
        let status = match outcome {
            LyricsOutcome::Local { lyrics } => {
                return Self::ready(track_key, lyrics, "local", song_offset)
            }
            LyricsOutcome::Online { lyrics } => {
                return Self::ready(track_key, lyrics, "online", song_offset)
            }
            LyricsOutcome::Instrumental => "instrumental",
            LyricsOutcome::NotFound => "notFound",
            LyricsOutcome::LocalOnly => "localOnly",
            LyricsOutcome::Offline => "offline",
            LyricsOutcome::RateLimited { .. } => "rateLimited",
            LyricsOutcome::Failed { message } => {
                log::warn!("lyrics lookup failed: {message}");
                "failed"
            }
        };
        Self {
            track_key: Some(track_key.to_owned()),
            status,
            song_offset,
            can_retry,
            ..Self::idle()
        }
    }
}

pub fn spawn(app: AppHandle, demo: bool, requests: Receiver<Request>) {
    let started = thread::Builder::new()
        .name("playback".into())
        .spawn(move || run(&app, demo, &requests));
    if let Err(error) = started {
        log::error!("could not start reading players: {error}");
    }
}

fn run(app: &AppHandle, demo: bool, requests: &Receiver<Request>) {
    let (source, include_other, ignored) = {
        let state = app.state::<AppState>();
        let settings = state.settings();
        (
            settings.source,
            settings.include_other_players,
            settings.ignored_players.clone(),
        )
    };
    let mut engine = Engine::new(source, include_other);
    engine.set_ignored(&ignored);
    let mut backends: Vec<Box<dyn Backend>> = if demo {
        vec![Box::new(DemoBackend::new())]
    } else {
        default_backends()
    };
    // Browser tabs the Singline extension reports, next to the system's players.
    if let Some(bridge) = app
        .state::<crate::browser_bridge::BrowserBridge>()
        .backend()
    {
        backends.push(Box::new(bridge));
    }
    // The song someone sings, for Sing to find. It takes over while it listens.
    backends.push(Box::new(MicrophoneBackend::new(
        app.state::<crate::sing::Sing>().feed(),
    )));
    let interval = backends
        .iter()
        .map(|backend| backend.poll_interval())
        .min()
        .unwrap_or(Duration::from_millis(500));
    let mut current_track: Option<Track> = None;

    loop {
        let readings: Vec<_> = backends
            .iter_mut()
            .flat_map(|backend| backend.poll())
            .collect();
        crate::players::observe(app, &readings);
        let now_playing = engine.choose(readings.clone());
        crate::history::observe(app, &readings, engine.current());
        let view = PlaybackView::new(&now_playing, Instant::now());
        let track = view.track.clone();
        {
            let state = app.state::<AppState>();
            *state.playback.lock().unwrap_or_else(|p| p.into_inner()) = view.clone();
        }
        let _ = app.emit("playback", &view);
        crate::now::update(app);
        crate::autohide::update(app, &view);
        crate::sleep::tick(app, &view, interval);
        if track.as_ref().map(|t| &t.key) != current_track.as_ref().map(|t| &t.key) {
            current_track = track;
            load_lyrics(app, current_track.clone(), demo, false);
            crate::shuffle::track_changed(
                app,
                current_track.as_ref().and_then(|t| t.source_id.as_deref()),
            );
        } else if track.is_some() {
            // Same song; keep the details fresh, such as a late duration.
            current_track = track;
        }

        let deadline = Instant::now() + interval;
        loop {
            match requests.recv_timeout(deadline.saturating_duration_since(Instant::now())) {
                Ok(Request::Command(command)) => {
                    if let Some(player) = engine.current().cloned() {
                        for backend in backends.iter_mut() {
                            match backend.send(&player, command) {
                                Err(CommandError::PlayerGone) => continue,
                                Err(error) => log::warn!("{player}: {error}"),
                                Ok(()) => {}
                            }
                            break;
                        }
                    }
                    break;
                }
                Ok(Request::Preference {
                    source,
                    include_other,
                    ignored,
                }) => {
                    engine.set_preference(source, include_other);
                    engine.set_ignored(&ignored);
                    break;
                }
                Ok(Request::ReloadLyrics { refresh }) => {
                    load_lyrics(app, current_track.clone(), demo, refresh)
                }
                Err(RecvTimeoutError::Timeout) => break,
                Err(RecvTimeoutError::Disconnected) => return,
            }
        }
    }
}

/// Loads the lyrics for `track` and sends them once they are found. A newer
/// request makes an older one's answer go unused.
pub fn load_lyrics(app: &AppHandle, track: Option<Track>, demo: bool, refresh: bool) {
    let state = app.state::<AppState>();
    let generation = state.lyrics_generation.fetch_add(1, Ordering::SeqCst) + 1;
    let Some(track) = track else {
        publish(app, LyricsView::idle());
        return;
    };
    let song_offset = state.store().timing_offset(&track.key);

    if demo && track.source_id.as_deref() == Some(DEMO_PLAYER_ID) {
        let lyrics = Lyrics::timed(lrc::parse(DEMO_LRC));
        publish(
            app,
            LyricsView::ready(&track.key, lyrics, "demo", song_offset),
        );
        return;
    }
    // Sing to find already chose these lyrics to line the singer up with.
    if let Some((lyrics, origin)) = app.state::<crate::sing::Sing>().lyrics_for(&track.key) {
        publish(
            app,
            LyricsView::ready(&track.key, lyrics, origin, song_offset),
        );
        return;
    }

    publish(app, LyricsView::loading(&track.key, song_offset));
    let online = state.settings().lookup_online;
    let service = state.lyrics.clone();
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let query = Query {
            title: track.title.clone(),
            artist: track.artist.clone(),
            album: track.album.clone(),
            duration: track.duration.unwrap_or(0.0),
        };
        let outcome = service
            .lyrics_for(&track.key, &query, LookupOptions { online, refresh })
            .await;
        let state = app.state::<AppState>();
        if state.lyrics_generation.load(Ordering::SeqCst) == generation {
            publish(
                &app,
                LyricsView::from_outcome(&track.key, outcome, song_offset),
            );
        }
    });
}

pub fn publish(app: &AppHandle, view: LyricsView) {
    let state = app.state::<AppState>();
    *state.lyrics_view.lock().unwrap_or_else(|p| p.into_inner()) = view.clone();
    let _ = app.emit("lyrics", &view);
    crate::now::update(app);
}

#[cfg(test)]
mod tests {
    use super::*;
    use lyrics_core::LyricLine;

    fn song(texts: &[&str]) -> Vec<LyricLine> {
        texts
            .iter()
            .enumerate()
            .map(|(index, text)| LyricLine::new(5.0 * index as f64, *text))
            .collect()
    }

    #[test]
    fn ready_lyrics_carry_the_chorus_starts() {
        let lines = song(&["Hold on", "Let go", "Verse", "Hold on", "Let go"]);
        let view = LyricsView::ready("key", Lyrics::timed(lines.clone()), "online", 0.0);
        assert_eq!(view.chorus, [0.0, 15.0]);
        let json = serde_json::to_value(&view).expect("the view serializes");
        assert_eq!(json["chorus"], serde_json::json!([0.0, 15.0]));

        let plain = LyricsView::ready("key", Lyrics::estimated(lines), "online", 0.0);
        assert!(
            plain.chorus.is_empty(),
            "estimated times are too rough to jump to"
        );
        let unique = LyricsView::ready("key", Lyrics::timed(song(&["A", "B"])), "local", 0.0);
        assert!(unique.chorus.is_empty());
        assert!(LyricsView::idle().chorus.is_empty());
        assert!(LyricsView::loading("key", 0.0).chorus.is_empty());
    }
}

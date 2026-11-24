//! Sing to find: sing a few words, and Singline finds the song, shows its
//! lyrics from the line you sang and follows you while you go on singing.
//!
//! A session runs on a thread of its own: the microphone feeds a recording,
//! Whisper turns voiced stretches into words on this device, Genius turns the
//! words into candidate songs, and the lyrics of the candidates decide which
//! song it is and where in it the singer stands. The song then plays through
//! the microphone player (`media_session::MicrophoneBackend`), so the overlay
//! follows it like any player. Only the words leave the device, never audio.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex, MutexGuard};
use std::thread;
use std::time::{Duration, Instant};

use lyrics_core::alignment::{self, FollowStep, Follower, HeardWord, SongWord};
use lyrics_core::Lyrics;
use lyrics_providers::{
    find_sung, find_sung_locally, GeniusClient, LookupOptions, LyricsOutcome, SearchError,
    SungMatch,
};
use media_session::{normalize, MicrophoneFeed, Track};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager, State};
use transcription::audio::Recording;
use transcription::capture::{Capture, CaptureError, Source};
use transcription::model::{self, ModelSpec, Retry};

use crate::AppState;

/// The event that carries [`SingView`] to the windows.
pub const EVENT: &str = "sing";
/// The level of the microphone, from 0 to 1, about twelve times a second.
pub const LEVEL_EVENT: &str = "sing-level";
/// The state of the speech model, as [`ModelView`].
pub const MODEL_EVENT: &str = "sing-model";

const GENIUS_API_CLIENTS: &str = "https://genius.com/api-clients";
/// A Genius token built into release builds, so they work without setup.
const BUILT_IN_TOKEN: Option<&str> = option_env!("SINGLINE_GENIUS_TOKEN");
const MODEL: ModelSpec<'static> = model::BASE;

/// Listening stops after this long without voice.
const QUIET_LIMIT: f64 = 30.0;
/// Voice needed before the first search.
const FIRST_SEARCH: f64 = 2.0;
/// Seconds between two searches while nothing is found.
const SEARCH_EVERY: f64 = 2.5;
/// After this much voice without a match, the song counts as not found.
const GIVE_UP: f64 = 20.0;
/// Whisper hears at most 30 s at a time.
const LONGEST_WINDOW: f64 = 28.0;
/// While following: how often, and how far back, to listen again.
const FOLLOW_EVERY: f64 = 3.0;
const FOLLOW_WINDOW: f64 = 7.0;
const CANDIDATES: usize = 4;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Phase {
    Idle,
    /// Loading or downloading the speech model.
    Preparing,
    Listening,
    Searching,
    Following,
    NotFound,
    Problem,
}

/// Why Sing to find cannot go on.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum SingProblem {
    /// This build has no Whisper.
    Unavailable,
    /// The processor lacks the instructions Whisper is built for.
    Processor,
    NoToken,
    Unauthorized,
    OnlineOff,
    Offline,
    RateLimited,
    NoMicrophone,
    Denied,
    Busy,
    /// The microphone sends nothing but silence, as macOS does without permission.
    Silent,
    Model {
        detail: String,
    },
    Failed {
        detail: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FoundSong {
    pub title: String,
    pub artist: String,
}

/// What the overlay shows about Sing to find.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SingView {
    pub phase: Phase,
    /// The words heard so far, while searching or after nothing was found.
    pub heard: String,
    pub song: Option<FoundSong>,
    /// Other candidates "Not this song" can switch to.
    pub alternatives: usize,
    pub problem: Option<SingProblem>,
}

impl SingView {
    fn idle() -> Self {
        Self {
            phase: Phase::Idle,
            heard: String::new(),
            song: None,
            alternatives: 0,
            problem: None,
        }
    }

    pub fn is_active(&self) -> bool {
        matches!(
            self.phase,
            Phase::Preparing | Phase::Listening | Phase::Searching | Phase::Following
        )
    }
}

/// The speech model, for the settings and the overlay.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelView {
    /// "missing", "downloading", "ready" or "failed".
    pub state: &'static str,
    pub received: u64,
    pub total: u64,
    pub error: Option<String>,
    pub name: &'static str,
    /// Set by SINGLINE_WHISPER_MODEL, so nothing is downloaded.
    pub custom: bool,
}

/// What the settings show about Sing to find.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SingSettings {
    /// The listener's own Genius token; empty when none was entered.
    pub token: String,
    /// This build comes with a token of its own.
    pub built_in: bool,
    /// This build can transcribe.
    pub available: bool,
    pub model: ModelView,
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct StoredConfig {
    genius_token: String,
}

enum Control {
    /// "Not this song": the next candidate.
    Next,
    /// Forget the song and listen again.
    Again,
}

struct Session {
    stop: Arc<AtomicBool>,
    control: Sender<Control>,
}

struct Download {
    cancel: Arc<AtomicBool>,
    received: u64,
}

#[derive(Default)]
struct Inner {
    token: String,
    view: Option<SingView>,
    session: Option<Session>,
    download: Option<Download>,
    download_error: Option<String>,
    /// Start listening once the model is downloaded.
    start_when_ready: bool,
    /// The lyrics of the song being followed, so the overlay shows exactly them.
    lyrics: Option<(String, Lyrics, &'static str)>,
}

pub struct Sing {
    feed: MicrophoneFeed,
    inner: Mutex<Inner>,
    config_path: Option<PathBuf>,
    models_dir: Option<PathBuf>,
    model_override: Option<PathBuf>,
    mic_file: Option<PathBuf>,
    genius_url: String,
    model_url: String,
    user_agent: String,
    debug: bool,
}

impl Sing {
    pub fn new(
        demo: bool,
        config_dir: Option<&Path>,
        data_dir: Option<&Path>,
        user_agent: String,
        debug: bool,
    ) -> Self {
        let config_path = config_dir
            .filter(|_| !demo)
            .map(|directory| directory.join("sing.json"));
        let token = config_path
            .as_deref()
            .and_then(|path| fs::read_to_string(path).ok())
            .and_then(|text| serde_json::from_str::<StoredConfig>(&text).ok())
            .map(|stored| stored.genius_token.trim().to_owned())
            .unwrap_or_default();
        let variable = |name: &str| std::env::var(name).ok().filter(|value| !value.is_empty());
        Self {
            feed: MicrophoneFeed::new(),
            inner: Mutex::new(Inner {
                token,
                ..Inner::default()
            }),
            config_path,
            models_dir: data_dir.map(|directory| directory.join("models")),
            model_override: variable("SINGLINE_WHISPER_MODEL").map(PathBuf::from),
            mic_file: variable("SINGLINE_MIC_FILE").map(PathBuf::from),
            genius_url: variable("SINGLINE_GENIUS_URL")
                .unwrap_or_else(|| lyrics_providers::genius::DEFAULT_BASE_URL.to_owned()),
            model_url: variable("SINGLINE_WHISPER_MODEL_URL")
                .unwrap_or_else(|| model::DEFAULT_BASE_URL.to_owned()),
            user_agent,
            debug,
        }
    }

    fn lock(&self) -> MutexGuard<'_, Inner> {
        self.inner.lock().unwrap_or_else(|p| p.into_inner())
    }

    /// The feed the microphone player reads.
    pub fn feed(&self) -> MicrophoneFeed {
        self.feed.clone()
    }

    pub fn view(&self) -> SingView {
        self.lock().view.clone().unwrap_or_else(SingView::idle)
    }

    pub fn is_active(&self) -> bool {
        self.view().is_active()
    }

    /// Starts listening by itself when a test plays a file instead of the microphone.
    pub fn starts_by_itself(&self) -> bool {
        self.mic_file.is_some()
    }

    /// The lyrics of the followed song, when `track_key` is that song.
    pub fn lyrics_for(&self, track_key: &str) -> Option<(Lyrics, &'static str)> {
        self.lock()
            .lyrics
            .as_ref()
            .filter(|(key, ..)| key == track_key)
            .map(|(_, lyrics, origin)| (lyrics.clone(), *origin))
    }

    fn token(&self) -> String {
        let own = self.lock().token.clone();
        if own.is_empty() {
            BUILT_IN_TOKEN.unwrap_or("").trim().to_owned()
        } else {
            own
        }
    }

    fn model_path(&self) -> Option<PathBuf> {
        if let Some(path) = &self.model_override {
            return path.is_file().then(|| path.clone());
        }
        MODEL.installed(self.models_dir.as_deref()?)
    }

    fn model_view(&self) -> ModelView {
        let inner = self.lock();
        let base = ModelView {
            state: "missing",
            received: 0,
            total: MODEL.size,
            error: None,
            name: MODEL.name,
            custom: self.model_override.is_some(),
        };
        if let Some(download) = &inner.download {
            return ModelView {
                state: "downloading",
                received: download.received,
                ..base
            };
        }
        drop(inner);
        if self.model_path().is_some() {
            return ModelView {
                state: "ready",
                received: MODEL.size,
                ..base
            };
        }
        let partial = self
            .models_dir
            .as_deref()
            .map_or(0, |dir| MODEL.partial_size(dir));
        match self.lock().download_error.clone() {
            Some(error) => ModelView {
                state: "failed",
                received: partial,
                error: Some(error),
                ..base
            },
            None => ModelView {
                received: partial,
                ..base
            },
        }
    }

    fn settings_view(&self) -> SingSettings {
        // Its own statement: the lock must be released before `model_view` takes it.
        let token = self.lock().token.clone();
        SingSettings {
            token,
            built_in: BUILT_IN_TOKEN.is_some_and(|token| !token.trim().is_empty()),
            available: transcription::can_transcribe(),
            model: self.model_view(),
        }
    }

    /// A step for the end-to-end checks. It names what was heard, so it
    /// stays out of the log file.
    fn log(&self, message: impl FnOnce() -> String) {
        if self.debug {
            log::debug!(target: crate::logging::CHECKS, "sing: {}", message());
        }
    }
}

fn publish(app: &AppHandle, view: SingView) {
    let sing = app.state::<Sing>();
    let was_active = sing.is_active();
    let active = view.is_active();
    sing.lock().view = Some(view.clone());
    let _ = app.emit(EVENT, &view);
    if was_active != active {
        crate::tray::refresh(app);
    }
}

fn change(app: &AppHandle, edit: impl FnOnce(&mut SingView)) {
    let mut view = app.state::<Sing>().view();
    edit(&mut view);
    publish(app, view);
}

fn fail(app: &AppHandle, problem: SingProblem) {
    let sing = app.state::<Sing>();
    sing.log(|| format!("problem {problem:?}"));
    sing.feed.stop();
    sing.lock().lyrics = None;
    publish(
        app,
        SingView {
            phase: Phase::Problem,
            problem: Some(problem),
            ..SingView::idle()
        },
    );
}

fn emit_model(app: &AppHandle) {
    let _ = app.emit(MODEL_EVENT, app.state::<Sing>().model_view());
}

/// Starts listening, or stops when it already listens.
pub fn toggle(app: &AppHandle) {
    if app.state::<Sing>().is_active() {
        stop(app);
    } else {
        start(app);
    }
}

/// Starts listening: downloads the model first when it is missing.
pub fn start(app: &AppHandle) {
    let sing = app.state::<Sing>();
    if sing.is_active() {
        return;
    }
    let state = app.state::<AppState>();
    // Turned off in the settings: nothing starts it, not even the menus.
    if !state.settings().sing_to_find && !sing.starts_by_itself() {
        return;
    }
    if !transcription::can_transcribe() {
        return fail(app, SingProblem::Unavailable);
    }
    if !transcription::processor_supported() {
        return fail(app, SingProblem::Processor);
    }
    // Songs you heard are found on the device; any other song needs Genius,
    // and so online lookups and a token.
    let online = state.settings().lookup_online;
    let known = state.lyrics.knows_songs(online);
    if !online && !known {
        return fail(app, SingProblem::OnlineOff);
    }
    if sing.token().is_empty() && !known {
        return fail(app, SingProblem::NoToken);
    }
    // The overlay comes back into view, also when auto-hide hid it.
    if !state.settings().overlay_visible {
        state.update_settings(app, |settings| settings.overlay_visible = true);
    }
    crate::autohide::wake(app);
    match sing.model_path() {
        Some(path) => spawn_session(app, path),
        None => {
            sing.lock().start_when_ready = true;
            publish(
                app,
                SingView {
                    phase: Phase::Preparing,
                    ..SingView::idle()
                },
            );
            download_model(app);
        }
    }
}

/// Stops listening; the other players are followed again.
pub fn stop(app: &AppHandle) {
    let sing = app.state::<Sing>();
    let session = {
        let mut inner = sing.lock();
        inner.start_when_ready = false;
        inner.lyrics = None;
        inner.session.take()
    };
    if let Some(session) = session {
        session.stop.store(true, Ordering::SeqCst);
    }
    sing.feed.stop();
    publish(app, SingView::idle());
}

fn send(app: &AppHandle, control: Control) {
    let sing = app.state::<Sing>();
    let sender = sing
        .lock()
        .session
        .as_ref()
        .map(|session| session.control.clone());
    match sender {
        Some(sender) => {
            let _ = sender.send(control);
        }
        None => start(app),
    }
}

fn spawn_session(app: &AppHandle, model_path: PathBuf) {
    let sing = app.state::<Sing>();
    let stop = Arc::new(AtomicBool::new(false));
    let (control, commands) = mpsc::channel();
    {
        let mut inner = sing.lock();
        if let Some(old) = inner.session.take() {
            old.stop.store(true, Ordering::SeqCst);
        }
        inner.session = Some(Session {
            stop: Arc::clone(&stop),
            control,
        });
        inner.start_when_ready = false;
        inner.lyrics = None;
    }
    publish(
        app,
        SingView {
            phase: Phase::Listening,
            ..SingView::idle()
        },
    );
    let source = match &sing.mic_file {
        Some(path) => Source::File(path.clone()),
        None => Source::Microphone,
    };
    let handle = app.clone();
    let started = thread::Builder::new().name("sing".into()).spawn(move || {
        let mut session = Listening::new(handle.clone(), Arc::clone(&stop));
        session.run(model_path, source, &commands);
        // A newer session may have taken over in the meantime.
        if !stop.load(Ordering::SeqCst) {
            let sing = handle.state::<Sing>();
            sing.lock().session = None;
        }
    });
    if let Err(error) = started {
        fail(
            app,
            SingProblem::Failed {
                detail: error.to_string(),
            },
        );
    }
}

/// One candidate the singer may be singing, ready to follow.
struct Candidate {
    found: SungMatch,
    words: Vec<SongWord>,
}

struct Listening {
    app: AppHandle,
    stop: Arc<AtomicBool>,
    recording: Arc<Mutex<Recording>>,
    microphone: bool,
    /// Audio time the current search started from.
    search_from: f64,
    last_attempt: Option<f64>,
    last_query: String,
    rejected: Vec<String>,
    candidates: Vec<Candidate>,
    following: Option<(Candidate, Follower, Track)>,
    moves: u64,
    last_follow: f64,
}

impl Listening {
    fn new(app: AppHandle, stop: Arc<AtomicBool>) -> Self {
        Self {
            app,
            stop,
            recording: Arc::new(Mutex::new(Recording::new(Duration::from_secs(90)))),
            microphone: true,
            search_from: 0.0,
            last_attempt: None,
            last_query: String::new(),
            rejected: Vec::new(),
            candidates: Vec::new(),
            following: None,
            moves: 0,
            last_follow: 0.0,
        }
    }

    fn sing(&self) -> State<'_, Sing> {
        self.app.state::<Sing>()
    }

    fn stopped(&self) -> bool {
        self.stop.load(Ordering::SeqCst)
    }

    fn recording(&self) -> MutexGuard<'_, Recording> {
        self.recording.lock().unwrap_or_else(|p| p.into_inner())
    }

    /// The audio clock now.
    fn now(&self) -> f64 {
        let recording = self.recording();
        match recording.started() {
            Some(started) => started.elapsed().as_secs_f64(),
            None => recording.duration(),
        }
    }

    fn instant_at(&self, audio_time: f64) -> Instant {
        let started = self.recording().started().unwrap_or_else(Instant::now);
        started + Duration::from_secs_f64(audio_time.max(0.0))
    }

    fn run(&mut self, model_path: PathBuf, source: Source, commands: &Receiver<Control>) {
        self.microphone = source == Source::Microphone;
        let sink_recording = Arc::clone(&self.recording);
        let capture = Capture::start(
            source,
            Box::new(move |samples| {
                sink_recording
                    .lock()
                    .unwrap_or_else(|p| p.into_inner())
                    .push(samples, Instant::now());
            }),
        );
        let capture = match capture {
            Ok(capture) => capture,
            Err(error) => return fail(&self.app, capture_problem(error)),
        };
        self.sing().feed.start();
        self.sing().log(|| {
            format!(
                "listening to the {}",
                if self.microphone {
                    "microphone"
                } else {
                    "file"
                }
            )
        });
        let meter = self.spawn_meter();
        let mut transcriber = match transcription_engine(&model_path) {
            Ok(transcriber) => transcriber,
            Err(problem) => {
                drop(capture);
                return fail(&self.app, problem);
            }
        };
        loop {
            if self.stopped() {
                break;
            }
            match commands.recv_timeout(Duration::from_millis(200)) {
                Ok(Control::Next) => self.next_candidate(),
                Ok(Control::Again) => self.listen_again(),
                Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => break,
            }
            if self.stopped() {
                break;
            }
            if let Some(error) = capture.failure() {
                drop(capture);
                return fail(&self.app, capture_problem(error));
            }
            let now = self.now();
            let (peak, last_voice) = {
                let recording = self.recording();
                (recording.peak(), recording.last_voice())
            };
            if self.microphone && now > 3.0 && peak == 0.0 {
                drop(capture);
                return fail(&self.app, SingProblem::Silent);
            }
            if now - last_voice.unwrap_or(0.0) > QUIET_LIMIT {
                self.sing().log(|| "quiet for 30 s, stopping".into());
                break;
            }
            let outcome = if self.following.is_some() {
                self.follow(&mut transcriber, now);
                Ok(())
            } else {
                self.search(&mut transcriber, now)
            };
            if let Err(problem) = outcome {
                drop(capture);
                return fail(&self.app, problem);
            }
        }
        drop(meter);
        drop(capture);
        if !self.stopped() {
            stop(&self.app);
        }
    }

    fn spawn_meter(&self) -> MeterGuard {
        let alive = Arc::new(AtomicBool::new(true));
        let running = Arc::clone(&alive);
        let recording = Arc::clone(&self.recording);
        let app = self.app.clone();
        let stop = Arc::clone(&self.stop);
        let _ = thread::Builder::new()
            .name("sing-meter".into())
            .spawn(move || {
                while running.load(Ordering::SeqCst) && !stop.load(Ordering::SeqCst) {
                    let level = recording.lock().unwrap_or_else(|p| p.into_inner()).level();
                    let _ = app.emit(LEVEL_EVENT, level);
                    thread::sleep(Duration::from_millis(80));
                }
                let _ = app.emit(LEVEL_EVENT, 0.0_f32);
            });
        MeterGuard(alive)
    }

    /// The words in the audio from `from` to `to`, on the audio clock.
    fn hear(
        &self,
        transcriber: &mut Transcriber,
        from: f64,
        to: f64,
    ) -> Result<(String, Vec<HeardWord>), SingProblem> {
        let (start, samples) = self.recording().samples(from, to);
        let transcript = transcriber
            .transcribe(&samples)
            .map_err(|detail| SingProblem::Model { detail })?;
        if transcript.is_hallucination() {
            return Ok((String::new(), Vec::new()));
        }
        let recording = self.recording();
        let words = transcript
            .words
            .iter()
            .map(|word| {
                let mut time = start + word.start;
                // Whisper dates a segment's first word to the silence before
                // it; the voice detection knows better where the phrase starts.
                if word.opens_segment {
                    if let Some(onset) = recording.onsets(time - 0.3, time + 1.5).first() {
                        time = *onset;
                    }
                }
                HeardWord::new(word.text.clone(), time)
            })
            .collect();
        Ok((transcript.text, words))
    }

    fn search(&mut self, transcriber: &mut Transcriber, now: f64) -> Result<(), SingProblem> {
        let (onset, voiced, voice_now) = {
            let recording = self.recording();
            let Some(onset) = recording.first_voice_after(self.search_from) else {
                return Ok(());
            };
            (onset, recording.voiced(onset, now), recording.is_voice())
        };
        if voiced < FIRST_SEARCH {
            return Ok(());
        }
        if let Some(last) = self.last_attempt {
            if now - last < SEARCH_EVERY {
                return Ok(());
            }
        } else if voice_now && voiced < 4.0 && now - onset < 6.0 {
            // Wait for the end of the phrase, so the first search has a whole line.
            return Ok(());
        }
        self.last_attempt = Some(now);
        let from = (onset - 0.3).max(now - LONGEST_WINDOW);
        let (text, heard) = self.hear(transcriber, from, now)?;
        self.sing().log(|| format!("heard {:?}", text));
        if heard.len() < alignment::MIN_WORDS {
            return self.maybe_give_up(voiced, &text);
        }
        let query = heard
            .iter()
            .map(|word| word.text.as_str())
            .collect::<Vec<_>>()
            .join(" ");
        if alignment::words(&query) == alignment::words(&self.last_query) {
            return self.maybe_give_up(voiced, &text);
        }
        self.last_query = query;
        change(&self.app, |view| {
            view.phase = Phase::Searching;
            view.heard = text.clone();
        });

        let app = self.app.clone();
        let sing = app.state::<Sing>();
        let service = app.state::<AppState>().lyrics.clone();
        let options = LookupOptions {
            online: app.state::<AppState>().settings().lookup_online,
            refresh: false,
        };
        // The songs you heard come first, found on the device. Genius is asked
        // only when none of them fits, and only with online lookups on.
        let local = find_sung_locally(&service, &heard, options.online);
        let fits = local
            .matches
            .iter()
            .find(|found| !self.rejects(found))
            .is_some_and(|found| found.alignment.is_confident());
        let found = if fits || !options.online {
            sing.log(|| format!("{} of the songs you heard fit", local.matches.len()));
            local
        } else {
            let search =
                GeniusClient::with_base_url(&sing.genius_url, &sing.token(), &sing.user_agent);
            let found = tauri::async_runtime::block_on(find_sung(
                &search,
                &service,
                &text,
                &heard,
                CANDIDATES,
                options,
                |title, artist| normalize::track_key(title, artist, None),
            ));
            match found {
                Ok(found) => found,
                // Without Genius, as offline or without a token, the songs you
                // heard may still fit once more has been sung.
                Err(_) if !local.matches.is_empty() => local,
                Err(error) => return Err(search_problem(error)),
            }
        };
        sing.log(|| {
            format!(
                "{} candidates: {}",
                found.candidates,
                found
                    .matches
                    .iter()
                    .map(|found| format!(
                        "{} by {} ({} words, {:.0} %)",
                        found.candidate.title,
                        found.candidate.artist,
                        found.alignment.matched,
                        found.alignment.confidence * 100.0
                    ))
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        });
        self.candidates = found
            .matches
            .into_iter()
            .filter(|found| !self.rejects(found))
            .filter(|found| found.alignment.matched >= 2)
            .map(|found| Candidate {
                words: alignment::song_words(&found.lyrics.lines),
                found,
            })
            .collect();
        if self
            .candidates
            .first()
            .is_some_and(|candidate| candidate.found.alignment.is_confident())
        {
            self.follow_next();
            return Ok(());
        }
        if self.candidates.is_empty() {
            if let Some(LyricsOutcome::Offline) = found.lookup_problem {
                return Err(SingProblem::Offline);
            }
        }
        self.maybe_give_up(voiced, &text)
    }

    /// Whether the listener said "Not this song" to `found` before.
    fn rejects(&self, found: &SungMatch) -> bool {
        self.rejected.contains(&normalize::track_key(
            &found.candidate.title,
            &found.candidate.artist,
            None,
        ))
    }

    fn maybe_give_up(&mut self, voiced: f64, heard: &str) -> Result<(), SingProblem> {
        if voiced >= GIVE_UP {
            self.sing().log(|| "no song found".into());
            self.sing().feed.stop();
            self.stop.store(true, Ordering::SeqCst);
            self.sing().lock().session = None;
            let heard = heard.to_owned();
            publish(
                &self.app,
                SingView {
                    phase: Phase::NotFound,
                    heard,
                    ..SingView::idle()
                },
            );
        }
        Ok(())
    }

    /// Follows the best remaining candidate.
    fn follow_next(&mut self) {
        if self.candidates.is_empty() {
            return self.listen_again();
        }
        let candidate = self.candidates.remove(0);
        let found = &candidate.found;
        let title = found.candidate.title.clone();
        let artist = found.candidate.artist.clone();
        let key = normalize::track_key(&title, &artist, None);
        let last_line = found
            .lyrics
            .lines
            .last()
            .map_or(0.0, |line| line.end.unwrap_or(line.time));
        let track = Track {
            key: key.clone(),
            source_id: None,
            title: title.clone(),
            artist: artist.clone(),
            album: found.album.clone(),
            duration: found.duration.or(Some(last_line + 5.0)),
            artwork_url: found.candidate.artwork_url.clone(),
        };
        let follower = Follower::new(found.alignment.offset);
        let now = self.now();
        let app = self.app.clone();
        let sing = app.state::<Sing>();
        sing.log(|| {
            format!(
                "found {title:?} by {artist:?} at {:.2} s (line {}, offset {:.2})",
                follower.position_at(now),
                found.alignment.line,
                found.alignment.offset
            )
        });
        sing.lock().lyrics = Some((
            key,
            found.lyrics.clone(),
            if found.own { "local" } else { "online" },
        ));
        sing.feed.follow(
            track.clone(),
            follower.position_at(now),
            self.instant_at(now),
        );
        self.moves = sing.feed.moves();
        self.last_follow = now;
        let alternatives = self.candidates.len();
        publish(
            &self.app,
            SingView {
                phase: Phase::Following,
                song: Some(FoundSong { title, artist }),
                alternatives,
                ..SingView::idle()
            },
        );
        self.following = Some((candidate, follower, track));
    }

    /// "Not this song".
    fn next_candidate(&mut self) {
        if let Some((current, ..)) = self.following.take() {
            self.rejected.push(normalize::track_key(
                &current.found.candidate.title,
                &current.found.candidate.artist,
                None,
            ));
        }
        self.follow_next();
    }

    /// Forgets the song and searches again from what comes next.
    fn listen_again(&mut self) {
        self.following = None;
        self.candidates.clear();
        self.sing().lock().lyrics = None;
        self.sing().feed.clear_song();
        self.search_from = self.now();
        self.last_attempt = None;
        self.last_query.clear();
        publish(
            &self.app,
            SingView {
                phase: Phase::Listening,
                ..SingView::idle()
            },
        );
    }

    /// Listens to the last few seconds and keeps the lyrics with the singer.
    fn follow(&mut self, transcriber: &mut Transcriber, now: f64) {
        if now - self.last_follow < FOLLOW_EVERY {
            return;
        }
        self.last_follow = now;
        let app = self.app.clone();
        let sing = app.state::<Sing>();
        if sing.feed.is_paused() {
            return;
        }
        let moves = sing.feed.moves();
        if moves != self.moves {
            // The listener jumped to a line: their timing wins.
            self.moves = moves;
            if let (Some(position), Some((_, follower, _))) = (
                sing.feed.position_at(Instant::now()),
                self.following.as_mut(),
            ) {
                *follower = Follower::new(position - now);
            }
            return;
        }
        let from = {
            let recording = self.recording();
            if recording.voiced(now - FOLLOW_WINDOW, now) < 1.5 {
                return;
            }
            // Start at the beginning of a phrase, so Whisper hears whole words
            // and dates the first one right.
            recording
                .onsets(now - FOLLOW_WINDOW, now - 2.0)
                .first()
                .map_or(now - FOLLOW_WINDOW, |onset| onset - 0.25)
        };
        let Ok((text, heard)) = self.hear(transcriber, from, now) else {
            return;
        };
        let at = self.instant_at(now);
        let Some((candidate, follower, track)) = self.following.as_mut() else {
            return;
        };
        if heard.len() < alignment::MIN_WORDS {
            return;
        }
        let first = heard.first().map_or(now, |word| word.time);
        let near = alignment::words_between(
            &candidate.words,
            follower.position_at(first) - 8.0,
            follower.position_at(now) + 12.0,
        );
        let alignment = alignment::align_within(&heard, &candidate.words, near)
            .filter(alignment::Alignment::is_confident)
            .or_else(|| alignment::align(&heard, &candidate.words));
        let Some(alignment) = alignment else {
            return;
        };
        let step = follower.observe(&alignment);
        let position = follower.position_at(now);
        sing.log(|| {
            format!(
                "heard {text:?}: {step:?}, now at {position:.2} s (line {})",
                alignment.line
            )
        });
        if matches!(step, FollowStep::Nudged { .. } | FollowStep::Jumped { .. }) {
            sing.feed.follow(track.clone(), position, at);
            self.moves = sing.feed.moves();
        }
    }
}

struct MeterGuard(Arc<AtomicBool>);

impl Drop for MeterGuard {
    fn drop(&mut self) {
        self.0.store(false, Ordering::SeqCst);
    }
}

#[cfg(feature = "sing-to-find")]
type Transcriber = WhisperEngine;

#[cfg(feature = "sing-to-find")]
struct WhisperEngine(transcription::Transcriber);

#[cfg(feature = "sing-to-find")]
impl WhisperEngine {
    fn transcribe(&mut self, samples: &[f32]) -> Result<transcription::Transcript, String> {
        self.0
            .transcribe(samples, None)
            .map_err(|error| error.to_string())
    }
}

#[cfg(feature = "sing-to-find")]
fn transcription_engine(path: &Path) -> Result<Transcriber, SingProblem> {
    transcription::Transcriber::load(path)
        .map(WhisperEngine)
        .map_err(|error| match error {
            transcription::TranscribeError::Processor => SingProblem::Processor,
            other => SingProblem::Model {
                detail: other.to_string(),
            },
        })
}

/// Without Whisper this build cannot transcribe; [`start`] says so first.
#[cfg(not(feature = "sing-to-find"))]
struct Transcriber;

#[cfg(not(feature = "sing-to-find"))]
impl Transcriber {
    fn transcribe(&mut self, _samples: &[f32]) -> Result<transcription::Transcript, String> {
        Err("this build cannot transcribe".into())
    }
}

#[cfg(not(feature = "sing-to-find"))]
fn transcription_engine(_path: &Path) -> Result<Transcriber, SingProblem> {
    Err(SingProblem::Unavailable)
}

fn capture_problem(error: CaptureError) -> SingProblem {
    match error {
        CaptureError::NoDevice => SingProblem::NoMicrophone,
        CaptureError::Denied => SingProblem::Denied,
        CaptureError::Busy => SingProblem::Busy,
        CaptureError::Unsupported => SingProblem::Unavailable,
        CaptureError::File(detail) | CaptureError::Failed(detail) => SingProblem::Failed { detail },
    }
}

fn search_problem(error: SearchError) -> SingProblem {
    match error {
        SearchError::NoToken => SingProblem::NoToken,
        SearchError::Unauthorized => SingProblem::Unauthorized,
        SearchError::Offline => SingProblem::Offline,
        SearchError::RateLimited { .. } => SingProblem::RateLimited,
        other => SingProblem::Failed {
            detail: other.to_string(),
        },
    }
}

/// Downloads the speech model in the background, with progress events.
fn download_model(app: &AppHandle) {
    let sing = app.state::<Sing>();
    let Some(dir) = sing.models_dir.clone() else {
        return fail(
            app,
            SingProblem::Model {
                detail: "no folder for the model".into(),
            },
        );
    };
    let cancel = Arc::new(AtomicBool::new(false));
    {
        let mut inner = sing.lock();
        if inner.download.is_some() {
            return;
        }
        inner.download = Some(Download {
            cancel: Arc::clone(&cancel),
            received: MODEL.partial_size(&dir),
        });
        inner.download_error = None;
    }
    emit_model(app);
    let url = MODEL.url(&sing.model_url);
    let http = model::http_client(&sing.user_agent);
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let mut last_emit = Instant::now();
        let progress_app = app.clone();
        let result = model::download(
            &MODEL,
            &url,
            &dir,
            &http,
            Retry::default(),
            &cancel,
            move |progress| {
                let sing = progress_app.state::<Sing>();
                if let Some(download) = sing.lock().download.as_mut() {
                    download.received = progress.received;
                }
                if last_emit.elapsed() >= Duration::from_millis(250) {
                    last_emit = Instant::now();
                    emit_model(&progress_app);
                }
            },
        )
        .await;
        let sing = app.state::<Sing>();
        let start_now = {
            let mut inner = sing.lock();
            inner.download = None;
            inner.download_error = result.as_ref().err().map(ToString::to_string);
            std::mem::take(&mut inner.start_when_ready)
        };
        emit_model(&app);
        match result {
            Ok(path) if start_now => spawn_session(&app, path),
            Ok(_) => {}
            Err(model::DownloadError::Cancelled) => {
                if start_now {
                    stop(&app);
                }
            }
            Err(error) if start_now => fail(
                &app,
                SingProblem::Model {
                    detail: error.to_string(),
                },
            ),
            Err(_) => {}
        }
    });
}

#[tauri::command]
pub fn sing_view(state: State<'_, Sing>) -> SingView {
    state.view()
}

#[tauri::command]
pub fn sing_toggle(app: AppHandle) -> SingView {
    toggle(&app);
    app.state::<Sing>().view()
}

#[tauri::command]
pub fn sing_stop(app: AppHandle) {
    stop(&app);
}

/// "Not this song".
#[tauri::command]
pub fn sing_next(app: AppHandle) {
    send(&app, Control::Next);
}

/// Listens again, or starts over after nothing was found.
#[tauri::command]
pub fn sing_again(app: AppHandle) {
    send(&app, Control::Again);
}

#[tauri::command]
pub fn sing_settings(state: State<'_, Sing>) -> SingSettings {
    state.settings_view()
}

#[tauri::command]
pub fn sing_set_token(state: State<'_, Sing>, token: String) -> SingSettings {
    let token = token.trim().to_owned();
    state.lock().token = token.clone();
    if let Some(path) = &state.config_path {
        let stored = StoredConfig {
            genius_token: token,
        };
        let written = path
            .parent()
            .map_or(Ok(()), fs::create_dir_all)
            .and_then(|()| {
                let text = serde_json::to_string_pretty(&stored).map_err(std::io::Error::other)?;
                let temporary = path.with_extension("json.tmp");
                fs::write(&temporary, text)?;
                fs::rename(&temporary, path)
            });
        if let Err(error) = written {
            log::error!("could not save the Genius token: {error}");
        }
    }
    state.settings_view()
}

#[tauri::command]
pub fn sing_download_model(app: AppHandle) -> ModelView {
    download_model(&app);
    app.state::<Sing>().model_view()
}

#[tauri::command]
pub fn sing_cancel_download(app: AppHandle) -> ModelView {
    let sing = app.state::<Sing>();
    if let Some(download) = &sing.lock().download {
        download.cancel.store(true, Ordering::SeqCst);
    }
    sing.model_view()
}

#[tauri::command]
pub fn sing_remove_model(app: AppHandle) -> ModelView {
    let sing = app.state::<Sing>();
    if sing.is_active() {
        stop(&app);
    }
    if let Some(dir) = &sing.models_dir {
        if let Err(error) = MODEL.remove(dir) {
            log::error!("could not remove the speech model: {error}");
        }
    }
    sing.lock().download_error = None;
    emit_model(&app);
    sing.model_view()
}

#[tauri::command]
pub fn sing_open_genius() {
    if let Err(error) = open::that(GENIUS_API_CLIENTS) {
        log::error!("could not open {GENIUS_API_CLIENTS}: {error}");
    }
}

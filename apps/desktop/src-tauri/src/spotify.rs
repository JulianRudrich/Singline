//! The optional Spotify connection: the listener's Client ID, the login
//! through the browser, ♥ for the playing song and Spotify's top lists. The
//! tokens stay inside `spotify-connect`; nothing here logs them.

use std::collections::HashMap;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Duration;

use serde::{Deserialize, Serialize};
use spotify_connect::callback::{CallbackPages, Page};
use spotify_connect::credentials::{CredentialStore, MemoryCredentials, SystemCredentials};
use spotify_connect::{
    pkce, Config, ListenError, LoginError, Persistence, SpotifyClient, SpotifyError, TimeRange,
    TopItem, TopKind, User,
};
use tauri::async_runtime::JoinHandle;
use tauri::{AppHandle, Emitter, Manager, State};

use crate::i18n::{resolve, Texts};
use crate::{windows, AppState};

/// The event that carries [`Status`] to every window.
pub const EVENT: &str = "spotify";
const DASHBOARD_URL: &str = "https://developer.spotify.com/dashboard";
/// How long the login waits for the browser.
const LOGIN_TIMEOUT: Duration = Duration::from_secs(5 * 60);
const TOP_LIMIT: usize = 10;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Phase {
    #[default]
    Disconnected,
    /// The login page is open in the browser.
    Waiting,
    /// Restoring a stored login or finishing a new one.
    Connecting,
    Connected,
    Failed,
}

/// Why the last attempt did not connect.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Problem {
    PortInUse,
    Browser,
    Denied,
    TimedOut,
    Offline,
    Expired,
    Forbidden { detail: Option<String> },
    Refused { detail: String },
    Failed { detail: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Account {
    pub id: String,
    pub name: String,
}

/// What the web interface shows about the connection.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    pub client_id: String,
    pub redirect_uri: &'static str,
    pub phase: Phase,
    pub account: Option<Account>,
    pub problem: Option<Problem>,
    /// The login is in the system's credential store and survives a restart.
    pub persistent: bool,
    /// The login lacks a scope the app uses now.
    pub needs_reconnect: bool,
    /// The login page, while waiting, for when no browser opened.
    pub login_url: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TopLists {
    pub artists: Vec<TopItem>,
    pub tracks: Vec<TopItem>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct StoredConfig {
    client_id: String,
}

#[derive(Default)]
struct Inner {
    client_id: String,
    phase: Phase,
    account: Option<Account>,
    problem: Option<Problem>,
    client: Option<Arc<SpotifyClient>>,
    login: Option<JoinHandle<()>>,
    login_url: Option<String>,
    persistent: bool,
    needs_reconnect: bool,
    top: HashMap<TimeRange, TopLists>,
    /// Counts logins and disconnects, so a late answer of an old one is dropped.
    generation: u64,
}

pub struct Spotify {
    /// `None` in the demo, which never changes the listener's files or keychain.
    config_path: Option<PathBuf>,
    user_agent: String,
    accounts_url: Option<String>,
    api_url: Option<String>,
    credentials: Arc<dyn CredentialStore>,
    inner: Mutex<Inner>,
}

impl Spotify {
    pub fn new(demo: bool, config_dir: Option<&Path>, user_agent: String) -> Self {
        let config_path = config_dir
            .filter(|_| !demo)
            .map(|directory| directory.join("spotify.json"));
        let client_id = config_path
            .as_deref()
            .and_then(|path| fs::read_to_string(path).ok())
            .and_then(|text| serde_json::from_str::<StoredConfig>(&text).ok())
            .map(|stored| stored.client_id.trim().to_owned())
            .unwrap_or_default();
        let credentials: Arc<dyn CredentialStore> = if demo {
            Arc::new(MemoryCredentials::new())
        } else {
            Arc::new(SystemCredentials::default())
        };
        let override_url = |name: &str| std::env::var(name).ok().filter(|url| !url.is_empty());
        Self {
            config_path,
            user_agent,
            accounts_url: override_url("SINGLINE_SPOTIFY_ACCOUNTS_URL"),
            api_url: override_url("SINGLINE_SPOTIFY_API_URL"),
            credentials,
            inner: Mutex::new(Inner {
                client_id,
                ..Inner::default()
            }),
        }
    }

    fn lock(&self) -> MutexGuard<'_, Inner> {
        self.inner.lock().unwrap_or_else(|p| p.into_inner())
    }

    pub fn status(&self) -> Status {
        let inner = self.lock();
        Status {
            client_id: inner.client_id.clone(),
            redirect_uri: pkce::REDIRECT_URI,
            phase: inner.phase,
            account: inner.account.clone(),
            problem: inner.problem.clone(),
            persistent: inner.persistent,
            needs_reconnect: inner.needs_reconnect,
            login_url: inner.login_url.clone(),
        }
    }

    /// The client of a working connection.
    pub fn client(&self) -> Option<Arc<SpotifyClient>> {
        let inner = self.lock();
        (inner.phase == Phase::Connected)
            .then(|| inner.client.clone())
            .flatten()
    }

    /// The Spotify user ID of a working connection.
    pub fn account_id(&self) -> Option<String> {
        let inner = self.lock();
        (inner.phase == Phase::Connected)
            .then(|| inner.account.as_ref().map(|account| account.id.clone()))
            .flatten()
    }

    fn new_client(&self, client_id: &str) -> Arc<SpotifyClient> {
        let mut config = Config::new(client_id, &self.user_agent);
        if let Some(url) = &self.accounts_url {
            config.accounts_url.clone_from(url);
        }
        if let Some(url) = &self.api_url {
            config.api_url.clone_from(url);
        }
        Arc::new(SpotifyClient::new(config, Arc::clone(&self.credentials)))
    }

    fn save_config(&self) {
        let Some(path) = &self.config_path else {
            return;
        };
        let stored = StoredConfig {
            client_id: self.lock().client_id.clone(),
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
            log::error!("could not save the Spotify Client ID: {error}");
        }
    }
}

fn emit(app: &AppHandle) {
    let _ = app.emit(EVENT, app.state::<Spotify>().status());
}

/// Changes the state when no newer login or disconnect came in between.
fn update(app: &AppHandle, generation: u64, change: impl FnOnce(&mut Inner)) {
    {
        let spotify = app.state::<Spotify>();
        let mut inner = spotify.lock();
        if inner.generation != generation {
            return;
        }
        change(&mut inner);
    }
    emit(app);
}

fn fail(app: &AppHandle, generation: u64, problem: Problem) {
    update(app, generation, |inner| {
        inner.phase = Phase::Failed;
        inner.problem = Some(problem);
        inner.login = None;
        inner.login_url = None;
    });
}

/// At launch: connects again with a stored login, without a browser.
pub fn restore(app: &AppHandle) {
    let spotify = app.state::<Spotify>();
    let client_id = spotify.lock().client_id.clone();
    if !pkce::is_valid_client_id(&client_id) {
        return;
    }
    let client = spotify.new_client(&client_id);
    let generation = {
        let mut inner = spotify.lock();
        inner.generation += 1;
        inner.client = Some(Arc::clone(&client));
        inner.phase = Phase::Connecting;
        inner.problem = None;
        inner.generation
    };
    emit(app);
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        if !client.has_login().await {
            update(&app, generation, |inner| {
                inner.phase = Phase::Disconnected;
                inner.client = None;
            });
            return;
        }
        finish_connecting(&app, &client, generation).await;
    });
}

async fn finish_connecting(app: &AppHandle, client: &SpotifyClient, generation: u64) {
    match client.me().await {
        Ok(user) => {
            let needs_reconnect = client.granted_all_scopes().await == Some(false);
            let persistent = client.persistence() == Some(Persistence::System);
            if !persistent {
                if let Some(reason) = client.store_problem() {
                    log::warn!("the Spotify login stays only until quit: {reason}");
                }
            }
            update(app, generation, |inner| {
                inner.phase = Phase::Connected;
                inner.account = Some(account(&user));
                inner.problem = None;
                inner.persistent = persistent;
                inner.needs_reconnect = needs_reconnect;
                inner.login = None;
                inner.login_url = None;
            });
        }
        Err(SpotifyError::LoginExpired | SpotifyError::NotConnected) => {
            update(app, generation, |inner| {
                inner.phase = Phase::Disconnected;
                inner.client = None;
                inner.problem = Some(Problem::Expired);
            });
        }
        Err(error) => fail(app, generation, problem(&error)),
    }
}

fn account(user: &User) -> Account {
    Account {
        id: user.id.clone(),
        name: user.name().to_owned(),
    }
}

fn problem(error: &SpotifyError) -> Problem {
    match error {
        SpotifyError::Offline | SpotifyError::Network(_) => Problem::Offline,
        SpotifyError::LoginExpired | SpotifyError::NotConnected => Problem::Expired,
        SpotifyError::Forbidden(detail) => Problem::Forbidden {
            detail: detail.clone(),
        },
        SpotifyError::LoginRefused(detail) => Problem::Refused {
            detail: detail.clone(),
        },
        other => Problem::Failed {
            detail: other.to_string(),
        },
    }
}

fn login_problem(error: &LoginError) -> Problem {
    match error {
        LoginError::Listen(ListenError::PortInUse(_)) => Problem::PortInUse,
        LoginError::Denied => Problem::Denied,
        LoginError::TimedOut => Problem::TimedOut,
        LoginError::Spotify(error) => problem(error),
        other => Problem::Failed {
            detail: other.to_string(),
        },
    }
}

/// Reacts to an error of any Spotify call: an expired login disconnects.
pub fn note_error(app: &AppHandle, error: &SpotifyError) {
    if matches!(
        error,
        SpotifyError::LoginExpired | SpotifyError::NotConnected
    ) {
        let generation = app.state::<Spotify>().lock().generation;
        update(app, generation, |inner| {
            inner.phase = Phase::Disconnected;
            inner.client = None;
            inner.account = None;
            inner.problem = Some(Problem::Expired);
            inner.top.clear();
        });
    }
}

fn callback_pages(app: &AppHandle) -> CallbackPages {
    let lang = resolve(app.state::<AppState>().settings().language);
    let texts = Texts::new(lang);
    let not_connected = texts.get("spotify.page.notConnected");
    CallbackPages {
        language: serde_json::to_value(lang)
            .ok()
            .and_then(|value| value.as_str().map(str::to_owned))
            .unwrap_or_else(|| "en".into()),
        connected: Page::new(
            &texts.get("spotify.page.connectedTitle"),
            &texts.get("spotify.page.connectedText"),
        ),
        denied: Page::new(&not_connected, &texts.get("spotify.page.deniedText")),
        failed: Page::new(&not_connected, &texts.get("spotify.page.failedText")),
        outdated: Page::new(
            &texts.get("spotify.page.outdatedTitle"),
            &texts.get("spotify.page.outdatedText"),
        ),
    }
}

async fn login(app: AppHandle, client: Arc<SpotifyClient>, generation: u64) {
    let pending = match client.start_login().await {
        Ok(pending) => pending,
        Err(error) => return fail(&app, generation, login_problem(&error)),
    };
    let url = pending.url().to_owned();
    update(&app, generation, |inner| {
        inner.login_url = Some(url.clone())
    });
    if open_external(url).await.is_err() {
        // The pane offers the link to copy instead.
        update(&app, generation, |inner| {
            inner.problem = Some(Problem::Browser)
        });
    }
    let pages = callback_pages(&app);
    match client.finish_login(pending, &pages, LOGIN_TIMEOUT).await {
        Ok(_) => {
            update(&app, generation, |inner| {
                inner.phase = Phase::Connecting;
                inner.problem = None;
                inner.login_url = None;
            });
            finish_connecting(&app, &client, generation).await;
            if app.state::<Spotify>().client().is_some() {
                // Back from the browser to the pane that started the login.
                windows::open_settings(&app);
            }
        }
        Err(error) => fail(&app, generation, login_problem(&error)),
    }
}

/// Opens a web page or a `spotify:` link with the system.
pub async fn open_external(target: String) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || open::that_detached(&target))
        .await
        .map_err(|error| error.to_string())?
        .map_err(|error| error.to_string())
}

/// A Spotify item worth opening: `spotify:track:…`, `…:artist:…`, `…:album:…`
/// or `…:playlist:…`.
fn spotify_item(uri: &str) -> Option<(&str, &str)> {
    let mut parts = uri.strip_prefix("spotify:")?.split(':');
    let (kind, id) = (parts.next()?, parts.next()?);
    let known = matches!(kind, "track" | "artist" | "album" | "playlist");
    let valid = !id.is_empty() && id.bytes().all(|byte| byte.is_ascii_alphanumeric());
    (known && valid && parts.next().is_none()).then_some((kind, id))
}

pub fn is_track_uri(uri: &str) -> bool {
    spotify_item(uri).is_some_and(|(kind, _)| kind == "track")
}

/// Opens an item in the Spotify app, or on open.spotify.com without one.
pub async fn open_item(uri: &str) -> Result<(), String> {
    let (kind, id) = spotify_item(uri).ok_or("not a Spotify link")?;
    if open_external(uri.to_owned()).await.is_ok() {
        return Ok(());
    }
    open_external(format!("https://open.spotify.com/{kind}/{id}")).await
}

#[tauri::command]
pub fn spotify_status(app: AppHandle, spotify: State<'_, Spotify>) -> Status {
    // A login stored at a launch without internet is tried again when the
    // listener looks at it.
    let retry = {
        let inner = spotify.lock();
        inner.phase == Phase::Failed
            && inner.problem == Some(Problem::Offline)
            && inner.login.is_none()
    };
    if retry {
        restore(&app);
    }
    spotify.status()
}

#[tauri::command]
pub fn spotify_set_client_id(
    app: AppHandle,
    spotify: State<'_, Spotify>,
    client_id: String,
) -> Status {
    let client_id: String = client_id.trim().chars().take(64).collect();
    let changed = {
        let mut inner = spotify.lock();
        let changed = inner.client_id != client_id;
        inner.client_id = client_id;
        changed
    };
    if changed {
        spotify.save_config();
        // A login belongs to one Client ID.
        disconnect(&app);
    }
    spotify.status()
}

#[tauri::command]
pub fn spotify_connect(app: AppHandle, spotify: State<'_, Spotify>) -> Status {
    let client_id = spotify.lock().client_id.clone();
    if !pkce::is_valid_client_id(&client_id) {
        return spotify.status();
    }
    let client = spotify.new_client(&client_id);
    let generation = {
        let mut inner = spotify.lock();
        if let Some(previous) = inner.login.take() {
            previous.abort();
        }
        inner.generation += 1;
        inner.client = Some(Arc::clone(&client));
        inner.phase = Phase::Waiting;
        inner.problem = None;
        inner.account = None;
        inner.login_url = None;
        inner.top.clear();
        inner.generation
    };
    let handle = tauri::async_runtime::spawn(login(app.clone(), client, generation));
    {
        let mut inner = spotify.lock();
        if inner.generation == generation && inner.phase == Phase::Waiting {
            inner.login = Some(handle);
        }
    }
    emit(&app);
    spotify.status()
}

#[tauri::command]
pub fn spotify_cancel_login(app: AppHandle, spotify: State<'_, Spotify>) -> Status {
    {
        let mut inner = spotify.lock();
        if matches!(inner.phase, Phase::Waiting | Phase::Connecting) {
            // Dropping the login closes its listener and frees the port.
            if let Some(login) = inner.login.take() {
                login.abort();
            }
            inner.generation += 1;
            inner.phase = Phase::Disconnected;
            inner.client = None;
            inner.problem = None;
            inner.login_url = None;
        }
    }
    emit(&app);
    spotify.status()
}

#[tauri::command]
pub fn spotify_disconnect(app: AppHandle, spotify: State<'_, Spotify>) -> Status {
    disconnect(&app);
    spotify.status()
}

/// Forgets the login, in memory and in the system's credential store.
fn disconnect(app: &AppHandle) {
    let spotify = app.state::<Spotify>();
    let client = {
        let mut inner = spotify.lock();
        if let Some(login) = inner.login.take() {
            login.abort();
        }
        inner.generation += 1;
        inner.phase = Phase::Disconnected;
        inner.account = None;
        inner.problem = None;
        inner.login_url = None;
        inner.persistent = false;
        inner.needs_reconnect = false;
        inner.top.clear();
        inner.client.take()
    };
    let credentials = Arc::clone(&spotify.credentials);
    tauri::async_runtime::spawn(async move {
        let forgotten = match client {
            Some(client) => client.forget().await,
            None => tauri::async_runtime::spawn_blocking(move || credentials.delete())
                .await
                .unwrap_or(Ok(())),
        };
        if let Err(error) = forgotten {
            log::error!("could not remove the Spotify login: {error}");
        }
    });
    emit(app);
}

#[tauri::command]
pub async fn spotify_open_dashboard() -> Result<(), String> {
    open_external(DASHBOARD_URL.to_owned()).await
}

#[tauri::command]
pub async fn spotify_open(uri: String) -> Result<(), String> {
    open_item(&uri).await
}

/// Opens the song playing in Spotify: by its link where the player reported
/// one, otherwise as a search for its title and artist, as on Windows, which
/// does not tell Spotify's track IDs. `web` opens the web player instead of
/// the app, for a song playing in the browser.
#[tauri::command]
pub async fn spotify_open_song(
    uri: Option<String>,
    title: String,
    artist: String,
    web: bool,
) -> Result<(), String> {
    let (app_link, web_link) =
        song_links(uri.as_deref(), &title, &artist).ok_or("no song to open")?;
    if !web && open_external(app_link).await.is_ok() {
        return Ok(());
    }
    open_external(web_link).await
}

/// Where a song opens in the Spotify app and on the web: its track, or a
/// search for its title and artist.
fn song_links(uri: Option<&str>, title: &str, artist: &str) -> Option<(String, String)> {
    if let Some((kind, id)) = uri
        .and_then(spotify_item)
        .filter(|(kind, _)| *kind == "track")
    {
        return Some((
            format!("spotify:{kind}:{id}"),
            format!("https://open.spotify.com/{kind}/{id}"),
        ));
    }
    let query = [title.trim(), artist.trim()]
        .into_iter()
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join(" ");
    if query.is_empty() {
        return None;
    }
    let query = encode(&query);
    Some((
        format!("spotify:search:{query}"),
        format!("https://open.spotify.com/search/{query}"),
    ))
}

/// Percent-encodes all but letters, digits and `-_.~`, for a link.
fn encode(text: &str) -> String {
    let mut encoded = String::with_capacity(text.len());
    for byte in text.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~') {
            encoded.push(char::from(byte));
        } else {
            let _ = write!(encoded, "%{byte:02X}");
        }
    }
    encoded
}

/// Whether the song is in Liked Songs; `None` without a connection.
#[tauri::command]
pub async fn spotify_is_liked(app: AppHandle, uri: String) -> Result<Option<bool>, String> {
    let Some(client) = app.state::<Spotify>().client() else {
        return Ok(None);
    };
    if !is_track_uri(&uri) {
        return Ok(None);
    }
    match client.is_saved(&uri).await {
        Ok(saved) => Ok(Some(saved)),
        Err(error) => {
            note_error(&app, &error);
            Ok(None)
        }
    }
}

/// Adds the song to Liked Songs or removes it; returns the new state.
#[tauri::command]
pub async fn spotify_set_liked(app: AppHandle, uri: String, liked: bool) -> Result<bool, String> {
    let client = app
        .state::<Spotify>()
        .client()
        .ok_or_else(|| SpotifyError::NotConnected.to_string())?;
    if !is_track_uri(&uri) {
        return Err("not a Spotify track".into());
    }
    match client.set_saved(&uri, liked).await {
        Ok(()) => Ok(liked),
        Err(error) => {
            note_error(&app, &error);
            Err(error.to_string())
        }
    }
}

/// Spotify's own top artists and tracks, kept in memory while the app runs.
#[tauri::command]
pub async fn spotify_top(app: AppHandle, range: TimeRange) -> Result<TopLists, String> {
    let spotify = app.state::<Spotify>();
    if let Some(cached) = spotify.lock().top.get(&range).cloned() {
        return Ok(cached);
    }
    let client = spotify
        .client()
        .ok_or_else(|| SpotifyError::NotConnected.to_string())?;
    let lists = async {
        Ok::<_, SpotifyError>(TopLists {
            artists: client.top(TopKind::Artists, range, TOP_LIMIT).await?,
            tracks: client.top(TopKind::Tracks, range, TOP_LIMIT).await?,
        })
    }
    .await;
    match lists {
        Ok(lists) => {
            spotify.lock().top.insert(range, lists.clone());
            Ok(lists)
        }
        Err(error) => {
            note_error(&app, &error);
            Err(error.to_string())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_song_opens_by_its_track_or_else_by_a_search() {
        assert_eq!(
            song_links(
                Some("spotify:track:4uLU6hMCjMI75M1A2tKUQC"),
                "Lady of Namek",
                "Tory Lanez"
            ),
            Some((
                "spotify:track:4uLU6hMCjMI75M1A2tKUQC".into(),
                "https://open.spotify.com/track/4uLU6hMCjMI75M1A2tKUQC".into()
            ))
        );
        // Windows reports no track; an episode is no song to open.
        for uri in [None, Some("spotify:episode:1a2b"), Some("not a link")] {
            assert_eq!(
                song_links(uri, "Lady of Namek", "Tory Lanez"),
                Some((
                    "spotify:search:Lady%20of%20Namek%20Tory%20Lanez".into(),
                    "https://open.spotify.com/search/Lady%20of%20Namek%20Tory%20Lanez".into()
                ))
            );
        }
        assert_eq!(song_links(None, "  ", ""), None);
    }

    #[test]
    fn search_words_are_encoded_for_a_link() {
        assert_eq!(encode("Beyoncé & Jay-Z"), "Beyonc%C3%A9%20%26%20Jay-Z");
        assert_eq!(encode("AC/DC #1?"), "AC%2FDC%20%231%3F");
    }

    #[test]
    fn opens_only_spotify_items() {
        assert_eq!(
            spotify_item("spotify:track:4uLU6h"),
            Some(("track", "4uLU6h"))
        );
        assert_eq!(
            spotify_item("spotify:artist:0OdUWJ0sBjDrqHygGUXeCF"),
            Some(("artist", "0OdUWJ0sBjDrqHygGUXeCF"))
        );
        assert_eq!(spotify_item("spotify:episode:abc"), None);
        assert_eq!(spotify_item("spotify:track:abc:extra"), None);
        assert_eq!(spotify_item("spotify:track:../x"), None);
        assert_eq!(spotify_item("https://example.com"), None);
        assert!(is_track_uri("spotify:track:abc"));
        assert!(!is_track_uri("spotify:album:abc"));
        assert!(!is_track_uri("demo"));
    }

    #[test]
    fn the_client_id_survives_a_restart_but_not_the_demo() {
        let directory =
            std::env::temp_dir().join(format!("singline-spotify-{}", std::process::id()));
        let spotify = Spotify::new(false, Some(&directory), "test".into());
        assert_eq!(spotify.status().client_id, "");
        spotify.lock().client_id = "0123456789abcdef0123456789abcdef".into();
        spotify.save_config();
        let again = Spotify::new(false, Some(&directory), "test".into());
        assert_eq!(again.status().client_id, "0123456789abcdef0123456789abcdef");
        assert_eq!(again.status().phase, Phase::Disconnected);
        assert_eq!(
            again.status().redirect_uri,
            "http://127.0.0.1:43821/callback"
        );
        let demo = Spotify::new(true, Some(&directory), "test".into());
        assert_eq!(demo.status().client_id, "");
        let _ = fs::remove_dir_all(directory);
    }

    #[test]
    fn problems_are_named_for_the_interface() {
        assert_eq!(problem(&SpotifyError::Offline), Problem::Offline);
        assert_eq!(
            login_problem(&LoginError::Listen(ListenError::PortInUse(43821))),
            Problem::PortInUse
        );
        let json = serde_json::to_value(Problem::Forbidden { detail: None }).expect("json");
        assert_eq!(
            json,
            serde_json::json!({"kind": "forbidden", "detail": null})
        );
    }
}

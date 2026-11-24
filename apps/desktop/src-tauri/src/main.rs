//! Singline: synced lyrics above your other windows.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod autohide;
mod browser_bridge;
mod commands;
mod dock;
mod history;
mod hotkey;
mod i18n;
mod logging;
mod menu_panel;
mod now;
mod own_lyrics;
mod playback;
mod players;
mod report;
mod settings;
mod shuffle;
mod sing;
mod sleep;
mod spotify;
mod system;
#[cfg(unix)]
mod terminal;
mod tray;
mod windows;

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{self, Sender};
use std::sync::{Arc, Mutex, MutexGuard};
use std::thread;
use std::time::Duration;

use lyrics_providers::{LrclibClient, LyricsService, Store};
use tauri::{AppHandle, Manager};
use tauri_plugin_global_shortcut::ShortcutState;

use playback::{LyricsView, PlaybackView, Request};
use settings::Settings;

pub struct AppState {
    pub demo: bool,
    /// Set by SINGLINE_DEBUG; the overlay then reports each line it shows.
    pub debug: bool,
    settings: Mutex<Settings>,
    /// `None` in the demo, which never changes the listener's files.
    settings_path: Option<PathBuf>,
    settings_dirty: AtomicBool,
    store: Arc<Mutex<Store>>,
    pub lyrics: Arc<LyricsService>,
    pub playback: Mutex<PlaybackView>,
    pub lyrics_view: Mutex<LyricsView>,
    pub lyrics_generation: AtomicU64,
    requests: Mutex<Sender<Request>>,
    pub hotkey_error: AtomicBool,
}

impl AppState {
    pub fn settings(&self) -> Settings {
        self.settings
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .clone()
    }

    pub fn store(&self) -> MutexGuard<'_, Store> {
        self.store.lock().unwrap_or_else(|p| p.into_inner())
    }

    pub fn request(&self, request: Request) {
        let _ = self
            .requests
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .send(request);
    }

    /// Changes the settings without side effects, for values the windows report
    /// themselves, such as the overlay position.
    pub fn update_settings_quietly(&self, change: impl FnOnce(&mut Settings)) {
        let mut settings = self.settings.lock().unwrap_or_else(|p| p.into_inner());
        change(&mut settings);
        *settings = settings.clone().sanitized();
        self.settings_dirty.store(true, Ordering::SeqCst);
    }

    /// Changes the settings and applies what changed to the windows, the tray,
    /// the player loop and the web interface.
    pub fn update_settings(&self, app: &AppHandle, change: impl FnOnce(&mut Settings)) {
        let (old, new) = {
            let mut settings = self.settings.lock().unwrap_or_else(|p| p.into_inner());
            let old = settings.clone();
            change(&mut settings);
            *settings = settings.clone().sanitized();
            (old, settings.clone())
        };
        if old == new {
            return;
        }
        self.settings_dirty.store(true, Ordering::SeqCst);

        if old.overlay_size() != new.overlay_size() {
            windows::apply_size(app, &new);
        }
        if old.click_through != new.click_through {
            windows::set_click_through(app, new.click_through);
        }
        if old.overlay_visible != new.overlay_visible || old.auto_hide != new.auto_hide {
            autohide::settings_changed(app, &old, &new);
        }
        if old.show_in_dock != new.show_in_dock {
            dock::refresh(app);
        }
        if old.global_offset != new.global_offset {
            now::update(app);
        }
        if (old.source, old.include_other_players, &old.ignored_players)
            != (new.source, new.include_other_players, &new.ignored_players)
        {
            self.request(Request::Preference {
                source: new.source,
                include_other: new.include_other_players,
                ignored: new.ignored_players.clone(),
            });
        }
        if old.lookup_online != new.lookup_online {
            self.request(Request::ReloadLyrics { refresh: false });
        }
        if old.language != new.language {
            let title = i18n::Texts::new(i18n::resolve(new.language)).get("settings.title");
            windows::set_settings_title(app, &title);
            let title = i18n::Texts::new(i18n::resolve(new.language)).get("editor.title");
            windows::set_editor_title(app, &title);
        }
        if old.sing_to_find && !new.sing_to_find {
            sing::stop(app);
        }
        let tray_changed = old.click_through != new.click_through
            || old.lyrics_only != new.lyrics_only
            || old.sing_to_find != new.sing_to_find
            || old.overlay_visible != new.overlay_visible
            || old.source != new.source
            || old.language != new.language;
        if tray_changed {
            tray::refresh(app);
        }
        let _ = tauri::Emitter::emit(app, "settings", &new);
        if old.language != new.language || old.hotkey != new.hotkey {
            commands::emit_app_info(app, self);
        }
    }

    pub fn save_settings_now(&self) {
        let Some(path) = &self.settings_path else {
            return;
        };
        self.settings_dirty.store(false, Ordering::SeqCst);
        if let Err(error) = self.settings().save(path) {
            log::error!("could not save the settings: {error}");
        }
    }
}

/// `--menu-panel` opens the tray panel, at startup or from a second launch, but
/// only with SINGLINE_DEBUG: Linux reports no tray clicks to open it with.
const MENU_PANEL_FLAG: &str = "--menu-panel";
/// `--karaoke` opens karaoke the same way, for screenshots.
const KARAOKE_FLAG: &str = "--karaoke";

/// A second launch makes the app reachable again, or opens the tray panel or karaoke.
fn launched_again(app: &AppHandle, arguments: &[String]) {
    let debug = app.try_state::<AppState>().is_some_and(|state| state.debug);
    let flag = |name: &str| debug && arguments.iter().any(|argument| argument == name);
    if flag(MENU_PANEL_FLAG) {
        menu_panel::open_for_debug(app);
    } else if flag(KARAOKE_FLAG) {
        windows::open_karaoke(app, None);
    } else {
        windows::reopen(app);
    }
}

/// However the welcome window closed, it does not open again by itself.
fn welcome_closed(app: &AppHandle) {
    let Some(state) = app.try_state::<AppState>() else {
        return;
    };
    state.update_settings_quietly(|settings| settings.welcomed = true);
    state.save_settings_now();
}

fn user_agent(app: &AppHandle) -> String {
    format!(
        "Singline/{} (https://github.com/JulianRudrich/Singline)",
        app.package_info().version
    )
}

fn setup(app: &mut tauri::App, demo: bool) -> Result<(), Box<dyn std::error::Error>> {
    let handle = app.handle().clone();
    logging::init(&handle);
    log::info!(
        "Singline {} started on {}",
        handle.package_info().version,
        system::version()
    );
    let override_dir = std::env::var_os("SINGLINE_DATA_DIR").map(PathBuf::from);
    let config_dir = override_dir
        .clone()
        .or_else(|| handle.path().app_config_dir().ok());
    let data_dir = override_dir.or_else(|| handle.path().app_data_dir().ok());
    app.manage(spotify::Spotify::new(
        demo,
        config_dir.as_deref(),
        user_agent(&handle),
    ));
    app.manage(shuffle::Shuffle::new(demo, data_dir.as_deref()));
    app.manage(sing::Sing::new(
        demo,
        config_dir.as_deref(),
        data_dir.as_deref(),
        user_agent(&handle),
        std::env::var_os("SINGLINE_DEBUG").is_some(),
    ));

    let settings_path = config_dir.map(|dir| dir.join("settings.json"));
    let settings = settings_path
        .as_deref()
        .map(Settings::load)
        .unwrap_or_default();
    let store = match (&data_dir, demo) {
        (Some(dir), false) => Store::open(&dir.join("singline.sqlite")).or_else(|error| {
            log::error!("could not open the song database, keeping data in memory: {error}");
            Store::open_in_memory()
        })?,
        _ => Store::open_in_memory()?,
    };
    let store = Arc::new(Mutex::new(store));
    let lrclib_url = std::env::var("SINGLINE_LRCLIB_URL")
        .unwrap_or_else(|_| lyrics_providers::lrclib::DEFAULT_BASE_URL.to_owned());
    let client = LrclibClient::with_base_url(&lrclib_url, &user_agent(&handle));
    let (sender, receiver) = mpsc::channel();

    app.manage(AppState {
        demo,
        debug: std::env::var_os("SINGLINE_DEBUG").is_some(),
        settings: Mutex::new(settings.clone()),
        settings_path: if demo { None } else { settings_path },
        settings_dirty: AtomicBool::new(false),
        store: Arc::clone(&store),
        lyrics: Arc::new(LyricsService::new(client, store)),
        playback: Mutex::new(PlaybackView::idle()),
        lyrics_view: Mutex::new(LyricsView::idle()),
        lyrics_generation: AtomicU64::new(0),
        requests: Mutex::new(sender),
        hotkey_error: AtomicBool::new(false),
    });
    app.manage(own_lyrics::EditorSession::default());
    app.manage(autohide::AutoHideState::new(settings.overlay_visible));
    app.manage(sleep::SleepState::default());
    app.manage(menu_panel::MenuPanel::default());
    app.manage(players::Players::default());
    app.manage(history::History::default());
    app.manage(dock::Dock::default());
    app.manage(now::Publisher::default());
    let bridge_store = handle
        .state::<AppState>()
        .settings_path
        .as_deref()
        .and_then(std::path::Path::parent)
        .map(|directory| directory.join("browser-bridge.json"));
    app.manage(browser_bridge::BrowserBridge::start(
        bridge_store,
        &handle.package_info().version.to_string(),
        demo,
        handle.state::<AppState>().debug,
    ));

    #[cfg(target_os = "macos")]
    app.set_activation_policy(tauri::ActivationPolicy::Accessory);

    windows::create_overlay(&handle, &settings)?;
    tray::create(&handle)?;
    dock::refresh(&handle);
    if !settings.welcomed && !demo {
        windows::open_welcome(&handle);
    }
    hotkey::register_saved(&handle, &handle.state::<AppState>(), &settings.hotkey);
    playback::spawn(handle.clone(), demo, receiver);
    spotify::restore(&handle);
    if handle.state::<sing::Sing>().starts_by_itself() {
        // SINGLINE_MIC_FILE plays a recording instead of the microphone, for tests.
        let starter = handle.clone();
        tauri::async_runtime::spawn(async move {
            tokio::time::sleep(Duration::from_millis(1500)).await;
            sing::start(&starter);
        });
    }
    if handle.state::<AppState>().debug {
        let flag = |name: &str| std::env::args().any(|argument| argument == name);
        if flag(MENU_PANEL_FLAG) {
            menu_panel::open_for_debug(&handle);
        }
        if flag(KARAOKE_FLAG) {
            windows::open_karaoke(&handle, None);
        }
    }

    #[cfg(unix)]
    if let Err(error) = quit_on_signals(&handle) {
        log::warn!("could not listen for the signals to quit: {error}");
    }
    let saver = handle.clone();
    thread::Builder::new()
        .name("settings".into())
        .spawn(move || loop {
            thread::sleep(Duration::from_millis(750));
            let state = saver.state::<AppState>();
            if state.settings_dirty.load(Ordering::SeqCst) {
                state.save_settings_now();
            }
        })?;
    Ok(())
}

/// Quits as the Quit item in the menu does when the system asks, at logout or
/// shutdown, or on Ctrl+C in a terminal: the song being heard still goes into
/// the listening history and the settings are saved. A second signal ends
/// Singline at once.
#[cfg(unix)]
fn quit_on_signals(app: &AppHandle) -> std::io::Result<()> {
    use signal_hook::consts::{SIGINT, SIGTERM};
    use signal_hook::low_level::signal_name;

    let mut signals = signal_hook::iterator::Signals::new([SIGTERM, SIGINT])?;
    let app = app.clone();
    thread::Builder::new()
        .name("signals".into())
        .spawn(move || {
            let mut signals = signals.forever();
            if let Some(signal) = signals.next() {
                log::info!("{}: quitting", signal_name(signal).unwrap_or("signal"));
                app.exit(0);
            }
            if signals.next().is_some() {
                std::process::exit(1);
            }
        })?;
    Ok(())
}

fn main() {
    // `singline now` prints the line being sung for a status line and
    // `singline lyrics` shows the lyrics in the terminal; both start nothing of
    // the app.
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    match arguments.first().map(String::as_str) {
        Some("now") => std::process::exit(now::print(&arguments[1..])),
        #[cfg(unix)]
        Some("lyrics") => std::process::exit(terminal::run(&arguments[1..])),
        _ => {}
    }
    let demo = std::env::args().any(|argument| argument == "--demo");
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(
            |app, arguments, _directory| launched_again(app, &arguments),
        ))
        // Opens the links in the settings in the browser.
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .plugin(tauri_plugin_dialog::init())
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, _shortcut, event| {
                    if event.state() == ShortcutState::Pressed {
                        windows::shortcut_pressed(app);
                    }
                })
                .build(),
        )
        .invoke_handler(tauri::generate_handler![
            commands::snapshot,
            commands::update_settings,
            commands::adjust_song_offset,
            commands::set_song_offset,
            commands::reload_lyrics,
            own_lyrics::open_editor,
            own_lyrics::close_editor,
            own_lyrics::editor_state,
            own_lyrics::stamp_line,
            own_lyrics::save_local_lyrics,
            own_lyrics::import_lyrics_file,
            own_lyrics::remove_local_lyrics,
            commands::player_command,
            commands::open_settings,
            commands::finish_welcome,
            commands::open_automation_settings,
            commands::open_stats,
            commands::open_karaoke,
            commands::close_karaoke,
            commands::karaoke_displays,
            commands::quit,
            commands::set_hotkey,
            commands::move_overlay,
            commands::set_launch_at_login,
            commands::report_line,
            report::report_problem,
            report::show_log_file,
            history::listening_stats,
            history::clear_history,
            browser_bridge::browser_bridge,
            browser_bridge::forget_browser,
            sleep::sleep_timer,
            sleep::set_sleep_timer,
            menu_panel::menu_panel_resize,
            menu_panel::close_menu_panel,
            spotify::spotify_status,
            spotify::spotify_set_client_id,
            spotify::spotify_connect,
            spotify::spotify_cancel_login,
            spotify::spotify_disconnect,
            spotify::spotify_open_dashboard,
            spotify::spotify_open,
            spotify::spotify_open_song,
            players::seen_players,
            spotify::spotify_is_liked,
            spotify::spotify_set_liked,
            spotify::spotify_top,
            shuffle::shuffle_open_window,
            shuffle::shuffle_view,
            shuffle::shuffle_load_sources,
            shuffle::shuffle_select,
            shuffle::shuffle_toggle_source,
            shuffle::shuffle_play,
            shuffle::shuffle_reset,
            shuffle::shuffle_restore_removed,
            shuffle::shuffle_set_keep_playing,
            shuffle::shuffle_remove_current,
            shuffle::shuffle_keep,
            sing::sing_view,
            sing::sing_toggle,
            sing::sing_stop,
            sing::sing_next,
            sing::sing_again,
            sing::sing_settings,
            sing::sing_set_token,
            sing::sing_download_model,
            sing::sing_cancel_download,
            sing::sing_remove_model,
            sing::sing_open_genius,
        ])
        .setup(move |app| setup(app, demo))
        .build(tauri::generate_context!())
        .expect("Singline could not start")
        .run(|app, event| match event {
            // Closing the settings window must not quit the app.
            tauri::RunEvent::ExitRequested {
                api, code: None, ..
            } => api.prevent_exit(),
            tauri::RunEvent::Exit => {
                now::clear(app);
                history::flush(app);
                app.state::<AppState>().save_settings_now();
            }
            tauri::RunEvent::WindowEvent {
                label,
                event: tauri::WindowEvent::Destroyed,
                ..
            } => {
                if label == windows::WELCOME {
                    welcome_closed(app);
                }
                dock::update(app, &label, true);
            }
            // Opening the app again from the Finder, Launchpad or the Dock, as
            // a second launch does elsewhere.
            #[cfg(target_os = "macos")]
            tauri::RunEvent::Reopen { .. } => windows::reopen(app),
            _ => {}
        });
}

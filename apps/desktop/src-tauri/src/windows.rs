//! The overlay, settings, welcome, lyrics editor and karaoke windows.

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use serde::Serialize;
use tauri::utils::config::WindowEffectsConfig;
use tauri::window::Color;
use tauri::{
    AppHandle, Emitter, LogicalPosition, LogicalSize, Manager, Monitor, Theme, WebviewUrl,
    WebviewWindow, WebviewWindowBuilder, WindowEvent,
};

use crate::settings::{Corner, Settings};
use crate::{dock, i18n, tray, AppState};

pub const OVERLAY: &str = "overlay";
pub const SETTINGS: &str = "settings";
pub const EDITOR: &str = "editor";
pub const STATS: &str = "stats";
pub const SHUFFLE: &str = "shuffle";
pub const KARAOKE: &str = "karaoke";
pub const WELCOME: &str = "welcome";

/// Distance from the screen edge, in logical pixels.
const MARGIN: f64 = 16.0;

pub fn create_overlay(app: &AppHandle, settings: &Settings) -> tauri::Result<WebviewWindow> {
    let (width, height) = settings.overlay_size();
    let position = settings
        .overlay_position
        .filter(|[x, y]| fits_on_a_monitor(app, *x, *y, width, height))
        .unwrap_or_else(|| {
            corner_position(
                app.primary_monitor().ok().flatten().as_ref(),
                Corner::TopRight,
                width,
                height,
            )
        });

    let window =
        WebviewWindowBuilder::new(app, OVERLAY, WebviewUrl::App("index.html#/overlay".into()))
            .title("Singline")
            .inner_size(width, height)
            .position(position[0], position[1])
            .decorations(false)
            .transparent(true)
            .shadow(false)
            .resizable(false)
            .always_on_top(true)
            .visible_on_all_workspaces(true)
            .skip_taskbar(true)
            .focused(false)
            .focusable(false)
            .accept_first_mouse(true)
            .visible(settings.overlay_visible)
            .build()?;
    window.set_ignore_cursor_events(settings.click_through)?;

    let handle = app.clone();
    window.on_window_event(move |event| {
        if let WindowEvent::Moved(physical) = event {
            let Some(overlay) = handle.get_webview_window(OVERLAY) else {
                return;
            };
            let scale = overlay.scale_factor().unwrap_or(1.0);
            let logical = physical.to_logical::<f64>(scale);
            let state = handle.state::<AppState>();
            state.update_settings_quietly(|settings| {
                settings.overlay_position = Some([logical.x, logical.y])
            });
        }
    });
    Ok(window)
}

/// Resizes the overlay to the layout and keeps it on its screen.
pub fn apply_size(app: &AppHandle, settings: &Settings) {
    let Some(window) = app.get_webview_window(OVERLAY) else {
        return;
    };
    let (width, height) = settings.overlay_size();
    let _ = window.set_size(LogicalSize::new(width, height));
    if let (Ok(Some(monitor)), Ok(position), Ok(scale)) = (
        window.current_monitor(),
        window.outer_position(),
        window.scale_factor(),
    ) {
        let current = position.to_logical::<f64>(scale);
        let [x, y] = clamp_into(&monitor, current.x, current.y, width, height);
        if (x - current.x).abs() > 0.5 || (y - current.y).abs() > 0.5 {
            let _ = window.set_position(LogicalPosition::new(x, y));
        }
    }
}

pub fn set_click_through(app: &AppHandle, enabled: bool) {
    if let Some(window) = app.get_webview_window(OVERLAY) {
        let _ = window.set_ignore_cursor_events(enabled);
    }
}

/// How long the overlay has to fade out before its window hides. Its last
/// frame is then clear, so it comes back without a flash of old lyrics.
const OVERLAY_FADE: Duration = Duration::from_millis(180);

/// Counts the overlay's shows and hides, so a hide still waiting for the fade
/// is dropped when the overlay is shown again meanwhile.
static OVERLAY_CHANGES: AtomicU64 = AtomicU64::new(0);

/// Shows or hides the overlay. The page fades in or out on the
/// `overlay-visibility` event; hiding waits for its fade.
pub fn set_overlay_visible(app: &AppHandle, visible: bool) {
    let Some(window) = app.get_webview_window(OVERLAY) else {
        return;
    };
    let change = OVERLAY_CHANGES.fetch_add(1, Ordering::SeqCst) + 1;
    let _ = app.emit_to(OVERLAY, "overlay-visibility", visible);
    if visible {
        let _ = window.show();
        // Some window managers forget "always on top" while a window is hidden.
        let _ = window.set_always_on_top(true);
        return;
    }
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(OVERLAY_FADE).await;
        let handle = app.clone();
        let _ = app.run_on_main_thread(move || {
            if OVERLAY_CHANGES.load(Ordering::SeqCst) != change {
                return;
            }
            if let Some(window) = handle.get_webview_window(OVERLAY) {
                let _ = window.hide();
            }
        });
    });
}

pub fn toggle_overlay(app: &AppHandle) {
    let state = app.state::<AppState>();
    let visible = !state.settings().overlay_visible;
    state.update_settings(app, |settings| settings.overlay_visible = visible);
}

/// The shortcut shows or hides the overlay; while karaoke is open, it leaves karaoke.
pub fn shortcut_pressed(app: &AppHandle) {
    if app.get_webview_window(KARAOKE).is_some() {
        close_karaoke(app);
    } else {
        toggle_overlay(app);
    }
}

pub fn move_to_corner(app: &AppHandle, corner: Corner) {
    let Some(window) = app.get_webview_window(OVERLAY) else {
        return;
    };
    let monitor = window
        .current_monitor()
        .ok()
        .flatten()
        .or_else(|| app.primary_monitor().ok().flatten());
    let settings = app.state::<AppState>().settings();
    let (width, height) = settings.overlay_size();
    let [x, y] = corner_position(monitor.as_ref(), corner, width, height);
    let _ = window.set_position(LogicalPosition::new(x, y));
}

pub fn open_settings(app: &AppHandle) {
    if show_existing(app, SETTINGS) {
        return;
    }
    let title = texts(app).get("settings.title");
    match glass_window(app, SETTINGS, "settings", &title, 700.0, 480.0) {
        Ok(_) => dock::update(app, SETTINGS, false),
        Err(error) => log::error!("could not open the settings: {error}"),
    }
}

/// The welcome window, shown once at the first start: where Singline lives,
/// the player it follows and the browser extension.
pub fn open_welcome(app: &AppHandle) {
    if show_existing(app, WELCOME) {
        return;
    }
    let title = texts(app).get("welcome.windowTitle");
    match glass_window(app, WELCOME, "welcome", &title, 560.0, 540.0) {
        Ok(_) => {
            dock::update(app, WELCOME, false);
            if app.state::<AppState>().debug {
                println!("welcome-window");
            }
        }
        Err(error) => log::error!("could not open the welcome window: {error}"),
    }
}

pub fn close_welcome(app: &AppHandle) {
    if let Some(window) = app.get_webview_window(WELCOME) {
        let _ = window.close();
    }
}

/// Brings the window `label` to the front if it is open.
fn show_existing(app: &AppHandle, label: &str) -> bool {
    let Some(window) = app.get_webview_window(label) else {
        return false;
    };
    let _ = window.unminimize();
    let _ = window.show();
    let _ = window.set_focus();
    true
}

fn texts(app: &AppHandle) -> i18n::Texts {
    i18n::Texts::new(i18n::resolve(app.state::<AppState>().settings().language))
}

/// A fixed-size window in the dark glass of the settings, showing `route`.
fn glass_window(
    app: &AppHandle,
    label: &str,
    route: &str,
    title: &str,
    width: f64,
    height: f64,
) -> tauri::Result<WebviewWindow> {
    let glass = glass(20.0);
    // The page matches the window from its first paint: see-through over glass,
    // and room for the traffic lights where it runs under the title bar.
    let mut look = Vec::new();
    if glass.is_some() {
        look.push("glass");
    }
    if cfg!(target_os = "macos") {
        look.push("titlebar=overlay");
    }
    let url = if look.is_empty() {
        format!("index.html#/{route}")
    } else {
        format!("index.html?{}#/{route}", look.join("&"))
    };
    let mut builder = WebviewWindowBuilder::new(app, label, WebviewUrl::App(url.into()))
        .title(title)
        .inner_size(width, height)
        .resizable(false)
        .maximizable(false)
        .theme(Some(Theme::Dark))
        .center()
        .focused(true);
    // As in the macOS app: the glass runs under a transparent title bar with a close button.
    #[cfg(target_os = "macos")]
    {
        builder = builder
            .title_bar_style(tauri::TitleBarStyle::Overlay)
            .hidden_title(true)
            .minimizable(false);
    }
    builder = match glass {
        Some(effects) => builder.transparent(true).effects(effects),
        None => builder.background_color(SOLID_BACKGROUND),
    };
    builder.build()
}

/// The "Your music" window. It is always dark, and its own background color
/// keeps it from flashing white while it loads. The page keeps the title in
/// the interface language.
pub fn open_stats(app: &AppHandle) {
    if let Some(window) = app.get_webview_window(STATS) {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
        return;
    }
    let title = i18n::Texts::new(i18n::resolve(app.state::<AppState>().settings().language))
        .get("stats.title");
    let built = WebviewWindowBuilder::new(app, STATS, WebviewUrl::App("index.html#/stats".into()))
        .title(title)
        .inner_size(880.0, 740.0)
        .min_inner_size(560.0, 460.0)
        .resizable(true)
        .center()
        .focused(true)
        .theme(Some(Theme::Dark))
        .background_color(Color(17, 17, 19, 255))
        .build();
    match built {
        Ok(_) => dock::update(app, STATS, false),
        Err(error) => log::error!("could not open your music: {error}"),
    }
}

/// The True Shuffle window, opened from the tray. Like "Your music", it is
/// always dark and starts out in its page's colour, so it never flashes white.
pub fn open_shuffle(app: &AppHandle) {
    if let Some(window) = app.get_webview_window(SHUFFLE) {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
        return;
    }
    let title = i18n::Texts::new(i18n::resolve(app.state::<AppState>().settings().language))
        .get("shuffle.windowTitle");
    let built =
        WebviewWindowBuilder::new(app, SHUFFLE, WebviewUrl::App("index.html#/shuffle".into()))
            .title(title)
            .inner_size(380.0, 600.0)
            .min_inner_size(340.0, 440.0)
            .resizable(true)
            .center()
            .focused(true)
            .theme(Some(Theme::Dark))
            .background_color(Color(17, 17, 19, 255))
            .build();
    match built {
        Ok(_) => dock::update(app, SHUFFLE, false),
        Err(error) => log::error!("could not open True Shuffle: {error}"),
    }
}

/// A display karaoke can fill, as its chooser lists it.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KaraokeDisplay {
    /// The top-left corner in physical pixels, which tells displays apart while they are connected.
    pub id: String,
    pub name: Option<String>,
    /// Logical pixels.
    pub width: f64,
    pub height: f64,
    /// Karaoke is on this display, or opens there.
    pub current: bool,
}

/// The displays, from left to right, marking the one karaoke is on or would open on.
pub fn karaoke_displays(app: &AppHandle) -> Vec<KaraokeDisplay> {
    let home = karaoke_home(app).map(|monitor| display_id(&monitor));
    let mut monitors = app.available_monitors().unwrap_or_default();
    monitors.sort_by_key(|monitor| (monitor.position().x, monitor.position().y));
    monitors
        .iter()
        .map(|monitor| {
            let size = monitor.size().to_logical::<f64>(monitor.scale_factor());
            let id = display_id(monitor);
            KaraokeDisplay {
                current: home.as_deref() == Some(id.as_str()),
                name: monitor.name().and_then(|name| display_name(name)),
                width: size.width,
                height: size.height,
                id,
            }
        })
        .collect()
}

fn display_id(monitor: &Monitor) -> String {
    let position = monitor.position();
    format!("{},{}", position.x, position.y)
}

/// The system's name for a display, without the device prefix Windows puts before it.
fn display_name(name: &str) -> Option<String> {
    let name = name.trim_start_matches(r"\\.\").trim();
    (!name.is_empty()).then(|| name.to_owned())
}

/// Where karaoke is, or else where the overlay is, or else the main display.
fn karaoke_home(app: &AppHandle) -> Option<Monitor> {
    [KARAOKE, OVERLAY]
        .into_iter()
        .filter_map(|label| app.get_webview_window(label))
        .find_map(|window| window.current_monitor().ok().flatten())
        .or_else(|| app.primary_monitor().ok().flatten())
        .or_else(|| app.available_monitors().ok()?.into_iter().next())
}

/// Opens karaoke fullscreen on `display` (an id from `karaoke_displays`) or
/// where the overlay is; when it is open, it moves to that display. The
/// overlay steps aside on the same display. On Windows, call this from an async
/// command or another thread.
pub fn open_karaoke(app: &AppHandle, display: Option<&str>) {
    let chosen = display.and_then(|id| {
        app.available_monitors()
            .ok()?
            .into_iter()
            .find(|monitor| display_id(monitor) == id)
    });
    if let Some(window) = app.get_webview_window(KARAOKE) {
        if let Some(monitor) = &chosen {
            fill(&window, monitor);
        }
        let _ = window.show();
        let _ = window.set_focus();
        let current = chosen.or_else(|| window.current_monitor().ok().flatten());
        make_room_for_karaoke(app, current.as_ref());
        return;
    }
    let monitor = chosen.or_else(|| karaoke_home(app));
    let title = i18n::Texts::new(i18n::resolve(app.state::<AppState>().settings().language))
        .get("karaoke.title");
    let mut builder =
        WebviewWindowBuilder::new(app, KARAOKE, WebviewUrl::App("index.html#/karaoke".into()))
            .title(title)
            .decorations(false)
            .theme(Some(Theme::Dark))
            .background_color(Color(0, 0, 0, 255))
            .focused(true);
    if let Some(monitor) = &monitor {
        let scale = monitor.scale_factor();
        let position = monitor.position().to_logical::<f64>(scale);
        let size = monitor.size().to_logical::<f64>(scale);
        builder = builder
            .position(position.x, position.y)
            .inner_size(size.width, size.height);
    }
    match builder.build() {
        Ok(window) => {
            match &monitor {
                Some(monitor) => fill(&window, monitor),
                None => {
                    let _ = window.set_fullscreen(true);
                }
            }
            let _ = window.set_focus();
            make_room_for_karaoke(app, monitor.as_ref());
        }
        Err(error) => log::error!("could not open karaoke: {error}"),
    }
}

/// Fills `monitor` with the karaoke window: macOS gets the simple fullscreen,
/// which needs no Space of its own, the others fullscreen on that display.
/// The window takes the display's place and size first, for window managers
/// without fullscreen.
fn fill(window: &WebviewWindow, monitor: &Monitor) {
    let position = *monitor.position();
    #[cfg(target_os = "macos")]
    let _ = window.set_simple_fullscreen(false);
    #[cfg(not(target_os = "macos"))]
    let _ = window.set_fullscreen(false);
    let _ = window.set_position(position);
    let _ = window.set_size(*monitor.size());
    #[cfg(target_os = "macos")]
    let _ = window.set_simple_fullscreen(true);
    #[cfg(not(target_os = "macos"))]
    let _ = window.set_fullscreen_on_monitor(tauri::PhysicalPosition::new(
        f64::from(position.x),
        f64::from(position.y),
    ));
}

pub fn close_karaoke(app: &AppHandle) {
    if let Some(window) = app.get_webview_window(KARAOKE) {
        // Leaving the simple fullscreen first brings the menu bar and the Dock back.
        #[cfg(target_os = "macos")]
        let _ = window.set_simple_fullscreen(false);
        let _ = window.close();
        let visible = app.state::<AppState>().settings().overlay_visible;
        set_overlay_visible(app, visible);
    }
}

/// The overlay stays on top of everything, so on the display karaoke fills it
/// steps aside until karaoke closes or moves on. Its setting stays as it is.
fn make_room_for_karaoke(app: &AppHandle, karaoke: Option<&Monitor>) {
    let Some(overlay) = app.get_webview_window(OVERLAY) else {
        return;
    };
    let shared = match (karaoke, overlay.current_monitor().ok().flatten()) {
        (Some(karaoke), Some(own)) => karaoke.position() == own.position(),
        _ => true,
    };
    let visible = app.state::<AppState>().settings().overlay_visible;
    set_overlay_visible(app, visible && !shared);
}

/// Behind the settings where the system offers no glass; the page paints the same.
const SOLID_BACKGROUND: Color = Color(22, 22, 24, 255);

/// Real behind-window blur, `radius` round at the corners where the system
/// draws them: the dark HUD material, as in the macOS app.
#[cfg(target_os = "macos")]
pub fn glass(radius: f64) -> Option<WindowEffectsConfig> {
    use tauri::window::{Effect, EffectState, EffectsBuilder};
    let effects = EffectsBuilder::new()
        .effect(Effect::HudWindow)
        .state(EffectState::Active)
        .radius(radius);
    Some(effects.build())
}

/// The acrylic system backdrop, which Windows offers from Windows 11 22H2
/// (build 22523). Windows rounds the corners itself.
#[cfg(windows)]
pub fn glass(_radius: f64) -> Option<WindowEffectsConfig> {
    use tauri::window::{Effect, EffectsBuilder};
    let supported = windows_version::OsVersion::current().build >= 22_523;
    supported.then(|| EffectsBuilder::new().effect(Effect::Acrylic).build())
}

/// No window effects here: the windows paint a solid background.
#[cfg(not(any(target_os = "macos", windows)))]
pub fn glass(_radius: f64) -> Option<WindowEffectsConfig> {
    None
}

pub fn set_settings_title(app: &AppHandle, title: &str) {
    if let Some(window) = app.get_webview_window(SETTINGS) {
        let _ = window.set_title(title);
    }
}

/// Opens the editor for your own lyrics, or brings it to the front when it is
/// already open. It is always dark and starts out in its page's colour. On
/// Windows, call this from an async command or another thread: building a
/// webview inside a synchronous command deadlocks there.
pub fn open_editor(app: &AppHandle) {
    if let Some(window) = app.get_webview_window(EDITOR) {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
        return;
    }
    let title = i18n::Texts::new(i18n::resolve(app.state::<AppState>().settings().language))
        .get("editor.title");
    let built =
        WebviewWindowBuilder::new(app, EDITOR, WebviewUrl::App("index.html#/editor".into()))
            .title(title)
            .inner_size(560.0, 560.0)
            .min_inner_size(440.0, 400.0)
            .resizable(true)
            .center()
            .focused(true)
            .theme(Some(Theme::Dark))
            .background_color(Color(14, 15, 17, 255))
            .build();
    match built {
        Ok(_) => dock::update(app, EDITOR, false),
        Err(error) => log::error!("could not open the lyrics editor: {error}"),
    }
}

pub fn close_editor(app: &AppHandle) {
    if let Some(window) = app.get_webview_window(EDITOR) {
        let _ = window.close();
    }
}

pub fn set_editor_title(app: &AppHandle, title: &str) {
    if let Some(window) = app.get_webview_window(EDITOR) {
        let _ = window.set_title(title);
    }
}

/// Opening the app again makes the overlay reachable: visible, clickable, with settings open.
/// A launch while the first instance is still starting is ignored.
pub fn reopen(app: &AppHandle) {
    let Some(state) = app.try_state::<AppState>() else {
        return;
    };
    state.update_settings(app, |settings| {
        settings.click_through = false;
        settings.overlay_visible = true;
    });
    open_settings(app);
    tray::refresh(app);
}

fn work_area(monitor: &Monitor) -> (f64, f64, f64, f64) {
    let scale = monitor.scale_factor();
    let area = monitor.work_area();
    let position = area.position.to_logical::<f64>(scale);
    let size = area.size.to_logical::<f64>(scale);
    (position.x, position.y, size.width, size.height)
}

fn corner_position(monitor: Option<&Monitor>, corner: Corner, width: f64, height: f64) -> [f64; 2] {
    let Some(monitor) = monitor else {
        return [MARGIN, MARGIN];
    };
    let (x, y, area_width, area_height) = work_area(monitor);
    let left = x + MARGIN;
    let right = x + area_width - MARGIN - width;
    let top = y + MARGIN;
    let bottom = y + area_height - MARGIN - height;
    match corner {
        Corner::TopLeft => [left, top],
        Corner::TopRight => [right.max(left), top],
        Corner::BottomLeft => [left, bottom.max(top)],
        Corner::BottomRight => [right.max(left), bottom.max(top)],
    }
}

fn clamp_into(monitor: &Monitor, x: f64, y: f64, width: f64, height: f64) -> [f64; 2] {
    let (left, top, area_width, area_height) = work_area(monitor);
    let max_x = (left + area_width - width).max(left);
    let max_y = (top + area_height - height).max(top);
    [x.clamp(left, max_x), y.clamp(top, max_y)]
}

fn fits_on_a_monitor(app: &AppHandle, x: f64, y: f64, width: f64, height: f64) -> bool {
    app.available_monitors()
        .unwrap_or_default()
        .iter()
        .any(|monitor| {
            let (left, top, area_width, area_height) = work_area(monitor);
            // At least a strip of the overlay must be on the screen to grab it.
            x + width > left + 40.0
                && x < left + area_width - 40.0
                && y >= top - 1.0
                && y < top + area_height - 30.0
        })
        && height > 0.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_names_lose_the_windows_device_prefix() {
        assert_eq!(display_name(r"\\.\DISPLAY2").as_deref(), Some("DISPLAY2"));
        assert_eq!(display_name("HDMI-1").as_deref(), Some("HDMI-1"));
        assert_eq!(display_name(" "), None);
    }
}

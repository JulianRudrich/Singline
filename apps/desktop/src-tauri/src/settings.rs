//! The listener's settings, stored as JSON next to the song database.

use std::fs;
use std::io;
use std::path::Path;

use media_session::SourcePreference;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Layout {
    Caption,
    Focus,
    #[default]
    Context,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Language {
    #[default]
    System,
    De,
    En,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Corner {
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    pub layout: Layout,
    /// Only the lyrics: no song, controls, microphone or timeline, even under the pointer.
    pub lyrics_only: bool,
    pub font_size: f64,
    pub background_opacity: f64,
    pub overlay_opacity: f64,
    pub width: f64,
    pub height: f64,
    /// Seconds; positive shows the lyrics earlier.
    pub global_offset: f64,
    pub lookup_online: bool,
    /// Records the songs you hear for "Your music", on this device only.
    pub keep_history: bool,
    pub source: SourcePreference,
    pub include_other_players: bool,
    /// Players never to follow, by name, such as a browser that plays videos.
    pub ignored_players: Vec<String>,
    pub click_through: bool,
    pub language: Language,
    /// The listener's own choice; auto-hide only hides on top of it.
    pub overlay_visible: bool,
    /// Hides the overlay a few seconds after playback stops.
    pub auto_hide: bool,
    /// macOS: Singline stays in the Dock and the app switcher, also without a window.
    pub show_in_dock: bool,
    /// Top-left corner in logical pixels, once the overlay was moved.
    pub overlay_position: Option<[f64; 2]>,
    pub hotkey: String,
    /// Karaoke's type size, as a factor of the size that fits its screen.
    pub karaoke_scale: f64,
    /// The welcome window was closed; it opens once, at the first start.
    pub welcomed: bool,
    /// Sing to find: off, the overlay shows no microphone and the menus no entry.
    pub sing_to_find: bool,
}

pub const DEFAULT_HOTKEY: &str = "Alt+Shift+L";

impl Default for Settings {
    fn default() -> Self {
        Self {
            layout: Layout::Context,
            lyrics_only: false,
            font_size: 25.0,
            background_opacity: 0.78,
            overlay_opacity: 1.0,
            width: 420.0,
            height: 240.0,
            global_offset: 0.0,
            lookup_online: true,
            keep_history: true,
            source: SourcePreference::Automatic,
            include_other_players: false,
            ignored_players: Vec::new(),
            click_through: false,
            language: Language::System,
            overlay_visible: true,
            auto_hide: false,
            show_in_dock: false,
            overlay_position: None,
            hotkey: DEFAULT_HOTKEY.to_owned(),
            karaoke_scale: 1.0,
            welcomed: false,
            sing_to_find: true,
        }
    }
}

impl Settings {
    /// Reads the settings, falling back to the defaults for anything missing or unreadable.
    pub fn load(path: &Path) -> Self {
        fs::read_to_string(path)
            .ok()
            .and_then(|text| serde_json::from_str::<Settings>(&text).ok())
            .unwrap_or_default()
            .sanitized()
    }

    /// Writes the settings atomically, so a crash never leaves half a file.
    pub fn save(&self, path: &Path) -> io::Result<()> {
        if let Some(directory) = path.parent() {
            fs::create_dir_all(directory)?;
        }
        let temporary = path.with_extension("json.tmp");
        let text = serde_json::to_string_pretty(self).map_err(io::Error::other)?;
        fs::write(&temporary, text)?;
        fs::rename(&temporary, path)
    }

    /// Applies the known fields of a JSON object. Unknown fields are ignored and
    /// a value of the wrong type rejects the whole change.
    pub fn merged(&self, patch: &serde_json::Value) -> Result<Self, String> {
        let serde_json::Value::Object(changes) = patch else {
            return Err("settings changes must be an object".into());
        };
        let mut value = serde_json::to_value(self).map_err(|error| error.to_string())?;
        if let serde_json::Value::Object(current) = &mut value {
            for (key, change) in changes {
                if current.contains_key(key) {
                    current.insert(key.clone(), change.clone());
                }
            }
        }
        serde_json::from_value::<Settings>(value)
            .map(Settings::sanitized)
            .map_err(|error| error.to_string())
    }

    /// Clamps every value into the range the interface offers.
    pub fn sanitized(mut self) -> Self {
        fn clamp(value: f64, min: f64, max: f64, fallback: f64) -> f64 {
            if value.is_finite() {
                value.clamp(min, max)
            } else {
                fallback
            }
        }
        let defaults = Settings::default();
        self.font_size = clamp(self.font_size, 14.0, 44.0, defaults.font_size).round();
        self.background_opacity = clamp(
            self.background_opacity,
            0.0,
            1.0,
            defaults.background_opacity,
        );
        self.overlay_opacity = clamp(self.overlay_opacity, 0.15, 1.0, defaults.overlay_opacity);
        self.width = clamp(self.width, 260.0, 800.0, defaults.width).round();
        self.height = clamp(self.height, 160.0, 500.0, defaults.height).round();
        self.global_offset = (clamp(self.global_offset, -5.0, 5.0, 0.0) * 10.0).round() / 10.0;
        self.karaoke_scale = (clamp(self.karaoke_scale, 0.6, 1.6, 1.0) * 20.0).round() / 20.0;
        self.overlay_position = self
            .overlay_position
            .filter(|[x, y]| x.is_finite() && y.is_finite());
        self.hotkey =
            crate::hotkey::normalize(&self.hotkey).unwrap_or_else(|_| DEFAULT_HOTKEY.to_owned());
        self
    }

    /// Width and height of the overlay in logical pixels for the chosen layout.
    pub fn overlay_size(&self) -> (f64, f64) {
        let height = match self.layout {
            Layout::Caption => self.height.min(150.0),
            Layout::Focus => self.height.min(205.0),
            Layout::Context => self.height,
        };
        (self.width, height)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn patches_only_known_fields_and_clamps_them() {
        let settings = Settings::default();
        let changed = settings
            .merged(&json!({ "fontSize": 90, "globalOffset": 1.234, "unknown": true, "layout": "caption", "karaokeScale": 1.234 }))
            .expect("valid change");
        assert_eq!(changed.font_size, 44.0);
        assert_eq!(changed.karaoke_scale, 1.25, "karaoke grows in steps of 5 %");
        assert_eq!(
            settings
                .merged(&json!({ "karaokeScale": 9 }))
                .expect("valid change")
                .karaoke_scale,
            1.6
        );
        assert_eq!(changed.global_offset, 1.2);
        assert_eq!(changed.layout, Layout::Caption);
        assert_eq!(changed.overlay_size(), (420.0, 150.0));
        assert!(
            settings.merged(&json!({ "fontSize": "large" })).is_err(),
            "wrong types are rejected"
        );
        assert!(settings.merged(&json!([1, 2])).is_err());
    }

    #[test]
    fn missing_or_broken_files_give_defaults() {
        let directory =
            std::env::temp_dir().join(format!("singline-settings-{}", std::process::id()));
        let path = directory.join("settings.json");
        assert_eq!(Settings::load(&path), Settings::default());

        let settings = Settings {
            source: SourcePreference::AppleMusic,
            overlay_position: Some([12.0, 34.0]),
            ..Settings::default()
        };
        settings.save(&path).expect("saved");
        assert_eq!(Settings::load(&path), settings);

        fs::write(&path, "{ not json").expect("written");
        assert_eq!(Settings::load(&path), Settings::default());
        fs::write(
            &path,
            r#"{"width": 9999, "overlayPosition": [1, 2], "hotkey": " "}"#,
        )
        .expect("written");
        assert_eq!(
            serde_json::from_str::<Settings>(r#"{"hotkey": "shift+alt+keyk"}"#)
                .expect("valid")
                .sanitized()
                .hotkey,
            "Alt+Shift+K",
            "shortcuts are kept in one spelling"
        );
        let loaded = Settings::load(&path);
        assert_eq!(
            (
                loaded.width,
                loaded.overlay_position,
                loaded.hotkey.as_str()
            ),
            (800.0, Some([1.0, 2.0]), DEFAULT_HOTKEY)
        );
        let _ = fs::remove_dir_all(directory);
    }

    #[test]
    fn files_from_before_a_switch_get_its_default() {
        let loaded: Settings = serde_json::from_str(r#"{ "fontSize": 30 }"#).expect("valid");
        assert!(!loaded.auto_hide, "auto-hide starts off");
        assert!(loaded.keep_history, "the listening history starts on");
        assert_eq!(
            loaded.karaoke_scale, 1.0,
            "karaoke starts at the size that fits"
        );
        let changed = loaded
            .merged(&json!({ "autoHide": true, "keepHistory": false }))
            .expect("valid change");
        assert!(changed.auto_hide && !changed.keep_history);
    }
}

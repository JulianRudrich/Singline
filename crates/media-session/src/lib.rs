//! Finds the music players that are open and reads what they play.
//!
//! Every operating system has its own way to expose media sessions:
//!
//! | System  | Backend                                              |
//! |---------|------------------------------------------------------|
//! | Linux   | MPRIS over D-Bus (`mpris`)                           |
//! | Windows | System Media Transport Controls (`smtc`)             |
//! | macOS   | AppleScript for Spotify and Music ([`applescript`])  |
//!
//! On every system, [`bridge`] adds the browser tabs that the Singline browser
//! extension reports, and [`youtube`] turns a YouTube video into a song.
//!
//! All backends produce the same [`Reading`]s. [`Engine`] decides which player
//! the overlay follows, and [`DemoBackend`] plays a built-in song for previews.
//! [`MicrophoneBackend`] reports the song someone sings, for Sing to find.

pub mod applescript;
#[cfg(any(windows, target_os = "macos", test))]
mod artwork;
mod backend;
pub mod bridge;
mod demo;
mod engine;
mod microphone;
mod model;
pub mod normalize;
pub mod youtube;

#[cfg(target_os = "linux")]
#[doc(hidden)]
pub mod fake_mpris;
#[cfg(target_os = "linux")]
pub mod mpris;

#[cfg(windows)]
pub mod smtc;

pub use backend::{default_backends, Backend, CommandError};
pub use demo::{DemoBackend, DEMO_LRC, DEMO_PLAYER_ID};
pub use engine::{Engine, NowPlaying, SourcePreference};
pub use microphone::{MicrophoneBackend, MicrophoneFeed, MICROPHONE_PLAYER_ID};
pub use model::{
    Capabilities, Command, PlaybackStatus, PlayerId, PlayerInfo, PlayerSnapshot, Problem, Reading,
    Service, Track,
};

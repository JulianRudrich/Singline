//! The players that report to Singline right now, for the list in the
//! settings where the listener switches off players never to follow.

use std::sync::Mutex;

use media_session::{Reading, Service};
use serde::Serialize;
use tauri::{AppHandle, Manager, State};

#[derive(Default)]
pub struct Players(Mutex<Vec<SeenPlayer>>);

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SeenPlayer {
    pub name: String,
    pub service: Service,
}

/// The players in `readings` by name, once each, A to Z. Sing to find's
/// microphone is no player to switch off.
fn seen(readings: &[Reading]) -> Vec<SeenPlayer> {
    let mut players: Vec<SeenPlayer> = readings
        .iter()
        .map(Reading::player)
        .filter(|player| player.service != Service::Microphone && !player.name.trim().is_empty())
        .map(|player| SeenPlayer {
            name: player.name.trim().to_owned(),
            service: player.service,
        })
        .collect();
    players.sort_by_key(|player| player.name.to_lowercase());
    players.dedup_by_key(|player| player.name.to_lowercase());
    players
}

/// Remembers the players of the latest reading.
pub fn observe(app: &AppHandle, readings: &[Reading]) {
    if let Some(players) = app.try_state::<Players>() {
        *players.0.lock().unwrap_or_else(|p| p.into_inner()) = seen(readings);
    }
}

/// The players reporting right now.
#[tauri::command]
pub fn seen_players(players: State<'_, Players>) -> Vec<SeenPlayer> {
    players.0.lock().unwrap_or_else(|p| p.into_inner()).clone()
}

#[cfg(test)]
mod tests {
    use std::time::Instant;

    use media_session::{
        Capabilities, PlaybackStatus, PlayerId, PlayerInfo, PlayerSnapshot, Problem,
    };

    use super::*;

    fn reading(name: &str, service: Service) -> Reading {
        Reading::Snapshot(PlayerSnapshot {
            player: PlayerInfo {
                id: PlayerId::new(name),
                name: name.into(),
                service,
                web: false,
            },
            status: PlaybackStatus::Playing,
            track: None,
            position: None,
            sampled_at: Instant::now(),
            rate: 1.0,
            capabilities: Capabilities::default(),
        })
    }

    #[test]
    fn lists_each_player_once_by_name_without_the_microphone() {
        let readings = vec![
            reading("Spotify", Service::Spotify),
            reading("firefox", Service::Other),
            reading("Firefox", Service::Other),
            reading("Microphone", Service::Microphone),
            Reading::Unavailable {
                player: PlayerInfo {
                    id: PlayerId::new("music"),
                    name: "Music".into(),
                    service: Service::AppleMusic,
                    web: false,
                },
                problem: Problem::PermissionDenied,
                playing: None,
            },
        ];
        let names: Vec<String> = seen(&readings)
            .into_iter()
            .map(|player| player.name)
            .collect();
        assert_eq!(names, ["firefox", "Music", "Spotify"]);
    }
}

//! Deciding which player the overlay follows.

use lyrics_core::{PlaybackActivity, PlaybackSelection};
use serde::{Deserialize, Serialize};

use crate::model::{
    PlaybackStatus, PlayerId, PlayerInfo, PlayerSnapshot, Problem, Reading, Service,
};

/// Which players the listener wants the overlay to follow.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SourcePreference {
    /// Follows whichever known service plays, and other players when allowed.
    #[default]
    Automatic,
    Spotify,
    AppleMusic,
    AmazonMusic,
    YouTubeMusic,
}

impl SourcePreference {
    pub fn service(self) -> Option<Service> {
        match self {
            SourcePreference::Automatic => None,
            SourcePreference::Spotify => Some(Service::Spotify),
            SourcePreference::AppleMusic => Some(Service::AppleMusic),
            SourcePreference::AmazonMusic => Some(Service::AmazonMusic),
            SourcePreference::YouTubeMusic => Some(Service::YouTubeMusic),
        }
    }

    fn accepts(self, player: &PlayerInfo, include_other: bool) -> bool {
        if player.service == Service::Microphone {
            return true;
        }
        match self.service() {
            Some(service) => player.service == service,
            None => player.service != Service::Other || include_other,
        }
    }
}

/// What the overlay should show after a poll.
#[derive(Debug, Clone, PartialEq)]
pub enum NowPlaying {
    /// Nothing the overlay follows has a track loaded.
    Idle,
    /// The player the overlay follows is open but cannot be read.
    Problem {
        player: PlayerInfo,
        problem: Problem,
    },
    /// A track is loaded, playing or paused.
    Track(PlayerSnapshot),
}

/// Turns the readings of all backends into one [`NowPlaying`].
#[derive(Debug, Clone, Default)]
pub struct Engine {
    selection: PlaybackSelection<PlayerId>,
    preference: SourcePreference,
    include_other: bool,
    /// Names of players never to follow, in lower case.
    ignored: Vec<String>,
}

impl Engine {
    pub fn new(preference: SourcePreference, include_other: bool) -> Self {
        Self {
            selection: PlaybackSelection::new(),
            preference,
            include_other,
            ignored: Vec::new(),
        }
    }

    /// Players the listener never wants followed, by name, whatever case.
    /// Changing them starts the selection over.
    pub fn set_ignored(&mut self, names: &[String]) {
        let ignored: Vec<String> = names
            .iter()
            .map(|name| name.trim().to_lowercase())
            .collect();
        if ignored != self.ignored {
            self.selection = PlaybackSelection::new();
            self.ignored = ignored;
        }
    }

    /// Sing to find's microphone is never ignored.
    fn ignores(&self, player: &PlayerInfo) -> bool {
        player.service != Service::Microphone
            && self
                .ignored
                .iter()
                .any(|name| *name == player.name.to_lowercase())
    }

    pub fn preference(&self) -> SourcePreference {
        self.preference
    }

    /// Changing what is followed starts the selection over.
    pub fn set_preference(&mut self, preference: SourcePreference, include_other: bool) {
        if (preference, include_other) != (self.preference, self.include_other) {
            let ignored = std::mem::take(&mut self.ignored);
            *self = Self::new(preference, include_other);
            self.ignored = ignored;
        }
    }

    /// The player that was followed after the last [`choose`](Self::choose).
    pub fn current(&self) -> Option<&PlayerId> {
        self.selection.current()
    }

    /// While Sing to find listens, the microphone is followed and nothing else.
    pub fn choose(&mut self, readings: Vec<Reading>) -> NowPlaying {
        let readings = crate::bridge::without_duplicate_tabs(readings);
        let singing = readings
            .iter()
            .any(|reading| reading.player().service == Service::Microphone);
        let accepted: Vec<Reading> = readings
            .into_iter()
            .filter(|reading| {
                let player = reading.player();
                if singing {
                    player.service == Service::Microphone
                } else {
                    self.preference.accepts(player, self.include_other) && !self.ignores(player)
                }
            })
            .collect();
        let activity: Vec<(PlayerId, PlaybackActivity)> = accepted
            .iter()
            .map(|reading| (reading.player().id.clone(), activity(reading)))
            .collect();
        let selected = self.selection.choose(&activity);

        let mut first_problem = None;
        let mut chosen = None;
        for reading in accepted {
            let is_selected = selected.as_ref() == Some(&reading.player().id);
            match reading {
                Reading::Snapshot(snapshot) if is_selected => {
                    if snapshot.track.is_some() && snapshot.status != PlaybackStatus::Stopped {
                        chosen = Some(NowPlaying::Track(snapshot));
                    }
                }
                Reading::Unavailable {
                    player, problem, ..
                } => {
                    let report = NowPlaying::Problem { player, problem };
                    if is_selected {
                        chosen = Some(report);
                    } else if first_problem.is_none() {
                        first_problem = Some(report);
                    }
                }
                Reading::Snapshot(_) => {}
            }
        }
        chosen.or(first_problem).unwrap_or(NowPlaying::Idle)
    }
}

fn activity(reading: &Reading) -> PlaybackActivity {
    match reading {
        Reading::Snapshot(snapshot) => match snapshot.status {
            PlaybackStatus::Playing if snapshot.track.is_some() && snapshot.position.is_some() => {
                PlaybackActivity::Playing
            }
            PlaybackStatus::Playing => PlaybackActivity::PlayingWithoutTiming,
            PlaybackStatus::Paused if snapshot.track.is_some() => PlaybackActivity::Paused,
            PlaybackStatus::Paused | PlaybackStatus::Stopped => PlaybackActivity::Idle,
        },
        Reading::Unavailable {
            playing: Some(true),
            ..
        } => PlaybackActivity::PlayingWithoutTiming,
        Reading::Unavailable { .. } => PlaybackActivity::Unknown,
    }
}

#[cfg(test)]
mod tests {
    use std::time::Instant;

    use super::*;
    use crate::model::{Capabilities, Track};

    fn info(id: &str, service: Service) -> PlayerInfo {
        PlayerInfo {
            id: PlayerId::new(id),
            name: id.into(),
            service,
            web: false,
        }
    }

    fn track(title: &str) -> Track {
        Track {
            key: format!("key-{title}"),
            source_id: None,
            title: title.into(),
            artist: "Artist".into(),
            album: None,
            duration: Some(200.0),
            artwork_url: None,
        }
    }

    fn reading(id: &str, service: Service, status: PlaybackStatus, title: Option<&str>) -> Reading {
        Reading::Snapshot(PlayerSnapshot {
            player: info(id, service),
            status,
            track: title.map(track),
            position: Some(5.0),
            sampled_at: Instant::now(),
            rate: 1.0,
            capabilities: Capabilities::default(),
        })
    }

    fn title(now: &NowPlaying) -> Option<&str> {
        match now {
            NowPlaying::Track(snapshot) => snapshot.track.as_ref().map(|t| t.title.as_str()),
            _ => None,
        }
    }

    #[test]
    fn follows_the_playing_service() {
        let mut engine = Engine::default();
        let now = engine.choose(vec![
            reading(
                "spotify",
                Service::Spotify,
                PlaybackStatus::Paused,
                Some("Old"),
            ),
            reading(
                "music",
                Service::AppleMusic,
                PlaybackStatus::Playing,
                Some("New"),
            ),
        ]);
        assert_eq!(title(&now), Some("New"));
        assert_eq!(engine.current(), Some(&PlayerId::new("music")));
    }

    #[test]
    fn other_players_are_ignored_unless_allowed() {
        let browser = || {
            vec![reading(
                "firefox",
                Service::Other,
                PlaybackStatus::Playing,
                Some("Video"),
            )]
        };
        assert_eq!(
            Engine::new(SourcePreference::Automatic, false).choose(browser()),
            NowPlaying::Idle
        );
        assert_eq!(
            title(&Engine::new(SourcePreference::Automatic, true).choose(browser())),
            Some("Video")
        );
    }

    #[test]
    fn ignored_players_are_never_followed() {
        let playing = || {
            vec![
                reading(
                    "Spotify",
                    Service::Spotify,
                    PlaybackStatus::Paused,
                    Some("Song"),
                ),
                reading(
                    "Firefox",
                    Service::Other,
                    PlaybackStatus::Playing,
                    Some("Video"),
                ),
            ]
        };
        let mut engine = Engine::new(SourcePreference::Automatic, true);
        assert_eq!(title(&engine.choose(playing())), Some("Video"));
        engine.set_ignored(&[" firefox ".into()]);
        assert_eq!(title(&engine.choose(playing())), Some("Song"));
        // A new source keeps the ignored players.
        engine.set_preference(SourcePreference::Automatic, false);
        engine.set_preference(SourcePreference::Automatic, true);
        assert_eq!(title(&engine.choose(playing())), Some("Song"));
        engine.set_ignored(&[]);
        assert_eq!(title(&engine.choose(playing())), Some("Video"));
    }

    #[test]
    fn the_microphone_is_never_ignored() {
        let mut engine = Engine::new(SourcePreference::Automatic, false);
        engine.set_ignored(&["Microphone".into()]);
        let now = engine.choose(vec![reading(
            "Microphone",
            Service::Microphone,
            PlaybackStatus::Playing,
            Some("Sung"),
        )]);
        assert_eq!(title(&now), Some("Sung"));
    }

    #[test]
    fn youtube_music_is_followed_like_the_music_apps() {
        let tabs = || {
            vec![
                reading(
                    "youtube-video",
                    Service::Other,
                    PlaybackStatus::Paused,
                    Some("Video"),
                ),
                reading(
                    "youtube-music",
                    Service::YouTubeMusic,
                    PlaybackStatus::Playing,
                    Some("Song"),
                ),
            ]
        };
        assert_eq!(
            title(&Engine::new(SourcePreference::Automatic, false).choose(tabs())),
            Some("Song")
        );
        let mut fixed = Engine::new(SourcePreference::YouTubeMusic, false);
        assert_eq!(title(&fixed.choose(tabs())), Some("Song"));
        assert_eq!(
            fixed.choose(vec![reading(
                "spotify",
                Service::Spotify,
                PlaybackStatus::Playing,
                Some("Other")
            )]),
            NowPlaying::Idle,
            "a fixed YouTube Music source ignores Spotify"
        );
    }

    #[test]
    fn a_fixed_source_ignores_the_rest() {
        let mut engine = Engine::new(SourcePreference::Spotify, true);
        let now = engine.choose(vec![
            reading(
                "music",
                Service::AppleMusic,
                PlaybackStatus::Playing,
                Some("Music"),
            ),
            reading(
                "spotify",
                Service::Spotify,
                PlaybackStatus::Paused,
                Some("Spotify"),
            ),
        ]);
        assert_eq!(title(&now), Some("Spotify"));
    }

    #[test]
    fn stopped_and_empty_players_are_idle() {
        let mut engine = Engine::default();
        assert_eq!(
            engine.choose(vec![reading(
                "spotify",
                Service::Spotify,
                PlaybackStatus::Stopped,
                Some("T")
            )]),
            NowPlaying::Idle
        );
        assert_eq!(
            engine.choose(vec![reading(
                "spotify",
                Service::Spotify,
                PlaybackStatus::Paused,
                None
            )]),
            NowPlaying::Idle
        );
        assert_eq!(engine.choose(Vec::new()), NowPlaying::Idle);
    }

    #[test]
    fn problems_are_reported_when_nothing_plays() {
        let denied = Reading::Unavailable {
            player: info("music", Service::AppleMusic),
            problem: Problem::PermissionDenied,
            playing: None,
        };
        let mut engine = Engine::default();
        assert!(matches!(
            engine.choose(vec![denied.clone()]),
            NowPlaying::Problem {
                problem: Problem::PermissionDenied,
                ..
            }
        ));
        let now = engine.choose(vec![
            denied,
            reading(
                "spotify",
                Service::Spotify,
                PlaybackStatus::Playing,
                Some("Song"),
            ),
        ]);
        assert_eq!(
            title(&now),
            Some("Song"),
            "a readable player wins over a problem"
        );
    }

    #[test]
    fn a_player_without_timing_still_counts_as_playing() {
        let blocked = Reading::Unavailable {
            player: info("music", Service::AppleMusic),
            problem: Problem::NoTrackData,
            playing: Some(true),
        };
        let mut engine = Engine::default();
        let now = engine.choose(vec![
            blocked,
            reading(
                "spotify",
                Service::Spotify,
                PlaybackStatus::Paused,
                Some("Paused"),
            ),
        ]);
        assert!(matches!(
            now,
            NowPlaying::Problem {
                problem: Problem::NoTrackData,
                ..
            }
        ));
    }

    #[test]
    fn the_microphone_takes_over_while_it_listens() {
        let spotify = || {
            reading(
                "spotify",
                Service::Spotify,
                PlaybackStatus::Playing,
                Some("Spotify"),
            )
        };
        let microphone = |title: Option<&str>| {
            let mut reading = reading(
                crate::MICROPHONE_PLAYER_ID,
                Service::Microphone,
                PlaybackStatus::Playing,
                title,
            );
            if title.is_none() {
                if let Reading::Snapshot(snapshot) = &mut reading {
                    snapshot.position = None;
                }
            }
            reading
        };
        // Even a fixed source gives way to Sing to find.
        let mut engine = Engine::new(SourcePreference::Spotify, false);
        assert_eq!(title(&engine.choose(vec![spotify()])), Some("Spotify"));
        assert_eq!(
            engine.choose(vec![spotify(), microphone(None)]),
            NowPlaying::Idle,
            "listening, before a song is found"
        );
        assert_eq!(
            title(&engine.choose(vec![spotify(), microphone(Some("Sung"))])),
            Some("Sung")
        );
        assert_eq!(
            engine.current(),
            Some(&PlayerId::new(crate::MICROPHONE_PLAYER_ID))
        );
        // Stopping returns to the player that kept playing.
        assert_eq!(title(&engine.choose(vec![spotify()])), Some("Spotify"));
    }

    #[test]
    fn changing_the_preference_starts_over() {
        let mut engine = Engine::default();
        engine.choose(vec![reading(
            "spotify",
            Service::Spotify,
            PlaybackStatus::Playing,
            Some("A"),
        )]);
        engine.set_preference(SourcePreference::AppleMusic, false);
        assert_eq!(engine.current(), None);
        assert_eq!(engine.preference(), SourcePreference::AppleMusic);
    }
}

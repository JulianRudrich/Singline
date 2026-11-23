// Mirrors the JSON the Rust side sends. Field names follow serde's camelCase.

export type Service = "spotify" | "appleMusic" | "amazonMusic" | "youTubeMusic" | "microphone" | "other";
export type SourcePreference = "automatic" | "spotify" | "appleMusic" | "amazonMusic" | "youTubeMusic";
export type Layout = "caption" | "focus" | "context";
export type Language = "system" | "de" | "en";
export type Corner = "topLeft" | "topRight" | "bottomLeft" | "bottomRight";

export interface PlayerInfo {
  id: string;
  name: string;
  service: Service;
  web: boolean;
}

export interface Capabilities {
  playPause: boolean;
  next: boolean;
  previous: boolean;
  seek: boolean;
}

export interface Track {
  key: string;
  sourceId: string | null;
  title: string;
  artist: string;
  album: string | null;
  duration: number | null;
  artworkUrl: string | null;
}

export type Problem =
  | { kind: "permissionDenied" }
  | { kind: "noTrackData" }
  | { kind: "readFailed"; detail: string };

export interface Playback {
  state: "idle" | "problem" | "track";
  player: PlayerInfo | null;
  problem: Problem | null;
  track: Track | null;
  playing: boolean;
  /** Seconds into the track at the moment the Rust side sent this. */
  position: number | null;
  rate: number;
  capabilities: Capabilities;
}

export interface LyricWord {
  start: number;
  end?: number;
  text: string;
}

export interface LyricLine {
  time: number;
  end?: number;
  text: string;
  words?: LyricWord[];
}

export type SyncLevel = "word" | "line" | "unsynced";

export interface Lyrics {
  lines: LyricLine[];
  sync: SyncLevel;
  instrumental: boolean;
}

export type LyricsStatus =
  | "idle"
  | "loading"
  | "ready"
  | "instrumental"
  | "notFound"
  | "localOnly"
  | "offline"
  | "rateLimited"
  | "failed";

export interface LyricsView {
  trackKey: string | null;
  status: LyricsStatus;
  origin: "local" | "online" | "demo" | null;
  lyrics: Lyrics | null;
  /** Lyric times where each chorus starts; empty when nothing repeats or the lyrics are not synced. */
  chorus: number[];
  songOffset: number;
  canRetry: boolean;
}

/** The song the lyrics editor belongs to, fixed when its window opens. */
export interface EditorState {
  trackKey: string;
  title: string;
  artist: string;
  /** Your own lyrics as LRC text, null before you add some. */
  lrc: string | null;
}

/** Replaces `length` characters at `start`; all counts are UTF-16 code units. */
export interface StampEdit {
  start: number;
  length: number;
  replacement: string;
  nextCaret: number;
}

/** Why the Rust side did not stamp a line. */
export type StampError = "otherTrack" | "noPosition" | "notALyricLine";

/** Why own lyrics could not be opened, imported, saved or removed. */
export type OwnLyricsError = "noTrack" | "tooLarge" | "notText" | "missingTimestamps" | "unreadable" | "storage";

export interface Settings {
  layout: Layout;
  /** Only the lyrics: no song, controls, microphone or timeline, even under the pointer. */
  lyricsOnly: boolean;
  fontSize: number;
  backgroundOpacity: number;
  overlayOpacity: number;
  width: number;
  height: number;
  globalOffset: number;
  lookupOnline: boolean;
  keepHistory: boolean;
  source: SourcePreference;
  includeOtherPlayers: boolean;
  /** Players never to follow, by name. */
  ignoredPlayers: string[];
  clickThrough: boolean;
  language: Language;
  overlayVisible: boolean;
  autoHide: boolean;
  /** macOS: Singline stays in the Dock and the app switcher, also without a window. */
  showInDock: boolean;
  hotkey: string;
  /** Karaoke's type size, as a factor of the size that fits the screen. */
  karaokeScale: number;
  /** The welcome window was closed; it opens once, at the first start. */
  welcomed: boolean;
  /** Sing to find: off, the overlay shows no microphone and the menus no entry. */
  singToFind: boolean;
}

export interface AppInfo {
  version: string;
  demo: boolean;
  /** Reports each shown line to the Rust side, for end-to-end checks. */
  debug: boolean;
  platform: "macos" | "windows" | "linux" | "other";
  /** The language the interface uses after resolving "system". */
  language: "de" | "en";
  hotkeyError: boolean;
  /** The shortcut "Reset" brings back. */
  defaultHotkey: string;
  launchAtLogin: boolean;
}

/** A display karaoke can fill, as its chooser lists it. */
export interface KaraokeDisplay {
  id: string;
  /** The system's name for it, when it has one. */
  name: string | null;
  /** Logical pixels. */
  width: number;
  height: number;
  /** Karaoke is on this display, or opens there. */
  current: boolean;
}

/** Why the Rust side did not take a shortcut. */
export type HotkeyError = "invalid" | "unsupported" | "needsModifier" | "taken";

/** The sleep timer, as the tray panel shows it. */
export interface SleepView {
  /** The running choice: "off", minutes such as "15", or "end". */
  choice: string;
  /** Every choice, in the order of the menu. */
  choices: string[];
  /** Whole minutes left while a set time runs. */
  minutesLeft: number | null;
  /** "End of track" needs a song to wait for. */
  endOfTrack: boolean;
}

export interface Snapshot {
  settings: Settings;
  playback: Playback;
  lyrics: LyricsView;
  app: AppInfo;
}

/** A browser that paired with Singline through the extension. */
export interface PairedBrowser {
  id: string;
  name: string;
  /** Seconds since 1970. */
  pairedAt: number;
  connected: boolean;
  /** Tabs it reports right now. */
  tabs: number;
}

/** The connection to the Singline browser extension, as the settings show it. */
export interface BridgeView {
  /** False in the demo. */
  available: boolean;
  port: number | null;
  error: string | null;
  /** Six digits to enter in the extension; valid while the settings show it. */
  pairingCode: string | null;
  browsers: PairedBrowser[];
}

/** A player reporting to Singline right now. */
export interface SeenPlayer {
  name: string;
  service: Service;
}

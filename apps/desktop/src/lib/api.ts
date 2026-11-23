import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type {
  AppInfo,
  BridgeView,
  Corner,
  EditorState,
  KaraokeDisplay,
  LyricsView,
  Playback,
  Settings,
  SleepView,
  Snapshot,
  StampEdit,
} from "./types";

export type PlayerCommand =
  | { kind: "playPause" }
  | { kind: "next" }
  | { kind: "previous" }
  | { kind: "seekTo"; position: number };

export const api = {
  snapshot: () => invoke<Snapshot>("snapshot"),
  updateSettings: (patch: Partial<Settings>) => invoke<Settings>("update_settings", { patch }),
  adjustSongOffset: (delta: number) => invoke<number>("adjust_song_offset", { delta }),
  setSongOffset: (value: number) => invoke<number>("set_song_offset", { value }),
  reloadLyrics: () => invoke<void>("reload_lyrics"),
  // Own lyrics. Failures reject with an OwnLyricsError or a StampError name.
  openEditor: () => invoke<void>("open_editor"),
  closeEditor: () => invoke<void>("close_editor"),
  editorState: () => invoke<EditorState | null>("editor_state"),
  stampLine: (draft: string, caret: number) => invoke<StampEdit>("stamp_line", { draft, caret }),
  saveLocalLyrics: (trackKey: string, text: string) => invoke<void>("save_local_lyrics", { trackKey, text }),
  /** Resolves to false when the listener closes the file picker without a choice. */
  importLyricsFile: (trackKey: string, title: string) => invoke<boolean>("import_lyrics_file", { trackKey, title }),
  removeLocalLyrics: (trackKey: string) => invoke<void>("remove_local_lyrics", { trackKey }),
  playerCommand: (command: PlayerCommand) => invoke<void>("player_command", { command }),
  // The browser extension. Asking for the state keeps pairing open while the code is shown.
  browserBridge: () => invoke<BridgeView>("browser_bridge"),
  forgetBrowser: (id: string) => invoke<BridgeView>("forget_browser", { id }),
  openSettings: () => invoke<void>("open_settings"),
  /** Closes the welcome window, which then does not open again by itself. */
  finishWelcome: () => invoke<void>("finish_welcome"),
  /** macOS: System Settings at Privacy & Security → Automation. */
  openAutomationSettings: () => invoke<void>("open_automation_settings"),
  openStats: () => invoke<void>("open_stats"),
  /** Opens karaoke fullscreen, on the overlay's display or the one given; moves it there when open. */
  openKaraoke: (display?: string) => invoke<void>("open_karaoke", { display: display ?? null }),
  closeKaraoke: () => invoke<void>("close_karaoke"),
  karaokeDisplays: () => invoke<KaraokeDisplay[]>("karaoke_displays"),
  quit: () => invoke<void>("quit"),
  /** Rejects with a HotkeyError name; the old shortcut then stays. */
  setHotkey: (hotkey: string) => invoke<Settings>("set_hotkey", { hotkey }),
  sleepTimer: () => invoke<SleepView>("sleep_timer"),
  setSleepTimer: (choice: string) => invoke<SleepView>("set_sleep_timer", { choice }),
  // The tray panel reports its height, so its window fits it.
  menuPanelResize: (height: number) => invoke<void>("menu_panel_resize", { height }),
  closeMenuPanel: () => invoke<void>("close_menu_panel"),
  moveOverlay: (corner: Corner) => invoke<void>("move_overlay", { corner }),
  setLaunchAtLogin: (enabled: boolean) => invoke<boolean>("set_launch_at_login", { enabled }),
  reportLine: (position: number, text: string) => invoke<void>("report_line", { position, text }),
  // Settings → About: a pre-filled issue on GitHub, and the log file in Finder or Explorer.
  reportProblem: () => invoke<void>("report_problem"),
  showLogFile: () => invoke<void>("show_log_file"),
};

export interface Listeners {
  playback?: (playback: Playback) => void;
  lyrics?: (lyrics: LyricsView) => void;
  settings?: (settings: Settings) => void;
  app?: (app: AppInfo) => void;
  sleep?: (sleep: SleepView) => void;
  /** The tray panel opens (play the entrance) or closes (fade out). */
  menuPanel?: (event: { open: boolean }) => void;
  /** The overlay is shown (fade in) or about to hide (fade out first). */
  overlayVisibility?: (visible: boolean) => void;
}

/** Subscribes to the events the Rust side sends; returns one function to stop. */
export async function subscribe(listeners: Listeners): Promise<UnlistenFn> {
  const stops = await Promise.all([
    listeners.playback && listen<Playback>("playback", (event) => listeners.playback?.(event.payload)),
    listeners.lyrics && listen<LyricsView>("lyrics", (event) => listeners.lyrics?.(event.payload)),
    listeners.settings && listen<Settings>("settings", (event) => listeners.settings?.(event.payload)),
    listeners.app && listen<AppInfo>("app", (event) => listeners.app?.(event.payload)),
    listeners.sleep && listen<SleepView>("sleep", (event) => listeners.sleep?.(event.payload)),
    listeners.menuPanel && listen<{ open: boolean }>("menu-panel", (event) => listeners.menuPanel?.(event.payload)),
    listeners.overlayVisibility &&
      listen<boolean>("overlay-visibility", (event) => listeners.overlayVisibility?.(event.payload)),
  ]);
  return () => stops.forEach((stop) => stop?.());
}

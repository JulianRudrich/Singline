// The Spotify connection as the Rust side reports it (serde camelCase), and
// the commands around it.

import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { MessageKey, Translate } from "../lib/i18n";
import type { Track } from "../lib/types";

export type SpotifyPhase = "disconnected" | "waiting" | "connecting" | "connected" | "failed";

export type SpotifyProblem =
  | { kind: "portInUse" }
  | { kind: "browser" }
  | { kind: "denied" }
  | { kind: "timedOut" }
  | { kind: "offline" }
  | { kind: "expired" }
  | { kind: "forbidden"; detail: string | null }
  | { kind: "refused"; detail: string }
  | { kind: "failed"; detail: string };

export interface SpotifyStatus {
  clientId: string;
  redirectUri: string;
  phase: SpotifyPhase;
  account: { id: string; name: string } | null;
  problem: SpotifyProblem | null;
  /** The login is in the system's credential store and survives a restart. */
  persistent: boolean;
  needsReconnect: boolean;
  /** The login page while waiting, for when no browser opened. */
  loginUrl: string | null;
}

export type TimeRange = "short" | "medium" | "long";

export interface TopItem {
  id: string;
  name: string;
  uri: string;
  /** The artists of a track; empty for an artist. */
  artists: string[];
}

export interface TopLists {
  artists: TopItem[];
  tracks: TopItem[];
}

export const spotify = {
  status: () => invoke<SpotifyStatus>("spotify_status"),
  setClientId: (clientId: string) => invoke<SpotifyStatus>("spotify_set_client_id", { clientId }),
  connect: () => invoke<SpotifyStatus>("spotify_connect"),
  cancelLogin: () => invoke<SpotifyStatus>("spotify_cancel_login"),
  disconnect: () => invoke<SpotifyStatus>("spotify_disconnect"),
  openDashboard: () => invoke<void>("spotify_open_dashboard"),
  open: (uri: string) => invoke<void>("spotify_open", { uri }),
  /** Opens the song in Spotify: by its track where known, else as a search. */
  openSong: (track: Track, web: boolean) =>
    invoke<void>("spotify_open_song", { uri: track.sourceId, title: track.title, artist: track.artist, web }),
  isLiked: (uri: string) => invoke<boolean | null>("spotify_is_liked", { uri }),
  setLiked: (uri: string, liked: boolean) => invoke<boolean>("spotify_set_liked", { uri, liked }),
  top: (range: TimeRange) => invoke<TopLists>("spotify_top", { range }),
};

export function onSpotifyStatus(callback: (status: SpotifyStatus) => void): Promise<UnlistenFn> {
  return listen<SpotifyStatus>("spotify", (event) => callback(event.payload));
}

/** Spotify Client IDs are 32 hexadecimal digits. */
export function isClientId(value: string): boolean {
  return /^[0-9a-f]{32}$/i.test(value.trim());
}

/** A Spotify track whose liked state can be asked. */
export function isTrackUri(uri: string | null | undefined): uri is string {
  return typeof uri === "string" && /^spotify:track:[A-Za-z0-9]+$/.test(uri);
}

/** The line next to the status dot. */
export function statusText(status: SpotifyStatus, t: Translate): string {
  switch (status.phase) {
    case "connected":
      return t("spotify.connected", { name: status.account?.name ?? "" });
    case "waiting":
      return status.problem ? problemText(status.problem, t) : t("spotify.waiting");
    case "connecting":
      return t("spotify.connecting");
    case "failed":
      return status.problem ? problemText(status.problem, t) : t("spotify.notConnected");
    default:
      return status.problem?.kind === "expired" ? problemText(status.problem, t) : t("spotify.notConnected");
  }
}

export function problemText(problem: SpotifyProblem, t: Translate): string {
  switch (problem.kind) {
    case "refused":
    case "failed":
      return t(`spotify.problem.${problem.kind}` as MessageKey, { detail: problem.detail });
    default:
      return t(`spotify.problem.${problem.kind}` as MessageKey);
  }
}

/** Copies text, also where the asynchronous clipboard is not allowed. */
export async function copyText(text: string): Promise<boolean> {
  try {
    await navigator.clipboard.writeText(text);
    return true;
  } catch {
    const field = document.createElement("textarea");
    field.value = text;
    field.setAttribute("readonly", "");
    field.style.position = "fixed";
    field.style.opacity = "0";
    document.body.append(field);
    field.select();
    const copied = document.execCommand("copy");
    field.remove();
    return copied;
  }
}

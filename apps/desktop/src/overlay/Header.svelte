<script lang="ts">
  // The header, like the Now Playing widget: cover art, title and artist,
  // entering one after another for each song. While the pointer is over the
  // overlay the playback controls and the microphone slide in; otherwise short
  // notes about the lyrics sit there. Both share one place, so they cross-fade
  // without pushing each other aside.
  import { prefersReducedMotion } from "svelte/motion";
  import { fade } from "svelte/transition";
  import type { Translate } from "../lib/i18n";
  import Icon from "../lib/Icon.svelte";
  import { duration, slideIn, slideOut, songExit } from "../lib/motion";
  import type { Capabilities, Track } from "../lib/types";
  import LikeButton from "../spotify/LikeButton.svelte";
  import Controls from "./Controls.svelte";
  import Cover from "./Cover.svelte";

  interface Props {
    track: Track | null;
    playerName: string;
    /** The controls the player offers, while they are shown. */
    controls: Capabilities | null;
    playing: boolean;
    notSynced: boolean;
    ownLyrics: boolean;
    paused: boolean;
    /** Show the Sing to find button, while the pointer is over the overlay. */
    mic: boolean;
    /** Sing to find listens or follows a song. */
    singing: boolean;
    t: Translate;
    onsettings: () => void;
    onsing: () => void;
  }

  let { track, playerName, controls, playing, notSynced, ownLyrics, paused, mic, singing, t, onsettings, onsing }: Props =
    $props();

  const details = $derived(track ? [track.title, track.artist, playerName].filter(Boolean).join(" · ") : undefined);
  const reduced = $derived(prefersReducedMotion.current);
  const tools = $derived(controls !== null || mic);
  const notes = $derived(notSynced || ownLyrics || paused);
</script>

<header>
  <div class="now-playing" title={details}>
    {#key track?.key ?? ""}
      <div class="song" out:songExit={{ reduced: prefersReducedMotion.current }}>
        <div class="reveal"><Cover url={track?.artworkUrl ?? null} /></div>
        <div class="names">
          <span class="title reveal" style:animation-delay="0.05s">{track?.title ?? t("app.name")}</span>
          {#if track?.artist}
            <span class="artist reveal" style:animation-delay="0.1s">{track.artist}</span>
          {/if}
        </div>
      </div>
    {/key}
  </div>
  <div class="end">
    {#if tools}
      <div class="tools" in:slideIn={{ reduced }} out:slideOut={{ reduced }}>
        {#if controls}
          <LikeButton uri={track?.sourceId} {t} />
          <Controls capabilities={controls} {playing} {t} />
        {/if}
        {#if mic}
          {@const label = singing ? t("overlay.stopSinging") : t("overlay.sing")}
          <button class="sing" class:on={singing} onclick={onsing} title={label} aria-label={label}>
            <svg width="16" height="16" viewBox="0 0 24 24" aria-hidden="true">
              <rect x="9" y="3" width="6" height="11" rx="3" />
              <path d="M5.5 11a6.5 6.5 0 0 0 13 0M12 17.5V21M8.5 21h7" />
            </svg>
          </button>
        {/if}
      </div>
    {:else if notes}
      <!-- The notes wait for the controls to make way. -->
      <div class="notes" in:fade={{ duration: duration.pane, delay: reduced ? 0 : 60 }} out:fade={{ duration: duration.dismiss }}>
        {#if notSynced}
          <span class="caption">{t("overlay.notSynced")}</span>
        {/if}
        {#if ownLyrics}
          <span class="badge" title={t("overlay.ownLyrics")} aria-label={t("overlay.ownLyrics")}>
            <Icon name="document" size={12} />
          </span>
        {/if}
        {#if paused}
          <span class="badge" title={t("overlay.paused")} aria-label={t("overlay.paused")}>
            <Icon name="pause" size={12} />
          </span>
        {/if}
      </div>
    {/if}
  </div>
  <button class="settings" onclick={onsettings} title={t("overlay.openSettings")} aria-label={t("overlay.openSettings")}>
    <svg width="19" height="19" viewBox="0 0 24 24" aria-hidden="true">
      <circle class="ring" cx="12" cy="12" r="9.5" />
      <circle cx="7.6" cy="12" r="1.35" />
      <circle cx="12" cy="12" r="1.35" />
      <circle cx="16.4" cy="12" r="1.35" />
    </svg>
  </button>
</header>

<style>
  header {
    flex: none;
    display: flex;
    align-items: center;
    gap: 10px;
    min-height: 36px;
  }

  .now-playing {
    flex: 1;
    min-width: 0;
    display: grid;
  }

  /* An old and a new song share the place while one leaves and the other arrives. */
  .song {
    grid-area: 1 / 1;
    display: flex;
    align-items: center;
    gap: 10px;
    min-width: 0;
  }

  .names {
    min-width: 0;
    display: flex;
    flex-direction: column;
  }

  .title,
  .artist {
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
  }

  .title {
    font-size: 13px;
    font-weight: 600;
    line-height: 1.3;
  }

  .artist {
    font-size: 11px;
    line-height: 1.3;
    color: rgb(255 255 255 / 0.6);
  }

  /* The tools and the notes share this place and stand at its right end. */
  .end {
    flex: none;
    display: grid;
    justify-items: end;
    align-items: center;
  }

  .tools,
  .notes {
    grid-area: 1 / 1;
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .caption {
    flex: none;
    font-size: 10px;
    color: rgb(255 255 255 / 0.5);
  }

  .badge {
    flex: none;
    display: grid;
    place-items: center;
    color: rgb(255 255 255 / 0.5);
  }

  .sing {
    flex: none;
    display: grid;
    place-items: center;
    width: 24px;
    height: 24px;
    border: 0;
    border-radius: 50%;
    padding: 0;
    background: none;
    color: rgb(255 255 255 / 0.6);
    cursor: pointer;
    transition:
      color var(--duration-press) var(--ease-out),
      background-color var(--duration-press) var(--ease-out),
      transform var(--duration-press) var(--ease-out);
  }

  .sing svg {
    fill: none;
    stroke: currentColor;
    stroke-width: 2;
    stroke-linecap: round;
    stroke-linejoin: round;
  }

  .sing:hover,
  .sing:focus-visible {
    color: #fff;
  }

  /* While it listens: a white disc with a black microphone. */
  .sing.on {
    background: #fff;
    color: #000;
  }

  .sing:active {
    transform: scale(var(--press-scale-icon));
  }

  .settings {
    flex: none;
    display: grid;
    place-items: center;
    width: 22px;
    height: 22px;
    border: 0;
    padding: 0;
    background: none;
    border-radius: 50%;
    color: rgb(255 255 255 / 0.6);
    cursor: pointer;
    transition:
      color var(--duration-press) var(--ease-out),
      transform var(--duration-press) var(--ease-out);
  }

  .settings:hover,
  .settings:focus-visible {
    color: #fff;
  }

  .settings:active {
    transform: scale(var(--press-scale-icon));
  }

  .sing:focus-visible,
  .settings:focus-visible {
    outline: 2px solid rgb(255 255 255 / 0.7);
    outline-offset: 2px;
  }

  .settings svg {
    fill: currentColor;
  }

  .settings .ring {
    fill: none;
    stroke: currentColor;
    stroke-width: 1.6;
  }

</style>

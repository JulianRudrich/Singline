<script lang="ts">
  // ♥ for the playing Spotify song: a white outline, filled white once the
  // song is in Liked Songs. Shown only with a Spotify connection and a known
  // track URI; Spotify is asked once per song.
  import { onMount } from "svelte";
  import type { Translate } from "../lib/i18n";
  import { isTrackUri, onSpotifyStatus, spotify } from "./api";

  let { uri, t }: { uri: string | null | undefined; t: Translate } = $props();

  let connected = $state(false);
  let liked = $state<boolean | null>(null);
  let busy = $state(false);

  const track = $derived(connected && isTrackUri(uri) ? uri : null);

  $effect(() => {
    const current = track;
    liked = null;
    if (!current) return;
    let fresh = true;
    spotify
      .isLiked(current)
      .then((answer) => {
        if (fresh) liked = answer;
      })
      .catch(() => {});
    return () => {
      fresh = false;
    };
  });

  async function toggle() {
    const target = track;
    if (!target || liked === null || busy) return;
    const wanted = !liked;
    busy = true;
    liked = wanted;
    try {
      const result = await spotify.setLiked(target, wanted);
      if (track === target) liked = result;
    } catch {
      if (track === target) liked = !wanted;
    } finally {
      busy = false;
    }
  }

  onMount(() => {
    let alive = true;
    let stop: (() => void) | undefined;
    void (async () => {
      const unlisten = await onSpotifyStatus((status) => (connected = status.phase === "connected"));
      if (!alive) return unlisten();
      stop = unlisten;
      connected = (await spotify.status()).phase === "connected";
    })();
    return () => {
      alive = false;
      stop?.();
    };
  });
</script>

{#if track && liked !== null}
  <button
    class="like"
    class:liked
    onclick={toggle}
    aria-pressed={liked}
    title={liked ? t("spotify.unlike") : t("spotify.like")}
    aria-label={liked ? t("spotify.unlike") : t("spotify.like")}
  >
    <svg width="17" height="17" viewBox="0 0 24 24" aria-hidden="true">
      <path d="M12 20s-8-4.9-8-10.7A4.3 4.3 0 0 1 12 6.8a4.3 4.3 0 0 1 8 2.5C20 15.1 12 20 12 20Z" />
    </svg>
  </button>
{/if}

<style>
  .like {
    flex: none;
    display: grid;
    place-items: center;
    width: 26px;
    height: 26px;
    padding: 0;
    border: 0;
    border-radius: 50%;
    background: none;
    color: #fff;
    cursor: pointer;
    transition:
      background-color var(--duration-press) var(--ease-out),
      transform var(--duration-press) var(--ease-out);
  }

  .like:hover {
    background: rgb(255 255 255 / 0.1);
  }

  .like:active {
    transform: scale(var(--press-scale-icon));
  }

  .like:focus-visible {
    outline: 2px solid rgb(255 255 255 / 0.7);
    outline-offset: 2px;
  }

  path {
    fill: none;
    stroke: currentColor;
    stroke-width: 1.8;
    stroke-linejoin: round;
  }

  .liked path {
    fill: currentColor;
  }

</style>

<script lang="ts">
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { onMount, untrack } from "svelte";
  import { prefersReducedMotion } from "svelte/motion";
  import { api, subscribe } from "../lib/api";
  import { PlaybackClock } from "../lib/clock";
  import { translator } from "../lib/i18n";
  import { clamp, currentIndex } from "../lib/lyrics";
  import { lyricMeasure } from "../lib/measure";
  import { songExit } from "../lib/motion";
  import { lyricSeekTarget, SeekHold } from "../lib/player";
  import { splitLongLines } from "../lib/segment";
  import { statusText } from "../lib/status";
  import type { AppInfo, LyricLine, LyricsView, Playback, Settings } from "../lib/types";
  import { isActive, onSing, sing as singApi, type ModelView, type SingView } from "../sing/api";
  import Header from "./Header.svelte";
  import LyricLines from "./LyricLines.svelte";
  import Scrubber from "./Scrubber.svelte";
  import SingFound from "./SingFound.svelte";
  import SingPanel from "./SingPanel.svelte";
  import Status from "./Status.svelte";

  let settings = $state<Settings | null>(null);
  let playback = $state<Playback | null>(null);
  let lyricsView = $state<LyricsView | null>(null);
  let app = $state<AppInfo | null>(null);
  let position = $state(0);
  let singView = $state<SingView | null>(null);
  let singLevel = $state(0);
  let singModel = $state<ModelView | null>(null);

  const clock = new PlaybackClock();
  const t = $derived(translator(app?.language ?? "en"));

  const track = $derived(playback?.state === "track" ? playback.track : null);
  const lyrics = $derived(
    lyricsView?.status === "ready" && lyricsView.lyrics && lyricsView.trackKey === track?.key
      ? lyricsView.lyrics
      : null,
  );
  const offset = $derived((settings?.globalOffset ?? 0) + (lyricsView?.songOffset ?? 0));
  const effective = $derived(position + offset);
  const layout = $derived(settings?.layout ?? "context");
  const lyricsOnly = $derived(settings?.lyricsOnly ?? false);
  const fontSize = $derived(settings?.fontSize ?? 25);

  // Lines too long for one row are split in two, once, to fit the lyrics area.
  let areaWidth = $state(0);
  let areaHeight = $state(0);
  const measure = $derived(lyricMeasure(fontSize));
  const lines: LyricLine[] = $derived(
    lyrics && areaWidth > 0
      ? splitLongLines(lyrics.lines, track?.duration ?? 0, (areaWidth * 0.97) / 0.6, measure)
      : (lyrics?.lines ?? []),
  );
  const active = $derived(currentIndex(lines, effective));
  const activeLine = $derived(active === null ? undefined : lines[active]);
  const showLyrics = $derived(lines.length > 0 && playback?.position !== null);

  // The mini-player appears while the pointer is over the overlay.
  let hovering = $state(false);
  let scrubbing = $state(false);
  const seekHold = new SeekHold();
  const controlsShown = $derived((hovering || scrubbing) && track !== null && !settings?.clickThrough && !lyricsOnly);
  const capabilities = $derived(playback?.capabilities);
  const transport = $derived(
    controlsShown && !!capabilities && (capabilities.previous || capabilities.playPause || capabilities.next),
  );
  const canSeek = $derived(track !== null && !!capabilities?.seek && playback?.position !== null);
  // The estimated times of plain lyrics are too rough to jump to.
  const jumpable = $derived(canSeek && lyrics !== null && lyrics.sync !== "unsynced");
  const chorusMarks = $derived(
    jumpable ? (lyricsView?.chorus ?? []).map((start) => lyricSeekTarget(start, offset)) : [],
  );

  $effect(() => {
    // Turning click-through on or hiding the overlay sends no "pointer left".
    if (settings?.clickThrough || settings?.overlayVisible === false) hovering = false;
  });

  // Sing to find: the listening panel replaces the lyrics area until a song is
  // found; then the found song shows above the lyrics for a moment.
  const singing = $derived(isActive(singView));
  const singPanel = $derived(
    singView !== null && ["preparing", "listening", "searching", "notFound", "problem"].includes(singView.phase),
  );
  const foundSong = $derived(singView?.phase === "following" ? singView.song : null);
  let justFound = $state(false);
  $effect(() => {
    if (!foundSong) return;
    justFound = true;
    const timer = setTimeout(() => (justFound = false), 8000);
    return () => clearTimeout(timer);
  });
  const micShown = $derived(hovering && !settings?.clickThrough && (settings?.singToFind ?? true));

  const playerName = $derived(
    playback?.player?.service === "microphone" ? t("sing.microphone") : (playback?.player?.name ?? ""),
  );
  const status = $derived(statusText(playback, lyricsView, settings, t));
  const searching = $derived(
    playback?.state === "track" && (!lyricsView || lyricsView.status === "loading" || lyricsView.status === "idle"),
  );
  const statusMark = $derived(
    playback?.state === "track" && (lyricsView?.status === "notFound" || lyricsView?.status === "instrumental"),
  );
  // Changes when the lyrics area shows something new: another song, lyrics arriving, another status.
  const contentKey = $derived(
    singPanel
      ? `sing|${singView?.phase}|${singView?.problem?.kind ?? ""}`
      : `${track?.key ?? ""}|${showLyrics ? `lyrics|${lyrics?.lines.length ?? 0}` : `status|${status}`}`,
  );

  // With SINGLINE_DEBUG set, every line change is reported for end-to-end checks.
  let reported: number | null | undefined;
  $effect(() => {
    if (!app?.debug || !showLyrics || active === reported) return;
    reported = active;
    const text = activeLine ? activeLine.text : "";
    void api.reportLine(Number(untrack(() => effective).toFixed(2)), text);
  });

  function applyPlayback(next: Playback) {
    playback = next;
    const now = performance.now();
    const reading = {
      position: next.position,
      playing: next.playing,
      rate: next.rate,
      duration: next.track?.duration ?? null,
      trackKey: next.track?.key ?? null,
    };
    // Right after a seek, players may still report where they were.
    if (seekHold.isStale(reading, now)) reading.position = clock.positionAt(now);
    clock.update(reading, now);
    position = clock.positionAt(now);
    startTicking();
  }

  /** Sends the player to `target` seconds and shows the new position at once. */
  function seekTo(target: number) {
    if (!playback || !track || !canSeek) return;
    const duration = track.duration && track.duration > 0 ? track.duration : Infinity;
    const destination = clamp(target, 0, duration);
    void api.playerCommand({ kind: "seekTo", position: destination });
    const now = performance.now();
    seekHold.start(destination, track.key, playback.rate, now);
    clock.seek(destination, now);
    position = clock.positionAt(now);
    startTicking();
  }

  /** Jumps so that exactly this line lights up, with the timing corrections applied. */
  function jumpTo(line: LyricLine) {
    seekTo(lyricSeekTarget(line.time, offset));
  }

  let frame = 0;
  function tick() {
    position = clock.positionAt(performance.now());
    frame = clock.isPlaying ? requestAnimationFrame(tick) : 0;
  }
  function startTicking() {
    if (frame === 0 && clock.isPlaying) frame = requestAnimationFrame(tick);
  }

  function startWindowDrag() {
    void getCurrentWindow().startDragging();
  }

  function drag(event: PointerEvent) {
    if (event.button !== 0 || (event.target as HTMLElement).closest("button, [data-no-drag]")) return;
    startWindowDrag();
  }

  onMount(() => {
    let stop: (() => void) | undefined;
    let alive = true;
    void (async () => {
      const unlisten = await subscribe({
        playback: applyPlayback,
        lyrics: (next) => (lyricsView = next),
        settings: (next) => (settings = next),
        app: (next) => (app = next),
      });
      const unlistenSing = await onSing({
        view: (next) => (singView = next),
        level: (next) => (singLevel = next),
        model: (next) => (singModel = next),
      });
      if (!alive) {
        unlistenSing();
        return unlisten();
      }
      stop = () => {
        unlisten();
        unlistenSing();
      };
      singView = await singApi.view();
      const snapshot = await api.snapshot();
      settings = snapshot.settings;
      lyricsView = snapshot.lyrics;
      app = snapshot.app;
      applyPlayback(snapshot.playback);
    })();
    return () => {
      alive = false;
      stop?.();
      cancelAnimationFrame(frame);
    };
  });
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<main
  class="overlay"
  style:--background-opacity={settings?.backgroundOpacity ?? 0.78}
  style:opacity={settings?.overlayOpacity ?? 1}
  onpointerdown={drag}
  onpointerenter={() => (hovering = true)}
  onpointermove={() => (hovering = true)}
  onpointerleave={() => (hovering = false)}
>
  <!-- Lyrics only leaves out the song, the controls and the timeline. -->
  {#if !lyricsOnly}
    <Header
      {track}
      {playerName}
      controls={transport && playback ? playback.capabilities : null}
      playing={playback?.playing ?? false}
      notSynced={lyrics?.sync === "unsynced" && showLyrics}
      ownLyrics={lyricsView?.origin === "local" && lyrics !== null}
      paused={track !== null && !playback?.playing}
      mic={micShown}
      {singing}
      {t}
      onsettings={() => api.openSettings()}
      onsing={() => singApi.toggle()}
    />
  {/if}

  {#if foundSong && (justFound || (hovering && !lyricsOnly))}
    <SingFound
      title={foundSong.title}
      artist={foundSong.artist}
      alternatives={(singView?.alternatives ?? 0) > 0}
      {t}
      onnext={() => singApi.next()}
      onagain={() => singApi.again()}
    />
  {/if}

  <div class="area" bind:clientWidth={areaWidth} bind:clientHeight={areaHeight}>
    {#key contentKey}
      <div class="content" out:songExit={{ reduced: prefersReducedMotion.current }}>
        {#if singPanel && singView}
          <SingPanel
            view={singView}
            level={singLevel}
            model={singModel}
            platform={app?.platform ?? "linux"}
            {t}
            onstop={() => singApi.stop()}
            onagain={() => singApi.again()}
            onsettings={() => api.openSettings()}
          />
        {:else if showLyrics && lyrics}
          <LyricLines
            {lines}
            current={active ?? -1}
            position={effective}
            sync={lyrics.sync}
            {layout}
            {fontSize}
            width={areaWidth}
            height={areaHeight}
            playing={playback?.playing ?? false}
            {measure}
            onjump={jumpable ? jumpTo : null}
            onwindowdrag={startWindowDrag}
            {t}
          />
        {:else}
          <Status
            text={status}
            mark={statusMark}
            playing={playback?.playing ?? false}
            delay={searching ? 0.35 : 0.1}
            busy={searching && lyricsView?.status === "loading"}
            onretry={lyricsView?.canRetry && playback?.state === "track" ? () => api.reloadLyrics() : null}
            {t}
          />
        {/if}
      </div>
    {/key}
  </div>

  {#if !lyricsOnly && track?.duration && track.duration > 0}
    <Scrubber
      {position}
      duration={track.duration}
      expanded={controlsShown}
      seekable={canSeek}
      marks={chorusMarks}
      {t}
      onseek={seekTo}
      onscrub={(held) => (scrubbing = held)}
    />
  {/if}
</main>

<style>
  /* Black and see-through at the background setting, white text, no colour. */
  .overlay {
    box-sizing: border-box;
    height: 100vh;
    display: flex;
    flex-direction: column;
    gap: 10px;
    padding: 16px;
    border-radius: 22px;
    background: rgb(0 0 0 / var(--background-opacity));
    color: #fff;
    overflow: hidden;
    cursor: default;
    user-select: none;
    -webkit-user-select: none;
  }

  /* Old and new content share the area while one leaves and the other arrives. */
  .area {
    flex: 1;
    min-height: 0;
    display: grid;
    grid-template: minmax(0, 1fr) / minmax(0, 1fr);
  }

  .content {
    grid-area: 1 / 1;
    min-width: 0;
    min-height: 0;
  }

  /* Content appearing fades in, rises and sharpens. */
  .overlay :global(.reveal) {
    animation: reveal 0.55s cubic-bezier(0.23, 1, 0.32, 1) both;
  }

  @keyframes reveal {
    from {
      opacity: 0;
      transform: translateY(8px);
      filter: blur(6px);
    }
  }

  @keyframes reveal-fade {
    from {
      opacity: 0;
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .overlay :global(.reveal) {
      animation-name: reveal-fade;
    }
  }
</style>

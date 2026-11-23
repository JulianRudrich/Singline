<script lang="ts">
  // The tray panel, after the macOS app's menu: the song playing with the
  // player's controls, quick switches, the sleep timer and the other windows.
  // The Rust side shows and hides the window and sizes it to this page; the
  // page plays the entrance and fades out.
  import { onMount, tick } from "svelte";
  import { prefersReducedMotion as reducedMotion } from "svelte/motion";
  import { api, subscribe } from "../lib/api";
  import { PlaybackClock } from "../lib/clock";
  import { translator } from "../lib/i18n";
  import { clamp } from "../lib/lyrics";
  import { dropIn, duration, easeOutCss, panelKeyframes, prefersReducedMotion } from "../lib/motion";
  import { SeekHold } from "../lib/player";
  import type { AppInfo, Playback, Settings, SleepView } from "../lib/types";
  import Mark from "../lib/ui/Mark.svelte";
  import { radioMove } from "../lib/ui/radio";
  import Toggle from "../lib/ui/Toggle.svelte";
  import Controls from "../overlay/Controls.svelte";
  import CoverLink from "../overlay/CoverLink.svelte";
  import Scrubber from "../overlay/Scrubber.svelte";
  import { shuffle } from "../shuffle/api";
  import { isActive, onSing, sing, type SingView } from "../sing/api";
  import { spotify } from "../spotify/api";
  import LikeButton from "../spotify/LikeButton.svelte";
  import MenuRow from "./MenuRow.svelte";

  let settings = $state<Settings | null>(null);
  let singView = $state<SingView | null>(null);
  const singing = $derived(isActive(singView));
  let playback = $state<Playback | null>(null);
  let app = $state<AppInfo | null>(null);
  let sleep = $state<SleepView | null>(null);
  let sleepOpen = $state(false);
  let position = $state(0);
  let surface = $state<HTMLElement>();
  let content = $state<HTMLElement>();

  const t = $derived(translator(app?.language ?? "en"));
  const mac = $derived(app?.platform === "macos");
  const track = $derived(playback?.state === "track" ? playback.track : null);
  const playerName = $derived(playback?.player?.name ?? "");
  // A song playing in Spotify opens there from its cover.
  const openInSpotify = $derived.by(() => {
    const song = track;
    const player = playback?.player;
    if (!song || player?.service !== "spotify") return undefined;
    return () => void spotify.openSong(song, player.web);
  });
  const details = $derived(track ? [track.title, track.artist, playerName].filter(Boolean).join(" · ") : undefined);
  const capabilities = $derived(playback?.capabilities);
  const transport = $derived(
    track !== null && !!capabilities && (capabilities.previous || capabilities.playPause || capabilities.next),
  );
  const length = $derived(track?.duration && track.duration > 0 ? track.duration : 0);
  const canSeek = $derived(track !== null && !!capabilities?.seek && playback?.position !== null);
  const sleepDetail = $derived.by(() => {
    if (!sleep || sleep.choice === "off") return undefined;
    return sleep.minutesLeft === null ? t("sleep.endOfTrack") : t("menu.sleepLeft", { minutes: sleep.minutesLeft });
  });

  // The position moves on between readings, as in the overlay, while the panel is open.
  const clock = new PlaybackClock();
  const seekHold = new SeekHold();
  let open = false;
  let frame = 0;

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

  function seekTo(target: number) {
    if (!playback || !track || !canSeek) return;
    const destination = clamp(target, 0, length || Infinity);
    void api.playerCommand({ kind: "seekTo", position: destination });
    const now = performance.now();
    seekHold.start(destination, track.key, playback.rate, now);
    clock.seek(destination, now);
    position = clock.positionAt(now);
    startTicking();
  }

  function advance() {
    position = clock.positionAt(performance.now());
    frame = open && clock.isPlaying ? requestAnimationFrame(advance) : 0;
  }

  function startTicking() {
    if (frame === 0 && open && clock.isPlaying) frame = requestAnimationFrame(advance);
  }

  function update(patch: Partial<Settings>) {
    if (settings) settings = { ...settings, ...patch };
    void api.updateSettings(patch).then((next) => (settings = next));
  }

  function sleepLabel(choice: string): string {
    if (choice === "off") return t("sleep.off");
    if (choice === "end") return t("menu.sleepEnd");
    return t("menu.sleepMinutes", { minutes: choice });
  }

  async function chooseSleep(choice: string) {
    sleep = await api.setSleepTimer(choice);
  }

  let sleepGroup = $state<HTMLElement>();

  // The arrow keys move through the choices, as in any radio group; "End of track" needs a song.
  function sleepKey(event: KeyboardEvent) {
    if (!sleep) return;
    const { choices, endOfTrack } = sleep;
    const next = radioMove(event.key, choices.indexOf(sleep.choice), choices.length, (index) => choices[index] !== "end" || endOfTrack);
    const choice = next === null ? undefined : choices[next];
    if (next === null || !choice) return;
    event.preventDefault();
    void chooseSleep(choice);
    sleepGroup?.querySelectorAll<HTMLElement>("[role=radio]")[next]?.focus();
  }

  /** The panel would close anyway once the new window takes the focus; this is quicker. */
  function openWindow(show: () => Promise<void>) {
    void show();
    void api.closeMenuPanel();
  }

  function animate(keyframes: Keyframe[], milliseconds: number, done?: () => void) {
    const node = surface;
    if (!node) return;
    if (typeof node.animate !== "function") {
      node.style.opacity = String(keyframes.at(-1)?.opacity ?? 1);
      done?.();
      return;
    }
    for (const running of node.getAnimations()) running.cancel();
    const animation = node.animate(keyframes, { duration: milliseconds, easing: easeOutCss, fill: "forwards" });
    if (done) animation.onfinish = done;
  }

  function opened() {
    open = true;
    animate(panelKeyframes(prefersReducedMotion()), duration.pane);
    startTicking();
  }

  // It stays faded out while hidden, so the next opening starts from nothing.
  function closed() {
    open = false;
    cancelAnimationFrame(frame);
    frame = 0;
    animate([{ opacity: 1 }, { opacity: 0 }], duration.dismiss, () => (sleepOpen = false));
  }

  function keydown(event: KeyboardEvent) {
    const command = mac ? event.metaKey : event.ctrlKey;
    if (event.key === "Escape") void api.closeMenuPanel();
    else if (command && event.key === ",") openWindow(api.openSettings);
    else if (command && event.key.toLowerCase() === "q") void api.quit();
    else return;
    event.preventDefault();
  }

  onMount(() => {
    let alive = true;
    let stop: (() => void) | undefined;
    let observer: ResizeObserver | undefined;
    void (async () => {
      const unlisten = await subscribe({
        playback: applyPlayback,
        settings: (next) => (settings = next),
        app: (next) => (app = next),
        sleep: (next) => (sleep = next),
        menuPanel: (event) => (event.open ? opened() : closed()),
      });
      const unlistenSing = await onSing({ view: (next) => (singView = next) });
      if (!alive) {
        unlistenSing();
        return unlisten();
      }
      stop = () => {
        unlisten();
        unlistenSing();
      };
      const [snapshot, timer, singing] = await Promise.all([api.snapshot(), api.sleepTimer(), sing.view()]);
      if (!alive) return;
      singView = singing;
      settings = snapshot.settings;
      app = snapshot.app;
      sleep = timer;
      applyPlayback(snapshot.playback);
      // The window shows once it knows the height, so it opens at the right size.
      await tick();
      let reported = 0;
      observer = new ResizeObserver(() => {
        const height = Math.ceil(content?.offsetHeight ?? 0);
        if (height <= 0 || height === reported) return;
        reported = height;
        void api.menuPanelResize(height);
      });
      if (content) observer.observe(content);
    })();
    return () => {
      alive = false;
      stop?.();
      observer?.disconnect();
      cancelAnimationFrame(frame);
    };
  });
</script>

<svelte:window onkeydown={keydown} />

<div class="surface" bind:this={surface}>
  {#if settings && app}
    <main class="panel" aria-label={t("app.name")} bind:this={content}>
      <section class="now" title={details}>
        <div class="song">
          {#if track}
            <CoverLink url={track.artworkUrl} size={44} label={t("overlay.openInSpotify")} onopen={openInSpotify} />
          {:else}
            <Mark size={44} />
          {/if}
          <div class="names">
            <span class="title">{track?.title ?? t("app.name")}</span>
            <span class="artist">{track ? track.artist : t("menu.idle")}</span>
            {#if track && playerName}<span class="player">{playerName}</span>{/if}
          </div>
          {#if track}<LikeButton uri={track.sourceId} {t} />{/if}
        </div>
        {#if track && length > 0}
          <div class="scrub">
            <Scrubber
              {position}
              duration={length}
              expanded={true}
              seekable={canSeek}
              marks={[]}
              {t}
              onseek={seekTo}
              onscrub={() => {}}
            />
          </div>
        {/if}
        {#if transport && capabilities}
          <div class="transport">
            <Controls {capabilities} playing={playback?.playing ?? false} {t} />
          </div>
        {/if}
      </section>

      <hr />

      <MenuRow icon="overlay" title={t("tray.showOverlay")} onclick={() => update({ overlayVisible: !settings?.overlayVisible })}>
        <Toggle
          label={t("tray.showOverlay")}
          checked={settings.overlayVisible}
          onchange={(overlayVisible) => update({ overlayVisible })}
        />
      </MenuRow>
      <MenuRow icon="pointer" title={t("tray.clickThrough")} onclick={() => update({ clickThrough: !settings?.clickThrough })}>
        <Toggle label={t("tray.clickThrough")} checked={settings.clickThrough} onchange={(clickThrough) => update({ clickThrough })} />
      </MenuRow>
      <MenuRow icon="lyrics" title={t("tray.lyricsOnly")} onclick={() => update({ lyricsOnly: !settings?.lyricsOnly })}>
        <Toggle label={t("tray.lyricsOnly")} checked={settings.lyricsOnly} onchange={(lyricsOnly) => update({ lyricsOnly })} />
      </MenuRow>
      <MenuRow icon="moon" title={t("tray.sleep")} detail={sleepDetail} expanded={sleepOpen} onclick={() => (sleepOpen = !sleepOpen)} />
      {#if sleepOpen && sleep}
        <div
          class="sleep"
          role="radiogroup"
          aria-label={t("tray.sleep")}
          bind:this={sleepGroup}
          in:dropIn={{ reduced: reducedMotion.current }}
        >
          {#each sleep.choices as choice (choice)}
            {@const selected = sleep.choice === choice}
            <button
              type="button"
              role="radio"
              aria-checked={selected}
              tabindex={selected ? 0 : -1}
              class:selected
              disabled={choice === "end" && !sleep.endOfTrack}
              onclick={() => chooseSleep(choice)}
              onkeydown={sleepKey}
            >
              {sleepLabel(choice)}
            </button>
          {/each}
        </div>
      {/if}

      <hr />

      {#if settings.singToFind || singing}
        <MenuRow
          icon="search"
          title={singing ? t("tray.stopSinging") : t("tray.singToFind")}
          onclick={() => openWindow(async () => void (await sing.toggle()))}
        />
      {/if}
      <MenuRow icon="mic" title={t("tray.karaoke")} onclick={() => openWindow(() => api.openKaraoke())} />
      <MenuRow icon="chart" title={t("tray.stats")} onclick={() => openWindow(api.openStats)} />
      <MenuRow icon="shuffle" title={t("tray.trueShuffle")} onclick={() => openWindow(shuffle.openWindow)} />
      <MenuRow icon="general" title={t("tray.settings")} shortcut={mac ? "⌘," : "Ctrl+,"} onclick={() => openWindow(api.openSettings)} />
      <MenuRow icon="power" title={t("tray.quit")} shortcut={mac ? "⌘Q" : "Ctrl+Q"} onclick={() => api.quit()} />
    </main>
  {/if}
</div>

<style>
  /* Neutral black: a faint tint over the system's glass, or solid near-black
     where there is none. White text, white accents, no colour. */
  .surface {
    position: fixed;
    inset: 0;
    border-radius: var(--window-radius, 16px);
    background: #161618;
    color: #fff;
    overflow: hidden;
    opacity: 0;
    cursor: default;
    user-select: none;
    -webkit-user-select: none;
  }

  :global(html[data-glass]) .surface {
    background: rgb(10 10 12 / 0.3);
  }

  /* A hairline edge, brighter along the top, like the settings window's.
     Drawn with inset shadows: a masked gradient ring lost its mask in the
     minified build and then covered the whole panel. */
  .surface::after {
    content: "";
    position: absolute;
    inset: 0;
    border-radius: inherit;
    box-shadow:
      inset 0 1px 0 rgb(255 255 255 / 0.14),
      inset 0 0 0 1px rgb(255 255 255 / 0.08);
    pointer-events: none;
  }

  .panel {
    --row-radius: max(4px, calc(var(--window-radius, 16px) - 6px));
    box-sizing: border-box;
    display: flex;
    flex-direction: column;
    gap: 1px;
    padding: 6px;
  }

  .now {
    display: flex;
    flex-direction: column;
    padding: 8px 8px 6px;
  }

  .song {
    display: flex;
    align-items: center;
    gap: 12px;
    min-width: 0;
  }

  .names {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
  }

  .title,
  .artist,
  .player {
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
  }

  .title {
    font-size: 13.5px;
    font-weight: 600;
    line-height: 1.3;
  }

  .artist {
    font-size: 12px;
    line-height: 1.3;
    color: rgb(255 255 255 / 0.6);
  }

  .player {
    font-size: 10.5px;
    line-height: 1.35;
    color: rgb(255 255 255 / 0.5);
  }

  /* The scrubber pulls itself up 4 px under the overlay's lyrics. */
  .scrub {
    margin-top: 12px;
    padding-top: 4px;
  }

  .transport {
    display: flex;
    justify-content: center;
    margin-top: 8px;
    min-height: 26px;
  }

  hr {
    width: calc(100% - 16px);
    height: 1px;
    margin: 4px auto;
    border: 0;
    background: rgb(255 255 255 / 0.1);
  }

  .sleep {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: 6px;
    padding: 2px 8px 6px;
  }

  .sleep button {
    height: 28px;
    padding: 0 4px;
    border: 0;
    border-radius: 8px;
    background: rgb(255 255 255 / 0.08);
    color: #fff;
    font: inherit;
    font-size: 11.5px;
    font-weight: 500;
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    cursor: default;
    transition:
      background-color var(--duration-press) var(--ease-out),
      color var(--duration-press) var(--ease-out),
      transform var(--duration-press) var(--ease-out);
  }

  .sleep button:hover:not(:disabled, .selected) {
    background: rgb(255 255 255 / 0.14);
  }

  .sleep button.selected {
    background: #fff;
    color: #161618;
  }

  .sleep button:active:not(:disabled) {
    transform: scale(var(--press-scale-subtle));
  }

  .sleep button:disabled {
    opacity: 0.4;
  }

  .sleep button:focus-visible {
    outline: 2px solid rgb(255 255 255 / 0.7);
    outline-offset: 2px;
  }
</style>

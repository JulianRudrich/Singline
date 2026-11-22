<script lang="ts">
  // The True Shuffle window, like Linory's menu section: pick a source or a
  // mix, Shuffle & play, progress, Up next, and the actions for the order
  // that plays. Black and white.
  import { onMount } from "svelte";
  import { api, subscribe } from "../lib/api";
  import { translator } from "../lib/i18n";
  import Button from "../lib/ui/Button.svelte";
  import Glyph from "../lib/ui/Glyph.svelte";
  import Toggle from "../lib/ui/Toggle.svelte";
  import { onSpotifyStatus, spotify } from "../spotify/api";
  import { activityText, isError, keptName, noticeText, onShuffle, shuffle, sourceName, type ShuffleView } from "./api";

  let language = $state<"de" | "en">("en");
  let view = $state<ShuffleView | null>(null);
  let mixing = $state(false);

  const t = $derived(translator(language));
  const busy = $derived(view?.activity != null);

  /** Sources, mixes shuffled before, and a new selection not saved as a mix yet. */
  const options = $derived.by(() => {
    if (!view) return [];
    const ids = [...view.sources.map((source) => source.id), ...view.mixes];
    if (!ids.includes(view.selected)) ids.push(view.selected);
    const current = view;
    return ids.map((id) => ({ id, label: sourceName(id, current.sources, t) }));
  });

  const status = $derived.by((): { text: string; error: boolean; reset: boolean } | null => {
    if (!view) return null;
    if (view.activity) return { text: activityText(view.activity, t, language), error: false, reset: false };
    if (view.notice) return { text: noticeText(view.notice, t), error: isError(view.notice), reset: false };
    if (view.total > 0 || view.played > 0) {
      return { text: t("shuffle.progress", { played: view.played, total: view.total }), error: false, reset: view.played > 0 };
    }
    if (!view.sourcesLoaded) return { text: t("shuffle.loadingSources"), error: false, reset: false };
    return null;
  });

  async function setKeepPlaying(on: boolean) {
    view = await shuffle.setKeepPlaying(on);
  }

  /** The whole row works the switch; the switch handles its own clicks. */
  function keepPlayingRow(event: MouseEvent) {
    if (view && !(event.target as HTMLElement).closest("button")) void setKeepPlaying(!view.keepPlaying);
  }

  async function keep() {
    if (!view) return;
    const source = view.activeSource ?? view.selected;
    view = await shuffle.keep(keptName(sourceName(source, view.sources, t), language, t));
  }

  onMount(() => {
    let alive = true;
    let stops: (() => void)[] = [];
    let wasConnected = false;
    const apply = (next: ShuffleView) => {
      view = next;
      if (next.connected && !wasConnected) void shuffle.loadSources();
      wasConnected = next.connected;
    };
    void (async () => {
      const unlisten = await Promise.all([
        subscribe({ app: (app) => (language = app.language) }),
        onShuffle(apply),
        onSpotifyStatus(async () => apply(await shuffle.view())),
      ]);
      if (!alive) return unlisten.forEach((stop) => stop());
      stops = unlisten;
      const [snapshot, current] = await Promise.all([api.snapshot(), shuffle.view()]);
      language = snapshot.app.language;
      apply(current);
    })();
    return () => {
      alive = false;
      stops.forEach((stop) => stop());
    };
  });
</script>

<svelte:head>
  <title>{t("shuffle.windowTitle")}</title>
</svelte:head>

{#if view}
  <main class="shuffle">
    <header>
      <h1>{t("shuffle.title")}</h1>
      <p class="subtitle">{t("shuffle.subtitle")}</p>
    </header>

    {#if !view.connected}
      <section class="card">
        <p class="hint">{t("shuffle.connectHint")}</p>
        <span class="open">
          <Button onclick={() => api.openSettings()}>{t("shuffle.openSettings")}<Glyph name="chevronRight" size={12} /></Button>
        </span>
      </section>
    {:else}
      <section class="card">
        <div class="picker">
          <select
            aria-label={t("shuffle.source")}
            value={view.selected}
            disabled={busy}
            onchange={async (event) => (view = await shuffle.select(event.currentTarget.value))}
          >
            {#each options as option (option.id)}
              <option value={option.id}>{option.label}</option>
            {/each}
          </select>
          <button
            class="icon"
            class:on={mixing || view.members.length > 1}
            aria-pressed={mixing}
            title={t("shuffle.mixSources")}
            aria-label={t("shuffle.mixSources")}
            disabled={busy}
            onclick={() => (mixing = !mixing)}
          >
            <svg width="16" height="16" viewBox="0 0 24 24" aria-hidden="true">
              <path d="M12 3 3 7.5l9 4.5 9-4.5Z" />
              <path d="m3 12 9 4.5 9-4.5M3 16.5 12 21l9-4.5" />
            </svg>
          </button>
        </div>

        {#if mixing}
          <div class="mix" role="group" aria-label={t("shuffle.mixSources")}>
            {#each view.sources as source (source.id)}
              <label class="mix-row">
                <input
                  type="checkbox"
                  checked={view.members.includes(source.id)}
                  disabled={busy}
                  onchange={async () => (view = await shuffle.toggleSource(source.id))}
                />
                <span class="ellipsis">{source.name ?? t("shuffle.liked")}</span>
              </label>
            {/each}
          </div>
        {/if}

        <button class="play" disabled={busy} onclick={async () => (view = await shuffle.play())}>
          {#if busy}
            <span class="spinner" aria-hidden="true"></span>
          {:else}
            <svg width="15" height="15" viewBox="0 0 24 24" aria-hidden="true">
              <path d="M16 3h5v5M4 20 21 3M21 16v5h-5M15 15l6 6M4 4l5 5" />
            </svg>
          {/if}
          {t("shuffle.play")}
        </button>

        {#if status}
          <p class="status" class:error={status.error} role="status">
            {status.text}
            {#if status.reset && !busy}
              <span aria-hidden="true">·</span>
              <button class="text" onclick={async () => (view = await shuffle.reset())}>{t("shuffle.reset")}</button>
            {/if}
          </p>
        {/if}

        {#if view.upNext.length > 0}
          <div class="up-next">
            <span class="caption">{t("shuffle.upNext")}</span>
            {#each view.upNext as item, index (index)}
              <button class="track" title={`${item.name} – ${item.artist}`} onclick={() => spotify.open(item.uri)}>
                <span class="ellipsis">{item.name}<span class="artist">{item.artist}</span></span>
              </button>
            {/each}
          </div>
        {/if}
      </section>

      <section class="card rows">
        <!-- The switch is what the keyboard reaches; the row only widens where a click works. -->
        <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
        <div class="row" onclick={keepPlayingRow}>
          <svg class="row-icon" width="17" height="17" viewBox="0 0 24 24" aria-hidden="true">
            <path d="M12 12c-2-2.7-3.6-4-5.5-4a4 4 0 0 0 0 8c1.9 0 3.5-1.3 5.5-4Zm0 0c2 2.7 3.6 4 5.5 4a4 4 0 0 0 0-8c-1.9 0-3.5 1.3-5.5 4Z" />
          </svg>
          <span class="row-text">
            <span>{t("shuffle.keepPlaying")}</span>
            <span class="hint">{t("shuffle.keepPlayingHint")}</span>
          </span>
          <Toggle label={t("shuffle.keepPlaying")} checked={view.keepPlaying} onchange={setKeepPlaying} />
        </div>
        {#if view.inShuffle}
          <button class="row" disabled={busy} onclick={async () => (view = await shuffle.removeCurrent())}>
            <svg class="row-icon" width="17" height="17" viewBox="0 0 24 24" aria-hidden="true">
              <circle cx="12" cy="12" r="9" /><path d="M8 12h8" />
            </svg>
            <span class="row-text">{t("shuffle.remove")}</span>
          </button>
        {/if}
        {#if view.hasShuffle}
          <button class="row" disabled={busy} onclick={keep}>
            <svg class="row-icon" width="17" height="17" viewBox="0 0 24 24" aria-hidden="true">
              <path d="M12 4v11m-4.5-4.5L12 15l4.5-4.5M5 19h14" />
            </svg>
            <span class="row-text">{t("shuffle.keep")}</span>
          </button>
        {/if}
        {#if view.removed > 0}
          <button class="row" disabled={busy} onclick={async () => (view = await shuffle.restoreRemoved())}>
            <svg class="row-icon" width="17" height="17" viewBox="0 0 24 24" aria-hidden="true">
              <path d="M9 14 4 9l5-5" /><path d="M4 9h10.5a5.5 5.5 0 0 1 0 11H11" />
            </svg>
            <span class="row-text">{t("shuffle.restore", { count: view.removed })}</span>
          </button>
        {/if}
      </section>
    {/if}
  </main>
{/if}

<style>
  /* Neutral black glass, as in Your music. */
  :global(html[data-view="shuffle"]) {
    color-scheme: dark;
    background: #111113;
  }

  :global(html[data-view="shuffle"] body) {
    min-height: 100%;
    height: auto;
    color: #fff;
    background:
      radial-gradient(110% 70% at 0% 0%, rgb(255 255 255 / 0.07), transparent 62%),
      linear-gradient(160deg, #1c1d20 0%, #121214 48%, #0a0a0b 100%);
    background-attachment: fixed;
  }

  .shuffle {
    display: flex;
    flex-direction: column;
    gap: 10px;
    padding: 22px 20px 20px;
    color: #fff;
    font-size: 13px;
  }

  header {
    margin-bottom: 6px;
  }

  h1 {
    margin: 0;
    font-size: 22px;
    font-weight: 600;
    letter-spacing: -0.01em;
  }

  .subtitle,
  .hint {
    margin: 4px 0 0;
    font-size: 12px;
    line-height: 1.4;
    color: rgb(255 255 255 / 0.55);
  }

  .card {
    display: flex;
    flex-direction: column;
    gap: 10px;
    padding: 16px;
    border-radius: 12px;
    background: rgb(255 255 255 / 0.05);
    border: 1px solid rgb(255 255 255 / 0.08);
  }

  .card.rows {
    gap: 2px;
    padding: 6px;
  }

  .open {
    align-self: flex-start;
  }

  .open :global(.button) {
    display: inline-flex;
    align-items: center;
    gap: 4px;
  }

  button {
    border: 0;
    background: none;
    color: inherit;
    font: inherit;
    cursor: pointer;
    transition:
      transform var(--duration-press) var(--ease-out),
      background-color var(--duration-press) var(--ease-out),
      color var(--duration-press) var(--ease-out);
  }

  button:active:not(:disabled) {
    transform: scale(var(--press-scale-subtle));
  }

  button:disabled {
    cursor: default;
    opacity: 0.45;
  }

  button:focus-visible,
  select:focus-visible {
    outline: 2px solid rgb(255 255 255 / 0.7);
    outline-offset: 2px;
  }

  .picker {
    display: flex;
    gap: 6px;
  }

  /* Drawn here, as WebKitGTK and others would paint a light system menu. */
  select {
    flex: 1;
    min-width: 0;
    padding: 6px 28px 6px 9px;
    border-radius: 8px;
    border: 1px solid rgb(255 255 255 / 0.14);
    background: rgb(0 0 0 / 0.25)
      url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='10' height='6' viewBox='0 0 10 6'%3E%3Cpath d='M1 1l4 4 4-4' fill='none' stroke='white' stroke-width='1.5' stroke-linecap='round' stroke-linejoin='round'/%3E%3C/svg%3E")
      no-repeat right 10px center;
    color: #fff;
    font: inherit;
    appearance: none;
    -webkit-appearance: none;
    transition: border-color var(--duration-press) var(--ease-out);
  }

  select:hover:not(:disabled) {
    border-color: rgb(255 255 255 / 0.24);
  }

  option {
    background: #1c1c1e;
    color: #fff;
  }

  .icon {
    flex: none;
    display: grid;
    place-items: center;
    width: 32px;
    border-radius: 8px;
    color: rgb(255 255 255 / 0.55);
  }

  .icon:hover:not(:disabled) {
    color: #fff;
  }

  .icon.on {
    color: #fff;
    background: rgb(255 255 255 / 0.12);
  }

  svg path,
  svg circle {
    fill: none;
    stroke: currentColor;
    stroke-width: 1.8;
    stroke-linecap: round;
    stroke-linejoin: round;
  }

  .mix {
    max-height: 150px;
    overflow-y: auto;
    padding: 4px 0;
    border-radius: 10px;
    background: rgb(255 255 255 / 0.04);
  }

  .mix-row {
    display: flex;
    align-items: center;
    gap: 9px;
    height: 28px;
    padding: 0 10px;
    font-size: 12px;
  }

  /* A white box with a dark tick, drawn here so no system colour shows. */
  .mix-row input {
    flex: none;
    width: 15px;
    height: 15px;
    margin: 0;
    border: 1.5px solid rgb(255 255 255 / 0.4);
    border-radius: 4px;
    background: transparent no-repeat center / 11px 11px;
    appearance: none;
    -webkit-appearance: none;
    cursor: pointer;
    transition:
      background-color var(--duration-press) var(--ease-out),
      border-color var(--duration-press) var(--ease-out);
  }

  .mix-row input:hover:not(:disabled) {
    border-color: rgb(255 255 255 / 0.7);
  }

  .mix-row input:checked {
    border-color: #fff;
    background-color: #fff;
    background-image: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 12 12'%3E%3Cpath d='M2.5 6.2l2.3 2.3 4.7-5' fill='none' stroke='%23111' stroke-width='1.8' stroke-linecap='round' stroke-linejoin='round'/%3E%3C/svg%3E");
  }

  .mix-row input:focus-visible {
    outline: 2px solid rgb(255 255 255 / 0.7);
    outline-offset: 2px;
  }

  .mix-row input:disabled {
    opacity: 0.45;
    cursor: default;
  }

  .play {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 7px;
    padding: 8px;
    border-radius: 10px;
    background: #fff;
    color: #000;
    font-size: 12.5px;
    font-weight: 600;
  }

  .play:hover:not(:disabled) {
    background: rgb(255 255 255 / 0.88);
  }

  .play:disabled {
    opacity: 0.55;
  }

  .status {
    margin: 0;
    font-size: 11.5px;
    font-variant-numeric: tabular-nums;
    color: rgb(255 255 255 / 0.55);
  }

  /* No colour for problems: they stand out by brightness. */
  .status.error {
    color: rgb(255 255 255 / 0.85);
  }

  .text {
    padding: 0;
    font-size: inherit;
    color: #fff;
    text-decoration: underline;
    text-decoration-color: rgb(255 255 255 / 0.4);
    text-underline-offset: 2px;
  }

  .text:hover {
    text-decoration-color: #fff;
  }

  .up-next {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .caption {
    font-size: 10px;
    font-weight: 600;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: rgb(255 255 255 / 0.5);
  }

  .track {
    display: flex;
    padding: 3px 6px;
    margin: 0 -6px;
    border-radius: 7px;
    font-size: 11.5px;
    text-align: left;
  }

  .track:hover,
  .row:hover:not(:disabled) {
    background: rgb(255 255 255 / 0.08);
  }

  .track:focus-visible,
  .row:focus-visible {
    outline-offset: -2px;
  }

  .artist {
    margin-left: 7px;
    color: rgb(255 255 255 / 0.55);
  }

  .ellipsis {
    min-width: 0;
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
  }

  .row {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 8px 10px;
    border-radius: 9px;
    text-align: left;
    transition: background-color var(--duration-press) var(--ease-out);
  }

  div.row {
    cursor: default;
  }

  .row-icon {
    flex: none;
    color: rgb(255 255 255 / 0.75);
  }

  .row-text {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
  }

  .row-text .hint {
    margin: 1px 0 0;
    font-size: 11px;
  }

  .spinner {
    width: 12px;
    height: 12px;
    box-sizing: border-box;
    border-radius: 50%;
    border: 2px solid rgb(0 0 0 / 0.2);
    border-top-color: #000;
    animation: spin 0.8s linear infinite;
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .spinner {
      animation-duration: 2.4s;
    }
  }
</style>

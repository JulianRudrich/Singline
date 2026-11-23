<script lang="ts">
  import { onMount } from "svelte";
  import { api, subscribe } from "../lib/api";
  import { translator } from "../lib/i18n";
  import { paneIn } from "../lib/motion";
  import type { AppInfo, HotkeyError, LyricsView, Playback, Settings } from "../lib/types";
  import Glyph from "../lib/ui/Glyph.svelte";
  import Sidebar from "../lib/ui/Sidebar.svelte";
  import { panes, type PaneContext } from "./panes";

  let settings = $state<Settings | null>(null);
  let playback = $state<Playback | null>(null);
  let lyricsView = $state<LyricsView | null>(null);
  let app = $state<AppInfo | null>(null);
  let selected = $state(panes[0]?.id ?? "");
  /**
   * Which way the last pane change went in the list: 1 down, -1 up. The first
   * pane appears with the window (0); later ones come in from that side.
   */
  let direction = $state(0);
  let content = $state<HTMLElement>();

  const language = $derived(app?.language ?? "en");
  const t = $derived(translator(language));
  const pane = $derived(panes.find((entry) => entry.id === selected) ?? panes[0]);
  const mac = $derived(app?.platform === "macos");

  $effect(() => {
    document.title = t("settings.title");
  });

  // The panes render only once the snapshot arrived, so settings and app are set.
  const ctx: PaneContext = {
    get settings() {
      return settings as Settings;
    },
    get app() {
      return app as AppInfo;
    },
    get playback() {
      return playback;
    },
    get lyrics() {
      return lyricsView;
    },
    get language() {
      return language;
    },
    get t() {
      return t;
    },
    async update(patch) {
      if (settings) settings = { ...settings, ...patch };
      settings = await api.updateSettings(patch);
    },
    async setHotkey(hotkey) {
      try {
        settings = await api.setHotkey(hotkey);
        return null;
      } catch (error) {
        const known: HotkeyError[] = ["invalid", "unsupported", "needsModifier", "taken"];
        return known.find((name) => name === error) ?? "taken";
      }
    },
    async setLaunchAtLogin(enabled) {
      const actual = await api.setLaunchAtLogin(enabled);
      if (app) app = { ...app, launchAtLogin: actual };
    },
    async adjustSongOffset(delta) {
      const value = await api.adjustSongOffset(delta);
      if (lyricsView) lyricsView = { ...lyricsView, songOffset: value };
    },
    async resetSongOffset() {
      const value = await api.setSongOffset(0);
      if (lyricsView) lyricsView = { ...lyricsView, songOffset: value };
    },
  };

  // ⌘Q or Ctrl+Q quits, as in the tray menu. A shortcut being recorded keeps the keys.
  function keydown(event: KeyboardEvent) {
    const command = mac ? event.metaKey : event.ctrlKey;
    if (event.defaultPrevented || !command || event.shiftKey || event.altKey) return;
    if (event.key.toLowerCase() !== "q") return;
    event.preventDefault();
    void api.quit();
  }

  function select(id: string) {
    if (id === selected) return;
    const order = (paneId: string) => panes.findIndex((entry) => entry.id === paneId);
    direction = order(id) < order(selected) ? -1 : 1;
    selected = id;
    content?.scrollTo({ top: 0 });
  }

  onMount(() => {
    let stop: (() => void) | undefined;
    let alive = true;
    void (async () => {
      const unlisten = await subscribe({
        playback: (next) => (playback = next),
        lyrics: (next) => (lyricsView = next),
        settings: (next) => (settings = next),
        app: (next) => (app = next),
      });
      if (!alive) return unlisten();
      stop = unlisten;
      const snapshot = await api.snapshot();
      settings = snapshot.settings;
      playback = snapshot.playback;
      lyricsView = snapshot.lyrics;
      app = snapshot.app;
    })();
    return () => {
      alive = false;
      stop?.();
    };
  });
</script>

<svelte:window onkeydown={keydown} />

{#if settings && app && pane}
  <!-- Like the macOS app, the window moves when dragged anywhere but on a control. -->
  <main class="window" data-tauri-drag-region="deep">
    <Sidebar
      title={t("app.name")}
      caption="v{app.version}"
      items={panes.map((entry) => ({ id: entry.id, icon: entry.icon, label: t(entry.title) }))}
      {selected}
      label={t("settings.sections")}
      panel="settings-pane"
      onselect={select}
    >
      {#snippet footer()}
        <button
          type="button"
          class="quit"
          aria-keyshortcuts={mac ? "Meta+Q" : "Control+Q"}
          title={mac ? "⌘Q" : "Ctrl+Q"}
          onclick={() => api.quit()}
        >
          <Glyph name="power" size={16} />
          <span>{t("settings.quit")}</span>
        </button>
      {/snippet}
    </Sidebar>
    <div class="content" id="settings-pane" role="tabpanel" aria-labelledby="tab-{pane.id}" bind:this={content}>
      {#key pane.id}
        <div use:paneIn={direction}>
          <header>
            <h1>{t(pane.title)}</h1>
            <p>{t(pane.subtitle)}</p>
          </header>
          <div class="cards">
            <pane.component {ctx} />
          </div>
        </div>
      {/key}
    </div>
  </main>
{/if}

<style>
  /* Dark glass: a neutral black tint over the window's blur, or over the solid
     background where there is none. */
  .window {
    --top-inset: 24px;
    position: fixed;
    inset: 0;
    display: flex;
    overflow: hidden;
    color: #fff;
    background: linear-gradient(135deg, rgb(48 48 48 / 0.72), rgb(22 22 22 / 0.72));
    cursor: default;
    user-select: none;
    -webkit-user-select: none;
    animation: window-in var(--duration-pane) var(--ease-out) both;
  }

  /* Under a transparent title bar (macOS) the glass is the whole window: round
     corners, light along the top edge and room for the window buttons. */
  :global(html[data-titlebar="overlay"]) .window {
    --top-inset: 44px;
    border-radius: 20px;
  }

  /* Inset shadows rather than a masked gradient ring, which lost its mask in
     the minified build and then washed over the whole window. */
  :global(html[data-titlebar="overlay"]) .window::after {
    content: "";
    position: absolute;
    inset: 0;
    border-radius: inherit;
    box-shadow:
      inset 0 1px 0 rgb(255 255 255 / 0.18),
      inset 0 0 0 1px rgb(255 255 255 / 0.09);
    pointer-events: none;
  }

  @keyframes window-in {
    from {
      opacity: 0;
    }
  }

  .content {
    flex: 1;
    min-width: 0;
    box-sizing: border-box;
    padding: var(--top-inset) 20px 24px 28px;
    border-left: 1px solid rgb(255 255 255 / 0.08);
    overflow-y: scroll;
  }

  /* A slim scroll bar that shows while the pointer is over the pane. */
  .content::-webkit-scrollbar {
    width: 8px;
  }

  .content::-webkit-scrollbar-thumb {
    border: 2px solid transparent;
    border-radius: 4px;
    background-clip: padding-box;
  }

  .content:hover::-webkit-scrollbar-thumb {
    background-color: rgb(255 255 255 / 0.2);
  }

  header {
    margin-bottom: 20px;
  }

  /* Like a section in the sidebar, only quieter. */
  .quit {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    padding: 8px 12px;
    border: 0;
    border-radius: 8px;
    background: none;
    color: rgb(255 255 255 / 0.5);
    font: inherit;
    font-size: 13px;
    font-weight: 500;
    text-align: left;
    cursor: default;
    transition:
      color var(--duration-press) var(--ease-out),
      background-color var(--duration-press) var(--ease-out),
      transform var(--duration-press) var(--ease-out);
  }

  .quit:hover {
    color: rgb(255 255 255 / 0.85);
    background: rgb(255 255 255 / 0.06);
  }

  .quit:active {
    transform: scale(var(--press-scale-subtle));
  }

  .quit:focus-visible {
    outline: 2px solid rgb(255 255 255 / 0.7);
    outline-offset: -2px;
  }

  h1 {
    margin: 0;
    font-size: 22px;
    font-weight: 600;
    line-height: 1.2;
  }

  header p {
    margin: 4px 0 0;
    font-size: 13px;
    color: rgb(255 255 255 / 0.6);
  }

  .cards {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
</style>

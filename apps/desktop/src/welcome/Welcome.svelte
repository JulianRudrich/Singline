<script lang="ts">
  import { onMount } from "svelte";
  import { api } from "../lib/api";
  import { hotkeyLabel } from "../lib/hotkey";
  import { translator } from "../lib/i18n";
  import { connect } from "../lib/live";
  import { paneIn } from "../lib/motion";
  import type { AppInfo, Playback, Settings, SourcePreference } from "../lib/types";
  import Glyph, { type GlyphName } from "../lib/ui/Glyph.svelte";
  import KeyCap from "../lib/ui/KeyCap.svelte";
  import Mark from "../lib/ui/Mark.svelte";
  import BrowserBridge from "../settings/BrowserBridge.svelte";
  import PlayerPicker from "./PlayerPicker.svelte";
  import TrayPreview from "./TrayPreview.svelte";

  // Shown once, at the first start: where Singline lives, the player it
  // follows, the browser extension, and what to do next.
  const steps = ["intro", "player", "browser", "ready"] as const;

  let settings = $state<Settings | null>(null);
  let playback = $state<Playback | null>(null);
  let app = $state<AppInfo | null>(null);
  let index = $state(0);
  /** The first step appears with the window; later ones fade in. */
  let moved = $state(false);

  const step = $derived(steps[index]);
  const last = $derived(index === steps.length - 1);
  const t = $derived(translator(app?.language ?? "en"));
  const platform = $derived(app?.platform ?? "other");
  const where = $derived(
    platform === "macos" ? t("welcome.whereMac") : platform === "windows" ? t("welcome.whereWindows") : t("welcome.whereLinux"),
  );
  const tips = $derived<{ icon: GlyphName; text: string }[]>([
    { icon: "pointer", text: t("welcome.tipControls") },
    { icon: "lyrics", text: t("welcome.tipLyricsOnly") },
    { icon: "overlay", text: t("welcome.tipClickThrough") },
    { icon: "mic", text: t("welcome.tipSing") },
  ]);

  $effect(() => {
    document.title = t("welcome.windowTitle");
  });

  function go(next: number) {
    index = Math.max(0, Math.min(steps.length - 1, next));
    moved = true;
  }

  function forward() {
    if (last) void api.finishWelcome();
    else go(index + 1);
  }

  async function choose(source: SourcePreference) {
    if (settings) settings = { ...settings, source };
    settings = await api.updateSettings({ source });
  }

  // Return moves on, unless a control has the focus and takes the key itself.
  function keydown(event: KeyboardEvent) {
    if (event.key !== "Enter" || event.defaultPrevented) return;
    if ((event.target as HTMLElement | null)?.closest("button, a, input")) return;
    event.preventDefault();
    forward();
  }

  onMount(() =>
    connect(
      {
        playback: (next) => (playback = next),
        settings: (next) => (settings = next),
        app: (next) => (app = next),
      },
      (snapshot) => {
        settings = snapshot.settings;
        playback = snapshot.playback;
        app = snapshot.app;
      },
    ),
  );
</script>

<svelte:window onkeydown={keydown} />

{#if settings && app}
  <!-- Like the settings, the window moves when dragged anywhere but on a control. -->
  <main class="window" data-tauri-drag-region="deep">
    {#key step}
      <section class="step" class:centered={step === "intro"} use:paneIn={moved} aria-labelledby="welcome-title">
        {#if step === "intro"}
          <div class="intro">
            <Mark size={56} />
            <h1 id="welcome-title">{t("welcome.title")}</h1>
            <p class="lead">{t("welcome.tagline")}</p>
          </div>
          <TrayPreview {platform} label={where} />
          <p class="note">{where} {t("welcome.reopen")}</p>
        {:else if step === "player"}
          <header>
            <h1 id="welcome-title">{t("welcome.playerTitle")}</h1>
            <p>{t("welcome.playerHint")}</p>
          </header>
          <PlayerPicker
            source={settings.source}
            {playback}
            {platform}
            {t}
            onchange={choose}
            onpermission={() => api.openAutomationSettings()}
          />
        {:else if step === "browser"}
          <header>
            <h1 id="welcome-title">{t("welcome.browserTitle")}</h1>
            <p>{t("welcome.browserHint")}</p>
          </header>
          <BrowserBridge />
          <a class="link" href="https://github.com/JulianRudrich/Singline#browser-extension" target="_blank" rel="noreferrer">
            {t("welcome.getExtension")}
            <Glyph name="external" size={13} />
          </a>
        {:else}
          <header>
            <h1 id="welcome-title">{t("welcome.readyTitle")}</h1>
            <p>{t("welcome.readyHint")}</p>
          </header>
          <ul class="tips">
            <li>
              <span class="icon"><Glyph name="keyboard" size={17} /></span>
              <span class="text">{t("welcome.tipShortcut")}</span>
              <KeyCap label={hotkeyLabel(settings.hotkey, platform)} />
            </li>
            {#each tips as tip (tip.icon)}
              <li>
                <span class="icon"><Glyph name={tip.icon} size={17} /></span>
                <span class="text">{tip.text}</span>
              </li>
            {/each}
          </ul>
        {/if}
      </section>
    {/key}

    <footer>
      <div class="dots" role="img" aria-label={t("welcome.progress", { step: index + 1, count: steps.length })}>
        {#each steps as name, position (name)}
          <span class="dot" class:current={position === index}></span>
        {/each}
      </div>
      <div class="buttons">
        {#if index > 0}
          <button type="button" class="secondary" onclick={() => go(index - 1)}>{t("welcome.back")}</button>
        {/if}
        <button type="button" class="primary" onclick={forward}>
          {index === 0 ? t("welcome.start") : last ? t("welcome.done") : t("welcome.next")}
        </button>
      </div>
    </footer>
  </main>
{/if}

<style>
  /* The dark glass of the settings: a neutral black tint over the window's
     blur, or over the solid background where there is none. */
  .window {
    --top-inset: 28px;
    position: fixed;
    inset: 0;
    box-sizing: border-box;
    display: flex;
    flex-direction: column;
    padding: var(--top-inset) 32px 22px;
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

  .step {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
    gap: 14px;
  }

  /* The first step stands on its own in the middle; the others start at the top. */
  .step.centered {
    justify-content: center;
    padding-bottom: 12px;
  }

  .intro {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 6px;
    margin-bottom: 4px;
    text-align: center;
  }

  h1 {
    margin: 0;
    font-size: 22px;
    font-weight: 600;
    line-height: 1.2;
  }

  .intro h1 {
    margin-top: 10px;
    font-size: 24px;
  }

  .lead,
  header p {
    margin: 0;
    font-size: 13px;
    color: rgb(255 255 255 / 0.6);
  }

  header p {
    margin-top: 4px;
  }

  .note {
    margin: 0;
    font-size: 12px;
    line-height: 1.5;
    color: rgb(255 255 255 / 0.7);
  }

  .link {
    align-self: flex-start;
    display: inline-flex;
    align-items: center;
    gap: 6px;
    margin-left: -10px;
    padding: 6px 10px;
    border-radius: 8px;
    color: #fff;
    font-size: 13px;
    font-weight: 500;
    text-decoration: none;
    transition: background-color var(--duration-press) var(--ease-out);
  }

  .link:hover {
    background: rgb(255 255 255 / 0.06);
  }

  .link:focus-visible {
    outline: 2px solid rgb(255 255 255 / 0.7);
    outline-offset: 0;
  }

  .tips {
    display: flex;
    flex-direction: column;
    gap: 8px;
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .tips li {
    display: flex;
    align-items: center;
    gap: 14px;
    padding: 11px 14px;
    border: 1px solid rgb(255 255 255 / 0.08);
    border-radius: 12px;
    background: rgb(255 255 255 / 0.05);
    font-size: 12.5px;
  }

  .icon {
    flex: none;
    display: grid;
    place-items: center;
    width: 22px;
    color: rgb(255 255 255 / 0.85);
  }

  .text {
    flex: 1;
    min-width: 0;
  }

  footer {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding-top: 16px;
  }

  .dots {
    display: flex;
    gap: 6px;
  }

  .dot {
    width: 6px;
    height: 6px;
    border-radius: 3px;
    background: rgb(255 255 255 / 0.25);
    transition:
      width var(--duration-pane) var(--ease-out),
      background-color var(--duration-pane) var(--ease-out);
  }

  .dot.current {
    width: 18px;
    background: #fff;
  }

  .buttons {
    display: flex;
    gap: 8px;
  }

  /* White with dark text: the accent is the text colour, as everywhere in Singline. */
  .primary,
  .secondary {
    height: 32px;
    padding: 0 16px;
    border: 0;
    border-radius: 8px;
    font: inherit;
    font-size: 13px;
    font-weight: 600;
    cursor: default;
    transition:
      transform var(--duration-press) var(--ease-out),
      background-color var(--duration-press) var(--ease-out),
      color var(--duration-press) var(--ease-out);
  }

  .primary {
    background: #fff;
    color: #111;
  }

  .primary:hover {
    background: rgb(255 255 255 / 0.88);
  }

  .secondary {
    background: none;
    color: rgb(255 255 255 / 0.75);
  }

  .secondary:hover {
    background: rgb(255 255 255 / 0.08);
    color: #fff;
  }

  .primary:active,
  .secondary:active {
    transform: scale(var(--press-scale));
  }

  .primary:focus-visible,
  .secondary:focus-visible {
    outline: 2px solid rgb(255 255 255 / 0.7);
    outline-offset: 2px;
  }

  @media (prefers-reduced-motion: reduce) {
    .dot {
      transition: none;
    }
  }
</style>

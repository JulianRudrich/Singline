<script lang="ts">
  import type { MessageKey, Translate } from "../lib/i18n";
  import type { AppInfo, Playback, SourcePreference } from "../lib/types";
  import Glyph from "../lib/ui/Glyph.svelte";
  import { radioMove } from "../lib/ui/radio";

  // The players to choose from, each with where it plays on this system, and a
  // live line underneath that says whether Singline already sees a song.
  let {
    source,
    playback,
    platform,
    t,
    onchange,
    onpermission,
  }: {
    source: SourcePreference;
    playback: Playback | null;
    platform: AppInfo["platform"];
    t: Translate;
    onchange: (source: SourcePreference) => void;
    onpermission: () => void;
  } = $props();

  const mac = $derived(platform === "macos");
  const services: SourcePreference[] = ["spotify", "appleMusic", "amazonMusic", "youTubeMusic"];
  const choices: SourcePreference[] = ["automatic", ...services];
  let group = $state<HTMLElement>();

  // The arrow keys choose the next player, as in any radio group.
  function keydown(event: KeyboardEvent) {
    const next = radioMove(event.key, choices.indexOf(source), choices.length);
    const choice = next === null ? undefined : choices[next];
    if (next === null || !choice) return;
    event.preventDefault();
    onchange(choice);
    group?.querySelectorAll<HTMLElement>("[role=radio]")[next]?.focus();
  }

  /** Where the player runs here: its app, the browser through the extension, or both. */
  function where(choice: SourcePreference): MessageKey {
    switch (choice) {
      case "automatic":
        return "welcome.whereAnything";
      case "spotify":
        return "welcome.whereAppOrBrowser";
      case "appleMusic":
        return platform === "linux" ? "welcome.whereBrowser" : "welcome.whereApp";
      case "amazonMusic":
        return platform === "windows" ? "welcome.whereAppOrBrowser" : "welcome.whereBrowser";
      default:
        return "welcome.whereBrowser";
    }
  }

  const name = $derived(t(`source.${source}` as MessageKey));
  const heard = $derived(
    playback?.state === "track" && playback.track && (source === "automatic" || playback.player?.service === source)
      ? playback.track
      : null,
  );
  const denied = $derived(playback?.state === "problem" && playback.problem?.kind === "permissionDenied");
  const status = $derived.by(() => {
    if (heard) return t("welcome.statusHeard", { title: heard.title, artist: heard.artist });
    if (denied) return t("welcome.statusDenied", { player: playback?.player?.name ?? name });
    if (source === "automatic") return t("welcome.statusPlayAny");
    if (where(source) === "welcome.whereBrowser") return t("welcome.statusBrowser", { player: name });
    if (mac && (source === "spotify" || source === "appleMusic")) return t("welcome.statusAsk", { player: name });
    return t("welcome.statusPlay", { player: name });
  });
</script>

<div class="choices" role="radiogroup" aria-label={t("welcome.playerTitle")} bind:this={group}>
  {#each choices as choice (choice)}
    <button
      type="button"
      role="radio"
      aria-checked={choice === source}
      tabindex={choice === source ? 0 : -1}
      class="choice"
      class:wide={choice === "automatic"}
      onclick={() => onchange(choice)}
      onkeydown={keydown}
    >
      <span class="name">{t(`source.${choice}` as MessageKey)}</span>
      <span class="where">{t(where(choice))}</span>
      {#if choice === source}
        <span class="check" aria-hidden="true">
          <svg width="10" height="10" viewBox="0 0 12 12"><path d="M2.5 6.2l2.3 2.3 4.7-5" /></svg>
        </span>
      {/if}
    </button>
  {/each}
</div>

<div class="status" class:heard aria-live="polite">
  <span class="icon">
    {#if heard}
      <svg width="16" height="16" viewBox="0 0 16 16" aria-hidden="true"><path d="M3.5 8.3l3 3 6-6.6" /></svg>
    {:else}
      <Glyph name="note" size={16} />
    {/if}
  </span>
  <span class="text">{status}</span>
  {#if denied && mac}
    <button type="button" class="action" onclick={onpermission}>{t("welcome.openAutomation")}</button>
  {/if}
</div>

<style>
  .choices {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 8px;
  }

  .choice {
    position: relative;
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 2px;
    padding: 11px 14px;
    border: 1px solid rgb(255 255 255 / 0.08);
    border-radius: 12px;
    background: rgb(255 255 255 / 0.05);
    color: #fff;
    font: inherit;
    text-align: left;
    cursor: default;
    transition:
      background-color var(--duration-pane) var(--ease-out),
      border-color var(--duration-pane) var(--ease-out),
      transform var(--duration-press) var(--ease-out);
  }

  .choice.wide {
    grid-column: 1 / -1;
  }

  .choice:hover {
    background: rgb(255 255 255 / 0.08);
  }

  .choice:active {
    transform: scale(var(--press-scale-subtle));
  }

  .choice[aria-checked="true"] {
    border-color: rgb(255 255 255 / 0.55);
    background: rgb(255 255 255 / 0.12);
  }

  .choice:focus-visible {
    outline: 2px solid rgb(255 255 255 / 0.7);
    outline-offset: 2px;
  }

  .name {
    font-size: 13px;
    font-weight: 600;
  }

  .where {
    font-size: 11.5px;
    color: rgb(255 255 255 / 0.55);
  }

  /* White with a dark tick, like the switches: no accent colour. */
  .check {
    position: absolute;
    top: 50%;
    right: 12px;
    display: grid;
    place-items: center;
    width: 18px;
    height: 18px;
    margin-top: -9px;
    border-radius: 50%;
    background: #fff;
  }

  .check svg,
  .status svg {
    fill: none;
    stroke-width: 1.8;
    stroke-linecap: round;
    stroke-linejoin: round;
  }

  .check svg {
    stroke: #111;
  }

  .status {
    display: flex;
    align-items: center;
    gap: 12px;
    margin-top: 12px;
    padding: 12px 14px;
    border-radius: 12px;
    background: rgb(255 255 255 / 0.05);
    color: rgb(255 255 255 / 0.75);
    font-size: 12px;
    transition:
      background-color var(--duration-pane) var(--ease-out),
      color var(--duration-pane) var(--ease-out);
  }

  .status.heard {
    background: rgb(255 255 255 / 0.12);
    color: #fff;
  }

  .icon {
    flex: none;
    display: grid;
    place-items: center;
    width: 20px;
  }

  .status svg {
    stroke: currentColor;
  }

  .text {
    flex: 1;
    min-width: 0;
  }

  .action {
    flex: none;
    height: 26px;
    padding: 0 10px;
    border: 0;
    border-radius: 7px;
    background: rgb(255 255 255 / 0.12);
    color: #fff;
    font: inherit;
    font-size: 12px;
    font-weight: 500;
    cursor: default;
  }

  .action:hover {
    background: rgb(255 255 255 / 0.18);
  }

  @media (prefers-reduced-motion: reduce) {
    .choice:active {
      transform: none;
    }
  }
</style>

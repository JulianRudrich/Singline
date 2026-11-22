<script lang="ts">
  // The lyrics area while Sing to find listens: a white microphone with a
  // level meter, what to do, the words heard so far, and the way out.
  import type { Translate } from "../lib/i18n";
  import type { AppInfo } from "../lib/types";
  import { needsSettings, percent, problemText, type ModelView, type SingView } from "../sing/api";

  interface Props {
    view: SingView;
    /** Microphone level from 0 to 1. */
    level: number;
    model: ModelView | null;
    platform: AppInfo["platform"];
    t: Translate;
    onstop: () => void;
    onagain: () => void;
    onsettings: () => void;
  }

  let { view, level, model, platform, t, onstop, onagain, onsettings }: Props = $props();

  /** Each bar's share of the level: highest in the middle. */
  const WEIGHTS = [0.55, 0.85, 1, 0.85, 0.55];
  const live = $derived(view.phase === "listening" || view.phase === "searching");
  const downloading = $derived(view.phase === "preparing" && model?.state === "downloading");
  const progress = $derived(downloading && model ? percent(model) : 0);

  const message = $derived.by(() => {
    switch (view.phase) {
      case "preparing":
        return downloading ? t("sing.downloading", { percent: progress }) : t("sing.preparing");
      case "searching":
        return t("sing.searching");
      case "notFound":
        return t("sing.notFound");
      case "problem":
        return view.problem ? problemText(view.problem, platform, t) : "";
      default:
        return t("sing.listening");
    }
  });
  const detail = $derived.by(() => {
    if (view.heard && (view.phase === "searching" || view.phase === "notFound")) return t("sing.heard", { words: view.heard });
    if (view.phase === "listening") return t("sing.listeningHint");
    if (downloading) return t("sing.downloadingHint");
    return "";
  });
</script>

<div class="sing reveal" class:quiet={!live} role="status">
  <div class="mark" aria-hidden="true">
    <span class="mic" class:live>
      <svg width="20" height="20" viewBox="0 0 24 24">
        <rect x="9" y="3" width="6" height="11" rx="3" />
        <path d="M5.5 11a6.5 6.5 0 0 0 13 0M12 17.5V21M8.5 21h7" />
      </svg>
    </span>
    {#if live}
      <span class="meter" title={t("sing.level")}>
        {#each WEIGHTS as weight, index (index)}
          <span class="bar" style:height="{4 + 22 * Math.min(1, level * weight * 1.35)}px"></span>
        {/each}
      </span>
    {/if}
  </div>
  <p class="message">{message}</p>
  {#if downloading}
    <span class="progress" aria-hidden="true"><span style:width="{progress}%"></span></span>
  {/if}
  {#if detail}
    <p class="detail">{detail}</p>
  {/if}
  <div class="actions">
    {#if view.phase === "notFound" || view.phase === "problem"}
      {#if view.problem && needsSettings(view.problem)}
        <button onclick={onsettings}>{t("sing.openSettings")}</button>
      {:else}
        <button onclick={onagain}>{t("sing.listenAgain")}</button>
      {/if}
    {/if}
    <button class="quiet-button" onclick={onstop}>{t("sing.stop")}</button>
  </div>
</div>

<style>
  .sing {
    height: 100%;
    box-sizing: border-box;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 8px;
    text-align: center;
  }

  .mark {
    display: flex;
    align-items: center;
    gap: 10px;
    height: 40px;
  }

  .mic {
    position: relative;
    display: grid;
    place-items: center;
    width: 38px;
    height: 38px;
    border-radius: 50%;
    border: 1.5px solid rgb(255 255 255 / 0.35);
    color: #fff;
    transition: border-color var(--duration-pane) var(--ease-out);
  }

  .mic.live {
    border-color: #fff;
  }

  /* While it listens, a soft ring keeps leaving the microphone. */
  .mic.live::after {
    content: "";
    position: absolute;
    inset: -1.5px;
    border-radius: 50%;
    border: 1.5px solid #fff;
    opacity: 0;
    animation: ripple 2.4s var(--ease-out) infinite;
    pointer-events: none;
  }

  .mic svg {
    fill: none;
    stroke: currentColor;
    stroke-width: 2;
    stroke-linecap: round;
    stroke-linejoin: round;
  }

  .meter {
    display: flex;
    align-items: center;
    gap: 3px;
    height: 28px;
  }

  .bar {
    width: 4px;
    border-radius: 2px;
    background: #fff;
    transition: height 90ms linear;
  }

  .message {
    margin: 0;
    font-size: 14px;
    font-weight: 600;
    line-height: 1.35;
    color: #fff;
  }

  .quiet .message {
    font-weight: 500;
    color: rgb(255 255 255 / 0.85);
  }

  .detail {
    margin: 0;
    max-width: 92%;
    font-size: 12px;
    line-height: 1.35;
    color: rgb(255 255 255 / 0.55);
    overflow: hidden;
    display: -webkit-box;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    -webkit-box-orient: vertical;
  }

  .progress {
    width: 60%;
    height: 3px;
    border-radius: 2px;
    background: rgb(255 255 255 / 0.15);
    overflow: hidden;
  }

  .progress span {
    display: block;
    height: 100%;
    background: #fff;
    transition: width 0.25s linear;
  }

  .actions {
    display: flex;
    gap: 8px;
    margin-top: 2px;
  }

  button {
    padding: 4px 11px;
    border: 1px solid rgb(255 255 255 / 0.3);
    border-radius: 7px;
    background: rgb(255 255 255 / 0.08);
    color: #fff;
    font: inherit;
    font-size: 11.5px;
    font-weight: 500;
    cursor: pointer;
    transition:
      background-color 0.15s,
      transform 0.14s cubic-bezier(0.23, 1, 0.32, 1);
  }

  button:hover,
  button:focus-visible {
    background: rgb(255 255 255 / 0.18);
  }

  button:active {
    transform: scale(0.96);
  }

  .quiet-button {
    border-color: transparent;
    background: none;
    color: rgb(255 255 255 / 0.6);
  }

  .quiet-button:hover,
  .quiet-button:focus-visible {
    background: rgb(255 255 255 / 0.08);
    color: #fff;
  }

  @keyframes ripple {
    0% {
      opacity: 0.4;
      transform: scale(1);
    }

    70%,
    100% {
      opacity: 0;
      transform: scale(1.45);
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .mic.live::after {
      animation: none;
    }

    .bar {
      transition: none;
    }
  }
</style>

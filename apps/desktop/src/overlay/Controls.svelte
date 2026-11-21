<script lang="ts">
  // Previous, play or pause, and next for the followed player. Only the
  // buttons the player supports appear. Where they come and go, the place that
  // shows them animates them.
  import { api, type PlayerCommand } from "../lib/api";
  import type { Translate } from "../lib/i18n";
  import type { Capabilities } from "../lib/types";

  let {
    capabilities,
    playing,
    t,
    large = false,
  }: { capabilities: Capabilities; playing: boolean; t: Translate; large?: boolean } = $props();

  function send(command: PlayerCommand) {
    void api.playerCommand(command);
  }
</script>

<div class="controls" class:large>
  {#if capabilities.previous}
    <button class="skip" onclick={() => send({ kind: "previous" })} title={t("player.previous")} aria-label={t("player.previous")}>
      <svg width="13" height="13" viewBox="0 0 24 24" aria-hidden="true">
        <path d="M11 6.5v11L3 12zM20.5 6.5v11l-8-5.5z" />
      </svg>
    </button>
  {/if}
  {#if capabilities.playPause}
    {@const label = playing ? t("player.pause") : t("player.play")}
    <button class="main" onclick={() => send({ kind: "playPause" })} title={label} aria-label={label}>
      <svg width="16" height="16" viewBox="0 0 24 24" aria-hidden="true">
        {#if playing}
          <path d="M6.5 4.5h3.2v15H6.5zM14.3 4.5h3.2v15h-3.2z" />
        {:else}
          <path d="M7 4.5v15l12.5-7.5z" />
        {/if}
      </svg>
    </button>
  {/if}
  {#if capabilities.next}
    <button class="skip" onclick={() => send({ kind: "next" })} title={t("player.next")} aria-label={t("player.next")}>
      <svg width="13" height="13" viewBox="0 0 24 24" aria-hidden="true">
        <path d="M3.5 6.5v11l8-5.5zM13 6.5v11l8-5.5z" />
      </svg>
    </button>
  {/if}
</div>

<style>
  .controls {
    flex: none;
    display: flex;
    align-items: center;
    gap: 14px;
  }

  button {
    display: grid;
    place-items: center;
    border: 0;
    padding: 0;
    background: none;
    color: rgb(255 255 255 / 0.6);
    cursor: pointer;
    transition:
      color 0.15s,
      transform 0.14s cubic-bezier(0.23, 1, 0.32, 1);
  }

  .skip {
    width: 18px;
    height: 18px;
  }

  .main {
    width: 24px;
    height: 24px;
  }

  button:hover,
  button:focus-visible {
    color: #fff;
  }

  button:active {
    transform: scale(0.86);
  }

  svg {
    fill: currentColor;
    stroke: currentColor;
    stroke-width: 1.6;
    stroke-linejoin: round;
  }

  /* For the karaoke bar. */
  .large {
    gap: 26px;
  }

  .large .skip {
    width: 32px;
    height: 32px;
  }

  .large .main {
    width: 44px;
    height: 44px;
  }

  .large .skip svg {
    width: 22px;
    height: 22px;
  }

  .large .main svg {
    width: 30px;
    height: 30px;
  }

  @media (prefers-reduced-motion: reduce) {
    button {
      transition: color 0.15s;
    }
  }
</style>

<script lang="ts">
  // The song Sing to find recognised, with a way to say it is the wrong one
  // and to listen again. It shows for a moment after the song is found and
  // whenever the pointer is over the overlay.
  import { prefersReducedMotion } from "svelte/motion";
  import { slide } from "svelte/transition";
  import type { Translate } from "../lib/i18n";
  import Icon from "../lib/Icon.svelte";

  interface Props {
    title: string;
    artist: string;
    /** Another candidate is there to switch to. */
    alternatives: boolean;
    t: Translate;
    onnext: () => void;
    onagain: () => void;
  }

  let { title, artist, alternatives, t, onnext, onagain }: Props = $props();
</script>

<div class="found" transition:slide={{ duration: prefersReducedMotion.current ? 0 : 200 }} data-no-drag>
  <span class="mic" aria-hidden="true">
    <svg width="12" height="12" viewBox="0 0 24 24">
      <rect x="9" y="3" width="6" height="11" rx="3" />
      <path d="M5.5 11a6.5 6.5 0 0 0 13 0M12 17.5V21" />
    </svg>
  </span>
  <span class="text" title={t("sing.found", { title, artist })}>{t("sing.found", { title, artist })}</span>
  {#if alternatives}
    <button class="next" onclick={onnext}>{t("sing.notThisSong")}</button>
  {/if}
  <button class="again" onclick={onagain} title={t("sing.listenAgain")} aria-label={t("sing.listenAgain")}>
    <Icon name="retry" size={13} />
  </button>
</div>

<style>
  .found {
    flex: none;
    display: flex;
    align-items: center;
    gap: 8px;
    min-width: 0;
    font-size: 11.5px;
    color: rgb(255 255 255 / 0.75);
  }

  .mic {
    flex: none;
    display: grid;
    place-items: center;
    width: 20px;
    height: 20px;
    border-radius: 50%;
    background: #fff;
    color: #000;
  }

  .mic svg {
    fill: none;
    stroke: currentColor;
    stroke-width: 2.4;
    stroke-linecap: round;
  }

  .text {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
  }

  button {
    flex: none;
    border: 0;
    background: none;
    color: rgb(255 255 255 / 0.6);
    font: inherit;
    cursor: pointer;
    transition:
      color var(--duration-press) var(--ease-out),
      background-color var(--duration-press) var(--ease-out),
      transform var(--duration-press) var(--ease-out);
  }

  .next {
    padding: 3px 9px;
    border: 1px solid rgb(255 255 255 / 0.3);
    border-radius: 6px;
    color: #fff;
    font-weight: 500;
  }

  .again {
    display: grid;
    place-items: center;
    width: 22px;
    height: 22px;
    padding: 0;
    border-radius: 50%;
  }

  button:hover,
  button:focus-visible {
    color: #fff;
    background-color: rgb(255 255 255 / 0.1);
  }

  button:active {
    transform: scale(var(--press-scale));
  }

  .again:active {
    transform: scale(var(--press-scale-icon));
  }

  button:focus-visible {
    outline: 2px solid rgb(255 255 255 / 0.7);
    outline-offset: 2px;
  }
</style>

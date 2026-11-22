<script lang="ts">
  // What the lyrics area says when there are no lyrics to show. Without lyrics
  // or for an instrumental, the three-bar mark stands above the text.
  import type { Translate } from "../lib/i18n";
  import Icon from "../lib/Icon.svelte";
  import BarsMark from "./BarsMark.svelte";

  interface Props {
    text: string;
    /** Show the mark: no lyrics were found, or the song is instrumental. */
    mark: boolean;
    playing: boolean;
    /** Seconds before the text appears; a quick search never shows. */
    delay: number;
    onretry: (() => void) | null;
    t: Translate;
  }

  let { text, mark, playing, delay, onretry, t }: Props = $props();
</script>

<div class="status reveal" class:mark style:animation-delay="{delay}s">
  {#if mark}
    <BarsMark {playing} />
  {/if}
  <div class="message">
    <p>{text}</p>
    {#if onretry}
      <button class="retry" onclick={onretry} title={t("overlay.retry")} aria-label={t("overlay.retry")}>
        <Icon name="retry" size={15} />
      </button>
    {/if}
  </div>
</div>

<style>
  .status {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 12px;
  }

  .status.mark {
    height: 100%;
    align-items: center;
    justify-content: center;
    text-align: center;
  }

  .message {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  p {
    margin: 0;
    font-size: 13px;
    line-height: 1.35;
    color: rgb(255 255 255 / 0.6);
  }

  .retry {
    flex: none;
    display: grid;
    place-items: center;
    width: 24px;
    height: 24px;
    border: 0;
    padding: 0;
    border-radius: 6px;
    background: none;
    color: rgb(255 255 255 / 0.6);
    cursor: pointer;
    transition:
      color var(--duration-press) var(--ease-out),
      transform var(--duration-press) var(--ease-out);
  }

  .retry:hover,
  .retry:focus-visible {
    color: #fff;
  }

  .retry:active {
    transform: scale(var(--press-scale-icon));
  }

  .retry:focus-visible {
    outline: 2px solid rgb(255 255 255 / 0.7);
    outline-offset: 2px;
  }
</style>

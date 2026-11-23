<script lang="ts">
  // The cover art, which opens the song in Spotify when `onopen` is given: an
  // arrow shows on it under the pointer and with the keyboard focus. Without
  // `onopen` it is the plain cover.
  import Glyph from "../lib/ui/Glyph.svelte";
  import Cover from "./Cover.svelte";

  let {
    url,
    size = 36,
    label,
    onopen,
  }: { url: string | null; size?: number; label: string; onopen?: () => void } = $props();
</script>

{#if onopen}
  <button
    class="link"
    style:border-radius="{Math.max(6, size * 0.12)}px"
    onclick={onopen}
    title={label}
    aria-label={label}
  >
    <Cover {url} {size} />
    <span class="arrow"><Glyph name="external" size={Math.max(14, Math.round(size * 0.42))} /></span>
  </button>
{:else}
  <Cover {url} {size} />
{/if}

<style>
  .link {
    all: unset;
    position: relative;
    display: block;
    flex: none;
    cursor: pointer;
    transition: transform var(--duration-press) var(--ease-out);
  }

  .link:active {
    transform: scale(var(--press-scale-icon));
  }

  .link:focus-visible {
    outline: 2px solid rgb(255 255 255 / 0.85);
    outline-offset: 2px;
  }

  /* The arrow darkens the picture a little, so it reads on any cover. */
  .arrow {
    position: absolute;
    inset: 0;
    display: grid;
    place-items: center;
    border-radius: inherit;
    background: rgb(0 0 0 / 0.5);
    color: #fff;
    opacity: 0;
    transition: opacity var(--duration-press) var(--ease-out);
  }

  .link:hover .arrow,
  .link:focus-visible .arrow {
    opacity: 1;
  }

  /* Without a picture, the note makes way for the arrow. */
  .link :global(.note) {
    transition: opacity var(--duration-press) var(--ease-out);
  }

  .link:hover :global(.note),
  .link:focus-visible :global(.note) {
    opacity: 0;
  }
</style>

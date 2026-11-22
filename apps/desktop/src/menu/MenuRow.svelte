<script lang="ts">
  import type { Snippet } from "svelte";
  import Glyph, { type GlyphName } from "../lib/ui/Glyph.svelte";

  // One row of the tray panel, like a menu item: icon and title, and on the
  // right a detail, a shortcut, a chevron or a control. Hovering lights it up.
  let {
    icon,
    title,
    detail,
    shortcut,
    expanded,
    onclick,
    children,
  }: {
    icon: GlyphName;
    title: string;
    detail?: string;
    shortcut?: string;
    /** For a row that opens a section below it. */
    expanded?: boolean;
    onclick: () => void;
    /** A control, such as a switch, that the whole row also works. */
    children?: Snippet;
  } = $props();

  function clickAround(event: MouseEvent) {
    // The control handles its own clicks.
    if (!(event.target as HTMLElement).closest("button")) onclick();
  }
</script>

{#snippet label()}
  <span class="icon"><Glyph name={icon} size={16} /></span>
  <span class="title">{title}</span>
  {#if detail}<span class="detail">{detail}</span>{/if}
  {#if shortcut}<span class="shortcut">{shortcut}</span>{/if}
{/snippet}

{#if children}
  <!-- The control inside is what the keyboard reaches; the row only widens where a click works. -->
  <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
  <div class="row" onclick={clickAround}>
    {@render label()}
    {@render children()}
  </div>
{:else}
  <button type="button" class="row" aria-expanded={expanded} {onclick}>
    {@render label()}
    {#if expanded !== undefined}
      <span class="chevron" class:open={expanded}><Glyph name="chevronRight" size={12} /></span>
    {/if}
  </button>
{/if}

<style>
  .row {
    box-sizing: border-box;
    width: 100%;
    min-height: 34px;
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 0 8px;
    border: 0;
    border-radius: var(--row-radius, 10px);
    background: transparent;
    color: #fff;
    font: inherit;
    font-size: 13px;
    text-align: left;
    cursor: default;
    transition:
      background-color var(--duration-press) var(--ease-out),
      transform var(--duration-press) var(--ease-out);
  }

  .row:hover {
    background: rgb(255 255 255 / 0.08);
  }

  .row:active {
    transform: scale(var(--press-scale-subtle));
  }

  .row:focus-visible {
    outline: 2px solid rgb(255 255 255 / 0.7);
    outline-offset: -2px;
  }

  .icon {
    flex: none;
    display: grid;
    place-items: center;
    width: 18px;
    color: rgb(255 255 255 / 0.7);
  }

  .title {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
  }

  .detail,
  .shortcut {
    flex: none;
    font-size: 12px;
    font-variant-numeric: tabular-nums;
  }

  .detail {
    color: rgb(255 255 255 / 0.6);
  }

  .shortcut {
    color: rgb(255 255 255 / 0.5);
  }

  .chevron {
    flex: none;
    display: grid;
    color: rgb(255 255 255 / 0.5);
    transition: transform var(--duration-pane) var(--ease-out);
  }

  .chevron.open {
    transform: rotate(90deg);
  }
</style>

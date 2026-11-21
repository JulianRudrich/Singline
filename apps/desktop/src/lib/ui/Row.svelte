<script lang="ts">
  import type { Snippet } from "svelte";
  import Card from "./Card.svelte";
  import Glyph, { type GlyphName } from "./Glyph.svelte";

  // One setting: icon, title and hint, with its control on the right.
  // It sits in its own card unless `card` is false.
  let {
    icon,
    title,
    hint,
    warning = false,
    card = true,
    children,
  }: { icon: GlyphName; title: string; hint?: string; warning?: boolean; card?: boolean; children?: Snippet } = $props();
</script>

{#snippet row()}
  <div class="row">
    <span class="icon"><Glyph name={icon} size={17} /></span>
    <span class="text">
      <span class="title">{title}</span>
      {#if hint}<span class="hint" class:warning>{hint}</span>{/if}
    </span>
    {#if children}<span class="control">{@render children()}</span>{/if}
  </div>
{/snippet}

{#if card}
  <Card>{@render row()}</Card>
{:else}
  {@render row()}
{/if}

<style>
  .row {
    display: flex;
    align-items: center;
    gap: 14px;
  }

  .icon {
    flex: none;
    display: grid;
    place-items: center;
    width: 26px;
    color: rgb(255 255 255 / 0.85);
  }

  .text {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .title {
    font-size: 13px;
    font-weight: 500;
  }

  .hint {
    font-size: 11.5px;
    color: rgb(255 255 255 / 0.55);
  }

  /* No colour for warnings: they stand out by brightness. */
  .hint.warning {
    color: rgb(255 255 255 / 0.92);
  }

  .control {
    flex: none;
    display: flex;
    align-items: center;
    gap: 8px;
    margin-left: 12px;
  }
</style>

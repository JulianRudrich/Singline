<script lang="ts">
  import type { Snippet } from "svelte";

  // A group of settings on a lightly frosted panel. With a label it is a named group.
  let { label, children }: { label?: string; children: Snippet } = $props();
</script>

<div class="card" role={label ? "group" : undefined} aria-label={label}>{@render children()}</div>

<style>
  .card {
    display: flex;
    flex-direction: column;
    gap: 14px;
    padding: 15px;
    border: 1px solid rgb(255 255 255 / 0.08);
    border-radius: 12px;
    background: rgb(255 255 255 / 0.05);
  }
</style>

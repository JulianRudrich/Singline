<script lang="ts" generics="T extends string">
  import { glide } from "./glide";

  // Mutually exclusive options; the selection glides like the one in the sidebar.
  // Compact ones size each option to its label, for a row of ranges.
  let {
    value,
    options,
    label,
    compact = false,
    onchange,
  }: {
    value: T;
    options: { value: T; label: string }[];
    label: string;
    compact?: boolean;
    onchange: (value: T) => void;
  } = $props();

  const index = $derived(options.findIndex((option) => option.value === value));
  let group = $state<HTMLElement>();

  function keydown(event: KeyboardEvent) {
    const step = event.key === "ArrowRight" ? 1 : event.key === "ArrowLeft" ? -1 : 0;
    const next = options[(index + step + options.length) % options.length];
    if (!step || !next) return;
    event.preventDefault();
    onchange(next.value);
    group?.querySelectorAll<HTMLElement>("[data-item]")[options.indexOf(next)]?.focus();
  }
</script>

<div class="segmented" class:compact role="radiogroup" aria-label={label} bind:this={group} use:glide={index}>
  <span class="pill" data-pill aria-hidden="true"></span>
  {#each options as option (option.value)}
    {@const selected = option.value === value}
    <button
      type="button"
      role="radio"
      data-item
      aria-checked={selected}
      tabindex={selected ? 0 : -1}
      class:selected
      onclick={() => onchange(option.value)}
      onkeydown={keydown}
    >
      {option.label}
    </button>
  {/each}
</div>

<style>
  .segmented {
    position: relative;
    display: flex;
    gap: 2px;
    padding: 2px;
    border-radius: 9px;
    background: rgb(0 0 0 / 0.2);
    box-shadow: inset 0 0 0 1px rgb(255 255 255 / 0.06);
  }

  .pill {
    position: absolute;
    top: 0;
    left: 0;
    border-radius: 7px;
    background: rgb(255 255 255 / 0.14);
    box-shadow:
      0 1px 2px rgb(0 0 0 / 0.25),
      inset 0 0 0 0.5px rgb(255 255 255 / 0.1);
    pointer-events: none;
    transition:
      transform var(--duration-pane) var(--ease-out),
      width var(--duration-pane) var(--ease-out);
  }

  button {
    position: relative;
    flex: 1 1 0;
    min-width: 0;
    height: 26px;
    padding: 0 10px;
    border: 0;
    border-radius: 7px;
    background: none;
    color: rgb(255 255 255 / 0.75);
    font: inherit;
    font-size: 13px;
    font-weight: 500;
    white-space: nowrap;
    cursor: default;
    transition:
      color var(--duration-pane) var(--ease-out),
      transform var(--duration-press) var(--ease-out);
  }

  button.selected {
    color: #fff;
  }

  button:hover:not(.selected) {
    color: #fff;
  }

  .compact button {
    flex: none;
    height: 24px;
    padding: 0 11px;
    font-size: 12.5px;
  }

  button:active {
    transform: scale(var(--press-scale-subtle));
  }

  button:focus-visible {
    outline: 2px solid rgb(255 255 255 / 0.7);
    outline-offset: -2px;
  }

  @media (prefers-reduced-motion: reduce) {
    .pill {
      transition: none;
    }
  }
</style>

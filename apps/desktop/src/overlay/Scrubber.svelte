<script lang="ts">
  // The progress line under the lyrics. While the pointer is over the overlay
  // it grows into a scrubber with the elapsed and remaining time, which open
  // from its ends: click or drag to seek, click a dot to jump to a chorus. It
  // keeps its place in the layout, so the lyrics never move, and the line moves
  // on with a transform, so playing costs no layout.
  import { prefersReducedMotion } from "svelte/motion";
  import { fade } from "svelte/transition";
  import type { Translate } from "../lib/i18n";
  import { clamp } from "../lib/lyrics";
  import { formatRemaining, formatTime } from "../lib/player";

  interface Props {
    /** Seconds into the track. */
    position: number;
    /** Length of the track in seconds, more than zero. */
    duration: number;
    /** The pointer is over the overlay. */
    expanded: boolean;
    /** The player can seek. */
    seekable: boolean;
    /** Player positions where a chorus starts. */
    marks: readonly number[];
    t: Translate;
    onseek: (position: number) => void;
    /** Reports a drag starting and ending, so the overlay stays open meanwhile. */
    onscrub: (held: boolean) => void;
    /** Larger, for the karaoke bar. */
    large?: boolean;
  }

  let { position, duration, expanded, seekable, marks, t, onseek, onscrub, large = false }: Props = $props();

  /** Seconds one arrow key moves. */
  const STEP = 5;
  const STEPS: Record<string, number> = { ArrowLeft: -STEP, ArrowDown: -STEP, ArrowRight: STEP, ArrowUp: STEP };

  let rail = $state<HTMLElement>();
  /** The natural widths of the times, which they open to. */
  let startWidth = $state(0);
  let endWidth = $state(0);
  /** Where the pointer holds the line while dragging, as a share of the track. */
  let held = $state<number | null>(null);

  const interactive = $derived(expanded && seekable);
  const shown = $derived(held === null ? clamp(position, 0, duration) : held * duration);
  const fraction = $derived(clamp(shown / duration, 0, 1));
  const fadeMotion = $derived({ duration: prefersReducedMotion.current ? 0 : 150 });

  function fractionAt(event: PointerEvent): number {
    const box = rail?.getBoundingClientRect();
    if (!box || box.width <= 0) return fraction;
    return clamp((event.clientX - box.left) / box.width, 0, 1);
  }

  function press(event: PointerEvent) {
    if (!interactive || event.button !== 0) return;
    (event.currentTarget as HTMLElement).setPointerCapture(event.pointerId);
    held = fractionAt(event);
    onscrub(true);
  }

  function move(event: PointerEvent) {
    if (held !== null) held = fractionAt(event);
  }

  // One seek when the pointer lets go, wherever it is by then.
  function release(event: PointerEvent) {
    if (held === null) return;
    const target = fractionAt(event) * duration;
    held = null;
    onscrub(false);
    onseek(target);
  }

  function cancel() {
    if (held === null) return;
    held = null;
    onscrub(false);
  }

  function key(event: KeyboardEvent) {
    const step = STEPS[event.key];
    if (event.key === "Home") onseek(0);
    else if (step !== undefined) onseek(clamp(shown + step, 0, duration));
    else return;
    event.preventDefault();
  }
</script>

<div class="scrubber" class:large class:expanded class:interactive class:held={held !== null} data-no-drag={interactive ? "" : undefined}>
  <!-- The slider tells assistive technology the times; these are for the eye. -->
  <span class="time start" aria-hidden="true" style:--open={startWidth > 0 ? `${startWidth}px` : null}>
    <span bind:clientWidth={startWidth}>{formatTime(shown)}</span>
  </span>
  <div class="rail" bind:this={rail}>
    <div
      class="bar"
      role="slider"
      tabindex={interactive ? 0 : -1}
      aria-hidden={interactive ? undefined : "true"}
      aria-label={t("player.position")}
      aria-valuemin={0}
      aria-valuemax={Math.floor(duration)}
      aria-valuenow={Math.floor(shown)}
      aria-valuetext={t("player.positionValue", { elapsed: formatTime(shown), duration: formatTime(duration) })}
      onpointerdown={press}
      onpointermove={move}
      onpointerup={release}
      onpointercancel={cancel}
      onlostpointercapture={cancel}
      onkeydown={key}
    >
      <div class="track"><div class="fill" style:--fraction={fraction.toFixed(5)}></div></div>
    </div>
    {#if interactive}
      {#each marks as mark, index (index)}
        {@const label = t("player.chorus", { time: formatTime(mark) })}
        <button
          class="mark"
          style:left="{clamp(mark / duration, 0, 1) * 100}%"
          title={label}
          aria-label={label}
          onclick={() => onseek(mark)}
          transition:fade={fadeMotion}
        ></button>
      {/each}
    {/if}
  </div>
  <span class="time end" aria-hidden="true" style:--open={endWidth > 0 ? `${endWidth}px` : null}>
    <span bind:clientWidth={endWidth}>−{formatRemaining(shown, duration)}</span>
  </span>
</div>

<style>
  /* Sits close to the faded bottom edge of the lyrics; its height never changes. */
  .scrubber {
    --gap: 8px;
    flex: none;
    height: 14px;
    margin-top: -4px;
    display: flex;
    align-items: center;
  }

  /* The times open from nothing, so the line shortens smoothly instead of jumping. */
  .time {
    flex: none;
    max-width: 0;
    overflow: hidden;
    opacity: 0;
    font-size: 10px;
    font-weight: 500;
    line-height: 1;
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
    color: rgb(255 255 255 / 0.62);
    transition:
      max-width var(--duration-dismiss) var(--ease-out),
      opacity var(--duration-dismiss) var(--ease-out);
  }

  .time > span {
    display: block;
    width: max-content;
  }

  .start > span {
    padding-right: var(--gap);
  }

  .end > span {
    padding-left: var(--gap);
  }

  .expanded .time {
    max-width: var(--open, 5em);
    opacity: 1;
    transition:
      max-width var(--duration-pane) var(--ease-out),
      opacity var(--duration-pane) var(--ease-out);
  }

  /* The whole height of the row, so the thin line is easy to hit. */
  .rail {
    flex: 1;
    position: relative;
    height: 100%;
    pointer-events: none;
  }

  .interactive .rail {
    pointer-events: auto;
  }

  .bar {
    position: absolute;
    inset: 0;
    display: flex;
    align-items: center;
    outline: none;
  }

  .interactive .bar {
    cursor: pointer;
  }

  .track {
    --nub: 2px;
    flex: 1;
    height: 2px;
    border-radius: 1px;
    background: rgb(255 255 255 / 0.1);
    overflow: hidden;
    transition:
      height var(--duration-pane) var(--ease-out),
      background-color var(--duration-pane) var(--ease-out);
  }

  .expanded .track,
  .held .track {
    --nub: 4px;
    height: 4px;
    border-radius: 2px;
    background: rgb(255 255 255 / 0.16);
  }

  .bar:focus-visible .track {
    box-shadow: 0 0 0 2px rgb(255 255 255 / 0.45);
  }

  /* The full width, slid left by what is still to come: at least a round nub shows. */
  .fill {
    width: 100%;
    height: 100%;
    border-radius: inherit;
    background: rgb(255 255 255 / 0.35);
    transform: translateX(calc((var(--fraction) - 1) * (100% - var(--nub))));
    transition: background-color var(--duration-pane) var(--ease-out);
  }

  .expanded .fill,
  .held .fill {
    background: rgb(255 255 255 / 0.85);
  }

  .mark {
    position: absolute;
    top: 50%;
    display: grid;
    place-items: center;
    width: 14px;
    height: 14px;
    margin: -7px 0 0 -7px;
    border: 0;
    padding: 0;
    border-radius: 50%;
    background: none;
    cursor: pointer;
  }

  .mark::before {
    content: "";
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: #fff;
    box-shadow: inset 0 0 0 1px rgb(0 0 0 / 0.25);
    transition: transform 0.14s cubic-bezier(0.23, 1, 0.32, 1);
  }

  .mark:hover::before,
  .mark:focus-visible::before {
    transform: scale(1.25);
  }

  .mark:active::before {
    transform: scale(0.8);
  }

  /* The karaoke bar is used from further away. */
  .large {
    --gap: 14px;
    height: 24px;
    margin-top: 0;
  }

  .large .time {
    font-size: 13px;
  }

  .large.expanded .track,
  .large.held .track {
    --nub: 6px;
    height: 6px;
    border-radius: 3px;
  }

  .large .mark {
    width: 20px;
    height: 20px;
    margin: -10px 0 0 -10px;
  }

  .large .mark::before {
    width: 10px;
    height: 10px;
  }

  @media (prefers-reduced-motion: reduce) {
    .track,
    .fill,
    .mark::before {
      transition: none;
    }

    .time,
    .expanded .time {
      transition: opacity var(--duration-pane) var(--ease-out);
    }
  }
</style>

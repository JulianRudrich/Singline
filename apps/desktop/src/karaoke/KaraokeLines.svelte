<script lang="ts">
  // The karaoke lyrics: the current line large and centred, lighting up word
  // by word, the next ones below it, smaller and dimmer. On a line change the
  // rows move on springs, as in the overlay: the line that is done moves up and
  // fades out, the next one grows into its place and a new one comes in from
  // below. Waits show three large dots that count down to the next line, which
  // brightens meanwhile.
  import { onDestroy, untrack } from "svelte";
  import { prefersReducedMotion } from "svelte/motion";
  import type { Translate } from "../lib/i18n";
  import { karaokeSlots, UPCOMING_OPACITY, type Scene } from "../lib/karaoke";
  import { lineFills, wordOpacity, wordWipe } from "../lib/lyrics";
  import type { Measure } from "../lib/measure";
  import { RowMotion, type MovingRow } from "../lib/rows";
  import type { LyricLine, SyncLevel } from "../lib/types";
  import BreakDots from "../overlay/BreakDots.svelte";

  interface Props {
    lines: readonly LyricLine[];
    scene: Scene;
    /** Seconds into the lyrics, with the timing corrections applied. */
    position: number;
    sync: SyncLevel;
    fontSize: number;
    width: number;
    height: number;
    /** How many lines show below the current one. */
    upcoming: number;
    playing: boolean;
    measure: Measure;
    t: Translate;
  }

  let { lines, scene, position, sync, fontSize, width, height, upcoming, playing, measure, t }: Props = $props();

  const current = $derived(scene.current);
  const slots = $derived(karaokeSlots(lines, current, { width, height, fontSize, upcoming, measure }));

  // Every row moves to its place on springs; rows keep them while they stay,
  // and a row that goes keeps moving until it has faded out.
  const motion = new RowMotion<{ place: number }>();
  /** The rows there when the lyrics appear; they come in one after another. */
  const firstRows = new Set(untrack(() => slots.map((slot) => slot.index)));
  const still = $derived(prefersReducedMotion.current);
  /** What the template draws: a fresh copy whenever a row starts leaving or goes. */
  interface Shown {
    index: number;
    /** -1 above, 0 for the current line, 1 and 2 below; -2 for a row on its way out. */
    place: number;
    motion: MovingRow<{ place: number }>;
  }
  let rows = $state.raw<Shown[]>([]);
  let frame = $state(0);
  let animation = 0;
  let last = 0;
  let lastCurrent: number | null = null;

  $effect(() => {
    const moved = lastCurrent === null ? 0 : current - lastCurrent;
    motion.update(
      slots.map((slot) => ({ ...slot, data: { place: slot.place } })),
      { moved, still, appear: lastCurrent === null },
    );
    lastCurrent = current;
    show();
    if (animation === 0) {
      last = performance.now();
      animation = requestAnimationFrame(step);
    }
  });

  function step(now: number) {
    const dt = Math.min(0.1, Math.max(0, now - last) / 1000);
    last = now;
    const moving = motion.step(dt);
    // Rows that faded out are gone.
    if (motion.size !== rows.length) show();
    frame += 1;
    animation = moving ? requestAnimationFrame(step) : 0;
  }

  function show() {
    rows = motion.list().map((row) => ({ index: row.index, place: row.leaving ? -2 : row.data.place, motion: row }));
  }

  onDestroy(() => cancelAnimationFrame(animation));

  /** Where a row is now. `_frame` makes it follow the animation. */
  function placed(row: MovingRow<{ place: number }>, _frame: number) {
    return { y: row.y.value, scale: row.scale.value, opacity: row.opacity.value };
  }

  const activeLine = $derived(lines[current]);
  const fills = $derived(
    activeLine && scene.dots === null ? lineFills(activeLine, lines[current + 1]?.time ?? null, position, sync) : [],
  );
  const spoken = $derived(
    scene.dots !== null ? (current < 0 ? t("overlay.intro") : t("overlay.instrumentalBreak")) : (activeLine?.text ?? ""),
  );
</script>

<div class="lines" style:font-size="{fontSize}px">
  <p class="spoken" aria-live="polite">{spoken}</p>
  {#each rows as row (row.index)}
    {@const place = placed(row.motion, frame)}
    {@const line = lines[row.index]}
    {@const opacity = Math.min(1, place.opacity * (1 + (row.place === 1 ? 0.8 * scene.countdown : 0)))}
    <!-- The next line brightens while the dots count down to it. The width
         follows the target size: it only keeps long lines whole. -->
    <div
      class="row"
      style:width="{width / Math.max(0.05, row.motion.scale.target)}px"
      style:transform="translate(-50%, {place.y.toFixed(2)}px) scale({place.scale.toFixed(4)})"
      style:opacity={opacity.toFixed(3)}
    >
      <div
        class="line"
        class:reveal={firstRows.has(row.index)}
        style:animation-delay={firstRows.has(row.index) ? `${0.12 + 0.07 * Math.max(0, row.place)}s` : null}
        aria-hidden="true"
      >
        {#if row.place === 0 && scene.dots !== null}
          <BreakDots progress={scene.dots} size={fontSize} breathing={playing} centered />
        {:else if line && line.text.length === 0}
          <BreakDots progress={0} size={fontSize} breathing={false} centered />
        {:else if line && row.place === 0 && fills.length > 0}
          <!-- The words start out as bright as the line was while it waited below. -->
          {#each fills as word, index (index)}{@const wipe = wordWipe(word.fill, wordOpacity(0, opacity, UPCOMING_OPACITY[0]))}<span class:wipe={wipe !== null} style:--wipe={wipe} style:opacity={wipe === null ? wordOpacity(word.fill, opacity, UPCOMING_OPACITY[0]).toFixed(3) : null}>{word.text}</span>{/each}
        {:else if line}
          {line.text}
        {/if}
      </div>
    </div>
  {/each}
</div>

<style>
  .lines {
    position: relative;
    width: 100%;
    height: 100%;
    overflow: hidden;
    font-weight: 600;
    line-height: 1.25;
  }

  /* Centred on the stage: half its own width to the left, scaled from its top middle. */
  .row {
    position: absolute;
    top: 0;
    left: 50%;
    height: 1.25em;
    transform-origin: 50% 0;
  }

  .line {
    height: 100%;
    overflow: hidden;
    white-space: pre;
    text-overflow: ellipsis;
    text-align: center;
    color: #fff;
  }

  .spoken {
    position: absolute;
    width: 1px;
    height: 1px;
    margin: 0;
    overflow: hidden;
    clip-path: inset(50%);
    white-space: nowrap;
  }
</style>

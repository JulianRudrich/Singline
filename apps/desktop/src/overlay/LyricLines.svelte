<script lang="ts">
  // The lyrics: rows around the active line, each keyed by its line, so on a
  // line change the rows scroll up and the next line grows into the active
  // place, while the line that is done moves on out and a new one comes in
  // from below. Neighbours are the same text, smaller and fainter; long lines
  // shrink to fit. A press on a line jumps there, unless it moves: then it
  // drags the window.
  import { onDestroy, untrack } from "svelte";
  import { prefersReducedMotion } from "svelte/motion";
  import type { Translate } from "../lib/i18n";
  import { clamp, lineFills, lineText, wordOpacity, wordWipe } from "../lib/lyrics";
  import type { Measure } from "../lib/measure";
  import { RowMotion, type MovingRow } from "../lib/rows";
  import type { Layout, LyricLine, SyncLevel } from "../lib/types";
  import BreakDots from "./BreakDots.svelte";

  interface Props {
    lines: readonly LyricLine[];
    /** Index of the active line, -1 before the first. */
    current: number;
    /** Seconds into the lyrics, with the timing corrections applied. */
    position: number;
    sync: SyncLevel;
    layout: Layout;
    fontSize: number;
    width: number;
    height: number;
    playing: boolean;
    measure: Measure;
    /** Jumps the song to a line; null when that is not possible. */
    onjump: ((line: LyricLine) => void) | null;
    /** Starts moving the window. */
    onwindowdrag: () => void;
    t: Translate;
  }

  let { lines, current, position, sync, layout, fontSize, width, height, playing, measure, onjump, onwindowdrag, t }: Props =
    $props();

  /** Neighbouring lines are the active line's text at this size. */
  const NEIGHBOUR = 0.78;
  /** How bright the lines next to the active one are, and the ones beyond. */
  const NEAR = 0.42;
  const FAR = 0.14;
  /** Line height as a share of the font size. */
  const LINE_HEIGHT = 1.19;
  const GAP = 4;
  /** Extra room above and below the active line. */
  const ACTIVE_PADDING = 4;
  /** Pixels a press may move before it drags the window instead of jumping. */
  const DRAG_DISTANCE = 4;

  interface Slot {
    index: number;
    /** Place from the top, for the staggered entrance. */
    order: number;
    active: boolean;
    y: number;
    scale: number;
    opacity: number;
    blur: number;
    /** From this row's top to the next one's: how far it travels coming in or going. */
    pitch: number;
  }

  const reach = $derived(layout === "context" ? 2 : layout === "focus" ? 1 : 0);

  const slots = $derived.by((): Slot[] => {
    const lineHeight = fontSize * LINE_HEIGHT;
    const rows = [];
    for (let index = current - reach; index <= current + reach; index += 1) {
      const distance = Math.abs(index - current);
      const line = lines[index];
      const active = distance === 0;
      const fit = line ? Math.min(1, (width * 0.97) / Math.max(1, measure(lineText(line)))) : 1;
      const scale = Math.min(active ? 1 : NEIGHBOUR, fit);
      rows.push({
        index,
        active,
        scale,
        opacity: !line && !active ? 0 : active ? 1 : distance === 1 ? NEAR : FAR,
        blur: distance >= 2 ? 1.2 : 0,
        extent: lineHeight * scale + (active ? 2 * ACTIVE_PADDING : 0),
      });
    }
    const total = rows.reduce((sum, row) => sum + row.extent, 0) + GAP * (rows.length - 1);
    let top = (height - total) / 2;
    return rows.map(({ extent, ...row }, order) => {
      const y = top + (row.active ? ACTIVE_PADDING : 0);
      top += extent + GAP;
      return { ...row, order, y, pitch: extent + GAP };
    });
  });

  // Every row moves to its place on springs; rows keep them while they stay,
  // and a row that goes keeps moving until it has faded out.
  interface Look {
    active: boolean;
    order: number;
  }
  const motion = new RowMotion<Look>();
  /** The rows there when the lyrics appear; they come in one after another. */
  const firstRows = new Set(untrack(() => slots.map((slot) => slot.index)));
  const still = $derived(prefersReducedMotion.current);
  /** What the template draws: a fresh copy whenever a row starts leaving or goes. */
  interface Shown {
    index: number;
    active: boolean;
    leaving: boolean;
    order: number;
    motion: MovingRow<Look>;
  }
  let rows = $state.raw<Shown[]>([]);
  let frame = $state(0);
  let animation = 0;
  let last = 0;
  let lastCurrent: number | null = null;

  $effect(() => {
    const moved = lastCurrent === null ? 0 : current - lastCurrent;
    motion.update(
      slots.map((slot) => ({ ...slot, data: { active: slot.active, order: slot.order } })),
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
    rows = motion.list().map((row) => ({
      index: row.index,
      active: row.data.active && !row.leaving,
      leaving: row.leaving,
      order: row.data.order,
      motion: row,
    }));
  }

  onDestroy(() => cancelAnimationFrame(animation));

  /** Where a row is now. `_frame` makes it follow the animation. */
  function placed(row: MovingRow<Look>, _frame: number) {
    return { y: row.y.value, scale: row.scale.value, opacity: row.opacity.value, blur: row.blur.value };
  }

  const activeLine = $derived(lines[current]);
  const fills = $derived(activeLine ? lineFills(activeLine, lines[current + 1]?.time ?? null, position, sync) : []);

  /** Progress through the intro or an instrumental break, or null while a line is sung. */
  const waiting = $derived.by((): number | null => {
    if (current < 0) {
      const first = lines[0];
      if (!first) return null;
      return first.time > 0 ? clamp(position / first.time, 0, 1) : 1;
    }
    if (!activeLine || activeLine.text.length > 0) return null;
    const end = lines[current + 1]?.time ?? activeLine.time + 4;
    return clamp((position - activeLine.time) / Math.max(0.1, end - activeLine.time), 0, 1);
  });

  let pressed: { x: number; y: number } | null = null;
  let dragged = false;

  function press(event: PointerEvent) {
    if (event.button !== 0) return;
    (event.currentTarget as Element).setPointerCapture(event.pointerId);
    pressed = { x: event.clientX, y: event.clientY };
    dragged = false;
  }

  function move(event: PointerEvent) {
    if (!pressed) return;
    if ((event.buttons & 1) === 0) {
      pressed = null;
      return;
    }
    if (Math.hypot(event.clientX - pressed.x, event.clientY - pressed.y) > DRAG_DISTANCE) {
      pressed = null;
      dragged = true;
      (event.currentTarget as Element).releasePointerCapture(event.pointerId);
      onwindowdrag();
    }
  }

  function jump(line: LyricLine) {
    pressed = null;
    if (dragged) {
      dragged = false;
      return;
    }
    onjump?.(line);
  }
</script>

<div class="lyrics" style:font-size="{fontSize}px">
  <p class="spoken" aria-live="polite">
    {activeLine ? activeLine.text || t("overlay.instrumentalBreak") : t("overlay.intro")}
  </p>
  {#each rows as row (row.index)}
    {@const place = placed(row.motion, frame)}
    {@const line = lines[row.index]}
    <!-- The width follows the target size, not every frame of the animation: it only keeps long lines whole. -->
    <div
      class="row"
      style:width="{width / Math.max(0.05, row.motion.scale.target)}px"
      style:transform="translateY({place.y.toFixed(2)}px) scale({place.scale.toFixed(4)})"
      style:opacity={place.opacity.toFixed(3)}
      style:filter={place.blur > 0.01 ? `blur(${place.blur.toFixed(2)}px)` : null}
    >
      <div
        class="inner"
        class:reveal={firstRows.has(row.index)}
        style:animation-delay={firstRows.has(row.index) ? `${0.12 + 0.05 * row.order}s` : null}
      >
        {#if line && onjump && !row.leaving}
          <button
            class="line"
            aria-label={t("player.jumpToLine", { line: line.text || t("overlay.instrumentalBreak") })}
            onpointerdown={press}
            onpointermove={move}
            onclick={() => jump(line)}
          >
            {@render content(row.active, line, place.opacity)}
          </button>
        {:else}
          <div class="line" aria-hidden="true">{@render content(row.active, line, place.opacity)}</div>
        {/if}
      </div>
    </div>
  {/each}
</div>

<!-- `brightness` is the row's opacity right now: the words of a line that has
     just become active start out as bright as a neighbour's. -->
{#snippet content(active: boolean, line: LyricLine | undefined, brightness: number)}
  {#if active && waiting !== null}
    <BreakDots progress={waiting} size={fontSize} breathing={playing} />
  {:else if active && fills.length > 0}
    <!-- The word being sung lights up from left to right; the others only need their opacity. -->
    {#each fills as word, index (index)}{@const wipe = wordWipe(word.fill, wordOpacity(0, brightness, NEAR))}<span class:wipe={wipe !== null} style:--wipe={wipe} style:opacity={wipe === null ? wordOpacity(word.fill, brightness, NEAR).toFixed(3) : null}>{word.text}</span>{/each}
  {:else if line}
    {lineText(line)}
  {/if}
{/snippet}

<style>
  /* The top and bottom of the lyrics fade out, so lines never end at a hard edge. */
  .lyrics {
    position: relative;
    height: 100%;
    overflow: hidden;
    font-weight: 600;
    line-height: 1.19;
    -webkit-mask-image: linear-gradient(to bottom, transparent 0, #000 12%, #000 88%, transparent 100%);
    mask-image: linear-gradient(to bottom, transparent 0, #000 12%, #000 88%, transparent 100%);
  }

  .row {
    position: absolute;
    top: 0;
    left: 0;
    height: 1.19em;
    transform-origin: 0 0;
  }

  .inner {
    height: 100%;
  }

  .line {
    display: block;
    width: 100%;
    height: 100%;
    overflow: hidden;
    white-space: pre;
    text-overflow: ellipsis;
    text-align: left;
    color: #fff;
    text-shadow: 0 1px 3px rgb(0 0 0 / 0.45);
  }

  button.line {
    border: 0;
    padding: 0;
    background: none;
    font: inherit;
    cursor: pointer;
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

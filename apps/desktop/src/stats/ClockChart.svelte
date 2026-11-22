<script lang="ts">
  import type { Translate } from "../lib/i18n";
  import { axisLabel, formatDuration, heatStep, hourRange, weekdays, type Language } from "./format";

  // The listening clock: weekdays (Monday first) by hours of the day, in steps
  // of white. Hover a cell, or focus it and use the arrow keys, for its exact time.
  let { clock, language, label, t }: { clock: number[][]; language: Language; label: string; t: Translate } = $props();

  const LABELS = 32;
  const GAP = 2;
  const ROW = 12;
  const HOURS = 18;
  const GRID = 7 * ROW + 6 * GAP;
  const HEIGHT = GRID + HOURS;
  /** White, from an empty hour to the busiest one; one hue, light to strong. */
  const STEPS = [0.07, 0.24, 0.44, 0.68, 0.94];
  const MARKS = [0, 6, 12, 18];

  let width = $state(0);
  let active = $state<{ day: number; hour: number } | null>(null);
  let svg = $state<SVGSVGElement>();

  const names = $derived(weekdays(language));
  const seconds = (day: number, hour: number) => clock[day]?.[hour] ?? 0;
  const max = $derived(Math.max(0, ...clock.flat()));
  const cell = $derived(Math.max(3, (width - LABELS - 23 * GAP) / 24));
  const hourNames = $derived(MARKS.map((hour) => axisLabel(new Date(2026, 0, 5, hour), "hour", "today", language)));

  const caption = $derived(
    active === null
      ? t("stats.chartHint")
      : `${names[active.day] ?? ""} ${hourRange(active.hour, language)} · ${formatDuration(seconds(active.day, active.hour), language, t)}`,
  );

  const x = (hour: number) => LABELS + hour * (cell + GAP);
  const y = (day: number) => day * (ROW + GAP);

  function point(event: PointerEvent) {
    if (!svg) return;
    const box = svg.getBoundingClientRect();
    const hour = Math.floor((event.clientX - box.left - LABELS + GAP / 2) / (cell + GAP));
    const day = Math.floor((event.clientY - box.top + GAP / 2) / (ROW + GAP));
    active = hour >= 0 && hour < 24 && day >= 0 && day < 7 ? { day, hour } : null;
  }

  function key(event: KeyboardEvent) {
    const current = active ?? { day: 0, hour: 0 };
    const moves: Record<string, [number, number]> = {
      ArrowLeft: [0, -1],
      ArrowRight: [0, 1],
      ArrowUp: [-1, 0],
      ArrowDown: [1, 0],
    };
    const move = moves[event.key];
    if (move) {
      event.preventDefault();
      active = {
        day: Math.max(0, Math.min(6, current.day + move[0])),
        hour: Math.max(0, Math.min(23, current.hour + move[1])),
      };
    } else if (event.key === "Home" || event.key === "End") {
      event.preventDefault();
      active = { day: current.day, hour: event.key === "Home" ? 0 : 23 };
    } else if (event.key === "Escape") {
      active = null;
    }
  }

  /** Keyboard focus starts on the busiest hour. */
  function focus() {
    if (active !== null) return;
    let best = { day: 0, hour: 0 };
    for (let day = 0; day < 7; day += 1) {
      for (let hour = 0; hour < 24; hour += 1) {
        if (seconds(day, hour) > seconds(best.day, best.hour)) best = { day, hour };
      }
    }
    active = best;
  }
</script>

<div class="chart" bind:clientWidth={width}>
  <svg
    bind:this={svg}
    width={Math.max(0, width)}
    height={HEIGHT}
    role="slider"
    tabindex="0"
    aria-label={label}
    aria-valuemin={0}
    aria-valuemax={167}
    aria-valuenow={active ? active.day * 24 + active.hour : 0}
    aria-valuetext={caption}
    onpointermove={point}
    onpointerleave={() => (active = null)}
    onkeydown={key}
    onfocus={focus}
    onblur={() => (active = null)}
  >
    {#if width > 0}
      {#each names as name, day (day)}
        <text class="axis" x="0" y={y(day) + ROW / 2} dy="0.35em">{name}</text>
        {#each { length: 24 } as _, hour (hour)}
          <rect
            class="cell"
            x={x(hour)}
            y={y(day)}
            width={cell}
            height={ROW}
            rx="2.5"
            style:fill-opacity={STEPS[heatStep(seconds(day, hour), max)]}
          />
        {/each}
      {/each}
      {#if active}
        <rect class="focus" x={x(active.hour) + 0.5} y={y(active.day) + 0.5} width={cell - 1} height={ROW - 1} rx="2.5" />
      {/if}
      {#each MARKS as hour, index (hour)}
        <text class="axis" x={x(hour)} y={GRID + 13}>{hourNames[index]}</text>
      {/each}
    {/if}
  </svg>
  <div class="footer">
    <p class="caption" aria-hidden="true">{caption}</p>
    <span class="legend" aria-hidden="true">
      {t("stats.chartLess")}
      {#each STEPS as opacity (opacity)}
        <span class="swatch" style:--opacity={opacity}></span>
      {/each}
      {t("stats.chartMore")}
    </span>
  </div>
  <!-- A table ignores a 1 px width, so the wrapper does the hiding. -->
  <div class="visually-hidden">
    <table>
      <caption>{label}</caption>
      <tbody>
        {#each names as name, day (day)}
          <tr>
            <th scope="row">{name}</th>
            {#each { length: 24 } as _, hour (hour)}
              <td>{hourRange(hour, language)}: {formatDuration(seconds(day, hour), language, t)}</td>
            {/each}
          </tr>
        {/each}
      </tbody>
    </table>
  </div>
</div>

<style>
  .chart {
    position: relative;
    min-width: 0;
  }

  svg {
    display: block;
    outline: none;
    border-radius: 4px;
  }

  svg:focus-visible {
    box-shadow: 0 0 0 2px rgb(255 255 255 / 0.5);
  }

  /* Another range fades each hour to its new step. */
  .cell {
    fill: #fff;
    transition: fill-opacity var(--duration-reveal) var(--ease-out);
  }

  @media (prefers-reduced-motion: reduce) {
    .cell {
      transition-duration: var(--duration-pane);
    }
  }

  .focus {
    fill: none;
    stroke: rgb(255 255 255 / 0.85);
    stroke-width: 1;
  }

  .axis {
    fill: rgb(255 255 255 / 0.55);
    font-size: 10px;
  }

  .footer {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-top: 8px;
  }

  .caption {
    flex: 1;
    min-width: 0;
    margin: 0;
    font-size: 11px;
    color: rgb(255 255 255 / 0.6);
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .legend {
    flex: none;
    display: flex;
    align-items: center;
    gap: 3px;
    font-size: 10px;
    color: rgb(255 255 255 / 0.55);
  }

  .swatch {
    width: 9px;
    height: 9px;
    border-radius: 2px;
    background: rgb(255 255 255 / var(--opacity));
  }

  .swatch:first-of-type {
    margin-left: 3px;
  }

  .swatch:last-of-type {
    margin-right: 3px;
  }

  .visually-hidden {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip-path: inset(50%);
    white-space: nowrap;
  }
</style>

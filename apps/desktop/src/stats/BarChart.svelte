<script lang="ts">
  import type { Translate } from "../lib/i18n";
  import { axisLabel, bucketLabel, formatDuration, labelEvery, parseLocal, ticks, type Language } from "./format";
  import type { Bucket, BucketUnit, StatsRange } from "./types";

  // Listening over time: one white bar per hour, day or month. Hover a bar, or
  // focus the chart and use the arrow keys, for its exact time.
  let {
    buckets,
    unit,
    range,
    language,
    label,
    t,
  }: { buckets: Bucket[]; unit: BucketUnit; range: StatsRange; language: Language; label: string; t: Translate } = $props();

  const PLOT = 140;
  const TOP = 8;
  const AXIS = 22;
  const LEFT = 52;
  const HEIGHT = TOP + PLOT + AXIS;
  const BASE = TOP + PLOT;

  let width = $state(0);
  let active = $state<number | null>(null);
  let svg = $state<SVGSVGElement>();

  const dates = $derived(buckets.map((bucket) => parseLocal(bucket.start)));
  const scale = $derived(ticks(Math.max(0, ...buckets.map((bucket) => bucket.seconds))));
  const top = $derived(scale.values[scale.values.length - 1] ?? 60);
  const plotWidth = $derived(Math.max(0, width - LEFT));
  const band = $derived(buckets.length > 0 ? plotWidth / buckets.length : 0);
  // Thin bars with air between them; at most 24 px wide.
  const barWidth = $derived(Math.max(1, Math.min(24, band - 2)));
  const every = $derived(labelEvery(buckets.length, plotWidth, range === "week" ? 30 : 40));
  const axis = $derived(dates.map((date) => axisLabel(date, unit, range, language)));
  const names = $derived(dates.map((date) => bucketLabel(date, unit, language)));

  function y(seconds: number): number {
    return BASE - (top > 0 ? (Math.min(seconds, top) / top) * PLOT : 0);
  }

  /** A bar with 4 px rounded corners at its end and square at the baseline. */
  function bar(index: number, seconds: number): string {
    const height = BASE - y(seconds);
    if (!(height >= 0.5)) return "";
    const x = LEFT + index * band + (band - barWidth) / 2;
    const r = Math.min(4, barWidth / 2, height);
    const end = BASE - height;
    return `M${x},${BASE}V${end + r}A${r},${r} 0 0 1 ${x + r},${end}H${x + barWidth - r}A${r},${r} 0 0 1 ${x + barWidth},${end + r}V${BASE}Z`;
  }

  function tickLabel(value: number): string {
    if (value === 0) return "0";
    return scale.unit === "hours"
      ? t("stats.axisHours", { value: value / 3_600 })
      : t("stats.axisMinutes", { value: value / 60 });
  }

  const caption = $derived.by(() => {
    const bucket = active === null ? undefined : buckets[active];
    if (active === null || !bucket) return t("stats.chartHint");
    return `${names[active] ?? ""} · ${formatDuration(bucket.seconds, language, t)}`;
  });

  function point(event: PointerEvent) {
    if (!svg || band <= 0) return;
    const x = event.clientX - svg.getBoundingClientRect().left - LEFT;
    const index = Math.floor(x / band);
    active = index >= 0 && index < buckets.length ? index : null;
  }

  function key(event: KeyboardEvent) {
    const last = buckets.length - 1;
    if (last < 0) return;
    const moves: Record<string, number> = {
      ArrowLeft: (active ?? last + 1) - 1,
      ArrowRight: (active ?? -1) + 1,
      Home: 0,
      End: last,
    };
    const next = moves[event.key];
    if (next !== undefined) {
      event.preventDefault();
      active = Math.max(0, Math.min(last, next));
    } else if (event.key === "Escape") {
      active = null;
    }
  }

  /** Keyboard focus starts on the latest bar with music. */
  function focus() {
    if (active !== null) return;
    let index = buckets.length - 1;
    while (index > 0 && !((buckets[index]?.seconds ?? 0) > 0)) index -= 1;
    active = index >= 0 ? index : null;
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
    aria-valuemax={Math.max(0, buckets.length - 1)}
    aria-valuenow={active ?? 0}
    aria-valuetext={caption}
    onpointermove={point}
    onpointerleave={() => (active = null)}
    onkeydown={key}
    onfocus={focus}
    onblur={() => (active = null)}
  >
    {#if width > 0}
      {#each scale.values as value (value)}
        <line class="grid" class:base={value === 0} x1={LEFT} x2={width} y1={y(value) + 0.5} y2={y(value) + 0.5} />
        <text class="tick" x={LEFT - 8} y={y(value)} dy="0.32em" text-anchor="end">{tickLabel(value)}</text>
      {/each}
      <!-- Keyed by the range too, so all bars of another range grow in, also those of days both show. -->
      {#each buckets as bucket, index (`${range}|${bucket.start}`)}
        <path
          class="bar"
          class:dim={active !== null && active !== index}
          style:animation-delay="{Math.min(index, 24) * 14}ms"
          d={bar(index, bucket.seconds)}
        />
        {#if index % every === 0}
          <text class="label" x={LEFT + index * band + band / 2} y={BASE + 15} text-anchor="middle">{axis[index]}</text>
        {/if}
      {/each}
    {/if}
  </svg>
  <p class="caption" aria-hidden="true">{caption}</p>
  <!-- A table ignores a 1 px width, so the wrapper does the hiding. -->
  <div class="visually-hidden">
    <table>
      <caption>{label}</caption>
      <tbody>
        {#each buckets as bucket, index (bucket.start)}
          <tr><th scope="row">{names[index]}</th><td>{formatDuration(bucket.seconds, language, t)}</td></tr>
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
    overflow: visible;
    outline: none;
    border-radius: 6px;
  }

  svg:focus-visible {
    box-shadow: 0 0 0 2px rgb(255 255 255 / 0.5);
  }

  .grid {
    stroke: rgb(255 255 255 / 0.08);
    stroke-width: 1;
  }

  .grid.base {
    stroke: rgb(255 255 255 / 0.2);
  }

  .tick,
  .label {
    fill: rgb(255 255 255 / 0.45);
    font-size: 10px;
    font-variant-numeric: tabular-nums;
  }

  /* New bars, as for another range, grow from the baseline one after another. */
  .bar {
    fill: #fff;
    opacity: 0.9;
    transform-box: fill-box;
    transform-origin: 50% 100%;
    transition: opacity var(--duration-press) var(--ease-out);
    animation: grow var(--duration-reveal) var(--ease-out) both;
  }

  @keyframes grow {
    from {
      transform: scaleY(0);
    }
  }

  @keyframes appear {
    from {
      opacity: 0;
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .bar {
      animation-name: appear;
    }
  }

  .bar.dim {
    opacity: 0.35;
  }

  .caption {
    margin: 8px 0 0;
    font-size: 11px;
    color: rgb(255 255 255 / 0.6);
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
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

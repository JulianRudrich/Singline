<script lang="ts">
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { onMount } from "svelte";
  import { translator, type MessageKey } from "../lib/i18n";
  import { connect } from "../lib/live";
  import type { AppInfo } from "../lib/types";
  import Segmented from "../lib/ui/Segmented.svelte";
  import HistorySettings from "../settings/HistorySettings.svelte";
  import { onHistory, statsApi } from "./api";
  import BarChart from "./BarChart.svelte";
  import ClockChart from "./ClockChart.svelte";
  import { formatCount, formatDuration, percent, splitNumbers } from "./format";
  import { RANGES, type ListeningSummary, type RankedEntry, type StatsRange } from "./types";

  const RANGE_KEY = "stats.range";

  let app = $state<AppInfo | null>(null);
  let range = $state<StatsRange>(storedRange());
  let summary = $state<ListeningSummary | null>(null);
  let dimmed = $state(false);
  let request = 0;

  const language = $derived(app?.language ?? "en");
  const t = $derived(translator(language));
  const duration = (seconds: number) => formatDuration(seconds, language, t);

  $effect(() => {
    const title = t("stats.title");
    document.title = title;
    void getCurrentWindow()
      .setTitle(title)
      .catch(() => {});
  });

  $effect(() => {
    void load(range);
  });

  function storedRange(): StatsRange {
    try {
      const stored = localStorage.getItem(RANGE_KEY);
      return RANGES.find((option) => option === stored) ?? "week";
    } catch {
      return "week";
    }
  }

  function choose(option: StatsRange) {
    range = option;
    try {
      localStorage.setItem(RANGE_KEY, option);
    } catch {
      // Remembering the range is a convenience only.
    }
  }

  /** Loads `wanted`. A slow answer dims the old numbers instead of clearing them. */
  async function load(wanted: StatsRange) {
    const id = ++request;
    const slow = setTimeout(() => {
      if (id === request) dimmed = true;
    }, 150);
    try {
      const next = await statsApi.summary(wanted);
      if (id === request) summary = next;
    } catch (error) {
      console.error("could not load your music", error);
    } finally {
      clearTimeout(slow);
      if (id === request) dimmed = false;
    }
  }

  onMount(() => {
    const stop = connect({ app: (next) => (app = next) }, (snapshot) => (app = snapshot.app));
    let stopHistory: (() => void) | undefined;
    let alive = true;
    void onHistory(() => void load(range)).then((unlisten) => {
      if (alive) stopHistory = unlisten;
      else unlisten();
    });
    return () => {
      alive = false;
      stop();
      stopHistory?.();
    };
  });
</script>

<main class="stats">
  <header class="reveal" style="--order: 0">
    <div class="heading">
      <h1>{t("stats.heading")}</h1>
      <p>{t("stats.subtitle")}</p>
    </div>
    <Segmented
      compact
      label={t("stats.range")}
      value={range}
      options={RANGES.map((option) => ({ value: option, label: t(`stats.range.${option}` as MessageKey) }))}
      onchange={choose}
    />
  </header>

  {#if summary && !summary.hasHistory}
    <section class="card empty reveal" style="--order: 1">
      <p>{t("stats.empty")}</p>
    </section>
  {:else if summary}
    <div class="content" class:dimmed>
      <section class="tiles reveal" style="--order: 1">
        {@render tile(
          t("stats.tileListening"),
          duration(summary.seconds),
          summary.range === "today" ? "" : t("stats.tileListeningDetail", { duration: duration(summary.dailyAverage) }),
        )}
        {@render tile(
          t("stats.tilePlays"),
          formatCount(summary.plays, language),
          summary.plays > 0 ? t("stats.tilePlaysDetail", { percent: percent(summary.syncedPlays, summary.plays) }) : "",
        )}
        {@render tile(t("stats.tileStreak"), formatCount(summary.streakDays, language), t("stats.tileStreakDetail"))}
        {@render tile(
          t("stats.tileSkips"),
          formatCount(summary.skips, language),
          summary.songs > 0 ? t("stats.tileSkipsDetail", { percent: percent(summary.skips, summary.songs) }) : "",
        )}
      </section>

      <section class="pair reveal" style="--order: 2">
        <div class="card">
          <h2>{t("stats.chartOverTime")}</h2>
          <BarChart buckets={summary.buckets} unit={summary.unit} range={summary.range} {language} label={t("stats.chartOverTime")} {t} />
        </div>
        <div class="card">
          <h2>{t("stats.chartClock")}</h2>
          <ClockChart clock={summary.clock} {language} label={t("stats.chartClock")} {t} />
        </div>
      </section>

      <section class="pair reveal" style="--order: 3">
        {@render ranking(t("stats.topSongs"), summary.topTracks)}
        {@render ranking(t("stats.topArtists"), summary.topArtists)}
      </section>
    </div>
  {/if}

  <section class="reveal" style="--order: 4">
    <HistorySettings hasHistory={summary?.hasHistory ?? false} />
  </section>
</main>

{#snippet tile(title: string, value: string, detail: string)}
  <div class="card tile">
    <span class="tile-title">{title}</span>
    <span class="tile-value">
      {#each splitNumbers(value) as part, index (index)}
        <span class:unit={!part.number}>{part.text}</span>
      {/each}
    </span>
    <span class="tile-detail">{detail || " "}</span>
  </div>
{/snippet}

{#snippet ranking(title: string, entries: RankedEntry[])}
  <div class="card">
    <h2>{title}</h2>
    {#if entries.length === 0}
      <p class="muted">{t("stats.emptyRange")}</p>
    {:else}
      {@const longest = Math.max(1, entries[0]?.seconds ?? 1)}
      <ol class="ranking">
        {#each entries as entry, index (entry.key)}
          <li>
            <span class="rank">{index + 1}</span>
            <span class="name">
              <span class="entry-title">{entry.title}</span>
              {#if entry.subtitle}<span class="entry-subtitle">{entry.subtitle}</span>{/if}
            </span>
            <span class="amount">{duration(entry.seconds)} · {t("stats.playsCount", { count: formatCount(entry.plays, language) })}</span>
            <span class="meter" style:--share={Math.max(0.01, entry.seconds / longest)} aria-hidden="true"></span>
          </li>
        {/each}
      </ol>
    {/if}
  </div>
{/snippet}

<style>
  /* Neutral black glass with white accents, whatever the system theme. */
  :global(html[data-view="stats"]) {
    color-scheme: dark;
    --text: #fff;
    background: #111113;
  }

  :global(html[data-view="stats"] body) {
    min-height: 100%;
    height: auto;
    color: #fff;
    background:
      radial-gradient(110% 70% at 0% 0%, rgb(255 255 255 / 0.07), transparent 62%),
      linear-gradient(160deg, #1c1d20 0%, #121214 48%, #0a0a0b 100%);
    background-attachment: fixed;
  }

  .stats {
    display: flex;
    flex-direction: column;
    gap: 12px;
    padding: 26px 28px 28px;
  }

  header {
    display: flex;
    flex-wrap: wrap;
    align-items: flex-end;
    justify-content: space-between;
    gap: 12px 16px;
    padding-bottom: 6px;
  }

  .heading {
    display: flex;
    flex-direction: column;
    gap: 4px;
    min-width: 0;
  }

  h1 {
    margin: 0;
    font-size: 22px;
    font-weight: 600;
    letter-spacing: -0.01em;
  }

  .heading p {
    margin: 0;
    font-size: 13px;
    color: rgb(255 255 255 / 0.6);
  }

  .card {
    padding: 16px;
    border-radius: 12px;
    background: rgb(255 255 255 / 0.05);
    border: 1px solid rgb(255 255 255 / 0.08);
    min-width: 0;
  }

  h2 {
    margin: 0 0 12px;
    font-size: 13px;
    font-weight: 500;
  }

  .empty p,
  .muted {
    margin: 0;
    font-size: 13px;
    color: rgb(255 255 255 / 0.7);
  }

  .muted {
    font-size: 12px;
    color: rgb(255 255 255 / 0.55);
  }

  .content {
    display: flex;
    flex-direction: column;
    gap: 12px;
    transition: opacity 200ms ease-out;
  }

  .content.dimmed {
    opacity: 0.55;
  }

  .tiles {
    display: grid;
    grid-template-columns: repeat(4, minmax(0, 1fr));
    gap: 12px;
  }

  .tile {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .tile-title {
    font-size: 11.5px;
    font-weight: 500;
    color: rgb(255 255 255 / 0.6);
  }

  .tile-value {
    font-size: 26px;
    font-weight: 600;
    letter-spacing: -0.01em;
    line-height: 1.15;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  /* Units such as "Std." stay small next to the numbers. */
  .tile-value .unit {
    font-size: 15px;
    font-weight: 500;
    letter-spacing: 0;
    color: rgb(255 255 255 / 0.75);
  }

  .tile-detail {
    font-size: 11px;
    color: rgb(255 255 255 / 0.55);
    white-space: pre;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .pair {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 12px;
  }

  .ranking {
    display: flex;
    flex-direction: column;
    gap: 7px;
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .ranking li {
    display: grid;
    grid-template-columns: 16px minmax(0, 1fr) auto;
    align-items: baseline;
    column-gap: 8px;
    row-gap: 4px;
  }

  .rank {
    font-size: 11px;
    font-weight: 500;
    text-align: right;
    color: rgb(255 255 255 / 0.55);
    font-variant-numeric: tabular-nums;
  }

  .name {
    font-size: 12.5px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .entry-subtitle {
    margin-left: 8px;
    color: rgb(255 255 255 / 0.5);
  }

  .amount {
    font-size: 11px;
    color: rgb(255 255 255 / 0.5);
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
  }

  /* Meters grow in, and ease to a new length when the numbers change. */
  .meter {
    grid-column: 2 / -1;
    height: 3px;
    width: calc(var(--share) * 100%);
    min-width: 3px;
    border-radius: 2px;
    background: rgb(255 255 255 / 0.85);
    transition: width var(--duration-reveal) var(--ease-out);
    animation: meter-in var(--duration-reveal) var(--ease-out) both;
  }

  @keyframes meter-in {
    from {
      width: 0;
    }
  }

  .reveal {
    animation: reveal 0.55s cubic-bezier(0.23, 1, 0.32, 1) both;
    animation-delay: calc(var(--order, 0) * 60ms);
  }

  @keyframes reveal {
    from {
      opacity: 0;
      transform: translateY(8px);
      filter: blur(6px);
    }
  }

  @media (max-width: 760px) {
    .tiles {
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }

    .pair {
      grid-template-columns: minmax(0, 1fr);
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .reveal {
      animation: none;
    }

    .content {
      transition: none;
    }

    .meter {
      transition: none;
      animation: none;
    }
  }
</style>

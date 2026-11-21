<script lang="ts">
  import { onMount, tick } from "svelte";
  import { api } from "../lib/api";
  import { translator } from "../lib/i18n";
  import { connect } from "../lib/live";
  import type { AppInfo, Settings } from "../lib/types";
  import Button from "../lib/ui/Button.svelte";
  import Row from "../lib/ui/Row.svelte";
  import Toggle from "../lib/ui/Toggle.svelte";
  import { statsApi } from "../stats/api";

  // Self-contained: reads and changes the settings itself, so it fits into any
  // window or pane. `hasHistory` false turns "Clear history" off.
  let { hasHistory = true }: { hasHistory?: boolean } = $props();

  /** An unanswered question goes away by itself. */
  const ASK_FOR = 8_000;

  let settings = $state<Settings | null>(null);
  let app = $state<AppInfo | null>(null);
  let asking = $state(false);
  let clearing = $state(false);
  let question = $state<HTMLElement>();
  let timer: ReturnType<typeof setTimeout> | undefined;
  const t = $derived(translator(app?.language ?? "en"));

  async function keep(keepHistory: boolean) {
    if (settings) settings = { ...settings, keepHistory };
    settings = await api.updateSettings({ keepHistory });
  }

  async function ask() {
    asking = true;
    clearTimeout(timer);
    timer = setTimeout(() => (asking = false), ASK_FOR);
    await tick();
    // Enter on the question must not delete anything: the focus goes to "No".
    question?.querySelector<HTMLButtonElement>("[data-cancel]")?.focus();
  }

  function stopAsking() {
    clearTimeout(timer);
    asking = false;
  }

  function escape(event: KeyboardEvent) {
    if (event.key === "Escape") stopAsking();
  }

  async function clear() {
    clearing = true;
    try {
      await statsApi.clear();
    } finally {
      clearing = false;
      stopAsking();
    }
  }

  onMount(() => {
    const stop = connect(
      { settings: (next) => (settings = next), app: (next) => (app = next) },
      (snapshot) => {
        settings = snapshot.settings;
        app = snapshot.app;
      },
    );
    return () => {
      stop();
      clearTimeout(timer);
    };
  });
</script>

{#if settings && app}
  <Row icon="history" title={t("history.keep")} hint={t("history.keepHint")}>
    {#if asking}
      <!-- svelte-ignore a11y_no_static_element_interactions -->
      <span class="question" bind:this={question} onkeydown={escape}>
        <span>{t("history.clearConfirm")}</span>
        <Button primary onclick={clear} disabled={clearing}>{t("history.clearYes")}</Button>
        <Button data-cancel onclick={stopAsking}>{t("history.clearNo")}</Button>
      </span>
    {:else}
      <Button onclick={ask} disabled={!hasHistory}>{t("history.clear")}</Button>
    {/if}
    <Toggle checked={settings.keepHistory} label={t("history.keep")} onchange={keep} />
  </Row>
{/if}

<style>
  .question {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 12px;
  }
</style>

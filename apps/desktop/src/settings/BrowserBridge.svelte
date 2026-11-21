<script lang="ts">
  import { onMount } from "svelte";
  import { api } from "../lib/api";
  import { translator } from "../lib/i18n";
  import { connect } from "../lib/live";
  import type { AppInfo, BridgeView } from "../lib/types";
  import Button from "../lib/ui/Button.svelte";
  import Card from "../lib/ui/Card.svelte";
  import Row from "../lib/ui/Row.svelte";

  // Self-contained: shows the code that pairs the Singline browser extension and the
  // browsers that paired, so it fits into any pane. While it is shown, pairing stays open.

  let app = $state<AppInfo | null>(null);
  let bridge = $state<BridgeView | null>(null);
  let busy = $state(false);
  const t = $derived(translator(app?.language ?? "en"));
  const code = $derived(bridge?.pairingCode ? `${bridge.pairingCode.slice(0, 3)} ${bridge.pairingCode.slice(3)}` : "");

  async function refresh() {
    try {
      bridge = await api.browserBridge();
    } catch {
      bridge = null;
    }
  }

  async function forget(id: string) {
    busy = true;
    try {
      bridge = await api.forgetBrowser(id);
    } finally {
      busy = false;
    }
  }

  onMount(() => {
    const stop = connect({ app: (next) => (app = next) }, (snapshot) => (app = snapshot.app));
    void refresh();
    const timer = setInterval(refresh, 2000);
    return () => {
      stop();
      clearInterval(timer);
    };
  });
</script>

{#if app && bridge?.available}
  <Card>
    <Row card={false} icon="puzzle" title={t("bridge.title")} hint={bridge.port === null ? t("bridge.unavailable") : t("bridge.hint")}>
      {#if code}
        <output class="code" aria-label={t("bridge.codeLabel")}>{code}</output>
      {/if}
    </Row>
    {#each bridge.browsers as browser (browser.id)}
      <div class="divider"></div>
      <Row card={false} icon="browser" title={browser.name} hint={browser.connected ? t("bridge.connected") : t("bridge.idle")}>
        <Button disabled={busy} onclick={() => forget(browser.id)}>{t("bridge.remove")}</Button>
      </Row>
    {/each}
  </Card>
{/if}

<style>
  .divider {
    height: 1px;
    background: rgb(255 255 255 / 0.08);
  }

  .code {
    font-size: 17px;
    font-weight: 600;
    font-variant-numeric: tabular-nums;
    letter-spacing: 0.08em;
    white-space: nowrap;
    user-select: all;
    -webkit-user-select: all;
  }
</style>

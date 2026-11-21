<script lang="ts">
  // Settings ▸ Spotify, built like Linory's pane: the connection card, a
  // guide to the listener's own Spotify app, and Spotify's top lists once
  // connected. Self-contained: it reads the language and the connection itself.
  import { onMount } from "svelte";
  import { api, subscribe } from "../lib/api";
  import { translator } from "../lib/i18n";
  import Button from "../lib/ui/Button.svelte";
  import Glyph from "../lib/ui/Glyph.svelte";
  import { copyText, isClientId, onSpotifyStatus, spotify, statusText, type SpotifyStatus } from "./api";
  import SpotifyTopLists from "./SpotifyTopLists.svelte";

  interface Props {
    /** Settings passes its context; this pane needs nothing from it. */
    ctx?: unknown;
    /** Draws a dark backdrop, for a window that is not dark itself. */
    framed?: boolean;
  }

  let { framed = false }: Props = $props();

  let language = $state<"de" | "en">("en");
  let status = $state<SpotifyStatus | null>(null);
  let draft = $state("");
  let copied = $state<"redirect" | "login" | null>(null);

  const t = $derived(translator(language));
  const connected = $derived(status?.phase === "connected");
  const busy = $derived(status?.phase === "waiting" || status?.phase === "connecting");
  const draftValid = $derived(isClientId(draft));

  /** Takes a new status without overwriting a Client ID that is being typed. */
  function apply(next: SpotifyStatus) {
    const editing = status !== null && draft.trim() !== status.clientId;
    status = next;
    if (!editing) draft = next.clientId;
  }

  async function saveClientId() {
    if (!status || draft.trim() === status.clientId) return;
    apply(await spotify.setClientId(draft));
  }

  async function connect() {
    await saveClientId();
    apply(await spotify.connect());
  }

  async function copy(which: "redirect" | "login", text: string) {
    if (!(await copyText(text))) return;
    copied = which;
    setTimeout(() => {
      if (copied === which) copied = null;
    }, 1500);
  }

  onMount(() => {
    let alive = true;
    let stops: (() => void)[] = [];
    void (async () => {
      const unlisten = await Promise.all([
        subscribe({ app: (app) => (language = app.language) }),
        onSpotifyStatus(apply),
      ]);
      if (!alive) return unlisten.forEach((stop) => stop());
      stops = unlisten;
      const [snapshot, current] = await Promise.all([api.snapshot(), spotify.status()]);
      language = snapshot.app.language;
      apply(current);
    })();
    return () => {
      alive = false;
      stops.forEach((stop) => stop());
    };
  });
</script>

{#if status}
  <div class="pane" class:framed>
    <section class="card">
      <div class="row">
        <label class="label" for="spotify-client-id">{t("spotify.clientId")}</label>
        <input
          id="spotify-client-id"
          class="field mono"
          spellcheck="false"
          autocomplete="off"
          placeholder="3f9c…"
          disabled={busy}
          bind:value={draft}
          onchange={saveClientId}
          onkeydown={(event) => event.key === "Enter" && draftValid && void connect()}
        />
      </div>
      {#if draft.trim() !== "" && !draftValid}
        <p class="hint indent">{t("spotify.clientIdHint")}</p>
      {/if}
      <div class="row">
        <span class="label">{t("spotify.redirectUri")}</span>
        <code class="mono uri" title={status.redirectUri}>{status.redirectUri}</code>
        <Button onclick={() => status && copy("redirect", status.redirectUri)}>
          {copied === "redirect" ? t("spotify.copied") : t("spotify.copy")}
        </Button>
      </div>
      <div class="divider"></div>
      <div class="row">
        <span class="dot" class:on={connected} aria-hidden="true"></span>
        <span class="status" role="status">{statusText(status, t)}</span>
        {#if busy}
          <span class="spinner" aria-hidden="true"></span>
        {/if}
        {#if connected}
          <Button onclick={async () => apply(await spotify.disconnect())}>{t("spotify.disconnect")}</Button>
        {:else if busy}
          <Button onclick={async () => apply(await spotify.cancelLogin())}>{t("spotify.cancel")}</Button>
        {:else}
          <Button primary disabled={!draftValid} onclick={connect}>{t("spotify.connect")}</Button>
        {/if}
      </div>
      {#if status.phase === "waiting"}
        <p class="hint">{t("spotify.waitingHint")}</p>
        {#if status.loginUrl}
          <button class="link" onclick={() => status?.loginUrl && copy("login", status.loginUrl)}>
            {copied === "login" ? t("spotify.copied") : t("spotify.copyLoginLink")}
          </button>
        {/if}
      {/if}
      {#if connected && !status.persistent}
        <p class="hint">{t("spotify.memoryOnly")}</p>
      {/if}
    </section>

    {#if connected && status.needsReconnect}
      <div class="note">
        <span>{t("spotify.reconnectHint")}</span>
        <Button onclick={connect}>{t("spotify.reconnect")}</Button>
      </div>
    {/if}

    {#if connected}
      <SpotifyTopLists {t} />
    {/if}

    <section class="card">
      <strong class="heading">{t("spotify.guideTitle")}</strong>
      <ol class="steps">
        <li>
          {t("spotify.guideStep1")}
          <button class="link" onclick={() => spotify.openDashboard()}>
            {t("spotify.openDashboard")}<Glyph name="external" size={12} />
          </button>
        </li>
        <li>{t("spotify.guideStep2")}</li>
        <li>{t("spotify.guideStep3")}</li>
        <li>{t("spotify.guideStep4")}</li>
      </ol>
      <p class="hint">{t("spotify.premiumNote")}</p>
    </section>
  </div>
{/if}

<style>
  .pane {
    display: flex;
    flex-direction: column;
    gap: 10px;
    color: #fff;
    font-size: 13px;
  }

  .pane.framed {
    padding: 18px;
    border-radius: 16px;
    background: #141618;
  }

  .card {
    display: flex;
    flex-direction: column;
    gap: 14px;
    padding: 16px;
    border-radius: 12px;
    background: rgb(255 255 255 / 0.05);
    border: 1px solid rgb(255 255 255 / 0.08);
  }

  .row {
    display: flex;
    align-items: center;
    gap: 12px;
    min-width: 0;
  }

  .label {
    flex: none;
    width: 100px;
    font-weight: 500;
  }

  .field {
    flex: 1;
    min-width: 0;
    padding: 6px 9px;
    border-radius: 7px;
    border: 1px solid rgb(255 255 255 / 0.14);
    background: rgb(0 0 0 / 0.25);
    color: inherit;
  }

  .field:focus {
    outline: none;
    border-color: rgb(255 255 255 / 0.5);
  }

  .field::placeholder {
    color: rgb(255 255 255 / 0.35);
  }

  .mono {
    font-family: ui-monospace, "SF Mono", "Cascadia Mono", Menlo, Consolas, "DejaVu Sans Mono", monospace;
    font-size: 12px;
  }

  .uri {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
    user-select: all;
    -webkit-user-select: all;
  }

  .divider {
    height: 1px;
    background: rgb(255 255 255 / 0.08);
  }

  .dot {
    flex: none;
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: rgb(255 255 255 / 0.35);
    transition:
      background-color var(--duration-pane) var(--ease-out),
      box-shadow var(--duration-pane) var(--ease-out);
  }

  /* Connected: white with a soft ring, no colour. */
  .dot.on {
    background: #fff;
    box-shadow: 0 0 0 3px rgb(255 255 255 / 0.14);
  }

  .status {
    flex: 1;
    min-width: 0;
    line-height: 1.35;
  }

  .hint {
    margin: 0;
    font-size: 12px;
    line-height: 1.4;
    color: rgb(255 255 255 / 0.55);
  }

  .hint.indent {
    margin-top: -6px;
    padding-left: 112px;
  }

  .link {
    align-self: flex-start;
    display: inline-flex;
    align-items: center;
    gap: 2px;
    padding: 0;
    border: 0;
    background: none;
    color: #fff;
    font: inherit;
    font-weight: 500;
    text-decoration: underline;
    text-decoration-color: rgb(255 255 255 / 0.4);
    text-underline-offset: 2px;
    cursor: pointer;
    transition: text-decoration-color var(--duration-press) var(--ease-out);
  }

  .link:hover {
    text-decoration-color: #fff;
  }

  .link:focus-visible {
    border-radius: 3px;
    outline: 2px solid rgb(255 255 255 / 0.7);
    outline-offset: 2px;
  }

  .note {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 0 4px;
    font-size: 12px;
    color: rgb(255 255 255 / 0.75);
  }

  .note span {
    flex: 1;
  }

  .heading {
    font-weight: 500;
  }

  .steps {
    margin: 0;
    padding-left: 20px;
    display: flex;
    flex-direction: column;
    gap: 6px;
    font-size: 12px;
    line-height: 1.4;
    color: rgb(255 255 255 / 0.75);
  }

  .spinner {
    flex: none;
    width: 12px;
    height: 12px;
    box-sizing: border-box;
    border-radius: 50%;
    border: 2px solid rgb(255 255 255 / 0.25);
    border-top-color: #fff;
    animation: spin 0.8s linear infinite;
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .spinner {
      animation-duration: 2.4s;
    }
  }
</style>

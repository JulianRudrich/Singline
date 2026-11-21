<script lang="ts">
  // Settings ▸ General ▸ Sing to find: start and stop it, the Genius token
  // with a short guide, the speech model and what leaves the computer.
  import { onMount } from "svelte";
  import Button from "../lib/ui/Button.svelte";
  import Card from "../lib/ui/Card.svelte";
  import Glyph from "../lib/ui/Glyph.svelte";
  import Note from "../lib/ui/Note.svelte";
  import Row from "../lib/ui/Row.svelte";
  import Toggle from "../lib/ui/Toggle.svelte";
  import { isActive, onSing, percent, sing, type ModelView, type SingSettings, type SingView } from "../sing/api";
  import type { PaneContext } from "./panes";

  let { ctx }: { ctx: PaneContext } = $props();
  const t = $derived(ctx.t);

  let current = $state<SingSettings | null>(null);
  let model = $state<ModelView | null>(null);
  let view = $state<SingView | null>(null);
  let draft = $state("");
  let saved = $state(false);

  const active = $derived(isActive(view));
  const enabled = $derived(ctx.settings.singToFind);
  const changed = $derived(current !== null && draft.trim() !== current.token);

  const modelHint = $derived.by(() => {
    if (!model) return "";
    if (model.custom) return t("sing.modelCustom");
    switch (model.state) {
      case "ready":
        return t("sing.modelReady");
      case "downloading":
        return t("sing.modelDownloading", { percent: percent(model) });
      case "failed":
        return t("sing.modelFailed", { error: model.error ?? "" });
      default:
        return t("sing.modelMissing");
    }
  });

  async function saveToken() {
    current = await sing.setToken(draft);
    draft = current.token;
    saved = true;
    setTimeout(() => (saved = false), 1500);
  }

  onMount(() => {
    let alive = true;
    let stop: (() => void) | undefined;
    void (async () => {
      const unlisten = await onSing({ view: (next) => (view = next), model: (next) => (model = next) });
      if (!alive) return unlisten();
      stop = unlisten;
      const [settings, state] = await Promise.all([sing.settings(), sing.view()]);
      current = settings;
      model = settings.model;
      draft = settings.token;
      view = state;
    })();
    return () => {
      alive = false;
      stop?.();
    };
  });
</script>

{#if current && model}
  <Card>
    <Row icon="mic" title={t("sing.title")} hint={enabled ? t("sing.hint") : t("sing.offHint")} card={false}>
      <Toggle label={t("sing.title")} checked={enabled} onchange={(singToFind) => ctx.update({ singToFind })} />
    </Row>
  {#if enabled}
    <div class="divider"></div>
    <Row icon="search" title={t("sing.listen")} hint={t("sing.listenHint")} card={false}>
      <Button disabled={!current.available} onclick={() => sing.toggle().then((next) => (view = next))}>
        {active ? t("sing.stop") : t("sing.start")}
      </Button>
    </Row>
    <div class="divider"></div>
    <div class="token">
      <label for="genius-token">{t("sing.token")}</label>
      <input
        id="genius-token"
        type="password"
        spellcheck="false"
        autocomplete="off"
        placeholder={t("sing.tokenPlaceholder")}
        bind:value={draft}
        onkeydown={(event) => event.key === "Enter" && changed && void saveToken()}
      />
      <Button disabled={!changed} onclick={saveToken}>{saved ? t("sing.saved") : t("sing.save")}</Button>
    </div>
    <p class="hint">{current.builtIn ? t("sing.tokenBuiltIn") : t("sing.tokenNeeded")}</p>
    <ol class="steps" aria-label={t("sing.guideTitle")}>
      <li>
        {t("sing.guideStep1")}
        <button class="link" onclick={() => sing.openGenius()}>
          {t("sing.openGenius")}<Glyph name="external" size={12} />
        </button>
      </li>
      <li>{t("sing.guideStep2")}</li>
      <li>{t("sing.guideStep3")}</li>
    </ol>
    <div class="divider"></div>
    <Row icon="download" title={t("sing.model")} hint={`${t("sing.modelHint", { name: model.name })} ${modelHint}`} card={false}>
      {#if model.custom}
        <!-- A model chosen for development: nothing to download. -->
      {:else if model.state === "downloading"}
        <Button onclick={async () => (model = await sing.cancelDownload())}>{t("sing.cancel")}</Button>
      {:else if model.state === "ready"}
        <Button onclick={async () => (model = await sing.removeModel())}>{t("sing.remove")}</Button>
      {:else}
        <Button disabled={!current.available} onclick={async () => (model = await sing.downloadModel())}>
          {model.received > 0 ? t("sing.resume") : t("sing.download")}
        </Button>
      {/if}
    </Row>
    {#if model.state === "downloading"}
      <span class="progress" aria-hidden="true"><span style:width="{percent(model)}%"></span></span>
    {/if}
  {/if}
  </Card>
  {#if enabled}
    <Note>{t("sing.privacy")}</Note>
  {/if}
{/if}

<style>
  .divider {
    height: 1px;
    background: rgb(255 255 255 / 0.08);
  }

  .token {
    display: flex;
    align-items: center;
    gap: 10px;
    padding-left: 40px;
  }

  label {
    flex: none;
    font-size: 13px;
    font-weight: 500;
  }

  input {
    flex: 1;
    min-width: 0;
    height: 26px;
    box-sizing: border-box;
    padding: 0 9px;
    border: 1px solid rgb(255 255 255 / 0.14);
    border-radius: 7px;
    background: rgb(0 0 0 / 0.25);
    color: #fff;
    font: inherit;
    font-size: 12px;
  }

  input:focus {
    outline: none;
    border-color: rgb(255 255 255 / 0.5);
  }

  input::placeholder {
    color: rgb(255 255 255 / 0.35);
  }

  .hint {
    margin: -6px 0 0;
    padding-left: 40px;
    font-size: 11.5px;
    line-height: 1.4;
    color: rgb(255 255 255 / 0.55);
  }

  .steps {
    margin: -4px 0 0;
    padding-left: 58px;
    display: flex;
    flex-direction: column;
    gap: 4px;
    font-size: 11.5px;
    line-height: 1.4;
    color: rgb(255 255 255 / 0.75);
  }

  .link {
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

  .progress {
    display: block;
    height: 3px;
    margin: -6px 0 0 40px;
    border-radius: 2px;
    background: rgb(255 255 255 / 0.15);
    overflow: hidden;
  }

  .progress span {
    display: block;
    height: 100%;
    background: #fff;
    transition: width 0.25s linear;
  }
</style>

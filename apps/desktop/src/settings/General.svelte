<script lang="ts">
  import { hotkeyLabel } from "../lib/hotkey";
  import type { MessageKey } from "../lib/i18n";
  import type { Language, SourcePreference } from "../lib/types";
  import Row from "../lib/ui/Row.svelte";
  import Select from "../lib/ui/Select.svelte";
  import Toggle from "../lib/ui/Toggle.svelte";
  import BrowserBridge from "./BrowserBridge.svelte";
  import HotkeyField, { type HotkeyStatus } from "./HotkeyField.svelte";
  import type { PaneContext } from "./panes";
  import Players from "./Players.svelte";
  import SingToFind from "./SingToFind.svelte";

  let { ctx }: { ctx: PaneContext } = $props();
  const t = $derived(ctx.t);
  const settings = $derived(ctx.settings);
  const app = $derived(ctx.app);
  const hotkey = $derived(hotkeyLabel(settings.hotkey, app.platform));

  let hotkeyStatus = $state<HotkeyStatus>({ kind: "idle" });
  const modifiers = $derived(
    t("settings.hotkeyModifiers", {
      ctrl: hotkeyLabel("Ctrl", app.platform),
      alt: hotkeyLabel("Alt", app.platform),
      super: hotkeyLabel("Super", app.platform),
    }),
  );
  const hotkeyHint = $derived.by(() => {
    const status = hotkeyStatus;
    if (status.kind === "recording") {
      if (status.problem === "needsModifier") return t("settings.hotkeyNeedsModifier", { modifiers });
      if (status.problem === "unsupported") return t("settings.hotkeyUnsupported");
      return t("settings.hotkeyRecording");
    }
    if (status.kind === "failed") {
      const tried = hotkeyLabel(status.hotkey, app.platform);
      if (status.error === "needsModifier") return t("settings.hotkeyNeedsModifier", { modifiers });
      if (status.error !== "taken") return t("settings.hotkeyUnsupported");
      // The old shortcut only keeps working if it worked before.
      return app.hotkeyError
        ? t("settings.hotkeyError", { hotkey: tried })
        : t("settings.hotkeyTaken", { hotkey: tried, previous: hotkey });
    }
    return app.hotkeyError ? t("settings.hotkeyError", { hotkey }) : t("settings.hotkeyHint");
  });
  const hotkeyWarning = $derived(
    hotkeyStatus.kind === "failed" ||
      (hotkeyStatus.kind === "recording" && hotkeyStatus.problem !== null) ||
      (hotkeyStatus.kind === "idle" && app.hotkeyError),
  );

  const sources: SourcePreference[] = ["automatic", "spotify", "appleMusic", "amazonMusic", "youTubeMusic"];
  const languages: Language[] = ["system", "de", "en"];
</script>

<Row icon="note" title={t("settings.source")} hint={t("settings.sourceHint")}>
  <Select
    label={t("settings.source")}
    value={settings.source}
    disabled={app.demo}
    options={sources.map((value) => ({ value, label: t(`source.${value}` as MessageKey) }))}
    onchange={(source) => ctx.update({ source })}
  />
</Row>

<Players {ctx} />

<BrowserBridge />

<SingToFind {ctx} />

<Row icon="eyeOff" title={t("settings.autoHide")} hint={t("settings.autoHideHint")}>
  <Toggle label={t("settings.autoHide")} checked={settings.autoHide} onchange={(autoHide) => ctx.update({ autoHide })} />
</Row>

{#if app.platform === "macos"}
  <Row icon="display" title={t("settings.showInDock")} hint={t("settings.showInDockHint")}>
    <Toggle label={t("settings.showInDock")} checked={settings.showInDock} onchange={(showInDock) => ctx.update({ showInDock })} />
  </Row>
{/if}

<Row icon="power" title={t("settings.launchAtLogin")} hint={t("settings.launchAtLoginHint")}>
  <Toggle label={t("settings.launchAtLogin")} checked={app.launchAtLogin} onchange={(enabled) => ctx.setLaunchAtLogin(enabled)} />
</Row>

<Row icon="globe" title={t("settings.language")} hint={t("settings.languageHint")}>
  <Select
    label={t("settings.language")}
    value={settings.language}
    options={languages.map((value) => ({ value, label: t(`language.${value}` as MessageKey) }))}
    onchange={(language) => ctx.update({ language })}
  />
</Row>

<Row icon="keyboard" title={t("settings.hotkey")} hint={hotkeyHint} warning={hotkeyWarning}>
  <HotkeyField
    hotkey={settings.hotkey}
    defaultHotkey={app.defaultHotkey}
    platform={app.platform}
    {t}
    onchange={(next) => ctx.setHotkey(next)}
    bind:status={hotkeyStatus}
  />
</Row>

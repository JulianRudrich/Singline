<script lang="ts">
  import { onMount, tick } from "svelte";
  import { api, subscribe } from "../lib/api";
  import {
    applyEdit,
    canSave,
    caretLine,
    checkDraft,
    draftStatus,
    isStampShortcut,
    normalizeNewlines,
    ownLyricsErrorKey,
    stampErrorKey,
    STAMP_SHORTCUT,
  } from "../lib/editor";
  import { hotkeyLabel } from "../lib/hotkey";
  import { translator, type MessageKey } from "../lib/i18n";
  import type { AppInfo, EditorState, StampEdit } from "../lib/types";
  import Glyph from "../lib/ui/Glyph.svelte";

  let app = $state<AppInfo | null>(null);
  /** The song this window belongs to; it stays the same when the song changes. */
  let editor = $state<EditorState | null>(null);
  let loaded = $state(false);
  /** The lyrics as saved, to tell whether the draft changed. */
  let saved = $state("");
  let text = $state("");
  let message = $state<MessageKey | null>(null);
  let saving = $state(false);
  let stamping = false;
  let textarea = $state<HTMLTextAreaElement>();

  const language = $derived(app?.language ?? "en");
  const t = $derived(translator(language));
  const platform = $derived(app?.platform ?? "other");
  const shortcut = $derived(hotkeyLabel(STAMP_SHORTCUT, platform));
  const check = $derived(checkDraft(text));
  const status = $derived(draftStatus(check, t, language));
  const savable = $derived(editor !== null && !saving && canSave(check, text, saved));

  $effect(() => {
    document.title = t("editor.title");
  });

  async function stamp() {
    if (!textarea || !editor || stamping || check.kind === "tooLarge") return;
    stamping = true;
    const draft = textarea.value;
    try {
      const edit = await api.stampLine(draft, textarea.selectionStart);
      // Typing while the stamp was on its way moved the text; the edit no longer fits.
      if (textarea.value === draft) await apply(textarea, edit);
    } catch (error) {
      message = stampErrorKey(error);
    } finally {
      stamping = false;
    }
  }

  async function apply(area: HTMLTextAreaElement, edit: StampEdit) {
    area.focus();
    area.setSelectionRange(edit.start, edit.start + edit.length);
    // Typing the stamp in keeps it in the field's undo history.
    if (!document.execCommand("insertText", false, edit.replacement)) {
      text = applyEdit(area.value, edit);
      await tick();
    }
    area.setSelectionRange(edit.nextCaret, edit.nextCaret);
    reveal(area, edit.nextCaret);
    message = null;
  }

  /** Scrolls the field so the caret's line and the one after it are visible. */
  function reveal(area: HTMLTextAreaElement, caret: number) {
    const style = getComputedStyle(area);
    const lineHeight = Number.parseFloat(style.lineHeight) || 20;
    const padding = Number.parseFloat(style.paddingTop) || 0;
    const top = padding + caretLine(area.value, caret) * lineHeight;
    if (top < area.scrollTop + padding) {
      area.scrollTop = Math.max(0, top - padding);
    } else if (top + 2 * lineHeight > area.scrollTop + area.clientHeight) {
      area.scrollTop = top + 2 * lineHeight - area.clientHeight;
    }
    area.scrollLeft = 0;
  }

  async function save() {
    if (!editor || !savable) return;
    saving = true;
    message = null;
    try {
      await api.saveLocalLyrics(editor.trackKey, text);
      saved = text;
      await api.closeEditor();
    } catch (error) {
      message = ownLyricsErrorKey(error, "save");
    } finally {
      saving = false;
    }
  }

  function keydown(event: KeyboardEvent) {
    if (!isStampShortcut(event, platform)) return;
    event.preventDefault();
    if (!event.repeat) void stamp();
  }

  onMount(() => {
    let stop: (() => void) | undefined;
    let alive = true;
    void (async () => {
      const unlisten = await subscribe({ app: (next) => (app = next) });
      if (!alive) return unlisten();
      stop = unlisten;
      const [snapshot, state] = await Promise.all([api.snapshot(), api.editorState()]);
      app = snapshot.app;
      editor = state;
      saved = normalizeNewlines(state?.lrc ?? "");
      text = saved;
      loaded = true;
      await tick();
      // Start at the first line, where stamping usually begins.
      textarea?.focus();
      textarea?.setSelectionRange(0, 0);
    })();
    return () => {
      alive = false;
      stop?.();
    };
  });
</script>

<svelte:window onkeydown={keydown} />

{#if app && loaded}
  <main class="editor-window">
    <header>
      <h1>{t("editor.heading")}</h1>
      {#if editor}
        <p class="track">{[editor.title, editor.artist].filter(Boolean).join(" · ")}</p>
      {/if}
    </header>

    {#if editor}
      <p class="hint">{t("editor.hint", { shortcut })}</p>
      <textarea
        bind:this={textarea}
        bind:value={text}
        oninput={() => (message = null)}
        aria-label={t("editor.textLabel")}
        placeholder={t("editor.placeholder")}
        spellcheck="false"
        autocomplete="off"
        autocapitalize="off"
        wrap="off"
      ></textarea>

      <div class="row">
        <button
          onclick={stamp}
          disabled={check.kind === "tooLarge"}
          title={t("editor.stampHelp", { shortcut })}
          aria-keyshortcuts={platform === "macos" ? "Meta+Shift+T" : "Control+Shift+T"}
        >
          {t("editor.stamp")}
          <kbd aria-hidden="true">{shortcut}</kbd>
        </button>
        <span class="status" aria-live="polite">{status}</span>
      </div>
      {#if message}
        <p class="message" role="alert"><Glyph name="alert" size={14} />{t(message, { title: editor.title })}</p>
      {/if}

      <div class="actions">
        <button onclick={() => api.closeEditor()}>{t("editor.cancel")}</button>
        <button class="primary" onclick={save} disabled={!savable}>{t("editor.save")}</button>
      </div>
    {:else}
      <p class="hint">{t("ownLyrics.noTrack")}</p>
    {/if}
  </main>
{/if}

<style>
  /* Linory's dark glass in black and white, whatever the system appearance. */
  :global(html[data-view="editor"]) {
    color-scheme: dark;
  }

  :global(html[data-view="editor"] body) {
    background: #0e0f11;
    color: #fff;
  }

  .editor-window {
    box-sizing: border-box;
    height: 100vh;
    display: flex;
    flex-direction: column;
    gap: 14px;
    padding: 22px 24px 20px;
    background:
      radial-gradient(120% 70% at 0% 0%, rgb(255 255 255 / 0.07), transparent 60%),
      linear-gradient(160deg, #1b1c1f, #0c0d0f);
    color: #fff;
  }

  header {
    display: flex;
    flex-direction: column;
    gap: 4px;
    min-width: 0;
  }

  h1 {
    margin: 0;
    font-size: 20px;
    font-weight: 600;
    letter-spacing: -0.01em;
  }

  .track {
    margin: 0;
    font-size: 13px;
    color: rgb(255 255 255 / 0.6);
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
  }

  .hint {
    margin: -2px 0 0;
    font-size: 11.5px;
    line-height: 1.45;
    color: rgb(255 255 255 / 0.55);
  }

  /* The text field is a card: radius 12, white 5 % fill, white 8 % border. */
  textarea {
    flex: 1;
    min-height: 160px;
    box-sizing: border-box;
    width: 100%;
    margin: 0;
    padding: 12px 14px;
    resize: none;
    border-radius: 12px;
    border: 1px solid rgb(255 255 255 / 0.08);
    background: rgb(255 255 255 / 0.05);
    color: rgb(255 255 255 / 0.92);
    caret-color: #fff;
    font-family: ui-monospace, "SF Mono", Menlo, "Cascadia Mono", Consolas, "DejaVu Sans Mono", "Liberation Mono", monospace;
    font-size: 13px;
    line-height: 1.55;
    white-space: pre;
    overflow: auto;
    transition:
      border-color var(--duration-press) var(--ease-out),
      background-color var(--duration-press) var(--ease-out);
  }

  textarea::placeholder {
    color: rgb(255 255 255 / 0.35);
  }

  textarea::selection {
    background: rgb(255 255 255 / 0.28);
    color: #fff;
  }

  textarea:focus {
    outline: none;
    border-color: rgb(255 255 255 / 0.3);
    background: rgb(255 255 255 / 0.07);
  }

  .row,
  .actions {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .status {
    flex: 1;
    text-align: right;
    font-size: 11.5px;
    color: rgb(255 255 255 / 0.55);
    font-variant-numeric: tabular-nums;
  }

  /* No colour for problems: they stand out by brightness and the sign. */
  .message {
    display: flex;
    align-items: flex-start;
    gap: 6px;
    margin: -6px 0 0;
    font-size: 11.5px;
    line-height: 1.35;
    color: #fff;
  }

  .message :global(svg) {
    flex: none;
    margin-top: 0.5px;
  }

  .actions {
    justify-content: flex-end;
  }

  button {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    padding: 6px 14px;
    border: 0;
    border-radius: 8px;
    background: rgb(255 255 255 / 0.1);
    color: #fff;
    font: inherit;
    font-size: 13px;
    font-weight: 500;
    cursor: pointer;
    transition:
      transform var(--duration-press) var(--ease-out),
      background-color var(--duration-press) var(--ease-out);
  }

  button:hover:not(:disabled) {
    background: rgb(255 255 255 / 0.16);
  }

  button:active:not(:disabled) {
    transform: scale(var(--press-scale-subtle));
  }

  button:focus-visible {
    outline: 2px solid rgb(255 255 255 / 0.85);
    outline-offset: 2px;
  }

  button:disabled {
    opacity: 0.35;
    cursor: default;
  }

  /* White is the only accent. */
  button.primary {
    background: #fff;
    color: #0c0d0f;
  }

  button.primary:hover:not(:disabled) {
    background: rgb(255 255 255 / 0.86);
  }

  kbd {
    padding: 1px 6px;
    border-radius: 5px;
    background: rgb(255 255 255 / 0.1);
    color: rgb(255 255 255 / 0.7);
    font: inherit;
    font-size: 11px;
  }

</style>

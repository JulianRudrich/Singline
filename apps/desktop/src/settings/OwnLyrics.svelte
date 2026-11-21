<script lang="ts">
  import { api } from "../lib/api";
  import { ownLyricsErrorKey, type OwnLyricsAction } from "../lib/editor";
  import type { MessageKey, Translate } from "../lib/i18n";
  import type { LyricsView, Track } from "../lib/types";
  import Button from "../lib/ui/Button.svelte";
  import Card from "../lib/ui/Card.svelte";
  import Glyph from "../lib/ui/Glyph.svelte";
  import Row from "../lib/ui/Row.svelte";

  // Your own lyrics for the song that plays: open the editor, import a file, remove them.
  let { track, lyrics, t }: { track: Track; lyrics: LyricsView | null; t: Translate } = $props();

  let failure = $state<{ trackKey: string; message: MessageKey } | null>(null);
  let busy = $state(false);

  const own = $derived(lyrics?.status === "ready" && lyrics.origin === "local");
  // A message belongs to the song it was about and disappears with it.
  const message = $derived(failure?.trackKey === track.key ? failure.message : null);

  async function run(action: OwnLyricsAction, task: (trackKey: string) => Promise<unknown>) {
    const trackKey = track.key;
    failure = null;
    busy = true;
    try {
      await task(trackKey);
    } catch (error) {
      failure = { trackKey, message: ownLyricsErrorKey(error, action) };
    } finally {
      busy = false;
    }
  }
</script>

<Card label={t("ownLyrics.title")}>
  <Row card={false} icon="document" title={t("ownLyrics.title")} hint={own ? t("ownLyrics.activeHint") : t("ownLyrics.hint")} />
  <div class="buttons">
    <Button disabled={busy} onclick={() => run("edit", () => api.openEditor())}>{t("ownLyrics.edit")}</Button>
    <Button disabled={busy} onclick={() => run("import", (trackKey) => api.importLyricsFile(trackKey, track.title))}>
      {t("ownLyrics.import")}
    </Button>
    {#if own}
      <Button disabled={busy} onclick={() => run("remove", (trackKey) => api.removeLocalLyrics(trackKey))}>
        {t("ownLyrics.remove")}
      </Button>
    {/if}
  </div>
  {#if message}
    <p class="message" role="alert"><Glyph name="alert" size={14} />{t(message)}</p>
  {/if}
</Card>

<style>
  /* Under the title, in line with the text. */
  .buttons,
  .message {
    margin-left: 40px;
  }

  .buttons {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }

  /* No colour for problems: they stand out by brightness and the sign. */
  .message {
    display: flex;
    align-items: flex-start;
    gap: 6px;
    margin-top: 0;
    margin-bottom: 0;
    font-size: 11.5px;
    line-height: 1.35;
    color: #fff;
  }

  .message :global(svg) {
    flex: none;
    margin-top: 0.5px;
  }
</style>

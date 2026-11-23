<script lang="ts">
  // "Also follow other players", and below it every player that reports right
  // now or was switched off, each with a switch: a player switched off is
  // never followed, such as a browser that plays videos next to the music.
  import { onMount } from "svelte";
  import { api } from "../lib/api";
  import type { SeenPlayer } from "../lib/types";
  import Card from "../lib/ui/Card.svelte";
  import Row from "../lib/ui/Row.svelte";
  import Toggle from "../lib/ui/Toggle.svelte";
  import type { PaneContext } from "./panes";

  let { ctx }: { ctx: PaneContext } = $props();
  const t = $derived(ctx.t);
  const settings = $derived(ctx.settings);
  const app = $derived(ctx.app);

  /** How often the list looks for players that started or quit. */
  const REFRESH_MS = 3000;

  let seen = $state<SeenPlayer[]>([]);

  const same = (a: string, b: string) => a.toLowerCase() === b.toLowerCase();
  const ignored = (name: string) => settings.ignoredPlayers.some((entry) => same(entry, name));
  // The players reporting now and the ones switched off, A to Z.
  const listed = $derived(
    [
      ...seen.map((player) => player.name),
      ...settings.ignoredPlayers.filter((name) => !seen.some((player) => same(player.name, name))),
    ].sort((a, b) => a.localeCompare(b)),
  );

  function setFollowed(name: string, follow: boolean) {
    const others = settings.ignoredPlayers.filter((entry) => !same(entry, name));
    ctx.update({ ignoredPlayers: follow ? others : [...others, name] });
  }

  onMount(() => {
    let alive = true;
    const refresh = async () => {
      try {
        const next = await api.seenPlayers();
        if (alive) seen = next;
      } catch (error) {
        console.error("could not list the players", error);
      }
    };
    void refresh();
    const timer = setInterval(refresh, REFRESH_MS);
    return () => {
      alive = false;
      clearInterval(timer);
    };
  });
</script>

<Card>
  <Row
    card={false}
    icon="players"
    title={t("settings.includeOther")}
    hint={listed.length > 0 ? t("settings.includeOtherListHint") : t("settings.includeOtherHint")}
  >
    <Toggle
      label={t("settings.includeOther")}
      checked={settings.includeOtherPlayers}
      disabled={app.demo || settings.source !== "automatic"}
      onchange={(includeOtherPlayers) => ctx.update({ includeOtherPlayers })}
    />
  </Row>
  {#each listed as name (name.toLowerCase())}
    <Row card={false} icon="note" title={name} hint={ignored(name) ? t("settings.playerIgnored") : undefined}>
      <Toggle
        label={t("settings.followPlayer", { name })}
        checked={!ignored(name)}
        disabled={app.demo}
        onchange={(follow) => setFollowed(name, follow)}
      />
    </Row>
  {/each}
</Card>

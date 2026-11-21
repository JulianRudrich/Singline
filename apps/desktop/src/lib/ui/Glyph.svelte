<script module lang="ts">
  export type GlyphName =
    | "general"
    | "appearance"
    | "timing"
    | "about"
    | "note"
    | "players"
    | "power"
    | "globe"
    | "keyboard"
    | "search"
    | "code"
    | "license"
    | "lyrics"
    | "external"
    | "chevrons"
    | "eyeOff"
    | "shuffle"
    | "overlay"
    | "pointer"
    | "moon"
    | "chart"
    | "chevronRight"
    | "mic"
    | "display"
    | "download"
    | "alert"
    | "document";
</script>

<script lang="ts">
  // Line icons for the settings, on a 24 × 24 grid in the current text colour.
  let { name, size = 16 }: { name: GlyphName; size?: number } = $props();
</script>

<svg
  width={size}
  height={size}
  viewBox="0 0 24 24"
  fill="none"
  stroke="currentColor"
  stroke-width="1.8"
  stroke-linecap="round"
  stroke-linejoin="round"
  aria-hidden="true"
>
  {#if name === "general"}
    <path d="M4 6h8M16 6h4M4 12h2M10 12h10M4 18h10M18 18h2" />
    <circle cx="14" cy="6" r="2" />
    <circle cx="8" cy="12" r="2" />
    <circle cx="16" cy="18" r="2" />
  {:else if name === "appearance"}
    <path d="M12 3 3 7.5l9 4.5 9-4.5L12 3Z" />
    <path d="m3 12 9 4.5 9-4.5M3 16.5l9 4.5 9-4.5" />
  {:else if name === "timing"}
    <circle cx="12" cy="13.5" r="7.5" />
    <path d="M12 13.5V10M10 2.5h4M12 2.5V6M18.5 7l1.5-1.5" />
  {:else if name === "about"}
    <circle cx="12" cy="12" r="9" />
    <path d="M12 11v5.5M12 7.5h.01" />
  {:else if name === "alert"}
    <circle cx="12" cy="12" r="9" />
    <path d="M12 7.5V13M12 16.5h.01" />
  {:else if name === "note"}
    <path d="M9 18V5l12-2v13" />
    <circle cx="6" cy="18" r="3" />
    <circle cx="18" cy="16" r="3" />
  {:else if name === "eyeOff"}
    <path d="M3 3l18 18M10.6 5.1A9.6 9.6 0 0 1 12 5c5.5 0 9 7 9 7a16.5 16.5 0 0 1-3.2 4.2M6.6 6.6C4.3 8.2 3 12 3 12s3.5 7 9 7a9.3 9.3 0 0 0 4.4-1.1M9.9 9.9a3 3 0 0 0 4.2 4.2" />
  {:else if name === "shuffle"}
    <path d="m18 14 4 4-4 4M18 2l4 4-4 4M2 18h2a4 4 0 0 0 3.3-1.7l5.4-8.6A4 4 0 0 1 16 6h6M2 6h2a4 4 0 0 1 3.6 2.2M22 18h-6a4 4 0 0 1-3.3-1.8l-.4-.5" />
  {:else if name === "overlay"}
    <rect x="3" y="8.5" width="13" height="11" rx="2" />
    <path d="M8 8.5V6.5a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v7a2 2 0 0 1-2 2h-3" />
  {:else if name === "pointer"}
    <path d="M5 3.5 19 10l-6.2 1.8L10 18 5 3.5Z" />
    <path d="m13.5 13.5 5 5" />
  {:else if name === "moon"}
    <path d="M20 14.2A8.2 8.2 0 1 1 9.8 4a6.6 6.6 0 0 0 10.2 10.2Z" />
  {:else if name === "chart"}
    <path d="M4 20h16M7 16.5v-5M12 16.5V6.5M17 16.5v-8" />
  {:else if name === "mic"}
    <rect x="9" y="3" width="6" height="11" rx="3" />
    <path d="M5.5 11a6.5 6.5 0 0 0 13 0M12 17.5V21M8.5 21h7" />
  {:else if name === "display"}
    <rect x="3" y="4" width="18" height="12" rx="2" />
    <path d="M9 20h6M12 16v4" />
  {:else if name === "chevronRight"}
    <path d="m9.5 6 6 6-6 6" />
  {:else if name === "players"}
    <rect x="3" y="4.5" width="18" height="15" rx="2.5" />
    <path d="M3 9h18M10.5 12.25v4.5l3.75-2.25-3.75-2.25Z" />
  {:else if name === "power"}
    <path d="M12 3v8M6.6 6.6a7.5 7.5 0 1 0 10.8 0" />
  {:else if name === "globe"}
    <circle cx="12" cy="12" r="9" />
    <path d="M3 12h18M12 3c2.5 2.6 3.8 5.6 3.8 9s-1.3 6.4-3.8 9c-2.5-2.6-3.8-5.6-3.8-9S9.5 5.6 12 3Z" />
  {:else if name === "keyboard"}
    <rect x="2.5" y="6" width="19" height="12" rx="2.5" />
    <path d="M6.5 10h.01M10 10h.01M14 10h.01M17.5 10h.01M8 14h8" />
  {:else if name === "search"}
    <circle cx="11" cy="11" r="6.5" />
    <path d="m20 20-4.4-4.4" />
  {:else if name === "code"}
    <path d="m8.5 7-5 5 5 5M15.5 7l5 5-5 5M13.5 4.5l-3 15" />
  {:else if name === "license" || name === "document"}
    <path d="M6 3h7l5 5v13H6V3Z" />
    <path d="M13 3v5h5M9 13h6M9 16.5h6" />
  {:else if name === "lyrics"}
    <path d="M4.5 5h15A1.5 1.5 0 0 1 21 6.5v9a1.5 1.5 0 0 1-1.5 1.5H10l-4.5 3.5V17h-1A1.5 1.5 0 0 1 3 15.5v-9A1.5 1.5 0 0 1 4.5 5Z" />
    <path d="M7.5 9.5h9M7.5 13h6" />
  {:else if name === "download"}
    <path d="M12 4v11M7.5 10.5 12 15l4.5-4.5M5 19.5h14" />
  {:else if name === "external"}
    <path d="M8 16 16 8M9.5 8H16v6.5" />
  {:else}
    <path d="m8 9.5 4-4 4 4M8 14.5l4 4 4-4" />
  {/if}
</svg>

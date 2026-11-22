<script lang="ts">
  // A switch. On is white with a dark knob: no accent colour. While pressed the
  // knob stretches towards the other side, as on the Mac.
  let {
    checked,
    label,
    disabled = false,
    onchange,
  }: { checked: boolean; label: string; disabled?: boolean; onchange: (checked: boolean) => void } = $props();
</script>

<button type="button" role="switch" class="toggle" aria-checked={checked} aria-label={label} {disabled} onclick={() => onchange(!checked)}>
  <span class="knob"></span>
</button>

<style>
  .toggle {
    position: relative;
    flex: none;
    width: 38px;
    height: 22px;
    padding: 0;
    border: 0;
    border-radius: 11px;
    background: rgb(255 255 255 / 0.16);
    cursor: default;
    transition: background-color var(--duration-pane) var(--ease-out);
  }

  .toggle[aria-checked="true"] {
    background: #fff;
  }

  .knob {
    position: absolute;
    top: 2px;
    left: 2px;
    width: 18px;
    height: 18px;
    border-radius: 9px;
    background: #fff;
    box-shadow: 0 1px 2px rgb(0 0 0 / 0.35);
    transition:
      transform var(--duration-pane) var(--ease-out),
      width var(--duration-press) var(--ease-out),
      background-color var(--duration-pane) var(--ease-out);
  }

  .toggle[aria-checked="true"] .knob {
    transform: translateX(16px);
    background: #161618;
  }

  .toggle:active:not(:disabled) .knob {
    width: 23px;
  }

  /* On, it grows to the left, so its right edge stays put. */
  .toggle[aria-checked="true"]:active:not(:disabled) .knob {
    transform: translateX(11px);
  }

  .toggle:disabled {
    opacity: 0.4;
  }

  .toggle:focus-visible {
    outline: 2px solid rgb(255 255 255 / 0.7);
    outline-offset: 2px;
  }

  @media (prefers-reduced-motion: reduce) {
    .knob {
      transition-property: background-color;
    }

    .toggle:active:not(:disabled) .knob {
      width: 18px;
    }

    .toggle[aria-checked="true"]:active:not(:disabled) .knob {
      transform: translateX(16px);
    }
  }
</style>

<script lang="ts">
  import { Tween } from "svelte/motion";
  import { cubicOut } from "svelte/easing";

  let {
    label,
    value,
    max,
    format,
    sub = "",
    color = "var(--accent)",
  }: {
    label: string;
    value: number | null;
    max: number;
    format: (v: number) => string;
    sub?: string;
    color?: string;
  } = $props();

  // O ponteiro e o número deslizam até o valor novo em vez de pular.
  const current = Tween.of(() => value ?? 0, { duration: 700, easing: cubicOut });

  // Arco de 270°, aberto embaixo.
  const R = 42;
  const ARC = 2 * Math.PI * R * 0.75;
  let fraction = $derived(Math.min(Math.max(current.current / max, 0), 1));
</script>

<div class="gauge">
  <svg viewBox="0 0 100 100" aria-hidden="true">
    <circle class="track" cx="50" cy="50" r={R} stroke-dasharray="{ARC} 999" />
    {#if value != null}
      <circle
        class="fill"
        cx="50"
        cy="50"
        r={R}
        stroke={color}
        stroke-dasharray="{ARC * fraction} 999"
        style:filter="drop-shadow(0 0 6px {color})"
      />
    {/if}
  </svg>
  <div class="center">
    <strong class="num">{value == null ? "—" : format(current.current)}</strong>
    {#if sub}<span class="muted">{sub}</span>{/if}
  </div>
  <span class="label">{label}</span>
</div>

<style>
  .gauge {
    position: relative;
    display: flex;
    flex-direction: column;
    align-items: center;
    min-width: 0;
  }

  svg {
    width: 100%;
    max-width: 150px;
    transform: rotate(135deg);
    overflow: visible;
  }

  circle {
    fill: none;
    stroke-width: 7;
    stroke-linecap: round;
  }

  .track {
    stroke: var(--surface-2);
  }

  .fill {
    transition: stroke 0.6s;
  }

  .center {
    position: absolute;
    top: 0;
    left: 0;
    right: 0;
    aspect-ratio: 1;
    max-width: 150px;
    margin: auto;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
  }

  strong {
    font-size: 22px;
    font-weight: 700;
    letter-spacing: -0.02em;
  }

  .center span {
    font-size: 11px;
  }

  .label {
    margin-top: -6px;
    font-size: 12px;
    font-weight: 600;
    color: var(--muted);
    text-align: center;
  }
</style>

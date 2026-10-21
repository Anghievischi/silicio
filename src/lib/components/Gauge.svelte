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

  // O número e a barra deslizam até o valor novo em vez de pular.
  const current = Tween.of(() => value ?? 0, { duration: 700, easing: cubicOut });

  // Medidor em blocos, como num painel de instrumentos.
  const SEGMENTS = 20;
  let lit = $derived(value == null ? 0 : Math.round(Math.min(Math.max(current.current / max, 0), 1) * SEGMENTS));
</script>

<div class="gauge">
  <span class="label">{label}</span>
  <strong class="num">{value == null ? "—" : format(current.current)}</strong>
  <div class="bar" aria-hidden="true">
    {#each Array(SEGMENTS) as _, i}
      <span style:background={i < lit ? color : undefined}></span>
    {/each}
  </div>
  <span class="sub muted">{sub || " "}</span>
</div>

<style>
  .gauge {
    display: flex;
    flex-direction: column;
    gap: 8px;
    min-width: 0;
    padding: 14px 16px;
    border: 2px solid var(--border);
    background: var(--surface);
  }

  .label {
    font-size: 11px;
    font-weight: 700;
    letter-spacing: 0.1em;
    text-transform: uppercase;
    color: var(--muted);
  }

  strong {
    font-family: var(--font-display);
    font-size: 52px;
    font-weight: 900;
    line-height: 0.9;
  }

  .bar {
    display: grid;
    grid-template-columns: repeat(20, minmax(0, 1fr));
    gap: 3px;
    height: 12px;
  }

  .bar span {
    background: var(--surface-2);
    transition: background 0.3s;
  }

  .sub {
    font-size: 11px;
    white-space: pre;
    overflow: hidden;
    text-overflow: ellipsis;
  }
</style>

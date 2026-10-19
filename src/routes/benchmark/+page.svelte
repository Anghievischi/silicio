<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { onMount } from "svelte";
  import Card from "$lib/components/Card.svelte";
  import Sparkline from "$lib/components/Sparkline.svelte";
  import { ghz } from "$lib/format";
  import type { BenchResult, CpuInfo, CpuLive, StressStatus } from "$lib/types";

  type Run = { date: string; seconds: number; single: number; multi: number; threads: number };

  const HISTORY_KEY = "silicio.bench.history";
  const DURATIONS = [5, 10, 30];
  const LIVE_POINTS = 120;

  let cpu = $state<CpuInfo | null>(null);
  let duration = $state(10);
  let phase = $state<"idle" | "single" | "multi">("idle");
  let phaseStart = $state(0);
  let now = $state(Date.now());
  let single = $state<BenchResult | null>(null);
  let multi = $state<BenchResult | null>(null);
  let history = $state<Run[]>([]);
  let error = $state<string | null>(null);

  let stress = $state<StressStatus | null>(null);
  let live = $state<CpuLive | null>(null);
  let clocks = $state<number[]>([]);
  let temps = $state<number[]>([]);
  let peakMhz = $state(0);

  let progress = $derived(phase === "idle" ? 0 : Math.min(1, (now - phaseStart) / 1000 / duration));
  let ratio = $derived(single && multi ? multi.score / single.score : null);
  let best = $derived({
    single: Math.max(1, single?.score ?? 0, ...history.map((h) => h.single)),
    multi: Math.max(1, multi?.score ?? 0, ...history.map((h) => h.multi)),
  });
  // Queda sustentada de clock sob carga total costuma indicar limite térmico ou de energia.
  let recentMhz = $derived(clocks.length >= 10 ? clocks.slice(-10).reduce((a, b) => a + b, 0) / 10 : null);
  let throttling = $derived(
    !!stress?.running && stress.seconds > 20 && peakMhz > 0 && recentMhz != null && recentMhz < peakMhz * 0.85,
  );

  function loadHistory() {
    try {
      history = JSON.parse(localStorage.getItem(HISTORY_KEY) ?? "[]");
    } catch {
      history = [];
    }
  }

  function saveHistory() {
    try {
      localStorage.setItem(HISTORY_KEY, JSON.stringify(history));
    } catch {
      // Sem armazenamento disponível, o histórico vale só para esta sessão.
    }
  }

  async function runBenchmark() {
    error = null;
    single = multi = null;
    try {
      phase = "single";
      phaseStart = Date.now();
      single = await invoke<BenchResult>("bench_run", { threads: 1, seconds: duration });
      phase = "multi";
      phaseStart = Date.now();
      multi = await invoke<BenchResult>("bench_run", { threads: null, seconds: duration });
      history = [
        { date: new Date().toISOString(), seconds: duration, single: single.score, multi: multi.score, threads: multi.threads },
        ...history,
      ].slice(0, 10);
      saveHistory();
    } catch (e) {
      error = String(e);
    } finally {
      phase = "idle";
    }
  }

  async function toggleStress() {
    error = null;
    try {
      if (stress?.running) {
        stress = await invoke<StressStatus>("stress_stop");
      } else {
        clocks = [];
        temps = [];
        peakMhz = 0;
        stress = await invoke<StressStatus>("stress_start");
      }
    } catch (e) {
      error = String(e);
    }
  }

  onMount(() => {
    loadHistory();
    invoke<CpuInfo>("cpu_info").then((c) => (cpu = c)).catch(() => {});
    invoke<StressStatus>("stress_status").then((s) => (stress = s)).catch(() => {});

    const tick = async () => {
      now = Date.now();
      if (!stress?.running) return;
      try {
        const [s, l] = await Promise.all([invoke<StressStatus>("stress_status"), invoke<CpuLive>("cpu_live")]);
        stress = s;
        live = l;
        clocks = [...clocks.slice(-(LIVE_POINTS - 1)), l.avg_mhz];
        if (l.temperature_c != null) temps = [...temps.slice(-(LIVE_POINTS - 1)), l.temperature_c];
        // O pico é medido nos primeiros segundos observados, antes de o calor acumular.
        if (s.seconds < 15 || clocks.length <= 15) peakMhz = Math.max(peakMhz, l.avg_mhz);
      } catch (e) {
        error = String(e);
      }
    };
    const timer = setInterval(tick, 1000);
    const clock = setInterval(() => (now = Date.now()), 200);
    return () => {
      clearInterval(timer);
      clearInterval(clock);
    };
  });

  const fmt = (n: number) => Math.round(n).toLocaleString("pt-BR");
  const time = (s: number) => `${Math.floor(s / 60)}:${String(Math.floor(s % 60)).padStart(2, "0")}`;
</script>

<header>
  <div>
    <span class="vendor">Benchmark</span>
    <h1>{cpu?.brand ?? "Processador"}</h1>
    <p class="muted">{cpu ? `${cpu.cores} núcleos · ${cpu.threads} threads` : ""}</p>
  </div>
</header>

{#if error}<p class="error">{error}</p>{/if}

<div class="grid">
  <Card title="Desempenho" span={2}>
    <div class="results">
      <div class="result">
        <span class="muted">Single-thread</span>
        <strong class="num">{single ? fmt(single.score) : phase === "single" ? "…" : "—"}</strong>
        <div class="bar"><div style:width="{single ? (single.score / best.single) * 100 : 0}%"></div></div>
      </div>
      <div class="result">
        <span class="muted">Multi-thread{multi ? ` (${multi.threads} threads)` : ""}</span>
        <strong class="num">{multi ? fmt(multi.score) : phase === "multi" ? "…" : "—"}</strong>
        <div class="bar"><div style:width="{multi ? (multi.score / best.multi) * 100 : 0}%"></div></div>
      </div>
      <div class="result">
        <span class="muted">Proporção multi ÷ single</span>
        <strong class="num">{ratio ? `${ratio.toFixed(2)}×` : "—"}</strong>
      </div>
    </div>

    <div class="controls">
      <label>
        Duração de cada etapa
        <select bind:value={duration} disabled={phase !== "idle"}>
          {#each DURATIONS as d}<option value={d}>{d} s</option>{/each}
        </select>
      </label>
      <button class="primary" onclick={runBenchmark} disabled={phase !== "idle" || !!stress?.running}>
        {#if phase === "idle"}Rodar benchmark{:else}{phase === "single" ? "Single-thread" : "Multi-thread"}… {Math.round(progress * 100)}%{/if}
      </button>
    </div>
    {#if phase !== "idle"}
      <div class="bar progress"><div style:width="{progress * 100}%"></div></div>
    {/if}
    <p class="hint muted">
      A pontuação é o número de blocos de trabalho (fractal de Mandelbrot + hash) concluídos por segundo. Compare só
      entre execuções do Silício. Feche outros programas para um resultado estável.
    </p>
  </Card>

  <Card title="Histórico">
    {#if history.length === 0}
      <p class="muted">Os últimos 10 resultados aparecem aqui.</p>
    {:else}
      <table>
        <thead>
          <tr><th>Data</th><th>Single</th><th>Multi</th></tr>
        </thead>
        <tbody>
          {#each history as h}
            <tr>
              <td class="muted">{new Date(h.date).toLocaleDateString("pt-BR")}</td>
              <td class="num">{fmt(h.single)}</td>
              <td class="num">{fmt(h.multi)}</td>
            </tr>
          {/each}
        </tbody>
      </table>
    {/if}
  </Card>

  <Card title="Teste de estresse" span={3}>
    <div class="stress-head">
      <div>
        <p class="muted">
          Carga máxima em todas as threads até você parar. Acompanhe clock e temperatura para ver se o processador reduz o
          clock por calor (throttling). Continua rodando se você trocar de aba.
        </p>
      </div>
      <button class="primary" class:danger={stress?.running} onclick={toggleStress} disabled={phase !== "idle"}>
        {stress?.running ? "Parar" : "Iniciar estresse"}
      </button>
    </div>

    {#if stress?.running || clocks.length > 0}
      <div class="stress-stats">
        <div><span class="muted">Tempo</span><strong class="num">{time(stress?.seconds ?? 0)}</strong></div>
        <div><span class="muted">Clock médio</span><strong class="num">{ghz(live?.avg_mhz)}</strong></div>
        <div><span class="muted">Pico inicial</span><strong class="num">{ghz(peakMhz)}</strong></div>
        <div>
          <span class="muted">Temperatura</span>
          <strong class="num">{live?.temperature_c != null ? `${live.temperature_c.toFixed(0)} °C` : "—"}</strong>
        </div>
        <div><span class="muted">Uso</span><strong class="num">{live ? `${live.usage.toFixed(0)}%` : "—"}</strong></div>
      </div>
      <div class="charts">
        <div>
          <span class="muted label">Clock (últimos 2 min)</span>
          <Sparkline values={clocks} max={Math.max(cpu?.clocks.max_mhz ?? 0, peakMhz, 1000)} capacity={LIVE_POINTS} />
        </div>
        <div>
          <span class="muted label">Temperatura</span>
          <Sparkline values={temps} max={105} capacity={LIVE_POINTS} />
        </div>
      </div>
      {#if throttling}
        <p class="warn">
          O clock médio dos últimos 10 s está mais de 15% abaixo do pico inicial. O processador provavelmente está limitado por temperatura
          ou energia; verifique a ventilação e o modo de energia.
        </p>
      {/if}
    {/if}
  </Card>
</div>

<style>
  th:not(:first-child),
  td:not(:first-child) {
    text-align: right;
  }

  td {
    font-size: 14px;
  }

  .results {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 16px;
  }

  .result {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .result span {
    font-size: 12px;
  }

  .result strong {
    font-size: 28px;
    font-weight: 700;
  }

  .bar {
    height: 6px;
    border-radius: 3px;
    background: var(--border);
    overflow: hidden;
  }

  .bar div {
    height: 100%;
    background: var(--accent);
    transition: width 0.3s ease;
  }

  .progress {
    margin-top: 12px;
  }

  .controls {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    margin-top: 20px;
  }

  label {
    display: flex;
    align-items: center;
    gap: 8px;
    color: var(--muted);
    font-size: 13px;
  }

  select {
    padding: 6px 8px;
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--surface-2);
    color: var(--text);
    font: inherit;
  }

  .primary {
    padding: 9px 18px;
    border: 0;
    border-radius: 9px;
    background: var(--accent);
    color: #fff;
    font-weight: 600;
  }

  .primary:disabled {
    opacity: 0.6;
    cursor: default;
  }

  .primary.danger {
    background: var(--hot);
  }

  .hint {
    margin: 14px 0 0;
    font-size: 11px;
  }

  .stress-head {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 16px;
  }

  .stress-head p {
    margin: 0;
    max-width: 75ch;
  }

  .stress-stats {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(120px, 1fr));
    gap: 12px;
    margin: 18px 0 12px;
  }

  .stress-stats div {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .stress-stats span {
    font-size: 12px;
  }

  .stress-stats strong {
    font-size: 20px;
    font-weight: 650;
  }

  .charts {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 16px;
  }

  .label {
    font-size: 11px;
  }

  .warn {
    margin: 12px 0 0;
    padding: 10px 12px;
    border-left: 3px solid var(--warn);
    border-radius: 8px;
    background: var(--surface-2);
    font-size: 13px;
  }

  @media (max-width: 760px) {
    .results,
    .charts {
      grid-template-columns: 1fr;
    }
  }
</style>

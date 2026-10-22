<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { onMount } from "svelte";
  import Card from "$lib/components/Card.svelte";
  import Sparkline from "$lib/components/Sparkline.svelte";
  import { bytes, clock, ghz } from "$lib/format";
  import { applyStatus, downsample, loadSession, session, sessionFindingText, startSession, stopSession, toCsv } from "$lib/gaming.svelte";
  import { i18n, t } from "$lib/i18n/index.svelte";
  import type { SessionReport, SessionSample, SessionStatus } from "$lib/types";

  const LIVE_POINTS = 60;

  let last = $state<SessionSample | null>(null);
  let seconds = $state(0);
  let gpuHistory = $state<number[]>([]);
  let threadHistory = $state<number[]>([]);
  let busy = $state(false);
  let error = $state<string | null>(null);
  let saved = $state<string | null>(null);

  let g = $derived(t().gaming);

  // Gráficos do laudo; os de temperatura só quando houve sensor.
  let charts = $derived.by(() => {
    const r = session.report;
    if (!r) return [];
    const list = [
      { title: g.chartGpu, max: 100, values: downsample(r.samples, (s) => s.gpu_usage ?? s.igpu_usage) },
      { title: g.chartThread, max: 100, values: downsample(r.samples, (s) => s.thread_max) },
    ];
    if (r.stats.cpu_temp) list.push({ title: g.chartCpuTemp, max: 105, values: downsample(r.samples, (s) => s.cpu_temp) });
    if (r.stats.gpu_temp) list.push({ title: g.chartGpuTemp, max: 105, values: downsample(r.samples, (s) => s.gpu_temp) });
    return list;
  });

  function track(status: SessionStatus) {
    applyStatus(status);
    seconds = status.seconds;
    if (status.last && status.last.t !== last?.t) {
      last = status.last;
      gpuHistory = [...gpuHistory.slice(-(LIVE_POINTS - 1)), status.last.gpu_usage ?? status.last.igpu_usage ?? 0];
      threadHistory = [...threadHistory.slice(-(LIVE_POINTS - 1)), status.last.thread_max];
    }
  }

  async function start() {
    busy = true;
    error = saved = null;
    last = null;
    gpuHistory = [];
    threadHistory = [];
    try {
      await startSession();
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }

  async function stop() {
    busy = true;
    try {
      await stopSession();
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }

  async function exportCsv(report: SessionReport) {
    const d = new Date();
    const pad = (n: number) => String(n).padStart(2, "0");
    const stamp = `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}-${pad(d.getHours())}${pad(d.getMinutes())}`;
    const name = `Silicio-${i18n.locale === "pt-BR" ? "sessao" : "session"}-${stamp}.csv`;
    try {
      saved = await invoke<string>("save_report", { name, contents: toCsv(report.samples) });
    } catch (e) {
      error = String(e);
    }
  }

  const open = (reveal: boolean) => invoke("open_report", { path: saved, reveal }).catch((e) => (error = String(e)));

  onMount(() => {
    loadSession()
      .then(track)
      .catch((e) => (error = String(e)));
    const timer = setInterval(async () => {
      if (!session.running) return;
      try {
        track(await invoke<SessionStatus>("session_status"));
      } catch (e) {
        error = String(e);
      }
    }, 1000);
    return () => clearInterval(timer);
  });

  const pct = (v: number | null | undefined) => (v == null ? "—" : `${v.toFixed(0)}%`);
  const temp = (v: number | null | undefined) => (v == null ? "—" : `${v.toFixed(0)} °C`);

  /** Valor de uma estatística já formatado. */
  function fmt(key: string, v: number): string {
    if (key.endsWith("_mhz")) return key === "cpu_mhz" ? ghz(v) : `${v.toFixed(0)} MHz`;
    if (key.endsWith("_temp")) return temp(v);
    if (key === "gpu_power") return `${v.toFixed(0)} W`;
    if (key.endsWith("_used")) return bytes(v);
    return pct(v);
  }
</script>

<header>
  <div>
    {#if session.running}<span class="vendor rec">● {g.recording}</span>{/if}
    <h1>{g.title}</h1>
    <p class="muted">{g.intro}</p>
  </div>
  {#if session.running}
    <button class="action stop" disabled={busy} onclick={stop}>{g.stop}</button>
  {:else}
    <button class="action" disabled={busy} onclick={start}>{busy ? g.starting : session.report ? g.newSession : g.start}</button>
  {/if}
</header>

{#if error}<p class="error">{g.error}: {error}</p>{/if}

{#if session.running}
  <section class="recording">
    <span class="time num">{clock(seconds)}</span>
    <p class="muted">{g.recordingHint}</p>
  </section>

  <div class="stats">
    <div class="stat"><span class="muted">{g.live.cpu}</span><strong class="num">{pct(last?.cpu_usage)}</strong></div>
    <div class="stat"><span class="muted">{g.live.thread}</span><strong class="num">{pct(last?.thread_max)}</strong></div>
    <div class="stat"><span class="muted">{g.live.gpu}</span><strong class="num">{pct(last?.gpu_usage ?? last?.igpu_usage)}</strong></div>
    <div class="stat"><span class="muted">{g.live.cpuTemp}</span><strong class="num">{temp(last?.cpu_temp)}</strong></div>
    <div class="stat"><span class="muted">{g.live.gpuTemp}</span><strong class="num">{temp(last?.gpu_temp)}</strong></div>
    <div class="stat"><span class="muted">{g.live.vram}</span><strong class="num">{bytes(last?.vram_used)}</strong></div>
    <div class="stat"><span class="muted">{g.live.ram}</span><strong class="num">{bytes(last?.ram_used)}</strong></div>
  </div>

  <div class="grid">
    <Card title={g.chartGpu} span={1}>
      <Sparkline values={gpuHistory} max={100} capacity={LIVE_POINTS} />
    </Card>
    <Card title={g.chartThread} span={1}>
      <Sparkline values={threadHistory} max={100} capacity={LIVE_POINTS} />
    </Card>
  </div>
{:else if session.report}
  {@const r = session.report}
  {@const verdict = g.verdict[r.verdict](r)}
  <section class="verdict {r.verdict}">
    <span class="label">{g.result}</span>
    <strong>{verdict.title}</strong>
    <p>{verdict.text}</p>
    <span class="muted small">{g.duration(clock(r.seconds), clock(r.load_seconds))}</span>
  </section>

  {#if r.verdict !== "idle"}
    <section class="limits">
      <span class="label-muted">{g.limitsTitle}</span>
      <div class="bar" aria-hidden="true">
        {#each r.limits as l}<span class="l-{l.code}" style:width="{l.pct}%"></span>{/each}
      </div>
      <ul class="legend">
        {#each r.limits as l}
          <li><span class="swatch l-{l.code}"></span>{g.limit[l.code]} <strong class="num">{l.pct}%</strong></li>
        {/each}
      </ul>
    </section>
  {/if}

  {#if r.findings.length}
    <ul class="findings">
      {#each r.findings as finding}
        {@const text = sessionFindingText(finding, r)}
        <li class={finding.severity}>
          <strong>{text.title}</strong>
          <p>{text.text}</p>
        </li>
      {/each}
    </ul>
  {:else if r.verdict !== "idle"}
    <p class="ok">{g.noIssues}</p>
  {/if}

  {#if r.verdict !== "idle"}
    <div class="grid">
      {#each charts as chart}
        <Card title={chart.title}>
          <Sparkline values={chart.values} max={chart.max} capacity={chart.values.length} />
        </Card>
      {/each}

      <Card title={g.stats} span={2}>
        <table>
          <thead><tr><th></th><th class="num">{g.avg}</th><th class="num">{g.max}</th></tr></thead>
          <tbody>
            {#each Object.entries(r.stats) as [key, s]}
              {#if s}
                <tr><th>{g.stat[key]}</th><td class="num">{fmt(key, s.avg)}</td><td class="num">{fmt(key, s.max)}</td></tr>
              {/if}
            {/each}
          </tbody>
        </table>
      </Card>
    </div>
  {/if}

  <div class="export">
    {#if saved}
      <span class="muted">{g.savedTo}</span>
      <code>{saved}</code>
      <button class="ghost" onclick={() => open(false)}>{t().report.open}</button>
      <button class="ghost" onclick={() => open(true)}>{t().report.reveal}</button>
    {:else}
      <button class="ghost" onclick={() => exportCsv(r)}>{g.exportCsv}</button>
    {/if}
  </div>
{:else}
  <section class="how">
    <ol>
      {#each g.howTo as step}<li>{step}</li>{/each}
    </ol>
  </section>
{/if}

<p class="muted note">{g.noFps}</p>

<style>
  .action {
    flex-shrink: 0;
    min-height: 44px;
    padding: 8px 18px;
    border: 2px solid var(--accent);
    background: var(--accent);
    color: var(--on-accent);
    font-size: 12px;
    font-weight: 700;
    letter-spacing: 0.1em;
    text-transform: uppercase;
  }

  .action.stop {
    border-color: var(--hot);
    background: var(--hot);
    color: #fff;
  }

  .action:disabled {
    opacity: 0.6;
  }

  .rec {
    background: var(--hot);
    color: #fff;
    animation: blink 2s steps(2, jump-none) infinite;
  }

  @keyframes blink {
    50% {
      opacity: 0.6;
    }
  }

  .recording,
  .how,
  .verdict {
    margin-bottom: 12px;
    padding: 16px 20px;
    border: 2px solid var(--border);
    background: var(--surface);
  }

  .time {
    font-family: var(--font-display);
    font-size: 64px;
    font-weight: 900;
    line-height: 0.9;
    color: var(--hot);
  }

  .recording p {
    margin: 8px 0 0;
    font-size: 13px;
  }

  .how ol {
    margin: 0;
    padding-left: 20px;
    line-height: 1.9;
  }

  .verdict {
    border-color: var(--accent);
  }

  .verdict.idle {
    border-color: var(--border);
  }

  .verdict .label {
    display: inline-block;
    padding: 2px 8px;
    background: var(--accent);
    color: var(--on-accent);
    font-size: 11px;
    font-weight: 700;
    letter-spacing: 0.12em;
    text-transform: uppercase;
  }

  .verdict strong {
    display: block;
    margin: 10px 0 6px;
    font-family: var(--font-display);
    font-size: 34px;
    font-weight: 900;
    line-height: 1;
    text-transform: uppercase;
  }

  .verdict p {
    margin: 0 0 8px;
    font-size: 13px;
    line-height: 1.5;
  }

  .small {
    font-size: 11px;
  }

  .limits {
    margin-bottom: 12px;
    padding: 14px 20px;
    border: 2px solid var(--border);
    background: var(--surface);
  }

  .label-muted {
    font-size: 11px;
    font-weight: 700;
    letter-spacing: 0.12em;
    text-transform: uppercase;
    color: var(--muted);
  }

  .bar {
    display: flex;
    height: 18px;
    margin: 10px 0;
    background: var(--surface-2);
  }

  .legend {
    display: flex;
    flex-wrap: wrap;
    gap: 6px 18px;
    margin: 0;
    padding: 0;
    list-style: none;
    font-size: 12px;
  }

  .legend li {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .swatch {
    width: 10px;
    height: 10px;
  }

  /* Uma cor por limite; "nada no limite" fica neutro. */
  .l-gpu {
    background: var(--accent);
  }

  .l-cpu {
    background: var(--warn);
  }

  .l-thermal {
    background: var(--hot);
  }

  .l-vram,
  .l-ram {
    background: var(--good);
  }

  .l-capped {
    background: var(--muted);
  }

  .findings {
    display: flex;
    flex-direction: column;
    gap: 10px;
    margin: 0 0 12px;
    padding: 0;
    list-style: none;
  }

  .findings li,
  .ok {
    padding: 12px 16px;
    border: 2px solid var(--border);
    border-left: 4px solid var(--accent);
    background: var(--surface);
  }

  .ok {
    margin: 0 0 12px;
    border-left-color: var(--good);
  }

  .findings li.warning {
    border-left-color: var(--warn);
  }

  .findings li.problem {
    border-left-color: var(--hot);
  }

  .findings p {
    margin: 6px 0 0;
    font-size: 13px;
    line-height: 1.5;
  }

  .grid {
    grid-template-columns: repeat(2, minmax(0, 1fr));
  }

  th.num,
  td.num {
    text-align: right;
  }

  .export {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 8px 12px;
    margin-top: 12px;
    font-size: 12px;
  }

  .export code {
    overflow-wrap: anywhere;
  }

  .note {
    margin: 16px 0 0;
    font-size: 11px;
  }
</style>

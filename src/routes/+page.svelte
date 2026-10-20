<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { onMount } from "svelte";
  import Card from "$lib/components/Card.svelte";
  import CoreMap from "$lib/components/CoreMap.svelte";
  import Gauge from "$lib/components/Gauge.svelte";
  import Sparkline from "$lib/components/Sparkline.svelte";
  import { bytes, ghz, or, smbiosDate } from "$lib/format";
  import { i18n, t } from "$lib/i18n/index.svelte";
  import { reportDialog } from "$lib/system.svelte";
  import type { BoardInfo, CpuInfo, CpuLive, Gpu, GpuLive, MemoryInfo, MemoryLive } from "$lib/types";

  const HISTORY = 60;

  let cpu = $state<CpuInfo | null>(null);
  let board = $state<BoardInfo | null>(null);
  let memory = $state<MemoryInfo | null>(null);
  let gpus = $state<Gpu[]>([]);
  let live = $state<CpuLive | null>(null);
  let memLive = $state<MemoryLive | null>(null);
  let gpuLive = $state<GpuLive[]>([]);
  let history = $state<number[]>([]);
  let error = $state<string | null>(null);

  let maxMhz = $derived(Math.max(cpu?.clocks.max_mhz ?? 0, live?.peak_mhz ?? 0, 1000));
  let gpuUsage = $derived.by(() => {
    const values = gpuLive.map((g) => g.usage).filter((u): u is number => u != null);
    return values.length ? Math.max(...values) : null;
  });
  let ramUsage = $derived(memLive ? (memLive.used_bytes / memLive.total_bytes) * 100 : null);
  let systemName = $derived(
    [board?.system.vendor, board?.system.product].filter(Boolean).join(" ") || board?.board.name || "",
  );

  onMount(() => {
    invoke<CpuInfo>("cpu_info")
      .then((d) => (cpu = d))
      .catch((e) => (error = String(e)));
    invoke<BoardInfo>("board_info").then((d) => (board = d)).catch(() => {});
    invoke<MemoryInfo>("memory_info").then((d) => (memory = d)).catch(() => {});
    invoke<Gpu[]>("gpu_info").then((d) => (gpus = d)).catch(() => {});

    const tick = async () => {
      const [c, m, g] = await Promise.allSettled([
        invoke<CpuLive>("cpu_live"),
        invoke<MemoryLive>("memory_live"),
        invoke<GpuLive[]>("gpu_live"),
      ]);
      if (c.status === "fulfilled") {
        live = c.value;
        history = [...history.slice(-(HISTORY - 1)), c.value.usage];
      }
      if (m.status === "fulfilled") memLive = m.value;
      if (g.status === "fulfilled") gpuLive = g.value;
    };
    tick();
    const timer = setInterval(tick, 1000);
    return () => clearInterval(timer);
  });

  function level(pct: number) {
    if (pct > 85) return "var(--hot)";
    if (pct > 60) return "var(--warn)";
    return "var(--accent)";
  }

  function tempColor(c: number) {
    if (c >= 85) return "var(--hot)";
    if (c >= 70) return "var(--warn)";
    return "var(--good)";
  }
</script>

{#if error}
  <p class="error">{t().cpu.error}: {error}</p>
{:else if !cpu}
  <div class="skeleton hero-sk"></div>
  <div class="gauges">
    {#each Array(5) as _}<div class="skeleton gauge-sk"></div>{/each}
  </div>
  <div class="skeleton block-sk"></div>
{:else}
  <section class="hero">
    <svg class="die" viewBox="0 0 120 120" aria-hidden="true">
      <rect x="22" y="22" width="76" height="76" rx="10" />
      <rect class="core-grid" x="34" y="34" width="52" height="52" rx="4" />
      {#each [36, 48, 60, 72, 84] as p}
        <line x1={p} y1="8" x2={p} y2="22" /><line x1={p} y1="98" x2={p} y2="112" />
        <line x1="8" y1={p} x2="22" y2={p} /><line x1="98" y1={p} x2="112" y2={p} />
      {/each}
    </svg>
    <div class="hero-text">
      <span class="vendor">{cpu.vendor}</span>
      <h1>{cpu.brand}</h1>
      <p class="muted">
        {#if cpu.codename}{cpu.codename.name} · {cpu.codename.microarch} · {cpu.codename.process} ·{/if}
        {cpu.cores} / {cpu.threads} threads
      </p>
      {#if systemName}
        <p class="system">
          {systemName}
          {#if board?.system.chassis}<span class="tag">{t().board.chassis[board.system.chassis] ?? board.system.chassis}</span>{/if}
        </p>
      {/if}
    </div>
    <button class="ghost export" onclick={() => (reportDialog.open = true)}>↓ {t().report.button}</button>
  </section>

  <div class="gauges">
    <Gauge label={t().overview.cpuLoad} value={live?.usage ?? null} max={100} format={(v) => `${v.toFixed(0)}%`} color={level(live?.usage ?? 0)} />
    <Gauge
      label={t().overview.clock}
      value={live?.avg_mhz ?? null}
      max={maxMhz}
      format={(v) => (v / 1000).toFixed(2)}
      sub="GHz · {t().overview.ofMax(ghz(maxMhz))}"
    />
    <Gauge
      label={t().common.temperature}
      value={live?.temperature_c ?? null}
      max={105}
      format={(v) => `${v.toFixed(0)}°`}
      sub={live?.temperature_c == null ? t().overview.noSensor : "°C"}
      color={tempColor(live?.temperature_c ?? 0)}
    />
    <Gauge
      label={t().overview.ram}
      value={ramUsage}
      max={100}
      format={(v) => `${v.toFixed(0)}%`}
      sub={memLive ? `${bytes(memLive.used_bytes)} / ${bytes(memLive.total_bytes)}` : ""}
      color={level(ramUsage ?? 0)}
    />
    <Gauge
      label={t().overview.gpuLoad}
      value={gpuUsage}
      max={100}
      format={(v) => `${v.toFixed(0)}%`}
      sub={gpuUsage == null ? t().overview.noSensor : ""}
      color={level(gpuUsage ?? 0)}
    />
  </div>

  <div class="grid">
    <Card title={t().coreMap.title} span={2}>
      <CoreMap info={cpu} {live} />
    </Card>

    <Card title={t().overview.activity}>
      <div class="activity">
        <span class="big num">{live ? live.usage.toFixed(0) : "—"}%</span>
        <span class="muted">{t().overview.activityHint(HISTORY)}</span>
      </div>
      <Sparkline values={history} max={100} capacity={HISTORY} />
      <dl class="mini">
        <dt>{t().cpu.avgClock}</dt><dd class="num">{ghz(live?.avg_mhz)}</dd>
        <dt>{t().cpu.maxClock}</dt><dd class="num">{ghz(cpu.clocks.max_mhz)}</dd>
        <dt>{t().cpu.microcode}</dt><dd class="num">{cpu.microcode ?? "—"}</dd>
      </dl>
    </Card>

    <a class="summary" href="/placa-mae">
      <Card title={t().tabs.board}>
        {#if board}
          <strong>{or(board.board.vendor)}</strong>
          <span>{or(board.board.name)}</span>
          <dl class="mini">
            <dt>{t().board.chipset}</dt><dd>{board.chipset?.name ?? "—"}</dd>
            <dt>BIOS</dt><dd>{or(board.bios.version)} · {smbiosDate(board.bios.date, i18n.locale)}</dd>
          </dl>
        {:else}
          <div class="skeleton line-sk"></div>
        {/if}
        <span class="more">{t().overview.details} →</span>
      </Card>
    </a>

    <a class="summary" href="/memoria">
      <Card title={t().tabs.memory}>
        {#if memory}
          <strong class="num">{bytes(memory.total_bytes)}</strong>
          <span>{t().overview.memorySummary(memory.kind ?? "—", memory.speed_mts ? `${memory.speed_mts} MT/s` : "—")}</span>
          <dl class="mini">
            <dt>{t().memory.type}</dt><dd>{t().memory.channels(memory.channels)}</dd>
            <dt>{t().memory.modules}</dt><dd class="num">{memory.slots.filter((s) => s.size_bytes > 0).length} / {memory.slots.length || "—"}</dd>
          </dl>
        {:else}
          <div class="skeleton line-sk"></div>
        {/if}
        <span class="more">{t().overview.details} →</span>
      </Card>
    </a>

    <a class="summary" href="/graficos">
      <Card title={t().tabs.gpu}>
        {#each gpus as gpu}
          <div class="gpu">
            <strong>{gpu.name ?? gpu.vendor ?? "GPU"}</strong>
            <span class="muted">
              {gpu.integrated ? t().gpu.integrated : t().gpu.dedicated}
              {#if gpu.vram_bytes} · {bytes(gpu.vram_bytes)}{/if}
            </span>
          </div>
        {:else}
          <span class="muted">{t().gpu.none}</span>
        {/each}
        <span class="more">{t().overview.details} →</span>
      </Card>
    </a>
  </div>
{/if}

<style>
  .hero {
    position: relative;
    display: flex;
    align-items: center;
    gap: 24px;
    margin-bottom: 20px;
    padding: 24px 28px;
    border: 1px solid color-mix(in srgb, var(--accent) 30%, var(--border));
    border-radius: 20px;
    overflow: hidden;
    background:
      radial-gradient(circle at 0% 0%, color-mix(in srgb, var(--accent) 28%, transparent), transparent 55%),
      radial-gradient(circle at 100% 120%, color-mix(in srgb, var(--accent) 14%, transparent), transparent 50%),
      var(--surface);
  }

  .die {
    flex-shrink: 0;
    width: 104px;
    filter: drop-shadow(0 0 18px color-mix(in srgb, var(--accent) 60%, transparent));
  }

  .die rect {
    fill: color-mix(in srgb, var(--accent) 22%, var(--surface-2));
    stroke: var(--accent);
    stroke-width: 2;
  }

  .die .core-grid {
    fill: color-mix(in srgb, var(--accent) 45%, var(--surface));
    stroke-width: 1;
    animation: pulse 3s ease-in-out infinite;
  }

  .die line {
    stroke: color-mix(in srgb, var(--accent) 70%, var(--muted));
    stroke-width: 3;
    stroke-linecap: round;
  }

  @keyframes pulse {
    50% {
      fill: color-mix(in srgb, var(--accent) 70%, var(--surface));
    }
  }

  .hero-text {
    flex: 1;
    min-width: 0;
  }

  .export {
    align-self: flex-start;
    background: color-mix(in srgb, var(--surface) 70%, transparent);
  }

  .hero h1 {
    font-size: 30px;
  }

  .hero p {
    margin: 0;
  }

  .system {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 8px;
    margin-top: 6px !important;
    font-weight: 500;
  }

  .tag {
    padding: 1px 8px;
    border-radius: 6px;
    font-size: 11px;
    background: var(--surface-2);
    border: 1px solid var(--border);
  }

  .gauges {
    display: grid;
    grid-template-columns: repeat(5, minmax(0, 1fr));
    gap: 12px;
    margin-bottom: 16px;
    padding: 18px 12px 14px;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: var(--surface);
  }

  .activity {
    display: flex;
    flex-direction: column;
    margin-bottom: 8px;
  }

  .big {
    font-size: 32px;
    font-weight: 700;
    line-height: 1.1;
  }

  .activity span:last-child {
    font-size: 12px;
  }

  .mini {
    margin-top: 12px;
    font-size: 13px;
  }

  .summary {
    display: contents;
    color: inherit;
    text-decoration: none;
  }

  .summary :global(section) {
    display: flex;
    flex-direction: column;
    gap: 2px;
    transition:
      border-color 0.2s,
      transform 0.2s;
  }

  .summary:hover :global(section) {
    border-color: var(--accent);
    transform: translateY(-2px);
  }

  .summary strong {
    font-size: 18px;
    font-weight: 650;
  }

  .gpu {
    display: flex;
    flex-direction: column;
    margin-bottom: 8px;
  }

  .gpu strong {
    font-size: 15px;
  }

  .more {
    margin-top: auto;
    padding-top: 12px;
    font-size: 12px;
    font-weight: 600;
    color: var(--accent);
  }

  .skeleton {
    border-radius: var(--radius);
    background: linear-gradient(90deg, var(--surface) 0%, var(--surface-2) 50%, var(--surface) 100%);
    background-size: 200% 100%;
    animation: shimmer 1.4s linear infinite;
  }

  .hero-sk {
    height: 150px;
    margin-bottom: 20px;
  }

  .gauge-sk {
    aspect-ratio: 1;
  }

  .block-sk {
    height: 260px;
  }

  .line-sk {
    height: 60px;
  }

  @keyframes shimmer {
    to {
      background-position: -200% 0;
    }
  }

  @media (max-width: 900px) {
    .gauges {
      grid-template-columns: repeat(3, minmax(0, 1fr));
    }
  }

  @media (max-width: 560px) {
    .hero {
      flex-direction: column;
      align-items: flex-start;
    }
    .gauges {
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }
  }
</style>

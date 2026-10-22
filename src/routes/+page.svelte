<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { onMount } from "svelte";
  import Card from "$lib/components/Card.svelte";
  import CoreMap from "$lib/components/CoreMap.svelte";
  import Gauge from "$lib/components/Gauge.svelte";
  import Sparkline from "$lib/components/Sparkline.svelte";
  import { batteryHealth, bytes, diskInterface, diskName, diskSize, ghz, or, smbiosDate } from "$lib/format";
  import { i18n, t } from "$lib/i18n/index.svelte";
  import { isWindows, reportDialog, shareDialog } from "$lib/system.svelte";
  import type { BoardInfo, CpuInfo, CpuLive, Disk, Gpu, GpuLive, MemoryInfo, MemoryLive, SystemBattery } from "$lib/types";
  import { loadUpgrades, upgrades } from "$lib/upgrades.svelte";
  import { diagnosis, findingCounts, findingText, loadDiagnosis } from "$lib/diagnosis.svelte";

  const HISTORY = 60;

  let cpu = $state<CpuInfo | null>(null);
  let board = $state<BoardInfo | null>(null);
  let memory = $state<MemoryInfo | null>(null);
  let gpus = $state<Gpu[]>([]);
  let disks = $state<Disk[] | null>(null);
  let battery = $state<SystemBattery | null>(null);
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
    invoke<Disk[]>("storage_info").then((d) => (disks = d)).catch(() => (disks = []));
    invoke<SystemBattery[]>("battery_info").then((d) => (battery = d[0] ?? null)).catch(() => {});
    loadUpgrades();
    loadDiagnosis();

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
    <div class="hero-text">
      <div class="tags">
        <span class="vendor">{cpu.vendor}</span>
        {#if cpu.codename}
          <span class="tag">{cpu.codename.name}</span>
          <span class="tag">{cpu.codename.microarch}</span>
          <span class="tag">{cpu.codename.process}</span>
        {/if}
        <span class="tag num">{cpu.cores}C / {cpu.threads}T</span>
      </div>
      <h1>{cpu.brand}</h1>
      {#if systemName}
        <p class="system muted">
          &gt; {systemName}
          {#if board?.system.chassis}<span class="chassis">[{t().board.chassis[board.system.chassis] ?? board.system.chassis}]</span>{/if}
        </p>
      {/if}
    </div>
    <div class="hero-actions">
      <button class="share" onclick={() => (shareDialog.open = true)}>{t().share.button}</button>
      <button class="ghost" onclick={() => (reportDialog.open = true)}>{t().report.button} <kbd>Ctrl+E</kbd></button>
    </div>
  </section>

  <div class="gauges" class:four={isWindows}>
    <Gauge label={t().overview.cpuLoad} value={live?.usage ?? null} max={100} format={(v) => `${v.toFixed(0)}%`} color={level(live?.usage ?? 0)} />
    <Gauge
      label={t().overview.clock}
      value={live?.avg_mhz ?? null}
      max={maxMhz}
      format={(v) => (v / 1000).toFixed(2)}
      sub="GHz · {t().overview.ofMax(ghz(maxMhz))}"
    />
    {#if !isWindows}
      <Gauge
        label={t().common.temperature}
        value={live?.temperature_c ?? null}
        max={105}
        format={(v) => `${v.toFixed(0)}°`}
        sub={live?.temperature_c == null ? t().overview.noSensor : "°C"}
        color={tempColor(live?.temperature_c ?? 0)}
      />
    {/if}
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

    <a class="summary" href="/armazenamento">
      <Card title={t().tabs.storage}>
        {#if disks}
          {#each disks.filter((d) => !d.removable) as disk}
            {@const free = disk.volumes.reduce((sum, v) => sum + v.available_bytes, 0)}
            <div class="gpu">
              <strong>{diskName(disk)}</strong>
              <span class="muted">
                {diskSize(disk.size_bytes)} · {diskInterface(disk)}
                {#if disk.health?.life_left != null} · {t().storage.lifeLeft.toLowerCase()} {disk.health.life_left}%{/if}
                {#if disk.volumes.length} · {t().overview.storageFree(diskSize(free))}{/if}
              </span>
            </div>
          {:else}
            <span class="muted">{t().storage.none}</span>
          {/each}
        {:else}
          <div class="skeleton line-sk"></div>
        {/if}
        <span class="more">{t().overview.details} →</span>
      </Card>
    </a>

    {#if battery}
      {@const health = batteryHealth(battery)}
      <a class="summary" href="/bateria">
        <Card title={t().tabs.battery}>
          <strong class="num">{battery.percent != null ? `${Math.round(battery.percent)}%` : "—"}</strong>
          <span>{t().battery.status[battery.status]}</span>
          <dl class="mini">
            <dt>{t().battery.health}</dt><dd class="num">{health != null ? `${health}% · ${t().battery.rating(health)}` : "—"}</dd>
            <dt>{t().battery.cycles}</dt><dd class="num">{or(battery.cycles)}</dd>
          </dl>
          <span class="more">{t().overview.details} →</span>
        </Card>
      </a>
    {/if}

    {#if diagnosis.data}
      {@const data = diagnosis.data}
      {@const top = data.findings.find((f) => f.severity !== "info")}
      <a class="upgrade health {data.status}" href="/diagnostico">
        <span class="label">{t().diagnosis.score} {data.score}/100</span>
        <strong>{top ? findingText(top).title : t().diagnosis.status[data.status]}</strong>
        <span class="muted">{findingCounts(data)}</span>
        <span class="more">{t().diagnosis.more} →</span>
      </a>
    {/if}

    {#if upgrades.data}
      <a class="upgrade" href="/upgrades">
        <span class="label">{t().upgrades.bestMove}</span>
        <strong>{t().upgrades.top[upgrades.data.top]}</strong>
        <span class="more">{t().upgrades.more} →</span>
      </a>
    {/if}
  </div>
{/if}

<style>
  .hero {
    position: relative;
    display: flex;
    flex-wrap: wrap;
    align-items: flex-end;
    gap: 20px;
    margin-bottom: 12px;
    padding: 24px;
    border: 2px solid var(--border);
    background: var(--surface);
  }

  .hero-text {
    flex: 1 1 420px;
    min-width: 0;
  }

  .tags {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }

  .tag {
    padding: 1px 8px;
    border: 1px solid var(--border);
    font-size: 11px;
    font-weight: 700;
    letter-spacing: 0.1em;
    text-transform: uppercase;
    color: var(--muted);
  }

  .hero h1 {
    margin: 14px 0 10px;
    font-size: clamp(40px, 6vw, 76px);
    line-height: 0.88;
  }

  .hero p {
    margin: 0;
  }

  .system {
    font-size: 12px;
  }

  .chassis {
    color: var(--warn);
  }

  .hero-actions {
    display: flex;
    flex-direction: column;
    gap: 8px;
    min-width: 220px;
  }

  .share {
    min-height: 40px;
    padding: 6px 14px;
    border: 2px solid var(--accent);
    background: var(--accent);
    color: var(--on-accent);
    font-size: 12px;
    font-weight: 700;
    letter-spacing: 0.1em;
    text-transform: uppercase;
    text-align: left;
  }

  .share:hover {
    filter: brightness(1.08);
  }

  .hero-actions .ghost {
    display: flex;
    justify-content: space-between;
    gap: 12px;
    min-height: 40px;
  }

  kbd {
    font: inherit;
    font-size: 10px;
    color: var(--muted);
  }

  .gauges {
    display: grid;
    grid-template-columns: repeat(5, minmax(0, 1fr));
    gap: 12px;
    margin-bottom: 12px;
  }

  .gauges.four {
    grid-template-columns: repeat(4, minmax(0, 1fr));
  }

  .upgrade {
    grid-column: 1 / -1;
    display: flex;
    flex-wrap: wrap;
    align-items: baseline;
    gap: 8px 16px;
    padding: 14px 18px;
    border: 2px solid var(--accent);
    background: var(--surface);
    color: inherit;
    text-decoration: none;
  }

  .upgrade .label {
    padding: 2px 8px;
    background: var(--accent);
    color: var(--on-accent);
    font-size: 11px;
    font-weight: 700;
    letter-spacing: 0.12em;
    text-transform: uppercase;
  }

  .health {
    border-color: var(--good);
  }

  .health .label {
    background: var(--good);
  }

  .health.attention {
    border-color: var(--warn);
  }

  .health.attention .label {
    background: var(--warn);
  }

  .health.problem {
    border-color: var(--hot);
  }

  .health.problem .label {
    background: var(--hot);
    color: #fff;
  }

  .upgrade .more {
    margin-left: auto;
    padding-top: 0;
  }

  .activity {
    display: flex;
    flex-direction: column;
    margin-bottom: 8px;
  }

  .big {
    font-family: var(--font-display);
    font-size: 56px;
    font-weight: 900;
    line-height: 0.95;
    color: var(--accent);
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
  }

  .summary strong {
    font-size: 15px;
    font-weight: 700;
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
    background: linear-gradient(90deg, var(--surface) 0%, var(--surface-2) 50%, var(--surface) 100%);
    background-size: 200% 100%;
    animation: shimmer 1.4s linear infinite;
  }

  .hero-sk {
    height: 190px;
    margin-bottom: 12px;
  }

  .gauge-sk {
    height: 130px;
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
    .gauges,
    .gauges.four {
      grid-template-columns: repeat(3, minmax(0, 1fr));
    }
  }

  @media (max-width: 560px) {
    .gauges,
    .gauges.four {
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }
  }
</style>

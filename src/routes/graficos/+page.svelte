<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { onMount } from "svelte";
  import Card from "$lib/components/Card.svelte";
  import Sparkline from "$lib/components/Sparkline.svelte";
  import { bytes, or } from "$lib/format";
  import { t } from "$lib/i18n/index.svelte";
  import type { Gpu, GpuLive } from "$lib/types";

  const HISTORY = 60;

  let gpus = $state<Gpu[] | null>(null);
  let live = $state<GpuLive[]>([]);
  let history = $state<Record<string, number[]>>({});
  let selected = $state(0);
  let error = $state<string | null>(null);

  let gpu = $derived(gpus?.[selected] ?? null);
  let now = $derived(live.find((l) => l.id === gpu?.id) ?? null);
  let vramPct = $derived(now?.vram_used_bytes && gpu?.vram_bytes ? (now.vram_used_bytes / gpu.vram_bytes) * 100 : null);

  onMount(() => {
    invoke<Gpu[]>("gpu_info")
      .then((data) => (gpus = data))
      .catch((e) => (error = String(e)));

    const tick = async () => {
      try {
        live = await invoke<GpuLive[]>("gpu_live");
        const next = { ...history };
        for (const s of live) next[s.id] = [...(next[s.id] ?? []).slice(-(HISTORY - 1)), s.usage ?? 0];
        history = next;
      } catch (e) {
        error = String(e);
      }
    };
    tick();
    const timer = setInterval(tick, 1000);
    return () => clearInterval(timer);
  });

  const mhz = (v: number | null | undefined) => (v ? `${v} MHz` : "—");
  const link = (gen: number | null, width: number | null) => (gen ? `PCIe ${gen}.0${width ? ` x${width}` : ""}` : "—");

  function tempColor(t: number) {
    if (t >= 85) return "var(--hot)";
    if (t >= 70) return "var(--warn)";
    return "var(--good)";
  }
</script>

{#if error}
  <p class="error">{t().gpu.error}: {error}</p>
{:else if !gpus}
  <p class="muted">{t().gpu.loading}</p>
{:else if gpus.length === 0}
  <header>
    <div>
      <h1>{t().tabs.gpu}</h1>
      <p class="muted">{t().gpu.none}</p>
    </div>
  </header>
{:else if gpu}
  <header>
    <div>
      <span class="vendor">{gpu.integrated == null ? or(gpu.vendor) : gpu.integrated ? t().gpu.integrated : t().gpu.dedicated}</span>
      <h1>{[gpu.vendor, gpu.name].filter(Boolean).join(" ") || t().gpu.card}</h1>
      <p class="muted">{[gpu.driver, gpu.driver_version].filter(Boolean).join(" · ")}</p>
    </div>
    {#if gpus.length > 1}
      <div class="switch" role="tablist">
        {#each gpus as g, i}
          <button class="ghost" class:active={i === selected} role="tab" aria-selected={i === selected} onclick={() => (selected = i)}>
            GPU {i + 1}
          </button>
        {/each}
      </div>
    {/if}
  </header>

  <div class="stats">
    <div class="stat">
      <span class="muted">{t().gpu.coreClock}</span>
      <strong class="num">{mhz(now?.core_mhz)}</strong>
    </div>
    <div class="stat">
      <span class="muted">{t().gpu.memoryClock}</span>
      <strong class="num">{mhz(now?.memory_mhz)}</strong>
    </div>
    <div class="stat">
      <span class="muted">{t().common.usage}</span>
      <strong class="num">{now?.usage != null ? `${now.usage}%` : "—"}</strong>
    </div>
    <div class="stat">
      <span class="muted">{t().common.temperature}</span>
      {#if now?.temperature_c != null}
        <strong class="num" style:color={tempColor(now.temperature_c)}>{now.temperature_c.toFixed(0)} °C</strong>
      {:else}
        <strong class="muted">—</strong>
      {/if}
    </div>
    <div class="stat">
      <span class="muted">{t().gpu.power}</span>
      <strong class="num">{now?.power_w != null ? `${now.power_w.toFixed(1)} W` : "—"}</strong>
    </div>
  </div>

  <div class="grid">
    <Card title={t().gpu.liveUsage} span={2}>
      <div class="usage-head">
        <span class="big num">{now?.usage != null ? `${now.usage}%` : "—"}</span>
        <div class="muted right">
          {#if now?.voltage_mv}<div>{t().common.voltage} <strong class="num">{(now.voltage_mv / 1000).toFixed(3)} V</strong></div>{/if}
          {#if now?.fan_rpm}<div>{t().gpu.fan} <strong class="num">{now.fan_rpm} RPM</strong></div>{/if}
          {#if now?.fan_percent != null}<div>{t().gpu.fan} <strong class="num">{now.fan_percent}%</strong></div>{/if}
        </div>
      </div>
      <Sparkline values={history[gpu.id] ?? []} max={100} capacity={HISTORY} />
      {#if !now}
        <p class="hint muted">{t().gpu.noLive}</p>
      {/if}
    </Card>

    <Card title={t().gpu.videoMemory}>
      <dl>
        <dt>{gpu.integrated ? t().gpu.reserved : "VRAM"}</dt><dd class="num">{bytes(gpu.vram_bytes)}</dd>
        {#if gpu.shared_bytes}<dt>{t().gpu.shared}</dt><dd class="num">{bytes(gpu.shared_bytes)}</dd>{/if}
        <dt>{t().gpu.inUse}</dt>
        <dd class="num">{bytes(now?.vram_used_bytes)}{#if vramPct != null}<span class="muted">{` · ${vramPct.toFixed(0)}%`}</span>{/if}</dd>
      </dl>
      {#if vramPct != null}
        <div class="bar"><div style:width="{vramPct}%"></div></div>
      {/if}
    </Card>

    <Card title={t().gpu.maxClocks}>
      <dl>
        <dt>{t().gpu.core}</dt><dd class="num">{mhz(gpu.max_core_mhz)}</dd>
        <dt>{t().gpu.memory}</dt><dd class="num">{mhz(gpu.max_memory_mhz)}</dd>
      </dl>
    </Card>

    <Card title={t().common.bus}>
      <dl>
        <dt>{t().common.pcieCurrent}</dt><dd class="num">{gpu.link ? link(gpu.link.current_gen, gpu.link.current_width) : "—"}</dd>
        <dt>{t().common.pcieMax}</dt><dd class="num">{gpu.link ? link(gpu.link.max_gen, gpu.link.max_width) : "—"}</dd>
        <dt>{t().gpu.address}</dt><dd class="num muted">{gpu.id}</dd>
      </dl>
    </Card>

    <Card title={t().gpu.firmware}>
      <dl>
        <dt>{t().gpu.vbios}</dt><dd class="num">{or(gpu.vbios)}</dd>
        <dt>Driver</dt><dd>{or(gpu.driver)}</dd>
        <dt>{t().common.version}</dt><dd class="num">{or(gpu.driver_version)}</dd>
      </dl>
    </Card>
  </div>
{/if}

<style>
  .switch {
    display: flex;
    gap: 6px;
  }

  .ghost.active {
    border-color: var(--accent);
    color: var(--accent);
  }

  .usage-head {
    display: flex;
    justify-content: space-between;
    gap: 12px;
    margin-bottom: 10px;
  }

  .big {
    font-size: 32px;
    font-weight: 700;
    line-height: 1.1;
  }

  .right {
    text-align: right;
    font-size: 12px;
  }

  .bar {
    height: 6px;
    margin-top: 14px;
    border-radius: 3px;
    background: var(--border);
    overflow: hidden;
  }

  .bar div {
    height: 100%;
    background: var(--accent);
    transition: width 0.4s ease;
  }

  .hint {
    margin: 10px 0 0;
    font-size: 11px;
  }
</style>

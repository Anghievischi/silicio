<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { onMount } from "svelte";
  import Card from "$lib/components/Card.svelte";
  import CoreMap from "$lib/components/CoreMap.svelte";
  import Sparkline from "$lib/components/Sparkline.svelte";
  import { ghz, hex, size } from "$lib/format";
  import { t } from "$lib/i18n/index.svelte";
  import { privileged, unlock } from "$lib/privileged.svelte";
  import type { CpuInfo, CpuLive } from "$lib/types";

  const HISTORY = 60;
  // Clock de referência nominal das plataformas atuais, usado até o MSR ser lido.
  const NOMINAL_BUS_MHZ = 100;
  // O helper com privilégio usa pkexec no Linux e o PawnIO (pelo UAC) no Windows.
  const isWindows = navigator.userAgent.includes("Windows");

  let bus = $derived(privileged.data?.bus_mhz ?? null);
  let busMhz = $derived(bus ?? NOMINAL_BUS_MHZ);
  let vcore = $derived.by(() => {
    const volts = privileged.data?.threads.map((thread) => thread.voltage).filter((v): v is number => v != null) ?? [];
    return volts.length ? Math.max(...volts) : null;
  });

  let info = $state<CpuInfo | null>(null);
  let live = $state<CpuLive | null>(null);
  let history = $state<number[]>([]);
  let error = $state<string | null>(null);
  let copied = $state(false);

  let l3 = $derived(info?.caches.filter((c) => c.level === 3).reduce((sum, c) => sum + c.size_kib * c.instances, 0) ?? 0);
  // No Windows o clock máximo da AMD só aparece depois da autorização.
  $effect(() => {
    const max = privileged.data?.max_mhz;
    if (max && info && !info.clocks.max_mhz) info.clocks.max_mhz = max;
  });

  let scaleMhz = $derived(Math.max(info?.clocks.max_mhz ?? 0, live?.peak_mhz ?? 0, 1000));

  onMount(() => {
    let timer: ReturnType<typeof setInterval>;

    invoke<CpuInfo>("cpu_info")
      .then((data) => {
        info = data;
      })
      .catch((e) => (error = String(e)));

    const tick = async () => {
      try {
        const sample = await invoke<CpuLive>("cpu_live");
        live = sample;
        history = [...history.slice(-(HISTORY - 1)), sample.avg_mhz];
      } catch (e) {
        error = String(e);
      }
    };
    tick();
    timer = setInterval(tick, 1000);
    return () => clearInterval(timer);
  });

  async function copyJson() {
    if (!info) return;
    await navigator.clipboard.writeText(JSON.stringify(info, null, 2));
    copied = true;
    setTimeout(() => (copied = false), 1500);
  }

  function tempColor(t: number) {
    if (t >= 85) return "var(--hot)";
    if (t >= 70) return "var(--warn)";
    return "var(--good)";
  }
</script>

{#if error}
  <p class="error">{t().cpu.error}: {error}</p>
{:else if !info}
  <p class="muted">{t().cpu.loading}</p>
{:else}
  <header>
    <div>
      <span class="vendor">{info.vendor}</span>
      <h1>{info.brand}</h1>
      <p class="muted">
        {#if info.codename}
          {info.codename.name} · {info.codename.microarch} · {info.codename.process}
        {:else}
          {t().cpu.familyModel(hex(info.signature.family), hex(info.signature.model))}
        {/if}
      </p>
    </div>
    <button class="ghost" onclick={copyJson}>{copied ? t().cpu.copied : t().cpu.copyJson}</button>
  </header>

  <div class="stats">
    <div class="stat">
      <span class="muted">{t().cpu.coresThreads}</span>
      <strong class="num">{info.cores} / {info.threads}</strong>
    </div>
    <div class="stat">
      <span class="muted">{t().cpu.avgClock}</span>
      <strong class="num">{ghz(live?.avg_mhz)}</strong>
    </div>
    <div class="stat">
      <span class="muted">{t().cpu.maxClock}</span>
      <strong class="num">{ghz(info.clocks.max_mhz)}</strong>
    </div>
    <div class="stat">
      <span class="muted">{t().cpu.l3Cache}</span>
      <strong class="num">{l3 ? size(l3) : "—"}</strong>
    </div>
    <div class="stat">
      <span class="muted">{t().common.temperature}</span>
      {#if live?.temperature_c != null}
        <strong class="num" style:color={tempColor(live.temperature_c)}>{live.temperature_c.toFixed(0)} °C</strong>
        {#if live.temperature_source === "acpi"}
          <span class="muted source" title={t().cpu.tempAcpiHint}>{t().cpu.tempAcpi}</span>
        {/if}
      {:else}
        <strong class="muted">—</strong>
      {/if}
    </div>
  </div>

  <div class="grid">
    <Card title={t().cpu.liveClock} span={2}>
      <div class="clock-head">
        <div>
          <span class="big num">{ghz(live?.avg_mhz)}</span>
          <span class="muted">{t().cpu.avgPeak(ghz(live?.peak_mhz))}</span>
        </div>
        <div class="muted right">
          <div>{t().cpu.totalUsage} <strong class="num">{live ? live.usage.toFixed(0) : "—"}%</strong></div>
          <div title={bus ? t().cpu.busMeasured : t().cpu.busEstimated}>
            {t().cpu.multiplier} {bus ? "" : "≈ "}<strong class="num">{live ? `x${(live.avg_mhz / busMhz).toFixed(1)}` : "—"}</strong>
            · bus {busMhz.toFixed(bus ? 2 : 0)} MHz
          </div>
          {#if info.clocks.driver}<div>{info.clocks.driver} · {info.clocks.governor}</div>{/if}
        </div>
      </div>
      <Sparkline values={history} max={scaleMhz} capacity={HISTORY} />
      <div class="axis muted">
        <span>{ghz(info.clocks.min_mhz)}</span>
        <span>{t().common.lastSeconds(HISTORY)}</span>
        <span>{ghz(scaleMhz)}</span>
      </div>
    </Card>

    <Card title={t().cpu.specification}>
      <dl>
        <dt>{t().common.manufacturer}</dt><dd>{info.vendor_id}</dd>
        <dt>{t().common.family}</dt><dd class="num">{hex(info.signature.family)} <span class="muted">(base {hex(info.signature.base_family)} + ext {hex(info.signature.ext_family)})</span></dd>
        <dt>{t().common.model}</dt><dd class="num">{hex(info.signature.model)} <span class="muted">(base {hex(info.signature.base_model)} · ext {hex(info.signature.ext_model)})</span></dd>
        <dt>Stepping</dt><dd class="num">{info.signature.stepping}</dd>
        <dt>{t().cpu.microcode}</dt><dd class="num">{info.microcode ?? "—"}</dd>
        {#if info.codename}
          <dt>{t().cpu.process}</dt><dd>{info.codename.process}</dd>
        {/if}
      </dl>
    </Card>

    <Card title={t().coreMap.title} span={2}>
      <CoreMap {info} {live} />
    </Card>

    <Card title={t().cpu.cache}>
      <table>
        <tbody>
          {#each info.caches as c}
            <tr>
              <th>L{c.level} <span class="muted">{t().cpu.cacheKind[c.kind] ?? c.kind}</span></th>
              <td class="num">{c.instances > 1 ? `${c.instances} × ` : ""}{size(c.size_kib)}</td>
              <td class="num muted">{c.ways}-way · {c.line_bytes} B</td>
            </tr>
          {/each}
        </tbody>
      </table>
    </Card>

    <Card title={t().cpu.advanced} span={3}>
      {#if privileged.data}
        {@const p = privileged.data}
        <div class="adv">
          <div><span class="muted">{t().cpu.socket}</span><strong>{p.processor?.socket ?? "—"}</strong></div>
          <div>
            <span class="muted">{t().cpu.vid}</span>
            <strong class="num">{vcore ? `${vcore.toFixed(3)} V` : "—"}</strong>
          </div>
          <div><span class="muted">{t().cpu.realBus}</span><strong class="num">{bus ? `${bus.toFixed(2)} MHz` : "—"}</strong></div>
          <div>
            <span class="muted">{t().cpu.baseMultiplier}</span>
            <strong class="num">{p.base_ratio ? `x${p.base_ratio.toFixed(1)}` : "—"}</strong>
          </div>
          <div>
            <span class="muted">{t().cpu.externalClock}</span>
            <strong class="num">{p.processor?.external_clock_mhz ? `${p.processor.external_clock_mhz} MHz` : "—"}</strong>
          </div>
        </div>
        {#if p.errors.length}
          <p class="hint muted">{t().cpu.someFailed(p.errors.join("; "))}</p>
        {/if}
      {:else}
        <div class="unlock">
          <p class="muted">{isWindows ? t().cpu.unlockTextWindows : t().cpu.unlockText}</p>
          <button class="ghost" onclick={unlock} disabled={privileged.loading}>
            {privileged.loading ? t().cpu.waitingAuth : t().cpu.unlock}
          </button>
        </div>
        {#if privileged.error}<p class="error hint">{privileged.error}</p>{/if}
      {/if}
    </Card>

    <Card title={t().cpu.instructions} span={3}>
      <div class="groups">
        {#each info.instructions as group}
          <div>
            <h3>{t().cpu.instructionGroup[group.name] ?? group.name}</h3>
            <div class="chips">
              {#each group.items as item}<span class="chip">{item}</span>{/each}
            </div>
          </div>
        {/each}
      </div>
    </Card>
  </div>
{/if}

<style>
  .clock-head {
    display: flex;
    justify-content: space-between;
    gap: 12px;
    margin-bottom: 10px;
  }
  .big {
    display: block;
    font-family: var(--font-display);
    font-size: 56px;
    font-weight: 900;
    line-height: 0.95;
    color: var(--accent);
  }
  .right {
    text-align: right;
    font-size: 12px;
  }
  .axis {
    display: flex;
    justify-content: space-between;
    margin-top: 6px;
    font-size: 11px;
  }
  .hint {
    margin: 10px 0 0;
    font-size: 11px;
  }
  .groups {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(220px, 1fr));
    gap: 16px;
  }
  h3 {
    margin: 0 0 8px;
    font-size: 13px;
    font-weight: 600;
  }
  .chips {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }
  .chip {
    padding: 3px 9px;
    border-radius: var(--radius-sm);
    font-size: 12px;
    background: var(--surface-2);
    border: 1px solid var(--border);
  }

  .source {
    font-size: 11px;
  }

  .adv {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(160px, 1fr));
    gap: 16px;
  }

  .adv div {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .adv span {
    font-size: 12px;
  }

  .adv strong {
    font-size: 18px;
    font-weight: 650;
  }

  .unlock {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
  }

  .unlock p {
    margin: 0;
    max-width: 70ch;
  }
</style>

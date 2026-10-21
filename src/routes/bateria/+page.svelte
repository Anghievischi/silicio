<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { onMount } from "svelte";
  import Card from "$lib/components/Card.svelte";
  import { batteryHealth, batteryTime, or } from "$lib/format";
  import { t } from "$lib/i18n/index.svelte";
  import type { SystemBattery } from "$lib/types";

  let batteries = $state<SystemBattery[] | null>(null);
  let error = $state<string | null>(null);

  const b = () => t().battery;

  // Carga e consumo mudam a todo momento; a cada 5 s acompanha sem pesar.
  onMount(() => {
    const load = () =>
      invoke<SystemBattery[]>("battery_info")
        .then((d) => (batteries = d))
        .catch((e) => (error ??= String(e)));
    load();
    const timer = setInterval(load, 5000);
    return () => clearInterval(timer);
  });

  const wh = (v: number | null) => (v != null ? `${v.toFixed(1)} Wh` : "—");

  function healthColor(pct: number) {
    if (pct >= 80) return "var(--good)";
    if (pct >= 60) return "var(--warn)";
    return "var(--hot)";
  }
</script>

{#if error && !batteries}
  <p class="error">{b().error}: {error}</p>
{:else if !batteries}
  <p class="muted">{b().loading}</p>
{:else if !batteries.length}
  <header><div><h1>{t().tabs.battery}</h1></div></header>
  <p class="muted">{b().none}</p>
{:else}
  <header>
    <div>
      <h1>{t().tabs.battery}</h1>
      <p class="muted">{[batteries[0].manufacturer, batteries[0].model].filter(Boolean).join(" · ") || "—"}</p>
    </div>
  </header>

  {#each batteries as bat}
    {@const health = batteryHealth(bat)}
    {@const time = batteryTime(bat)}
    <div class="grid">
      <Card title={b().health}>
        {#if health != null}
          <div class="figure" style:--status={healthColor(health)}>
            <strong class="big num">{health}%</strong>
            <span class="badge">{b().rating(health)}</span>
          </div>
          <div class="bar" style:--status={healthColor(health)}><div style:width="{health}%"></div></div>
          <p class="muted small">{b().healthHint(wh(bat.full_wh), wh(bat.design_wh))}</p>
          <p class="small">{b().ratingHint(health)}</p>
        {:else}
          <strong class="big num">—</strong>
        {/if}
      </Card>

      <Card title={b().charge}>
        <div class="figure">
          <strong class="big num">{bat.percent != null ? `${Math.round(bat.percent)}%` : "—"}</strong>
          <span class="badge">{b().status[bat.status]}</span>
        </div>
        <div class="bar charge"><div style:width="{bat.percent ?? 0}%"></div></div>
        <dl>
          {#if bat.power_w != null}
            <dt>{bat.status === "charging" ? b().charging : b().power}</dt><dd class="num">{bat.power_w.toFixed(1)} W</dd>
          {/if}
          {#if time}
            <dt>{bat.status === "charging" ? b().timeToFull : b().timeLeft}</dt>
            <dd class="num">{b().duration(time.hours, time.minutes)}</dd>
          {/if}
          <dt>{b().ac}</dt><dd>{bat.ac_online == null ? "—" : bat.ac_online ? b().acOn : b().acOff}</dd>
        </dl>
        {#if bat.status === "not_charging"}<p class="muted small">{b().notChargingHint}</p>{/if}
      </Card>

      <Card title={b().cycles}>
        <strong class="big num">{or(bat.cycles)}</strong>
        <p class="muted small">{b().cyclesHint}</p>
      </Card>

      <Card title={b().details} span={3}>
        <dl>
          <dt>{b().design}</dt><dd class="num">{wh(bat.design_wh)}</dd>
          <dt>{b().full}</dt><dd class="num">{wh(bat.full_wh)}</dd>
          <dt>{b().now}</dt><dd class="num">{wh(bat.now_wh)}</dd>
          {#if health != null}<dt>{b().wear}</dt><dd class="num">{100 - health}%</dd>{/if}
          <dt>{b().voltage}</dt><dd class="num">{bat.voltage_v != null ? `${bat.voltage_v.toFixed(2)} V` : "—"}</dd>
          <dt>{b().technology}</dt><dd>{or(bat.technology)}</dd>
          <dt>{t().common.manufacturer}</dt><dd>{or(bat.manufacturer)}</dd>
          <dt>{b().model}</dt><dd class="num">{or(bat.model)}</dd>
        </dl>
      </Card>
    </div>
  {/each}
{/if}

<style>
  .grid + .grid {
    margin-top: 12px;
  }

  .figure {
    display: flex;
    align-items: baseline;
    flex-wrap: wrap;
    gap: 10px;
    margin-bottom: 10px;
  }

  .big {
    font-family: var(--font-display);
    font-size: 56px;
    font-weight: 900;
    line-height: 0.95;
    color: var(--status, var(--accent));
  }

  .badge {
    padding: 2px 8px;
    border-radius: var(--radius-sm);
    font-size: 12px;
    font-weight: 600;
    color: var(--status, var(--accent));
    background: color-mix(in srgb, var(--status, var(--accent)) 14%, transparent);
  }

  .bar {
    height: 6px;
    margin-bottom: 12px;
    border-radius: var(--radius-sm);
    background: var(--border);
    overflow: hidden;
  }

  .bar div {
    height: 100%;
    background: var(--status, var(--accent));
    transition: width 0.4s ease;
  }

  .small {
    margin: 8px 0 0;
    font-size: 12px;
  }
</style>

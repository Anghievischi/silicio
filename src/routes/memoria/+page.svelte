<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { onMount } from "svelte";
  import Card from "$lib/components/Card.svelte";
  import Sparkline from "$lib/components/Sparkline.svelte";
  import MemoryAdvice from "$lib/components/upgrades/MemoryAdvice.svelte";
  import { bytes, or } from "$lib/format";
  import { t } from "$lib/i18n/index.svelte";
  import type { MemoryInfo, MemoryLive } from "$lib/types";
  import { loadUpgrades, upgrades } from "$lib/upgrades.svelte";

  const HISTORY = 60;

  let info = $state<MemoryInfo | null>(null);
  let live = $state<MemoryLive | null>(null);
  let history = $state<number[]>([]);
  let error = $state<string | null>(null);

  let modules = $derived(info?.slots.filter((s) => s.size_bytes > 0) ?? []);
  let installed = $derived(modules.reduce((sum, s) => sum + s.size_bytes, 0));
  let usedPct = $derived(live ? (live.used_bytes / live.total_bytes) * 100 : 0);

  type Check = { level: "ok" | "warn" | "info"; text: string };

  let checks = $derived.by<Check[]>(() => {
    if (!info) return [];
    const m = t().memory;
    const list: Check[] = [];
    if (modules.length > 0 && info.channels < 2) {
      list.push({ level: "warn", text: m.singleChannel });
    }
    const slow = modules.filter((s) => s.speed_mts && s.configured_mts && s.configured_mts < s.speed_mts);
    if (slow.length > 0) {
      list.push({ level: "warn", text: m.slow(slow[0].configured_mts!, slow[0].speed_mts!) });
    }
    if (new Set(modules.map((s) => s.part_number)).size > 1) {
      list.push({ level: "info", text: m.mixed });
    }
    if (list.length === 0 && modules.length > 0) {
      list.push({ level: "ok", text: m.allGood });
    }
    return list;
  });

  onMount(() => {
    invoke<MemoryInfo>("memory_info")
      .then((data) => (info = data))
      .catch((e) => (error = String(e)));
    loadUpgrades();

    const tick = async () => {
      try {
        const sample = await invoke<MemoryLive>("memory_live");
        live = sample;
        history = [...history.slice(-(HISTORY - 1)), (sample.used_bytes / sample.total_bytes) * 100];
      } catch (e) {
        error = String(e);
      }
    };
    tick();
    const timer = setInterval(tick, 1000);
    return () => clearInterval(timer);
  });

  function usageColor(pct: number) {
    if (pct > 90) return "var(--hot)";
    if (pct > 75) return "var(--warn)";
    return "var(--good)";
  }
</script>

{#if error}
  <p class="error">{t().memory.error}: {error}</p>
{:else if !info}
  <p class="muted">{t().memory.loading}</p>
{:else}
  <header>
    <div>
      {#if info.kind}<span class="vendor">{info.kind}</span>{/if}
      <h1>{bytes(installed || info.total_bytes)} {info.kind ?? ""}</h1>
      <p class="muted">
        {[t().memory.channels(info.channels), info.speed_mts && `${info.speed_mts} MT/s`].filter(Boolean).join(" · ")}
      </p>
    </div>
  </header>

  <div class="stats">
    <div class="stat">
      <span class="muted">{t().memory.installed}</span>
      <strong class="num">{bytes(installed || info.total_bytes)}</strong>
    </div>
    <div class="stat">
      <span class="muted">{t().memory.usable}</span>
      <strong class="num">{bytes(info.total_bytes)}</strong>
    </div>
    <div class="stat">
      <span class="muted">{t().memory.effectiveSpeed}</span>
      <strong class="num">{info.speed_mts ? `${info.speed_mts} MT/s` : "—"}</strong>
    </div>
    <div class="stat">
      <span class="muted">{t().memory.dramClock}</span>
      <strong class="num">{info.speed_mts ? `${info.speed_mts / 2} MHz` : "—"}</strong>
    </div>
    <div class="stat">
      <span class="muted">{t().memory.maxCapacity}</span>
      <strong class="num">{bytes(info.max_capacity_bytes)}</strong>
    </div>
  </div>

  <div class="grid">
    <Card title={t().memory.liveUsage} span={2}>
      <div class="usage-head">
        <div>
          <span class="big num">{usedPct.toFixed(0)}%</span>
          <span class="muted">{t().memory.usedFree(bytes(live?.used_bytes), bytes(live?.available_bytes))}</span>
        </div>
        <div class="muted right">
          Swap <strong class="num">{bytes(live?.swap_used_bytes)}</strong> {t().memory.swapOf} {bytes(live?.swap_total_bytes)}
        </div>
      </div>
      <div class="bar"><div style:width="{usedPct}%" style:background={usageColor(usedPct)}></div></div>
      <Sparkline values={history} max={100} capacity={HISTORY} />
    </Card>

    <Card title={t().memory.diagnostics}>
      {#if checks.length === 0}
        <p class="muted">{t().memory.noModulesToCheck}</p>
      {:else}
        <ul class="checks">
          {#each checks as check}
            <li class={check.level}>{check.text}</li>
          {/each}
        </ul>
      {/if}
    </Card>

    {#if upgrades.data}
      <Card title="{t().tabs.upgrades} · {t().upgrades.memory}" span={3}>
        <MemoryAdvice memory={upgrades.data.memory} />
      </Card>
    {/if}

    <Card title={t().memory.modules} span={3}>
      {#if info.slots.length === 0}
        <p class="muted">{t().memory.noModules}</p>
      {:else}
        <div class="slots">
          {#each info.slots as slot}
            <div class="slot" class:empty={slot.size_bytes === 0}>
              <div class="slot-head">
                <strong>{or(slot.bank ?? slot.locator)}</strong>
                <span class="muted">{or(slot.locator)}</span>
              </div>
              {#if slot.size_bytes === 0}
                <p class="muted">{t().memory.emptySlot}</p>
              {:else}
                <span class="slot-size num">{bytes(slot.size_bytes)}</span>
                <dl>
                  <dt>{t().memory.type}</dt><dd>{or(slot.kind)} {or(slot.form_factor)}</dd>
                  <dt>{t().common.manufacturer}</dt><dd>{or(slot.manufacturer)}</dd>
                  <dt>{t().common.model}</dt><dd class="num">{or(slot.part_number)}</dd>
                  <dt>{t().common.speed}</dt>
                  <dd class="num">
                    {or(slot.configured_mts)} MT/s
                    {#if slot.speed_mts && slot.speed_mts !== slot.configured_mts}<span class="muted">{t().memory.nominal(slot.speed_mts)}</span>{/if}
                  </dd>
                  <dt>{t().memory.ranks}</dt><dd class="num">{or(slot.rank)}</dd>
                  <dt>{t().common.bus}</dt><dd class="num">{slot.data_width ? `${slot.data_width} bits` : "—"}</dd>
                  <dt>{t().memory.detail}</dt><dd>{or(slot.detail)}</dd>
                </dl>
              {/if}
            </div>
          {/each}
        </div>
        <p class="hint muted">{t().memory.spdHint} <a href="/spd">{t().tabs.spd}</a>.</p>
      {/if}
    </Card>
  </div>
{/if}

<style>
  .usage-head {
    display: flex;
    justify-content: space-between;
    gap: 12px;
    margin-bottom: 12px;
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

  .bar {
    height: 6px;
    margin-bottom: 10px;
    border-radius: var(--radius-sm);
    background: var(--border);
    overflow: hidden;
  }

  .bar div {
    height: 100%;
    transition: width 0.4s ease;
  }

  .checks {
    display: flex;
    flex-direction: column;
    gap: 10px;
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .checks li {
    padding: 10px 12px;
    border-left: 3px solid;
    border-radius: var(--radius-sm);
    background: var(--surface-2);
    font-size: 13px;
  }

  .checks .ok {
    border-color: var(--good);
  }

  .checks .warn {
    border-color: var(--warn);
  }

  .checks .info {
    border-color: var(--accent);
  }

  .slots {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
    gap: 12px;
  }

  .slot {
    padding: 14px 16px;
    border-radius: var(--radius-sm);
    background: var(--surface-2);
  }

  .slot.empty {
    opacity: 0.6;
  }

  .slot-head {
    display: flex;
    justify-content: space-between;
    gap: 8px;
    font-size: 12px;
  }

  .slot-size {
    display: block;
    margin: 6px 0 12px;
    font-size: 24px;
    font-weight: 700;
  }

  .slot dl {
    font-size: 13px;
  }

  .hint {
    margin: 12px 0 0;
    font-size: 11px;
  }
</style>

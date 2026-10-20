<script lang="ts">
  import { t } from "$lib/i18n/index.svelte";
  import type { Core, CpuInfo, CpuLive } from "$lib/types";

  let { info, live }: { info: CpuInfo; live: CpuLive | null } = $props();

  // Núcleos agrupados como no chip: cada grupo divide um cache L3 (CCX na AMD).
  let groups = $derived.by(() => {
    const map = new Map<number, Core[]>();
    for (const core of info.topology) map.set(core.cluster, [...(map.get(core.cluster) ?? []), core]);
    return [...map.entries()].sort(([a], [b]) => a - b).map(([, cores]) => cores);
  });

  function load(core: Core) {
    if (!live) return 0;
    const usages = core.threads.map((i) => live.threads[i]?.usage ?? 0);
    return usages.reduce((a, b) => a + b, 0) / usages.length;
  }

  function clock(core: Core) {
    const mhz = Math.max(0, ...core.threads.map((i) => live?.threads[i]?.mhz ?? 0));
    return mhz ? (mhz / 1000).toFixed(2) : "—";
  }

  // Escala de calor: ocioso → cor de destaque → vermelho.
  function heat(usage: number) {
    if (usage < 50) return `color-mix(in oklab, var(--accent) ${Math.round(8 + usage * 1.1)}%, var(--surface-2))`;
    return `color-mix(in oklab, var(--hot) ${Math.round((usage - 50) * 2)}%, color-mix(in oklab, var(--accent) 63%, var(--surface-2)))`;
  }
</script>

<div class="map">
  {#each groups as cores, g}
    <div class="group">
      {#if groups.length > 1}<span class="group-name muted">{t().coreMap.group(g + 1)}</span>{/if}
      <div class="cores" style:--cols={Math.min(cores.length, 4)}>
        {#each cores as core}
          {@const usage = load(core)}
          <div
            class="core"
            class:hot={usage > 80}
            style:background={heat(usage)}
            title="C{core.id} · {usage.toFixed(0)}%{core.kind ? ` · ${t().coreMap[core.kind]}` : ''}"
          >
            <div class="top">
              <span class="id">C{core.id}</span>
              {#if core.kind}<span class="kind" class:e={core.kind === "efficiency"}>{core.kind === "performance" ? "P" : "E"}</span>{/if}
            </div>
            <span class="ghz num">{clock(core)}</span>
            <div class="threads">
              {#each core.threads as thread}
                <span style:height="{Math.max(live?.threads[thread]?.usage ?? 0, 6)}%"></span>
              {/each}
            </div>
          </div>
        {/each}
      </div>
    </div>
  {/each}
</div>
<div class="legend muted">
  <span>{t().coreMap.idle}</span>
  <span class="scale"></span>
  <span>{t().coreMap.full}</span>
  <span class="legend-text">{t().coreMap.legend}</span>
</div>

<style>
  .map {
    display: flex;
    flex-wrap: wrap;
    gap: 14px;
  }

  .group {
    flex: 1 1 260px;
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 10px;
    border: 1px dashed var(--border);
    border-radius: 12px;
  }

  .group-name {
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.04em;
  }

  .cores {
    display: grid;
    grid-template-columns: repeat(var(--cols), minmax(64px, 1fr));
    gap: 8px;
  }

  .core {
    position: relative;
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 8px 9px;
    border-radius: 10px;
    border: 1px solid color-mix(in srgb, var(--text) 8%, transparent);
    transition:
      background 0.6s ease,
      box-shadow 0.6s ease;
  }

  .core.hot {
    box-shadow: 0 0 14px color-mix(in srgb, var(--hot) 55%, transparent);
  }

  .top {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .id {
    font-size: 11px;
    font-weight: 600;
    opacity: 0.75;
  }

  .kind {
    padding: 0 5px;
    border-radius: 5px;
    font-size: 10px;
    font-weight: 700;
    background: color-mix(in srgb, var(--text) 18%, transparent);
  }

  .kind.e {
    opacity: 0.7;
  }

  .ghz {
    font-size: 17px;
    font-weight: 700;
  }

  .threads {
    display: flex;
    align-items: flex-end;
    gap: 3px;
    height: 16px;
  }

  .threads span {
    flex: 1;
    border-radius: 2px;
    background: color-mix(in srgb, var(--text) 55%, transparent);
    transition: height 0.5s ease;
  }

  .legend {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 8px;
    margin-top: 12px;
    font-size: 11px;
  }

  .scale {
    width: 90px;
    height: 6px;
    border-radius: 3px;
    background: linear-gradient(90deg, var(--surface-2), var(--accent), var(--hot));
  }

  .legend-text {
    flex-basis: 100%;
  }
</style>

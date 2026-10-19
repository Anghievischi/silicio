<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { onMount } from "svelte";
  import Card from "$lib/components/Card.svelte";
  import Sparkline from "$lib/components/Sparkline.svelte";
  import { bytes, or } from "$lib/format";
  import type { MemoryInfo, MemoryLive } from "$lib/types";

  const HISTORY = 60;

  let info = $state<MemoryInfo | null>(null);
  let live = $state<MemoryLive | null>(null);
  let history = $state<number[]>([]);
  let error = $state<string | null>(null);

  let modules = $derived(info?.slots.filter((s) => s.size_bytes > 0) ?? []);
  let installed = $derived(modules.reduce((t, s) => t + s.size_bytes, 0));
  let usedPct = $derived(live ? (live.used_bytes / live.total_bytes) * 100 : 0);

  type Check = { level: "ok" | "warn" | "info"; text: string };

  let checks = $derived.by<Check[]>(() => {
    if (!info) return [];
    const list: Check[] = [];
    if (modules.length > 0 && info.channels < 2) {
      list.push({
        level: "warn",
        text: "Memória em single channel. Um segundo módulo em outro canal dobra a largura de banda, o que faz muita diferença em gráficos integrados.",
      });
    }
    const slow = modules.filter((s) => s.speed_mts && s.configured_mts && s.configured_mts < s.speed_mts);
    if (slow.length > 0) {
      list.push({
        level: "warn",
        text: `Módulos rodando a ${slow[0].configured_mts} MT/s, abaixo dos ${slow[0].speed_mts} MT/s nominais. Verifique se o XMP/EXPO está ativado no BIOS.`,
      });
    }
    if (new Set(modules.map((s) => s.part_number)).size > 1) {
      list.push({ level: "info", text: "Módulos de modelos diferentes. Funciona, mas kits iguais costumam ser mais estáveis." });
    }
    if (list.length === 0 && modules.length > 0) {
      list.push({ level: "ok", text: "Configuração de memória sem problemas aparentes." });
    }
    return list;
  });

  onMount(() => {
    invoke<MemoryInfo>("memory_info")
      .then((data) => (info = data))
      .catch((e) => (error = String(e)));

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

  function channelLabel(n: number) {
    return ["—", "Single channel", "Dual channel", "Triple channel", "Quad channel"][n] ?? `${n} canais`;
  }

  function usageColor(pct: number) {
    if (pct > 90) return "var(--hot)";
    if (pct > 75) return "var(--warn)";
    return "var(--good)";
  }
</script>

{#if error}
  <p class="error">Não foi possível ler a memória: {error}</p>
{:else if !info}
  <p class="muted">Lendo a memória…</p>
{:else}
  <header>
    <div>
      {#if info.kind}<span class="vendor">{info.kind}</span>{/if}
      <h1>{bytes(installed || info.total_bytes)} {info.kind ?? ""}</h1>
      <p class="muted">
        {[channelLabel(info.channels), info.speed_mts && `${info.speed_mts} MT/s`].filter(Boolean).join(" · ")}
      </p>
    </div>
  </header>

  <div class="stats">
    <div class="stat">
      <span class="muted">Instalada</span>
      <strong class="num">{bytes(installed || info.total_bytes)}</strong>
    </div>
    <div class="stat">
      <span class="muted">Utilizável pelo sistema</span>
      <strong class="num">{bytes(info.total_bytes)}</strong>
    </div>
    <div class="stat">
      <span class="muted">Velocidade efetiva</span>
      <strong class="num">{info.speed_mts ? `${info.speed_mts} MT/s` : "—"}</strong>
    </div>
    <div class="stat">
      <span class="muted">Clock real da DRAM</span>
      <strong class="num">{info.speed_mts ? `${info.speed_mts / 2} MHz` : "—"}</strong>
    </div>
    <div class="stat">
      <span class="muted">Capacidade máxima</span>
      <strong class="num">{bytes(info.max_capacity_bytes)}</strong>
    </div>
  </div>

  <div class="grid">
    <Card title="Uso ao vivo" span={2}>
      <div class="usage-head">
        <div>
          <span class="big num">{usedPct.toFixed(0)}%</span>
          <span class="muted">{bytes(live?.used_bytes)} em uso · {bytes(live?.available_bytes)} livres</span>
        </div>
        <div class="muted right">
          Swap <strong class="num">{bytes(live?.swap_used_bytes)}</strong> de {bytes(live?.swap_total_bytes)}
        </div>
      </div>
      <div class="bar"><div style:width="{usedPct}%" style:background={usageColor(usedPct)}></div></div>
      <Sparkline values={history} max={100} capacity={HISTORY} />
    </Card>

    <Card title="Diagnóstico">
      {#if checks.length === 0}
        <p class="muted">Sem dados de módulos para analisar.</p>
      {:else}
        <ul class="checks">
          {#each checks as check}
            <li class={check.level}>{check.text}</li>
          {/each}
        </ul>
      {/if}
    </Card>

    <Card title="Módulos" span={3}>
      {#if info.slots.length === 0}
        <p class="muted">O sistema não informou os módulos instalados.</p>
      {:else}
        <div class="slots">
          {#each info.slots as slot}
            <div class="slot" class:empty={slot.size_bytes === 0}>
              <div class="slot-head">
                <strong>{or(slot.bank ?? slot.locator)}</strong>
                <span class="muted">{or(slot.locator)}</span>
              </div>
              {#if slot.size_bytes === 0}
                <p class="muted">Slot vazio</p>
              {:else}
                <span class="slot-size num">{bytes(slot.size_bytes)}</span>
                <dl>
                  <dt>Tipo</dt><dd>{or(slot.kind)} {or(slot.form_factor)}</dd>
                  <dt>Fabricante</dt><dd>{or(slot.manufacturer)}</dd>
                  <dt>Modelo</dt><dd class="num">{or(slot.part_number)}</dd>
                  <dt>Velocidade</dt>
                  <dd class="num">
                    {or(slot.configured_mts)} MT/s
                    {#if slot.speed_mts && slot.speed_mts !== slot.configured_mts}<span class="muted">(nominal {slot.speed_mts})</span>{/if}
                  </dd>
                  <dt>Ranks</dt><dd class="num">{or(slot.rank)}</dd>
                  <dt>Barramento</dt><dd class="num">{slot.data_width ? `${slot.data_width} bits` : "—"}</dd>
                  <dt>Detalhe</dt><dd>{or(slot.detail)}</dd>
                </dl>
              {/if}
            </div>
          {/each}
        </div>
        <p class="hint muted">Timings detalhados e a tabela JEDEC estão na aba <a href="/spd">Módulos (SPD)</a>.</p>
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
    margin-bottom: 10px;
    border-radius: 3px;
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
    border-radius: 8px;
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
    border-radius: 12px;
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

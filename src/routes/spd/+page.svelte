<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { onMount } from "svelte";
  import Card from "$lib/components/Card.svelte";
  import { or } from "$lib/format";
  import type { SpdModule } from "$lib/types";

  let modules = $state<SpdModule[] | null>(null);
  let selected = $state(0);
  let error = $state<string | null>(null);

  let current = $derived(modules?.[selected] ?? null);
  let d = $derived(current?.decoded ?? null);

  onMount(() => {
    invoke<SpdModule[]>("spd_info")
      .then((data) => (modules = data))
      .catch((e) => (error = String(e)));
  });

  const ns = (v: number) => `${+v.toFixed(3)} ns`;
  // Largura de banda de pico: MT/s × 8 bytes por transferência (barramento de 64 bits).
  const bandwidth = (mts: number, bus: number) => `${((mts * bus) / 8 / 1000).toFixed(1)} GB/s`;
</script>

{#if error}
  <p class="error">Não foi possível ler o SPD: {error}</p>
{:else if !modules}
  <p class="muted">Lendo os módulos…</p>
{:else if modules.length === 0}
  <header>
    <div>
      <h1>Módulos (SPD)</h1>
      <p class="muted">Nenhum EEPROM SPD disponível.</p>
    </div>
  </header>
  <Card title="Como habilitar">
    <p>
      O Linux expõe o SPD pelos drivers <code>ee1004</code> (DDR4) e <code>spd5118</code> (DDR5). Em alguns sistemas é
      preciso carregar o driver do SMBus (<code>i2c-piix4</code> na AMD, <code>i2c-i801</code> na Intel). Notebooks às
      vezes bloqueiam o acesso no firmware.
    </p>
  </Card>
{:else}
  <header>
    <div>
      <span class="vendor">{current?.kind} {current?.module_type ?? ""}</span>
      <h1>{d ? `${d.size_mb / 1024} GB ${d.pc_name}` : `Slot ${current?.slot}`}</h1>
      <p class="muted">{d ? `${or(d.module_manufacturer)} · ${or(d.part_number)}` : ""}</p>
    </div>
    <div class="slots" role="tablist">
      {#each modules as m, i}
        <button class="ghost" class:active={i === selected} role="tab" aria-selected={i === selected} onclick={() => (selected = i)}>
          Slot {m.slot}
        </button>
      {/each}
    </div>
  </header>

  {#if !d}
    <Card title="Módulo">
      <p class="muted">A decodificação de módulos {current?.kind} ainda não foi implementada.</p>
    </Card>
  {:else}
    <div class="stats">
      <div class="stat">
        <span class="muted">Velocidade máxima</span>
        <strong class="num">{d.max_speed_mts} MT/s</strong>
      </div>
      <div class="stat">
        <span class="muted">Largura de banda</span>
        <strong class="num">{bandwidth(d.max_speed_mts, d.bus_width)}</strong>
      </div>
      <div class="stat">
        <span class="muted">Timings</span>
        <strong class="num">{d.table[0] ? `${d.table[0].cl}-${d.table[0].trcd}-${d.table[0].trp}-${d.table[0].tras}` : "—"}</strong>
      </div>
      <div class="stat">
        <span class="muted">Tensão</span>
        <strong class="num">{d.voltage}</strong>
      </div>
      <div class="stat">
        <span class="muted">XMP / EXPO</span>
        <strong>{d.xmp ? `XMP ${d.xmp.version} · ${d.xmp.profiles} perfil(is)` : "Não"}</strong>
      </div>
    </div>

    <div class="grid">
      <Card title="Módulo">
        <dl>
          <dt>Fabricante</dt><dd>{or(d.module_manufacturer)}</dd>
          <dt>Chips DRAM</dt><dd>{or(d.dram_manufacturer)}</dd>
          <dt>Modelo</dt><dd class="num">{or(d.part_number)}</dd>
          <dt>Fabricação</dt><dd class="num">{d.manufactured ? `semana ${d.manufactured[1]} de ${d.manufactured[0]}` : "—"}</dd>
          <dt>Revisão SPD</dt><dd class="num">{d.spd_revision}</dd>
          <dt>Integridade</dt>
          <dd><span class="badge" class:bad={!d.crc_ok}>{d.crc_ok ? "CRC OK" : "CRC inválido"}</span></dd>
        </dl>
      </Card>

      <Card title="Organização">
        <dl>
          <dt>Tamanho</dt><dd class="num">{d.size_mb / 1024} GB</dd>
          <dt>Ranks</dt><dd class="num">{d.ranks}</dd>
          <dt>Chips</dt><dd class="num">x{d.device_width} · {d.die_density_gbit} Gb</dd>
          <dt>Barramento</dt><dd class="num">{d.bus_width} bits</dd>
          <dt>Bancos</dt><dd class="num">{d.bank_groups} grupos × {d.banks_per_group}</dd>
          <dt>Slot</dt><dd class="num">{current?.slot} <span class="muted">(I²C {current?.address})</span></dd>
        </dl>
      </Card>

      <Card title="Timings mínimos">
        <dl>
          <dt>tCK mín.</dt><dd class="num">{ns(d.timings_ns.tck_min)}</dd>
          <dt>tAA (CAS)</dt><dd class="num">{ns(d.timings_ns.taa)}</dd>
          <dt>tRCD</dt><dd class="num">{ns(d.timings_ns.trcd)}</dd>
          <dt>tRP</dt><dd class="num">{ns(d.timings_ns.trp)}</dd>
          <dt>tRAS</dt><dd class="num">{ns(d.timings_ns.tras)}</dd>
          <dt>tRC</dt><dd class="num">{ns(d.timings_ns.trc)}</dd>
          <dt>tRFC1</dt><dd class="num">{ns(d.timings_ns.trfc1)}</dd>
        </dl>
      </Card>

      <Card title="Tabela de timings JEDEC" span={3}>
        <div class="table-wrap">
          <table class="timings">
            <thead>
              <tr>
                <th>Velocidade</th>
                <th>Clock</th>
                <th>CL</th>
                <th>tRCD</th>
                <th>tRP</th>
                <th>tRAS</th>
                <th>tRC</th>
              </tr>
            </thead>
            <tbody>
              {#each d.table as bin, i}
                <tr class:top={i === 0}>
                  <td>DDR4-{bin.speed_mts}</td>
                  <td>{bin.clock_mhz} MHz</td>
                  <td>{bin.cl}</td>
                  <td>{bin.trcd}</td>
                  <td>{bin.trp}</td>
                  <td>{bin.tras}</td>
                  <td>{bin.trc}</td>
                </tr>
              {/each}
            </tbody>
          </table>
        </div>
        <p class="hint muted">
          CAS suportadas: {d.cas_supported.join(", ")}. Ciclos calculados com o arredondamento da JEDEC.
        </p>
      </Card>
    </div>
  {/if}
{/if}

<style>
  .slots {
    display: flex;
    gap: 6px;
  }

  .ghost.active {
    border-color: var(--accent);
    color: var(--accent);
  }

  .badge {
    padding: 1px 8px;
    border-radius: 6px;
    font-size: 12px;
    color: var(--good);
    background: color-mix(in srgb, var(--good) 14%, transparent);
  }

  .badge.bad {
    color: var(--hot);
    background: color-mix(in srgb, var(--hot) 14%, transparent);
  }

  .table-wrap {
    overflow-x: auto;
  }

  .timings th,
  .timings td {
    text-align: right;
    font-variant-numeric: tabular-nums;
    font-size: 14px;
  }

  .timings th:first-child,
  .timings td:first-child {
    text-align: left;
  }

  .timings thead th {
    font-size: 12px;
    font-weight: 500;
    color: var(--muted);
  }

  .timings tr.top td {
    color: var(--accent);
    font-weight: 600;
  }

  .hint {
    margin: 12px 0 0;
    font-size: 11px;
  }

  code {
    font-size: 12px;
    padding: 1px 5px;
    border-radius: 5px;
    background: var(--surface-2);
  }
</style>

<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { onMount } from "svelte";
  import Card from "$lib/components/Card.svelte";
  import { hex, or, smbiosDate } from "$lib/format";
  import type { BoardInfo, PciDevice, PcieLink } from "$lib/types";

  let info = $state<BoardInfo | null>(null);
  let error = $state<string | null>(null);

  onMount(() => {
    invoke<BoardInfo>("board_info")
      .then((data) => (info = data))
      .catch((e) => (error = String(e)));
  });

  const deviceName = (d: PciDevice | null) => (d ? [d.vendor, d.name ?? hex(d.device_id)].filter(Boolean).join(" ") : "—");
  const link = (gen: number | null, width: number | null) => (gen ? `PCIe ${gen}.0${width ? ` x${width}` : ""}` : "—");

  /** Com a GPU ociosa, a geração cai para economizar energia; a largura não deveria cair. */
  const narrowLink = (l: PcieLink) => !!l.current_width && !!l.max_width && l.current_width < l.max_width;
</script>

{#if error}
  <p class="error">Não foi possível ler a placa-mãe: {error}</p>
{:else if !info}
  <p class="muted">Lendo a placa-mãe…</p>
{:else}
  <header>
    <div>
      {#if info.system.chassis}<span class="vendor">{info.system.chassis}</span>{/if}
      <h1>{or(info.system.product ?? info.board.name)}</h1>
      <p class="muted">{or(info.system.vendor ?? info.board.vendor)}</p>
    </div>
  </header>

  <div class="stats">
    <div class="stat">
      <span class="muted">Fabricante da placa</span>
      <strong>{or(info.board.vendor)}</strong>
    </div>
    <div class="stat">
      <span class="muted">Modelo da placa</span>
      <strong>{or(info.board.name)}</strong>
    </div>
    <div class="stat">
      <span class="muted">Versão do BIOS</span>
      <strong class="num">{or(info.bios.version)}</strong>
    </div>
    <div class="stat">
      <span class="muted">Data do BIOS</span>
      <strong class="num">{smbiosDate(info.bios.date)}</strong>
    </div>
  </div>

  <div class="grid">
    <Card title="Sistema">
      <dl>
        <dt>Fabricante</dt><dd>{or(info.system.vendor)}</dd>
        <dt>Produto</dt><dd>{or(info.system.product)}</dd>
        <dt>Família</dt><dd>{or(info.system.family)}</dd>
        <dt>Tipo</dt><dd>{or(info.system.chassis)}</dd>
      </dl>
    </Card>

    <Card title="Placa-mãe">
      <dl>
        <dt>Fabricante</dt><dd>{or(info.board.vendor)}</dd>
        <dt>Modelo</dt><dd>{or(info.board.name)}</dd>
        <dt>Revisão</dt><dd class="num">{or(info.board.version)}</dd>
      </dl>
    </Card>

    <Card title="BIOS">
      <dl>
        <dt>Marca</dt><dd>{or(info.bios.vendor)}</dd>
        <dt>Versão</dt><dd class="num">{or(info.bios.version)}</dd>
        <dt>Data</dt><dd class="num">{smbiosDate(info.bios.date)}</dd>
        <dt>Release</dt><dd class="num">{or(info.bios.release)}</dd>
      </dl>
    </Card>

    <Card title="Chipset" span={3}>
      <dl>
        <dt>Chipset</dt>
        <dd>
          {deviceName(info.chipset)}
          {#if info.chipset}<span class="muted num">· {hex(info.chipset.vendor_id)}:{hex(info.chipset.device_id)}</span>{/if}
        </dd>
        <dt>Southbridge</dt>
        <dd>
          {deviceName(info.southbridge)}
          {#if info.southbridge?.revision != null}<span class="muted num">· rev. {hex(info.southbridge.revision)}</span>{/if}
        </dd>
      </dl>
    </Card>

    <Card title="Interface gráfica" span={3}>
      {#if info.graphics.length === 0}
        <p class="muted">Nenhum adaptador de vídeo PCI encontrado.</p>
      {:else}
        <table>
          <thead>
            <tr>
              <th>Adaptador</th>
              <th>Barramento</th>
              <th>Link atual</th>
              <th>Máximo suportado</th>
            </tr>
          </thead>
          <tbody>
            {#each info.graphics as gpu}
              <tr>
                <td>{deviceName(gpu)}</td>
                <td class="num muted">{gpu.address}</td>
                <td class="num">
                  {gpu.link ? link(gpu.link.current_gen, gpu.link.current_width) : "—"}
                  {#if gpu.link && narrowLink(gpu.link)}<span class="warn">largura reduzida</span>{/if}
                </td>
                <td class="num">{gpu.link ? link(gpu.link.max_gen, gpu.link.max_width) : "—"}</td>
              </tr>
            {/each}
          </tbody>
        </table>
        <p class="hint muted">A geração do link pode cair com a GPU ociosa para economizar energia; isso é normal.</p>
      {/if}
    </Card>
  </div>
{/if}

<style>
  thead th {
    font-size: 12px;
    font-weight: 500;
    color: var(--muted);
  }

  td:last-child,
  th:last-child {
    text-align: right;
  }

  td {
    font-size: 14px;
  }

  .warn {
    margin-left: 6px;
    padding: 1px 7px;
    border-radius: 6px;
    font-size: 11px;
    color: var(--warn);
    background: color-mix(in srgb, var(--warn) 14%, transparent);
  }

  .hint {
    margin: 10px 0 0;
    font-size: 11px;
  }
</style>

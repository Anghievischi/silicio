<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { onMount } from "svelte";
  import Card from "$lib/components/Card.svelte";
  import CpuAdvice from "$lib/components/upgrades/CpuAdvice.svelte";
  import GpuAdvice from "$lib/components/upgrades/GpuAdvice.svelte";
  import { hex, or, smbiosDate } from "$lib/format";
  import { i18n, t } from "$lib/i18n/index.svelte";
  import type { BoardInfo, PciDevice, PcieLink } from "$lib/types";
  import { loadUpgrades, upgrades } from "$lib/upgrades.svelte";

  let info = $state<BoardInfo | null>(null);
  let error = $state<string | null>(null);

  let chassis = $derived(info?.system.chassis ? (t().board.chassis[info.system.chassis] ?? info.system.chassis) : null);

  onMount(() => {
    invoke<BoardInfo>("board_info")
      .then((data) => (info = data))
      .catch((e) => (error = String(e)));
    loadUpgrades();
  });

  const deviceName = (d: PciDevice | null) => (d ? [d.vendor, d.name ?? hex(d.device_id)].filter(Boolean).join(" ") : "—");
  const link = (gen: number | null, width: number | null) => (gen ? `PCIe ${gen}.0${width ? ` x${width}` : ""}` : "—");
  const date = (d: string | null) => smbiosDate(d, i18n.locale);

  /** Com a GPU ociosa, a geração cai para economizar energia; a largura não deveria cair. */
  const narrowLink = (l: PcieLink) => !!l.current_width && !!l.max_width && l.current_width < l.max_width;
</script>

{#if error}
  <p class="error">{t().board.error}: {error}</p>
{:else if !info}
  <p class="muted">{t().board.loading}</p>
{:else}
  <header>
    <div>
      {#if chassis}<span class="vendor">{chassis}</span>{/if}
      <h1>{or(info.system.product ?? info.board.name)}</h1>
      <p class="muted">{or(info.system.vendor ?? info.board.vendor)}</p>
    </div>
  </header>

  <div class="stats">
    <div class="stat">
      <span class="muted">{t().board.boardVendor}</span>
      <strong>{or(info.board.vendor)}</strong>
    </div>
    <div class="stat">
      <span class="muted">{t().board.boardModel}</span>
      <strong>{or(info.board.name)}</strong>
    </div>
    <div class="stat">
      <span class="muted">{t().board.biosVersion}</span>
      <strong class="num">{or(info.bios.version)}</strong>
    </div>
    <div class="stat">
      <span class="muted">{t().board.biosDate}</span>
      <strong class="num">{date(info.bios.date)}</strong>
    </div>
  </div>

  <div class="grid">
    <Card title={t().board.system}>
      <dl>
        <dt>{t().common.manufacturer}</dt><dd>{or(info.system.vendor)}</dd>
        <dt>{t().board.product}</dt><dd>{or(info.system.product)}</dd>
        <dt>{t().common.family}</dt><dd>{or(info.system.family)}</dd>
        <dt>{t().board.type}</dt><dd>{or(chassis)}</dd>
      </dl>
    </Card>

    <Card title={t().board.board}>
      <dl>
        <dt>{t().common.manufacturer}</dt><dd>{or(info.board.vendor)}</dd>
        <dt>{t().common.model}</dt><dd>{or(info.board.name)}</dd>
        <dt>{t().board.revision}</dt><dd class="num">{or(info.board.version)}</dd>
      </dl>
    </Card>

    <Card title="BIOS">
      <dl>
        <dt>{t().board.brand}</dt><dd>{or(info.bios.vendor)}</dd>
        <dt>{t().common.version}</dt><dd class="num">{or(info.bios.version)}</dd>
        <dt>{t().common.date}</dt><dd class="num">{date(info.bios.date)}</dd>
        <dt>Release</dt><dd class="num">{or(info.bios.release)}</dd>
      </dl>
    </Card>

    <Card title={t().board.chipset} span={3}>
      <dl>
        <dt>{t().board.chipset}</dt>
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

    <Card title={t().board.graphicsInterface} span={3}>
      {#if info.graphics.length === 0}
        <p class="muted">{t().board.noGpu}</p>
      {:else}
        <table>
          <thead>
            <tr>
              <th>{t().board.adapter}</th>
              <th>{t().common.bus}</th>
              <th>{t().common.pcieCurrent}</th>
              <th>{t().board.maxSupported}</th>
            </tr>
          </thead>
          <tbody>
            {#each info.graphics as gpu}
              <tr>
                <td>{deviceName(gpu)}</td>
                <td class="num muted">{gpu.address}</td>
                <td class="num">
                  {gpu.link ? link(gpu.link.current_gen, gpu.link.current_width) : "—"}
                  {#if gpu.link && narrowLink(gpu.link)}<span class="warn">{t().board.narrow}</span>{/if}
                </td>
                <td class="num">{gpu.link ? link(gpu.link.max_gen, gpu.link.max_width) : "—"}</td>
              </tr>
            {/each}
          </tbody>
        </table>
        <p class="hint muted">{t().board.linkHint}</p>
      {/if}
    </Card>

    {#if upgrades.data}
      <Card title={t().upgrades.gpu} span={2}>
        <GpuAdvice gpu={upgrades.data.gpu} />
      </Card>
      <Card title="{t().upgrades.cpu} · {t().upgrades.socket}">
        <CpuAdvice cpu={upgrades.data.cpu} />
        <a class="more" href="/upgrades">{t().upgrades.more} →</a>
      </Card>
    {/if}
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
    border-radius: var(--radius-sm);
    font-size: 11px;
    color: var(--warn);
    background: color-mix(in srgb, var(--warn) 14%, transparent);
  }

  .hint {
    margin: 10px 0 0;
    font-size: 11px;
  }

  .more {
    display: inline-block;
    margin-top: 14px;
    font-size: 12px;
    font-weight: 600;
  }
</style>

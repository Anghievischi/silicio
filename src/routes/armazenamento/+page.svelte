<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { onMount } from "svelte";
  import Card from "$lib/components/Card.svelte";
  import { diskInterface, diskName, diskSize, or } from "$lib/format";
  import { i18n, t } from "$lib/i18n/index.svelte";
  import type { Disk } from "$lib/types";

  let disks = $state<Disk[] | null>(null);
  let error = $state<string | null>(null);

  const s = () => t().storage;
  const total = $derived(disks?.reduce((sum, d) => sum + d.size_bytes, 0) ?? 0);
  const isWindows = navigator.userAgent.includes("Windows");

  // A temperatura muda; o resto quase nunca. Uma leitura a cada 10 s basta.
  onMount(() => {
    const load = () =>
      invoke<Disk[]>("storage_info")
        .then((d) => (disks = d))
        .catch((e) => (error ??= String(e)));
    load();
    const timer = setInterval(load, 10_000);
    return () => clearInterval(timer);
  });

  const n = (v: number) => v.toLocaleString(i18n.locale);

  function statusColor(status: string | undefined) {
    if (status === "bad") return "var(--hot)";
    if (status === "warning") return "var(--warn)";
    return "var(--good)";
  }

  /** SSDs esquentam mais que HDs; acima de 70 °C a maioria reduz a velocidade. */
  function tempColor(c: number, d: Disk) {
    const [warm, hot] = d.media === "hdd" ? [45, 55] : [60, 70];
    if (c >= hot) return "var(--hot)";
    if (c >= warm) return "var(--warn)";
    return "inherit";
  }

  /** O link NVMe cai de geração em repouso para economizar energia. */
  const slowerNow = (d: Disk) => !!d.link?.current_gen && !!d.link.max_gen && d.link.current_gen < d.link.max_gen;
</script>

{#if error && !disks}
  <p class="error">{s().error}: {error}</p>
{:else if !disks}
  <p class="muted">{s().loading}</p>
{:else}
  <header>
    <div>
      <h1>{t().tabs.storage}</h1>
      <p class="muted">{s().summary(disks.length, diskSize(total))}</p>
    </div>
  </header>

  {#if !disks.length}
    <p class="muted">{s().none}</p>
  {/if}

  <div class="grid">
    {#each disks as d (d.id)}
      {@const h = d.health}
      <Card title={diskName(d)} span={3}>
        <div class="disk">
          <div class="health" style:--status={statusColor(h?.status)}>
            {#if h}
              <span class="muted">{h.life_left != null ? s().lifeLeft : s().health}</span>
              <strong class="big num">{h.life_left != null ? `${h.life_left}%` : s().status[h.status]}</strong>
              {#if h.life_left != null}
                <div class="bar"><div style:width="{h.life_left}%"></div></div>
              {/if}
              <span class="badge">{s().health}: {s().status[h.status]}</span>
              <p class="muted small">{s().statusHint[h.status]}</p>
            {:else}
              <span class="muted">{s().health}</span>
              <strong class="big num">—</strong>
              <p class="muted small">{isWindows ? s().noHealth : s().noHealthLinux}</p>
            {/if}
          </div>

          <dl>
            <dt>{s().capacity}</dt><dd class="num">{diskSize(d.size_bytes)}</dd>
            <dt>{s().type}</dt>
            <dd>
              {d.media ? s()[d.media] : "—"}
              {#if d.removable}<span class="tag">{s().removable}</span>{/if}
            </dd>
            <dt>{s().interface}</dt>
            <dd class="num">
              {diskInterface(d)}
              {#if slowerNow(d)}<span class="muted">· {s().linkNow(`PCIe ${d.link?.current_gen}.0`)}</span>{/if}
            </dd>
            <dt>{s().temperature}</dt>
            <dd class="num" style:color={d.temperature_c != null ? tempColor(d.temperature_c, d) : undefined}>
              {d.temperature_c != null ? `${Math.round(d.temperature_c)} °C` : "—"}
            </dd>
            {#if h}
              <dt>{s().written}</dt><dd class="num">{diskSize(h.written_bytes)}</dd>
              {#if h.read_bytes != null}<dt>{s().read}</dt><dd class="num">{diskSize(h.read_bytes)}</dd>{/if}
              <dt>{s().powerOn}</dt>
              <dd class="num">{h.power_on_hours != null ? s().hours(n(h.power_on_hours), n(Math.round(h.power_on_hours / 24))) : "—"}</dd>
              {#if h.power_cycles != null}<dt>{s().powerCycles}</dt><dd class="num">{n(h.power_cycles)}</dd>{/if}
              {#if h.unsafe_shutdowns != null}<dt>{s().unsafeShutdowns}</dt><dd class="num">{n(h.unsafe_shutdowns)}</dd>{/if}
              {#if h.media_errors != null}<dt>{s().mediaErrors}</dt><dd class="num">{n(h.media_errors)}</dd>{/if}
              {#if h.bad_sectors != null}
                <dt>{s().badSectors}</dt>
                <dd class="num" style:color={h.bad_sectors > 0 ? "var(--warn)" : undefined}>{n(h.bad_sectors)}</dd>
              {/if}
              {#if h.available_spare != null}<dt>{s().spare}</dt><dd class="num">{h.available_spare}%</dd>{/if}
            {/if}
            <dt>{s().firmware}</dt><dd class="num">{or(d.firmware)}</dd>
          </dl>
        </div>

        {#if d.volumes.length}
          <h3>{s().volumes}</h3>
          <ul class="volumes">
            {#each d.volumes as v (v.mount_point)}
              {@const used = v.total_bytes ? ((v.total_bytes - v.available_bytes) / v.total_bytes) * 100 : 0}
              <li>
                <div class="volume-head">
                  <strong class="num">{v.mount_point}</strong>
                  <span class="muted">{[v.label, v.file_system].filter(Boolean).join(" · ")}</span>
                  <span class="free num">{s().free(diskSize(v.available_bytes), diskSize(v.total_bytes))}</span>
                </div>
                <div class="bar">
                  <div style:width="{used}%" style:background={used > 90 ? "var(--hot)" : used > 75 ? "var(--warn)" : "var(--accent)"}></div>
                </div>
              </li>
            {/each}
          </ul>
        {/if}
      </Card>
    {/each}
  </div>

  {#if disks.some((d) => d.health?.life_left != null)}
    <p class="hint muted">{s().lifeHint}</p>
  {/if}
{/if}

<style>
  .disk {
    display: grid;
    grid-template-columns: minmax(200px, 260px) 1fr;
    gap: 24px;
  }

  @media (max-width: 760px) {
    .disk {
      grid-template-columns: 1fr;
    }
  }

  .health {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .big {
    font-family: var(--font-display);
    font-size: 56px;
    font-weight: 900;
    line-height: 0.95;
    color: var(--status);
  }

  .badge {
    align-self: flex-start;
    padding: 2px 8px;
    border-radius: var(--radius-sm);
    font-size: 12px;
    font-weight: 600;
    color: var(--status);
    background: color-mix(in srgb, var(--status) 14%, transparent);
  }

  .small {
    margin: 0;
    font-size: 12px;
  }

  .bar {
    height: 6px;
    border-radius: var(--radius-sm);
    background: var(--border);
    overflow: hidden;
  }

  .bar div {
    height: 100%;
    background: var(--status, var(--accent));
    transition: width 0.4s ease;
  }

  .tag {
    margin-left: 6px;
    padding: 1px 7px;
    border-radius: var(--radius-sm);
    font-size: 11px;
    background: var(--surface-2);
  }

  h3 {
    margin: 20px 0 10px;
    font-size: 13px;
    font-weight: 600;
    color: var(--muted);
  }

  .volumes {
    display: flex;
    flex-direction: column;
    gap: 12px;
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .volume-head {
    display: flex;
    flex-wrap: wrap;
    align-items: baseline;
    gap: 4px 10px;
    margin-bottom: 6px;
    font-size: 13px;
  }

  .free {
    margin-left: auto;
  }

  .hint {
    margin: 12px 0 0;
    font-size: 12px;
  }
</style>

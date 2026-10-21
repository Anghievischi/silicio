<script lang="ts">
  import { t } from "$lib/i18n/index.svelte";
  import type { Upgrades } from "$lib/types";

  let { cpu }: { cpu: Upgrades["cpu"] } = $props();

  let u = $derived(t().upgrades);
</script>

{#if cpu.path === "soldered"}
  <p class="note">{u.cpuSoldered}</p>
{:else if cpu.path === "unknown"}
  <p class="note muted">{u.cpuUnknown}</p>
{:else}
  <dl>
    <dt>{u.socket}</dt><dd class="num"><strong>{cpu.socket}</strong></dd>
    <dt>{u.ceiling}</dt><dd>{cpu.ceiling.join(" · ")}</dd>
  </dl>
  <ul class="notes">
    {#if cpu.at_ceiling}<li>{u.atCeiling}</li>{/if}
    {#if cpu.platform_end}<li class="warn">{u.platformEnd}</li>{/if}
    {#if cpu.check_bios && !cpu.at_ceiling}<li>{u.checkBios}</li>{/if}
  </ul>
{/if}

<style>
  .note {
    margin: 0;
  }

  dl {
    font-size: 13px;
  }

  .notes {
    display: flex;
    flex-direction: column;
    gap: 6px;
    margin: 12px 0 0;
    padding: 0;
    list-style: none;
    font-size: 12px;
  }

  .notes li::before {
    content: "> ";
    color: var(--accent);
    font-family: var(--font-mono);
  }

  .notes li.warn::before {
    content: "! ";
    color: var(--warn);
  }
</style>

<script lang="ts">
  import { bytes } from "$lib/format";
  import { t } from "$lib/i18n/index.svelte";
  import type { Upgrades } from "$lib/types";
  import { memoryAction, memorySpec } from "$lib/upgrades.svelte";

  let { memory }: { memory: Upgrades["memory"] } = $props();

  let u = $derived(t().upgrades);
  let module = $derived(bytes(memory.module_bytes));
  let buying = $derived(["dual-channel", "add", "replace"].includes(memory.action));
</script>

<p class="action" class:go={buying}>{memoryAction(memory)}</p>

<dl>
  {#if buying}
    <dt>{u.memSpec}</dt><dd class="num">{memory.module_count} × {module} {memorySpec(memory)}</dd>
    <dt>{u.memTarget}</dt><dd class="num">{bytes(memory.installed_bytes)} → <strong>{bytes(memory.target_bytes)}</strong></dd>
  {/if}
  {#if memory.slots_total > 0}
    <dt>{u.memSlots}</dt><dd class="num">{memory.slots_used} / {memory.slots_total}</dd>
  {/if}
  <dt>{u.memMax}</dt><dd class="num">{bytes(memory.max_bytes)}</dd>
</dl>

{#if buying}<p class="muted small">{u.memMatch}</p>{/if}

<style>
  .action {
    margin: 0 0 12px;
    font-weight: 600;
  }

  .action.go::before {
    content: "> ";
    color: var(--accent);
    font-family: var(--font-mono);
  }

  dl {
    font-size: 13px;
  }

  .small {
    margin: 10px 0 0;
    font-size: 11px;
  }
</style>

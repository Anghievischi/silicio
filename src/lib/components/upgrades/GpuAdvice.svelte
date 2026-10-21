<script lang="ts">
  import { t } from "$lib/i18n/index.svelte";
  import type { Upgrades } from "$lib/types";
  import { gpuPath } from "$lib/upgrades.svelte";

  let { gpu }: { gpu: Upgrades["gpu"] } = $props();

  let u = $derived(t().upgrades);
</script>

{#if gpu.path === "none"}
  <p class="note">{u.gpuNone}</p>
{:else}
  <div class="head">
    <span class="tag">{gpuPath(gpu)}</span>
    <span class="tier">{u.tier}: <strong>{u.tiers[gpu.tier]}</strong></span>
  </div>
  {#if gpu.path === "external"}<p class="muted small">{u.gpuExternalHint}</p>{/if}
  {#if gpu.current}
    <dl>
      <dt>{u.current}</dt><dd>{gpu.current}</dd>
    </dl>
  {/if}

  <h3>{u.suggestions}</h3>
  <ul class="cards">
    {#each gpu.suggestions as card}
      <li>
        <strong>{card.name}</strong>
        <span class="specs num">
          <span class:warn={gpu.narrow_slot && card.lanes < 16}>{u.lanes(card.lanes)}</span>
          · {u.vram(card.vram_gb)}
        </span>
      </li>
    {/each}
  </ul>
  <p class="muted small">{u.tierHint}</p>

  <ul class="notes">
    {#if gpu.narrow_slot}<li class="warn">{u.narrowSlot}</li>{/if}
    {#if gpu.no_rebar}<li>{u.noRebar}</li>{/if}
    <li>{u.psu(gpu.psu_watts)}</li>
    {#if gpu.path === "slot"}<li>{u.checkSpace}</li>{/if}
  </ul>
{/if}

<style>
  .head {
    display: flex;
    flex-wrap: wrap;
    justify-content: space-between;
    align-items: center;
    gap: 8px;
    margin-bottom: 10px;
  }

  .tag {
    padding: 3px 8px;
    background: var(--accent);
    color: var(--on-accent);
    font-family: var(--font-mono);
    font-size: 11px;
    font-weight: 700;
    letter-spacing: 0.08em;
    text-transform: uppercase;
  }

  .tier {
    font-size: 12px;
    color: var(--muted);
  }

  .tier strong {
    color: var(--text);
  }

  h3 {
    margin: 14px 0 8px;
    font-family: var(--font-mono);
    font-size: 11px;
    font-weight: 700;
    letter-spacing: 0.12em;
    text-transform: uppercase;
    color: var(--muted);
  }

  .cards {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(170px, 1fr));
    gap: 8px;
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .cards li {
    display: flex;
    flex-direction: column;
    gap: 4px;
    padding: 10px 12px;
    border: 2px solid var(--border);
    background: var(--bg);
  }

  .specs {
    font-family: var(--font-mono);
    font-size: 11px;
    color: var(--muted);
  }

  .specs .warn {
    color: var(--warn);
    font-weight: 700;
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

  .note {
    margin: 0;
  }

  .small {
    margin: 8px 0 0;
    font-size: 11px;
  }
</style>

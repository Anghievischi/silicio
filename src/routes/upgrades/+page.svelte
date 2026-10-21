<script lang="ts">
  import { onMount } from "svelte";
  import Card from "$lib/components/Card.svelte";
  import CpuAdvice from "$lib/components/upgrades/CpuAdvice.svelte";
  import GpuAdvice from "$lib/components/upgrades/GpuAdvice.svelte";
  import MemoryAdvice from "$lib/components/upgrades/MemoryAdvice.svelte";
  import { t } from "$lib/i18n/index.svelte";
  import { loadUpgrades, upgrades } from "$lib/upgrades.svelte";

  onMount(loadUpgrades);

  let u = $derived(t().upgrades);
</script>

{#if upgrades.error}
  <p class="error">{u.error}: {upgrades.error}</p>
{:else if !upgrades.data}
  <p class="muted">{u.loading}</p>
{:else}
  {@const data = upgrades.data}
  <header>
    <div>
      <span class="vendor">{u.form[data.form]}</span>
      <h1>{u.title}</h1>
      <p class="muted">{u.intro}</p>
    </div>
  </header>

  <section class="best">
    <span class="label">{u.bestMove}</span>
    <strong>{u.top[data.top]}</strong>
  </section>

  <!-- Sem como trocar a placa de vídeo, a memória vira o card principal. -->
  {@const gpuOpen = data.gpu.path !== "none"}
  <div class="grid">
    <div class="slot" style:order={gpuOpen ? 0 : 1} style:grid-column="span {gpuOpen ? 2 : 1}">
      <Card title={u.gpu}>
        <GpuAdvice gpu={data.gpu} />
      </Card>
    </div>
    <div class="slot" style:grid-column="span {gpuOpen ? 1 : 2}">
      <Card title={u.memory}>
        <MemoryAdvice memory={data.memory} />
      </Card>
    </div>
    <div class="slot" style:order="2" style:grid-column="1 / -1">
    <Card title={u.cpu}>
      <CpuAdvice cpu={data.cpu} />
    </Card>
    </div>
  </div>

  <p class="muted disclaimer">{u.disclaimer}</p>
{/if}

<style>
  .best {
    display: flex;
    flex-wrap: wrap;
    align-items: baseline;
    gap: 8px 16px;
    margin-bottom: 16px;
    padding: 16px 20px;
    border: 2px solid var(--accent);
    background: var(--surface);
  }

  .label {
    padding: 2px 8px;
    background: var(--accent);
    color: var(--on-accent);
    font-family: var(--font-mono);
    font-size: 11px;
    font-weight: 700;
    letter-spacing: 0.12em;
    text-transform: uppercase;
  }

  .best strong {
    font-size: 16px;
  }

  .slot {
    display: flex;
    min-width: 0;
  }

  .slot > :global(section) {
    flex: 1;
  }

  @media (max-width: 1000px) {
    .slot {
      grid-column: 1 / -1 !important;
    }
  }

  .disclaimer {
    margin: 16px 0 0;
    font-size: 11px;
  }
</style>

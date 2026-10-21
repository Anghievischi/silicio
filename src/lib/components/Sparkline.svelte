<script lang="ts">
  let { values, max, capacity = 60 }: { values: number[]; max: number; capacity?: number } = $props();

  // Barras verticais, uma por amostra; as vagas ainda sem dado ficam à esquerda.
  let bars = $derived([
    ...Array(Math.max(capacity - values.length, 0)).fill(null),
    ...values.slice(-capacity).map((v) => Math.max(Math.min(v / max, 1), 0) * 100),
  ]);
</script>

<div class="spark" aria-hidden="true">
  {#each bars as h, i}
    <span class:last={i === bars.length - 1} style:height="{h ?? 0}%"></span>
  {/each}
</div>

<style>
  .spark {
    display: flex;
    align-items: flex-end;
    gap: 2px;
    height: 70px;
    border-bottom: 2px solid var(--border);
  }

  span {
    flex: 1;
    min-height: 1px;
    background: color-mix(in srgb, var(--accent) 55%, var(--surface-2));
    transition: height 0.4s ease;
  }

  span.last {
    background: var(--accent);
  }
</style>

<script lang="ts">
  let { values, max, capacity = 60 }: { values: number[]; max: number; capacity?: number } = $props();

  const W = 300;
  const H = 70;

  let points = $derived(
    values
      .map((v, i) => {
        const x = ((capacity - values.length + i) / (capacity - 1)) * W;
        const y = H - (Math.min(v, max) / max) * (H - 4) - 2;
        return `${x.toFixed(1)},${y.toFixed(1)}`;
      })
      .join(" "),
  );
</script>

<svg viewBox="0 0 {W} {H}" preserveAspectRatio="none" aria-hidden="true">
  {#if values.length > 1}
    <polygon points="{points} {W},{H} {points.split(' ')[0].split(',')[0]},{H}" />
    <polyline {points} />
  {/if}
</svg>

<style>
  svg {
    width: 100%;
    height: 70px;
    display: block;
  }

  polyline {
    fill: none;
    stroke: var(--accent);
    stroke-width: 2;
    vector-effect: non-scaling-stroke;
    stroke-linejoin: round;
  }

  polygon {
    fill: color-mix(in srgb, var(--accent) 14%, transparent);
  }
</style>

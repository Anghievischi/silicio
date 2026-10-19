<script lang="ts">
  import type { Snippet } from "svelte";

  let { title, span = 1, children }: { title: string; span?: number; children: Snippet } = $props();
</script>

<section style:grid-column="span {span}">
  <h2>{title}</h2>
  {@render children()}
</section>

<style>
  section {
    padding: 18px 20px;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: var(--surface);
    min-width: 0;
  }

  h2 {
    margin: 0 0 14px;
    font-size: 12px;
    font-weight: 600;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: var(--muted);
  }

  @media (max-width: 1000px) {
    section {
      grid-column: 1 / -1 !important;
    }
  }
</style>

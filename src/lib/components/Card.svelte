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
    padding: 16px 18px;
    border: 2px solid var(--border);
    background: var(--surface);
    min-width: 0;
  }

  h2 {
    margin: 0 0 14px;
    font-family: var(--font-display);
    font-size: 24px;
    font-weight: 900;
    line-height: 1;
    letter-spacing: 0.04em;
    text-transform: uppercase;
    color: var(--text);
  }

  @media (max-width: 1000px) {
    section {
      grid-column: 1 / -1 !important;
    }
  }
</style>

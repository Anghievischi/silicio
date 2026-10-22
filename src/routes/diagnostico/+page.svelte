<script lang="ts">
  import { onMount } from "svelte";
  import { tabs } from "$lib/components/Topbar.svelte";
  import { AREA_TAB, diagnosis, findingCounts, findingText, loadDiagnosis } from "$lib/diagnosis.svelte";
  import { t } from "$lib/i18n/index.svelte";
  import { system } from "$lib/system.svelte";

  onMount(loadDiagnosis);

  let d = $derived(t().diagnosis);

  /** Nome traduzido da aba de um achado; a de bateria só existe em notebooks. */
  function tabName(href: string) {
    const tab = tabs.find((x) => x.href === href);
    if (!tab || (tab.key === "battery" && !system.hasBattery)) return null;
    return t().tabs[tab.key];
  }
</script>

{#if diagnosis.error}
  <p class="error">{d.error}: {diagnosis.error}</p>
{:else if !diagnosis.data}
  <p class="muted">{d.loading}</p>
{:else}
  {@const data = diagnosis.data}
  <header>
    <div>
      <span class="vendor">{d.status[data.status]}</span>
      <h1>{d.title}</h1>
      <p class="muted">{d.intro}</p>
    </div>
  </header>

  <section class="score {data.status}">
    <div>
      <span class="label">{d.score}</span>
      <strong class="num">{data.score}<small>/100</small></strong>
    </div>
    <span class="counts">{findingCounts(data)}</span>
  </section>

  {#if data.findings.length === 0}
    <p class="none">{d.none}</p>
  {:else}
    <ul class="findings">
      {#each data.findings as finding}
        {@const text = findingText(finding)}
        {@const href = AREA_TAB[finding.area]}
        {@const tab = tabName(href)}
        <li class={finding.severity}>
          <div class="head">
            <span class="sev">{d.severity[finding.severity]}</span>
            <strong>{text.title}</strong>
            {#if finding.subject}<span class="muted subject">{finding.subject}</span>{/if}
          </div>
          <p>{text.text}</p>
          {#if tab}<a {href}>{d.open(tab)} →</a>{/if}
        </li>
      {/each}
    </ul>
  {/if}

  <p class="muted disclaimer">{d.disclaimer}</p>
{/if}

<style>
  .score {
    display: flex;
    flex-wrap: wrap;
    align-items: flex-end;
    justify-content: space-between;
    gap: 8px 16px;
    margin-bottom: 12px;
    padding: 16px 20px;
    border: 2px solid var(--good);
    background: var(--surface);
  }

  .score.attention {
    border-color: var(--warn);
  }

  .score.problem {
    border-color: var(--hot);
  }

  .label {
    display: block;
    font-size: 11px;
    font-weight: 700;
    letter-spacing: 0.12em;
    text-transform: uppercase;
    color: var(--muted);
  }

  .score strong {
    font-family: var(--font-display);
    font-size: 64px;
    font-weight: 900;
    line-height: 0.9;
  }

  .score small {
    font-size: 24px;
    color: var(--muted);
  }

  .counts {
    font-size: 13px;
    font-weight: 700;
  }

  .none {
    padding: 16px 20px;
    border: 2px solid var(--border);
    border-left: 4px solid var(--good);
    background: var(--surface);
  }

  .findings {
    display: flex;
    flex-direction: column;
    gap: 10px;
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .findings li {
    padding: 14px 18px;
    border: 2px solid var(--border);
    border-left: 4px solid var(--accent);
    background: var(--surface);
  }

  .findings li.warning {
    border-left-color: var(--warn);
  }

  .findings li.problem {
    border-left-color: var(--hot);
  }

  .head {
    display: flex;
    flex-wrap: wrap;
    align-items: baseline;
    gap: 4px 10px;
  }

  .sev {
    padding: 1px 6px;
    border: 1px solid currentColor;
    font-size: 10px;
    font-weight: 700;
    letter-spacing: 0.12em;
    text-transform: uppercase;
    color: var(--accent);
  }

  .warning .sev {
    color: var(--warn);
  }

  .problem .sev {
    color: var(--hot);
  }

  .head strong {
    font-size: 15px;
  }

  .subject {
    font-size: 12px;
  }

  .findings p {
    margin: 8px 0 0;
    font-size: 13px;
    line-height: 1.5;
  }

  .findings a {
    display: inline-block;
    margin-top: 8px;
    font-size: 12px;
    font-weight: 600;
    color: var(--accent);
    text-decoration: none;
  }

  .disclaimer {
    margin: 16px 0 0;
    font-size: 11px;
  }
</style>

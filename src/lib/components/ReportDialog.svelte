<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { t } from "$lib/i18n/index.svelte";
  import { collect, fileName, render, type Format } from "$lib/report";
  import { reportDialog } from "$lib/system.svelte";

  let dialog: HTMLDialogElement;
  let busy = $state(false);
  let saved = $state<string | null>(null);
  let error = $state<string | null>(null);
  let copied = $state(false);

  const formats: { id: Format; icon: string }[] = [
    { id: "html", icon: "Aa" },
    { id: "md", icon: "M↓" },
    { id: "json", icon: "{ }" },
  ];

  $effect(() => {
    if (reportDialog.open && !dialog.open) {
      saved = error = null;
      dialog.showModal();
    } else if (!reportDialog.open && dialog.open) {
      dialog.close();
    }
  });

  async function save(format: Format) {
    busy = true;
    error = null;
    try {
      const contents = render(format, await collect());
      saved = await invoke<string>("save_report", { name: fileName(format), contents });
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }

  async function copy() {
    busy = true;
    try {
      await navigator.clipboard.writeText(render("md", await collect()));
      copied = true;
      setTimeout(() => (copied = false), 1500);
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }

  const open = (reveal: boolean) =>
    invoke("open_report", { path: saved, reveal }).catch((e) => (error = String(e)));
</script>

<dialog bind:this={dialog} onclose={() => (reportDialog.open = false)}>
  <h2>{t().report.button}</h2>
  <p class="muted">{t().report.intro} {t().report.privacy}</p>

  {#if saved}
    <div class="saved">
      <span class="muted">{t().report.saved}</span>
      <code>{saved}</code>
      <div class="actions">
        <button class="primary" onclick={() => open(false)}>{t().report.open}</button>
        <button class="ghost" onclick={() => open(true)}>{t().report.reveal}</button>
      </div>
    </div>
  {:else}
    <p class="muted small">{t().report.choose}</p>
    <div class="formats">
      {#each formats as f}
        {@const label = f.id === "html" ? t().report.html : f.id === "md" ? t().report.markdown : t().report.json}
        {@const hint = f.id === "html" ? t().report.htmlHint : f.id === "md" ? t().report.markdownHint : t().report.jsonHint}
        <button class="format" disabled={busy} onclick={() => save(f.id)}>
          <span class="icon">{f.icon}</span>
          <strong>{label}</strong>
          <span class="muted">{hint}</span>
        </button>
      {/each}
    </div>
  {/if}

  {#if busy}<p class="muted small">{t().report.collecting}</p>{/if}
  {#if error}<p class="error small">{error}</p>{/if}

  <div class="footer">
    <button class="ghost" disabled={busy} onclick={copy}>{copied ? t().report.copied : t().report.copy}</button>
    <button class="ghost" onclick={() => (reportDialog.open = false)}>{t().report.close}</button>
  </div>
</dialog>

<style>
  dialog {
    width: min(620px, calc(100vw - 32px));
    padding: 24px;
    border: 2px solid var(--border);
    border-radius: 0;
    background: var(--surface);
    color: var(--text);
    box-shadow: 0 24px 80px rgb(0 0 0 / 0.45);
  }

  dialog::backdrop {
    background: rgb(0 0 0 / 0.5);
    backdrop-filter: blur(4px);
  }

  h2 {
    margin: 0 0 6px;
    font-family: var(--font-display);
    font-size: 28px;
    font-weight: 900;
    line-height: 1;
    text-transform: uppercase;
  }

  p {
    margin: 0 0 14px;
  }

  .small {
    font-size: 12px;
  }

  .formats {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: 10px;
  }

  .format {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 4px;
    padding: 14px;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: var(--surface-2);
    text-align: left;
    transition:
      border-color 0.2s,
      transform 0.2s;
  }

  .format:hover:not(:disabled) {
    border-color: var(--accent);
    transform: translateY(-2px);
  }

  .format .icon {
    display: grid;
    place-items: center;
    width: 34px;
    height: 34px;
    margin-bottom: 4px;
    border-radius: var(--radius-sm);
    font-size: 13px;
    font-weight: 700;
    color: var(--accent);
    background: color-mix(in srgb, var(--accent) 16%, transparent);
  }

  .format span:last-child {
    font-size: 12px;
  }

  .saved {
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 14px;
    border-radius: var(--radius-sm);
    background: color-mix(in srgb, var(--good) 10%, var(--surface-2));
    border: 1px solid color-mix(in srgb, var(--good) 40%, var(--border));
  }

  code {
    overflow-wrap: anywhere;
    font-size: 12px;
  }

  .actions,
  .footer {
    display: flex;
    gap: 8px;
  }

  .footer {
    justify-content: flex-end;
    margin-top: 18px;
  }

  .primary {
    padding: 8px 14px;
    border: 0;
    border-radius: var(--radius-sm);
    background: var(--accent);
    color: var(--on-accent);
    font-weight: 600;
  }

  @media (max-width: 560px) {
    .formats {
      grid-template-columns: 1fr;
    }
  }
</style>

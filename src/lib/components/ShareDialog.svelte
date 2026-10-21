<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { t } from "$lib/i18n/index.svelte";
  import { collect } from "$lib/report";
  import { cardSvg, toPng, type Theme } from "$lib/share";
  import { shareDialog } from "$lib/system.svelte";

  let dialog: HTMLDialogElement;
  let data = $state<Awaited<ReturnType<typeof collect>> | null>(null);
  let theme = $state<Theme>("dark");
  let withBench = $state(true);
  let busy = $state(false);
  let saved = $state<string | null>(null);
  let copied = $state(false);
  let error = $state<string | null>(null);
  const canCopy = typeof ClipboardItem !== "undefined";

  let svg = $derived(data ? cardSvg(data, theme, withBench) : "");
  let preview = $derived(svg ? `data:image/svg+xml;charset=utf-8,${encodeURIComponent(svg)}` : "");

  $effect(() => {
    if (shareDialog.open && !dialog.open) {
      saved = error = null;
      data = null;
      dialog.showModal();
      collect()
        .then((d) => (data = d))
        .catch((e) => (error = String(e)));
    } else if (!shareDialog.open && dialog.open) {
      dialog.close();
    }
  });

  function fileName() {
    const d = new Date();
    const pad = (n: number) => String(n).padStart(2, "0");
    return `Silicio-card-${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}-${pad(d.getHours())}${pad(d.getMinutes())}.png`;
  }

  async function save() {
    busy = true;
    error = null;
    try {
      const png = new Uint8Array(await (await toPng(svg)).arrayBuffer());
      saved = await invoke<string>("save_image", png, { headers: { name: fileName() } });
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }

  async function copy() {
    busy = true;
    error = null;
    try {
      await navigator.clipboard.write([new ClipboardItem({ "image/png": toPng(svg) })]);
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

<dialog bind:this={dialog} onclose={() => (shareDialog.open = false)}>
  <header>
    <div>
      <h2>{t().share.title}</h2>
      <p class="muted">{t().share.hint}</p>
    </div>
    <button class="ghost" onclick={() => (shareDialog.open = false)}>{t().report.close}</button>
  </header>

  <div class="preview">
    {#if preview}
      <img src={preview} alt={t().share.title} />
    {:else}
      <div class="placeholder muted">{t().share.generating}</div>
    {/if}
  </div>

  <div class="options">
    <div class="segmented" role="radiogroup" aria-label={t().share.theme}>
      {#each [["dark", t().share.dark], ["light", t().share.light]] as [id, label]}
        <button role="radio" aria-checked={theme === id} class:on={theme === id} onclick={() => (theme = id as Theme)}>{label}</button>
      {/each}
    </div>
    <label class="check" title={data && !data.bench ? t().share.noBench : ""}>
      <input type="checkbox" bind:checked={withBench} disabled={!data?.bench} />
      {t().share.includeBench}
    </label>
    <div class="actions">
      {#if canCopy}
        <button class="ghost" disabled={!svg || busy} onclick={copy}>{copied ? t().share.copied : t().share.copy}</button>
      {/if}
      <button class="primary" disabled={!svg || busy} onclick={save}>↓ {t().share.save}</button>
    </div>
  </div>

  {#if data && !data.bench}<p class="muted small">{t().share.noBench}</p>{/if}
  {#if saved}
    <div class="saved">
      <span class="muted">{t().share.saved}</span>
      <code>{saved}</code>
      <div class="actions">
        <button class="primary" onclick={() => open(false)}>{t().report.open}</button>
        <button class="ghost" onclick={() => open(true)}>{t().report.reveal}</button>
      </div>
    </div>
  {/if}
  {#if error}<p class="error small">{error}</p>{/if}
</dialog>

<style>
  dialog {
    width: min(860px, calc(100vw - 32px));
    max-height: calc(100vh - 48px);
    padding: 22px 24px;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: var(--surface);
    color: var(--text);
    box-shadow: 0 24px 80px rgb(0 0 0 / 0.45);
  }

  dialog::backdrop {
    background: rgb(0 0 0 / 0.5);
    backdrop-filter: blur(4px);
  }

  header {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    gap: 16px;
    margin-bottom: 14px;
  }

  h2 {
    margin: 0 0 4px;
    font-size: 20px;
  }

  header p {
    margin: 0;
  }

  .preview img,
  .placeholder {
    display: block;
    width: 100%;
    aspect-ratio: 1200 / 630;
    border-radius: var(--radius-sm);
    border: 1px solid var(--border);
  }

  .placeholder {
    display: grid;
    place-items: center;
    background: var(--surface-2);
  }

  .options {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 16px;
    margin-top: 14px;
  }

  .segmented {
    display: flex;
    padding: 3px;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: var(--surface-2);
  }

  .segmented button {
    padding: 5px 12px;
    border: 0;
    border-radius: var(--radius-sm);
    background: none;
    font-size: 13px;
  }

  .segmented .on {
    background: var(--surface);
    color: var(--accent);
    font-weight: 600;
  }

  .check {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 13px;
  }

  .actions {
    display: flex;
    gap: 8px;
    margin-left: auto;
  }

  .primary {
    padding: 8px 14px;
    border: 0;
    border-radius: var(--radius-sm);
    background: var(--accent);
    color: var(--on-accent);
    font-weight: 600;
  }

  .primary:disabled {
    opacity: 0.6;
  }

  .saved {
    display: flex;
    flex-direction: column;
    gap: 8px;
    margin-top: 14px;
    padding: 14px;
    border-radius: var(--radius-sm);
    background: color-mix(in srgb, var(--good) 10%, var(--surface-2));
    border: 1px solid color-mix(in srgb, var(--good) 40%, var(--border));
  }

  .saved .actions {
    margin-left: 0;
  }

  code {
    overflow-wrap: anywhere;
    font-size: 12px;
  }

  .small {
    font-size: 12px;
  }
</style>

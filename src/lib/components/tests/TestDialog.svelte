<script lang="ts">
  import type { Snippet } from "svelte";
  import { t } from "$lib/i18n/index.svelte";

  let {
    title,
    open = $bindable(false),
    // O teste de teclado precisa do Esc como tecla comum, sem fechar a janela.
    closeOnEscape = true,
    children,
  }: { title: string; open?: boolean; closeOnEscape?: boolean; children: Snippet } = $props();

  let dialog: HTMLDialogElement;

  $effect(() => {
    if (open && !dialog.open) dialog.showModal();
    else if (!open && dialog.open) dialog.close();
  });
</script>

<dialog
  bind:this={dialog}
  onclose={() => (open = false)}
  oncancel={(e) => {
    if (!closeOnEscape) e.preventDefault();
  }}
>
  <header>
    <h2>{title}</h2>
    <button class="ghost" onclick={() => (open = false)}>{t().tests.close}</button>
  </header>
  {#if open}{@render children()}{/if}
</dialog>

<style>
  dialog {
    width: min(980px, calc(100vw - 32px));
    max-height: calc(100vh - 48px);
    padding: 22px 24px;
    border: 1px solid var(--border);
    border-radius: 18px;
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
    align-items: center;
    margin-bottom: 14px;
  }

  h2 {
    margin: 0;
    font-size: 20px;
  }
</style>

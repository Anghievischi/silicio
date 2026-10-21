<script lang="ts">
  import { onMount } from "svelte";
  import { openUrl } from "@tauri-apps/plugin-opener";
  import { relaunch } from "@tauri-apps/plugin-process";
  import { check, type Update } from "@tauri-apps/plugin-updater";
  import { t } from "$lib/i18n/index.svelte";

  const RELEASES = "https://github.com/Anghievischi/silicio/releases/latest";

  let update = $state<Update | null>(null);
  let progress = $state<number | null>(null);
  let failed = $state(false);
  let dismissed = $state(false);

  onMount(() => {
    // Espera a interface carregar; sem internet ou sem release nova, fica em silêncio.
    const timer = setTimeout(() => {
      check()
        .then((found) => (update = found))
        .catch(() => {});
    }, 5000);
    return () => clearTimeout(timer);
  });

  async function install() {
    if (!update) return;
    let total = 0;
    let done = 0;
    progress = 0;
    try {
      await update.downloadAndInstall((event) => {
        if (event.event === "Started") total = event.data.contentLength ?? 0;
        if (event.event === "Progress") {
          done += event.data.chunkLength;
          progress = total ? Math.round((done / total) * 100) : 0;
        }
      });
      await relaunch();
    } catch {
      // Ex.: pacote do AUR (sem dpkg) ou instalação sem permissão de escrita.
      failed = true;
      progress = null;
    }
  }
</script>

{#if update && !dismissed}
  <div class="banner" role="status">
    <span>
      <strong>{t().update.available(update.version)}</strong>
      {#if failed}<span class="muted">· {t().update.failed}</span>{/if}
    </span>
    <div class="actions">
      {#if failed}
        <button class="primary" onclick={() => openUrl(RELEASES)}>{t().update.download}</button>
      {:else}
        <button class="primary" onclick={install} disabled={progress !== null}>
          {progress === null ? t().update.install : t().update.installing(progress)}
        </button>
      {/if}
      <button class="ghost" onclick={() => (dismissed = true)} disabled={progress !== null}>{t().update.later}</button>
    </div>
  </div>
{/if}

<style>
  .banner {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    margin-bottom: 20px;
    padding: 12px 16px;
    border: 1px solid color-mix(in srgb, var(--accent) 45%, var(--border));
    border-radius: var(--radius);
    background: color-mix(in srgb, var(--accent) 10%, var(--surface));
  }

  .actions {
    display: flex;
    gap: 8px;
    flex-shrink: 0;
  }

  .primary {
    padding: 7px 14px;
    border: 0;
    border-radius: var(--radius-sm);
    background: var(--accent);
    color: var(--on-accent);
    font-weight: 600;
  }

  .primary:disabled {
    opacity: 0.7;
    cursor: default;
  }
</style>

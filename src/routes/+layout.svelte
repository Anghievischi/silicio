<script lang="ts">
  import "../app.css";
  import { invoke } from "@tauri-apps/api/core";
  import { onMount } from "svelte";
  import { goto } from "$app/navigation";
  import StatusBar from "$lib/components/StatusBar.svelte";
  import Topbar, { tabs } from "$lib/components/Topbar.svelte";
  import ReportDialog from "$lib/components/ReportDialog.svelte";
  import ShareDialog from "$lib/components/ShareDialog.svelte";
  import UpdateBanner from "$lib/components/UpdateBanner.svelte";
  import { loadSystem, reportDialog } from "$lib/system.svelte";
  import { i18n } from "$lib/i18n/index.svelte";
  import type { CpuInfo } from "$lib/types";

  let { children } = $props();

  // Ctrl+1, Ctrl+2… alternam entre as abas; Ctrl+E exporta o relatório.
  function onKeydown(e: KeyboardEvent) {
    if (!e.ctrlKey || e.altKey || e.shiftKey) return;
    if (e.key.toLowerCase() === "e") {
      e.preventDefault();
      reportDialog.open = true;
      return;
    }
    const href = tabs[Number(e.key) - 1]?.href;
    if (href) {
      e.preventDefault();
      goto(href);
    }
  }

  // A cor de destaque de todas as abas acompanha o fabricante do processador.
  onMount(() => {
    document.documentElement.lang = i18n.locale;
    loadSystem();
    invoke<CpuInfo>("cpu_info")
      .then((cpu) => (document.documentElement.dataset.vendor = cpu.vendor))
      .catch(() => {});
  });
</script>

<svelte:window onkeydown={onKeydown} />

<div class="shell">
  <Topbar />
  <main>
    <UpdateBanner />
    {@render children()}
  </main>
  <StatusBar />
</div>

<ReportDialog />
<ShareDialog />

<style>
  .shell {
    display: grid;
    grid-template-rows: auto 1fr auto;
    height: 100vh;
  }

  main {
    min-height: 0;
    overflow-y: auto;
    padding: 12px 12px 24px;
  }
</style>

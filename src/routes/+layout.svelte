<script lang="ts">
  import "../app.css";
  import { invoke } from "@tauri-apps/api/core";
  import { onMount } from "svelte";
  import { goto } from "$app/navigation";
  import Sidebar, { tabs } from "$lib/components/Sidebar.svelte";
  import type { CpuInfo } from "$lib/types";

  let { children } = $props();

  // Ctrl+1, Ctrl+2… alternam entre as abas implementadas.
  function onKeydown(e: KeyboardEvent) {
    if (!e.ctrlKey || e.altKey || e.shiftKey) return;
    const href = tabs.filter((t) => t.href)[Number(e.key) - 1]?.href;
    if (href) {
      e.preventDefault();
      goto(href);
    }
  }

  // A cor de destaque de todas as abas acompanha o fabricante do processador.
  onMount(() => {
    invoke<CpuInfo>("cpu_info")
      .then((cpu) => (document.documentElement.dataset.vendor = cpu.vendor))
      .catch(() => {});
  });
</script>

<svelte:window onkeydown={onKeydown} />

<div class="shell">
  <Sidebar />
  <main>{@render children()}</main>
</div>

<style>
  .shell {
    display: grid;
    grid-template-columns: 236px 1fr;
    height: 100vh;
  }

  main {
    overflow-y: auto;
    padding: 28px 32px 40px;
  }

  @media (max-width: 760px) {
    .shell {
      grid-template-columns: 64px 1fr;
    }
    main {
      padding: 20px 16px;
    }
  }
</style>

<script lang="ts">
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { onMount } from "svelte";
  import { t } from "$lib/i18n/index.svelte";

  let { onclose }: { onclose: () => void } = $props();

  const COLORS = ["#000000", "#ffffff", "#ff0000", "#00ff00", "#0000ff", "#808080", "#ffff00", "#00ffff", "#ff00ff"];
  let index = $state(0);
  let hint = $state(true);

  const step = (n: number) => (index = (index + n + COLORS.length) % COLORS.length);

  onMount(() => {
    // Sem tela cheia (permissão negada, outro sistema), o teste segue na janela.
    const fullscreen = (on: boolean) => {
      try {
        getCurrentWindow().setFullscreen(on).catch(() => {});
      } catch {
        // Fora do Tauri.
      }
    };
    fullscreen(true);
    const timer = setTimeout(() => (hint = false), 2500);
    const onKey = (e: KeyboardEvent) => {
      e.preventDefault();
      e.stopImmediatePropagation();
      if (e.key === "Escape") onclose();
      else if (e.key === "ArrowLeft" || e.key === "ArrowUp") step(-1);
      else step(1);
    };
    window.addEventListener("keydown", onKey, true);
    return () => {
      clearTimeout(timer);
      window.removeEventListener("keydown", onKey, true);
      fullscreen(false);
    };
  });
</script>

<!-- svelte-ignore a11y_click_events_have_key_events -->
<div
  class="screen"
  role="button"
  tabindex="-1"
  style:background={COLORS[index]}
  onclick={() => step(1)}
  oncontextmenu={(e) => {
    e.preventDefault();
    step(-1);
  }}
>
  {#if hint}<p class="hint">{t().tests.pixelsHint} · {index + 1}/{COLORS.length}</p>{/if}
</div>

<style>
  .screen {
    position: fixed;
    inset: 0;
    z-index: 1000;
    cursor: none;
  }

  .hint {
    position: absolute;
    bottom: 32px;
    left: 50%;
    transform: translateX(-50%);
    margin: 0;
    padding: 10px 16px;
    border-radius: 10px;
    background: rgb(0 0 0 / 0.7);
    color: #fff;
    font-size: 13px;
  }
</style>

<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { onMount } from "svelte";
  import { t } from "$lib/i18n/index.svelte";
  import { isWindows } from "$lib/system.svelte";
  import type { CpuLive } from "$lib/types";

  // Os Ryzen 7000 ficam em 95 °C sob carga por projeto e a maioria dos Intel reduz
  // o clock perto dos 100 °C: abaixo disso o aviso seria alarme falso.
  const LIMIT_C = 95;
  const INTERVAL_MS = 5000;
  // Seis leituras seguidas (30 s) acima do limite; picos curtos são normais.
  const SAMPLES = 6;
  const SNOOZE_MS = 30 * 60 * 1000;

  let hot = $state(0);
  let peak = $state(0);
  let snoozedUntil = $state(0);

  let visible = $derived(hot >= SAMPLES && Date.now() >= snoozedUntil);

  onMount(() => {
    // Sem sensor do processador no Windows, não há o que vigiar.
    if (isWindows) return;
    const tick = async () => {
      try {
        const live = await invoke<CpuLive>("cpu_live");
        const temp = live.temperature_c;
        if (temp != null && temp >= LIMIT_C) {
          hot += 1;
          peak = Math.max(peak, temp);
        } else {
          hot = 0;
          peak = 0;
        }
      } catch {
        // Sem leitura, sem aviso.
      }
    };
    const timer = setInterval(tick, INTERVAL_MS);
    return () => clearInterval(timer);
  });

  function snooze() {
    snoozedUntil = Date.now() + SNOOZE_MS;
  }
</script>

{#if visible}
  <div class="banner" role="alert">
    <div>
      <strong>{t().heat.title(Math.round(peak))}</strong>
      <p class="muted">{t().heat.text}</p>
    </div>
    <button class="ghost" onclick={snooze}>{t().heat.dismiss}</button>
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
    border: 1px solid color-mix(in srgb, var(--hot) 55%, var(--border));
    border-radius: var(--radius);
    background: color-mix(in srgb, var(--hot) 12%, var(--surface));
  }

  .banner strong {
    color: var(--hot);
  }

  .banner p {
    margin: 4px 0 0;
    font-size: 13px;
  }

  button {
    flex-shrink: 0;
  }
</style>

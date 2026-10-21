<script lang="ts">
  import { onDestroy } from "svelte";
  import { t } from "$lib/i18n/index.svelte";

  let ctx: AudioContext | null = null;
  let playing = $state<string | null>(null);
  let stopCurrent: (() => void) | null = null;

  function audio() {
    ctx ??= new AudioContext();
    return ctx;
  }

  /** Toca um tom com entrada e saída suaves; `pan` vai de -1 (esquerda) a 1 (direita). */
  function play(id: string, pan: number, seconds: number, sweep = false) {
    stopCurrent?.();
    const ac = audio();
    const osc = ac.createOscillator();
    const gain = ac.createGain();
    const panner = ac.createStereoPanner();
    const start = ac.currentTime;
    osc.type = "sine";
    if (sweep) {
      osc.frequency.setValueAtTime(40, start);
      osc.frequency.exponentialRampToValueAtTime(16000, start + seconds);
    } else {
      osc.frequency.value = 440;
    }
    panner.pan.value = pan;
    gain.gain.setValueAtTime(0, start);
    gain.gain.linearRampToValueAtTime(0.25, start + 0.05);
    gain.gain.setValueAtTime(0.25, start + seconds - 0.1);
    gain.gain.linearRampToValueAtTime(0, start + seconds);
    osc.connect(gain).connect(panner).connect(ac.destination);
    osc.start(start);
    osc.stop(start + seconds);
    playing = id;
    stopCurrent = () => {
      try {
        osc.stop();
      } catch {
        // Já terminou.
      }
    };
    osc.onended = () => {
      if (playing === id) playing = null;
    };
  }

  onDestroy(() => {
    stopCurrent?.();
    ctx?.close();
  });
</script>

<p class="muted">{t().tests.speakersHint}</p>

{#snippet speaker(flip = false)}
  <svg viewBox="0 0 24 24" class:flip aria-hidden="true">
    <path d="M3 9h4l5-4v14l-5-4H3Z M15.5 9a4 4 0 0 1 0 6 M18.5 6a8 8 0 0 1 0 12" />
  </svg>
{/snippet}

<div class="speakers">
  <button class="speaker" class:on={playing === "left"} onclick={() => play("left", -1, 1.2)}>
    <span class="cone">{@render speaker(true)}</span>{t().tests.left}
  </button>
  <button class="speaker" class:on={playing === "both"} onclick={() => play("both", 0, 1.2)}>
    <span class="cone">{@render speaker(true)}{@render speaker()}</span>{t().tests.both}
  </button>
  <button class="speaker" class:on={playing === "right"} onclick={() => play("right", 1, 1.2)}>
    <span class="cone">{@render speaker()}</span>{t().tests.right}
  </button>
</div>
<button class="ghost sweep" class:on={playing === "sweep"} onclick={() => play("sweep", 0, 6, true)}>
  {t().tests.sweep} · 40 Hz → 16 kHz
</button>

<style>
  .speakers {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 12px;
  }

  .speaker {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 8px;
    padding: 22px 12px;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: var(--surface-2);
    font-weight: 600;
    transition:
      border-color 0.2s,
      box-shadow 0.2s;
  }

  .cone {
    display: flex;
    gap: 4px;
    color: var(--accent);
  }

  svg {
    width: 34px;
    height: 34px;
    fill: none;
    stroke: currentColor;
    stroke-width: 1.8;
    stroke-linecap: round;
    stroke-linejoin: round;
  }

  .flip {
    transform: scaleX(-1);
  }

  .on {
    border-color: var(--accent);
    box-shadow: 0 0 18px color-mix(in srgb, var(--accent) 45%, transparent);
  }

  .sweep {
    margin-top: 12px;
    width: 100%;
  }
</style>

<script lang="ts">
  import { onMount } from "svelte";
  import { t } from "$lib/i18n/index.svelte";

  // Taxas comuns de polling: o valor medido é arredondado para a mais próxima.
  const TIERS = [125, 250, 500, 1000, 2000, 4000, 8000];
  const WINDOW_MS = 500;
  const raw = "onpointerrawupdate" in window;

  let area: HTMLDivElement;
  let now = $state(0);
  let max = $state(0);
  let pressed = $state(new Set<number>());
  let tested = $state(new Set<number>());
  let wheelUp = $state(0);
  let wheelDown = $state(0);
  let stamps: number[] = [];

  const tier = (hz: number) => (hz < 100 ? null : TIERS.reduce((a, b) => (Math.abs(b - hz) < Math.abs(a - hz) ? b : a)));

  function record(times: number[]) {
    stamps.push(...times);
    const cutoff = stamps[stamps.length - 1] - WINDOW_MS;
    stamps = stamps.filter((s) => s > cutoff);
    if (stamps.length > 4) {
      const span = stamps[stamps.length - 1] - stamps[0];
      if (span > 0) {
        now = Math.round(((stamps.length - 1) / span) * 1000);
        max = Math.max(max, now);
      }
    }
  }

  onMount(() => {
    // pointerrawupdate (WebView2) entrega cada evento do mouse; sem ele, os eventos agrupados por quadro.
    const onMove = (e: PointerEvent) => {
      const list = !raw && e.getCoalescedEvents ? e.getCoalescedEvents() : [e];
      record((list.length ? list : [e]).map((x) => x.timeStamp));
    };
    const type = raw ? "pointerrawupdate" : "pointermove";
    area.addEventListener(type, onMove as EventListener);
    const idle = setInterval(() => {
      if (stamps.length && performance.now() - stamps[stamps.length - 1] > 300) {
        stamps = [];
        now = 0;
      }
    }, 200);
    return () => {
      area.removeEventListener(type, onMove as EventListener);
      clearInterval(idle);
    };
  });

  // mousedown/mouseup disparam para cada botão; pointerdown só para o primeiro de um acorde.
  function down(e: MouseEvent) {
    // Os botões laterais (3 e 4) navegariam para trás/frente no app.
    e.preventDefault();
    pressed = new Set(pressed).add(e.button);
    tested = new Set(tested).add(e.button);
  }

  function up(e: MouseEvent) {
    e.preventDefault();
    const next = new Set(pressed);
    next.delete(e.button);
    pressed = next;
  }

  function wheel(e: WheelEvent) {
    e.preventDefault();
    if (e.deltaY < 0) wheelUp++;
    else if (e.deltaY > 0) wheelDown++;
  }

  // Ordem de exibição: esquerdo, meio, direito, voltar, avançar (códigos 0, 1, 2, 3, 4).
  const BUTTONS = [0, 1, 2, 3, 4];
</script>

<p class="muted">{t().tests.mouseHint}</p>

<!-- Área de captura: recebe todos os botões e a roda de propósito. -->
<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<div
  class="area"
  bind:this={area}
  role="application"
  onmousedown={down}
  onmouseup={up}
  onauxclick={(e) => e.preventDefault()}
  oncontextmenu={(e) => e.preventDefault()}
  onwheel={wheel}
>
  <div class="rate">
    <div>
      <span class="muted">{t().tests.pollingNow}</span>
      <strong class="num">{now || "—"}<small> Hz</small></strong>
    </div>
    <div>
      <span class="muted">{t().tests.pollingMax}</span>
      <strong class="num">{max || "—"}<small> Hz</small></strong>
      {#if tier(max)}<span class="tier">≈ {tier(max)} Hz</span>{/if}
    </div>
  </div>
</div>

<div class="row">
  <div>
    <h3>{t().tests.buttons}</h3>
    <div class="buttons">
      {#each BUTTONS as b}
        <span class="btn" class:down={pressed.has(b)} class:ok={tested.has(b)}>{t().tests.mouseButtons[b]}</span>
      {/each}
    </div>
  </div>
  <div>
    <h3>{t().tests.wheel}</h3>
    <div class="buttons">
      <span class="btn" class:ok={wheelUp > 0}>↑ <span class="num">{wheelUp}</span></span>
      <span class="btn" class:ok={wheelDown > 0}>↓ <span class="num">{wheelDown}</span></span>
    </div>
  </div>
</div>

{#if !raw}<p class="muted small">{t().tests.browserLimit}</p>{/if}

<style>
  .area {
    display: grid;
    place-items: center;
    height: 260px;
    border: 2px dashed color-mix(in srgb, var(--accent) 50%, var(--border));
    border-radius: var(--radius-sm);
    background:
      radial-gradient(circle at 50% 50%, color-mix(in srgb, var(--accent) 12%, transparent), transparent 70%),
      var(--surface-2);
    cursor: crosshair;
    user-select: none;
    touch-action: none;
  }

  .rate {
    display: flex;
    gap: 48px;
    pointer-events: none;
  }

  .rate div {
    display: flex;
    flex-direction: column;
    align-items: center;
  }

  .rate strong {
    font-size: 40px;
    font-weight: 700;
  }

  .rate small {
    font-size: 16px;
    color: var(--muted);
  }

  .tier {
    font-size: 12px;
    font-weight: 600;
    color: var(--accent);
  }

  .row {
    display: flex;
    flex-wrap: wrap;
    gap: 32px;
    margin-top: 16px;
  }

  h3 {
    margin: 0 0 8px;
    font-size: 12px;
    color: var(--muted);
    text-transform: uppercase;
    letter-spacing: 0.06em;
  }

  .buttons {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }

  .btn {
    padding: 6px 12px;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: var(--surface-2);
    font-size: 13px;
    font-weight: 600;
  }

  .btn.ok {
    background: color-mix(in srgb, var(--good) 25%, var(--surface-2));
    border-color: color-mix(in srgb, var(--good) 50%, var(--border));
  }

  .btn.down {
    background: var(--accent);
    color: var(--on-accent);
  }

  .small {
    font-size: 12px;
  }
</style>

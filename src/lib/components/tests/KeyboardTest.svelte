<script lang="ts">
  import { onMount } from "svelte";
  import { t } from "$lib/i18n/index.svelte";

  // Layout físico pelo `KeyboardEvent.code`, que não depende do idioma do teclado.
  // Cada tecla é [código, largura em unidades].
  type Key = [string, number];
  const k = (codes: string[], w = 1): Key[] => codes.map((c) => [c, w]);
  const letters = (s: string) => [...s].map((c) => `Key${c}`);
  const ROWS: Key[][] = [
    [["Escape", 1], ["", 0.5], ...k(["F1", "F2", "F3", "F4", "F5", "F6", "F7", "F8", "F9", "F10", "F11", "F12"])],
    [["Backquote", 1], ...k([..."1234567890"].map((d) => `Digit${d}`)), ["Minus", 1], ["Equal", 1], ["Backspace", 2]],
    [["Tab", 1.5], ...k(letters("QWERTYUIOP")), ["BracketLeft", 1], ["BracketRight", 1], ["Backslash", 1.5]],
    [["CapsLock", 1.75], ...k(letters("ASDFGHJKL")), ["Semicolon", 1], ["Quote", 1], ["Enter", 2.25]],
    [["ShiftLeft", 1.25], ["IntlBackslash", 1], ...k(letters("ZXCVBNM")), ["Comma", 1], ["Period", 1], ["Slash", 1], ["IntlRo", 1], ["ShiftRight", 1.75]],
    [["ControlLeft", 1.25], ["MetaLeft", 1.25], ["AltLeft", 1.25], ["Space", 5.5], ["AltRight", 1.25], ["MetaRight", 1.25], ["ContextMenu", 1.25], ["ControlRight", 1.25]],
  ];
  const NAV: Key[][] = [
    k(["Insert", "Home", "PageUp"]),
    k(["Delete", "End", "PageDown"]),
    [["", 1], ["ArrowUp", 1], ["", 1]],
    k(["ArrowLeft", "ArrowDown", "ArrowRight"]),
  ];
  const LABELS: Record<string, string> = {
    Escape: "Esc", Backquote: "`", Minus: "-", Equal: "=", Backspace: "⌫", Tab: "Tab",
    BracketLeft: "[", BracketRight: "]", Backslash: "\\", CapsLock: "Caps", Semicolon: ";",
    Quote: "'", Enter: "Enter", ShiftLeft: "Shift", ShiftRight: "Shift", IntlBackslash: "\\",
    IntlRo: "Ro", Comma: ",", Period: ".", Slash: "/", ControlLeft: "Ctrl", ControlRight: "Ctrl",
    MetaLeft: "Win", MetaRight: "Win", AltLeft: "Alt", AltRight: "AltGr", ContextMenu: "☰",
    Space: "", Insert: "Ins", Home: "Home", PageUp: "PgUp", Delete: "Del", End: "End",
    PageDown: "PgDn", ArrowUp: "↑", ArrowDown: "↓", ArrowLeft: "←", ArrowRight: "→",
  };
  // Onde o navegador expõe o layout (WebView2), as teclas mostram o caractere real (Ç no ABNT2).
  let layout = $state<Map<string, string>>(new Map());
  const label = (code: string) => {
    const real = layout.get(code);
    if (real?.trim()) return real.toUpperCase();
    return LABELS[code] ?? code.replace(/^(Key|Digit)/, "");
  };
  const known = new Set([...ROWS, ...NAV].flat().map(([c]) => c));

  let down = $state(new Set<string>());
  let tested = $state(new Set<string>());
  let rollover = $state(0);

  onMount(() => {
    (navigator as Navigator & { keyboard?: { getLayoutMap(): Promise<Map<string, string>> } }).keyboard
      ?.getLayoutMap()
      .then((map) => (layout = new Map(map)))
      .catch(() => {});
    // Fase de captura: as teclas não chegam aos atalhos do app enquanto o teste está aberto.
    const onDown = (e: KeyboardEvent) => {
      e.preventDefault();
      e.stopImmediatePropagation();
      down = new Set(down).add(e.code);
      tested = new Set(tested).add(e.code);
      rollover = Math.max(rollover, down.size);
    };
    const onUp = (e: KeyboardEvent) => {
      e.preventDefault();
      e.stopImmediatePropagation();
      const next = new Set(down);
      next.delete(e.code);
      down = next;
    };
    const onBlur = () => (down = new Set());
    window.addEventListener("keydown", onDown, true);
    window.addEventListener("keyup", onUp, true);
    window.addEventListener("blur", onBlur);
    return () => {
      window.removeEventListener("keydown", onDown, true);
      window.removeEventListener("keyup", onUp, true);
      window.removeEventListener("blur", onBlur);
    };
  });

  let extra = $derived([...tested].filter((c) => !known.has(c)));
</script>

<p class="muted">{t().tests.keyboardHint}</p>

{#snippet board(rows: Key[][])}
  <div class="board">
    {#each rows as row}
      <div class="row">
        {#each row as [code, w]}
          {#if code}
            <span class="key" class:down={down.has(code)} class:ok={tested.has(code)} style:flex-grow={w}>{label(code)}</span>
          {:else}
            <span class="gap" style:flex-grow={w}></span>
          {/if}
        {/each}
      </div>
    {/each}
  </div>
{/snippet}

<div class="keyboard">
  {@render board(ROWS)}
  <div class="nav">{@render board(NAV)}</div>
</div>

{#if extra.length}
  <p class="extra">{#each extra as code}<span class="key ok">{code}</span>{/each}</p>
{/if}

<div class="stats">
  <strong>{t().tests.pressed(tested.size)}</strong>
  <strong>{t().tests.rollover(rollover)}</strong>
  <button class="ghost" onclick={() => ((tested = new Set()), (rollover = 0))}>{t().tests.reset}</button>
</div>
<p class="muted small">{t().tests.rolloverHint}</p>

<style>
  .keyboard {
    display: flex;
    gap: 14px;
    align-items: flex-end;
  }

  .board {
    display: flex;
    flex-direction: column;
    gap: 5px;
    flex: 1;
  }

  .nav {
    width: 150px;
    flex-shrink: 0;
  }

  .row {
    display: flex;
    gap: 5px;
  }

  .key,
  .gap {
    flex-basis: 0;
    min-width: 0;
  }

  .key {
    display: grid;
    place-items: center;
    height: 38px;
    padding: 0 2px;
    border: 1px solid var(--border);
    border-radius: 7px;
    background: var(--surface-2);
    font-size: 12px;
    font-weight: 600;
    overflow: hidden;
    white-space: nowrap;
    transition:
      background 0.15s,
      transform 0.1s;
  }

  .key.ok {
    background: color-mix(in srgb, var(--good) 25%, var(--surface-2));
    border-color: color-mix(in srgb, var(--good) 50%, var(--border));
  }

  .key.down {
    background: var(--accent);
    color: #fff;
    transform: translateY(1px);
  }

  .extra {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }

  .extra .key {
    padding: 0 8px;
  }

  .stats {
    display: flex;
    align-items: center;
    gap: 20px;
    margin-top: 16px;
  }

  .stats button {
    margin-left: auto;
  }

  .small {
    font-size: 12px;
  }

  @media (max-width: 760px) {
    .nav {
      display: none;
    }
  }
</style>

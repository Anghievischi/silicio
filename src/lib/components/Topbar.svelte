<script lang="ts" module>
  import type { Messages } from "$lib/i18n/pt-BR";
  import { system } from "$lib/system.svelte";

  // As abas seguem as do CPU-Z, mais Upgrades, Diagnóstico e Jogos; `key` aponta para o nome traduzido.
  export const tabs: { href: string; key: keyof Messages["tabs"] }[] = [
    { href: "/", key: "overview" },
    { href: "/processador", key: "cpu" },
    { href: "/placa-mae", key: "board" },
    { href: "/memoria", key: "memory" },
    { href: "/spd", key: "spd" },
    { href: "/graficos", key: "gpu" },
    { href: "/armazenamento", key: "storage" },
    { href: "/bateria", key: "battery" },
    { href: "/perifericos", key: "peripherals" },
    { href: "/benchmark", key: "bench" },
    { href: "/upgrades", key: "upgrades" },
    { href: "/diagnostico", key: "diagnosis" },
    { href: "/jogos", key: "gaming" },
  ];

  /** Abas mostradas: a de bateria só em notebooks. */
  export const visibleTabs = () => tabs.filter((tab) => tab.key !== "battery" || system.hasBattery);

  /** Atalho da aba na posição `i`: Ctrl+1 … Ctrl+9 e Ctrl+0 para a décima. */
  export const shortcut = (i: number) => (i < 9 ? String(i + 1) : i === 9 ? "0" : null);
</script>

<script lang="ts">
  import { page } from "$app/state";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { onMount } from "svelte";
  import { i18n, LOCALES, setLocale, t, type Locale } from "$lib/i18n/index.svelte";
  import { reportDialog } from "$lib/system.svelte";

  // No Windows a janela vem sem a barra do sistema (tauri.windows.conf.json) e
  // os botões de minimizar, maximizar e fechar são desenhados aqui.
  const win = getCurrentWindow();
  let custom = $state(false);
  let maximized = $state(false);

  onMount(() => {
    win.isDecorated().then((d) => (custom = !d)).catch(() => {});
    win.isMaximized().then((m) => (maximized = m)).catch(() => {});
    const off = win.onResized(() => win.isMaximized().then((m) => (maximized = m)).catch(() => {}));
    return () => off.then((f) => f()).catch(() => {});
  });
</script>

<div class="top">
  <div class="titlebar" data-tauri-drag-region>
    <div class="logo" data-tauri-drag-region>
      <svg class="mark" viewBox="160 160 704 704" aria-hidden="true">
        <rect x="160" y="160" width="704" height="704" fill="#1C1915" />
        <path d="M330 260H764V764H260V330Z" fill="#EDE4D3" />
        <rect x="652" y="260" width="28" height="504" fill="#E5533D" />
        <rect x="680" y="260" width="28" height="504" fill="#F28C28" />
        <rect x="708" y="260" width="28" height="504" fill="#F2C14E" />
        <rect x="736" y="260" width="28" height="504" fill="#4FB3A9" />
        <g fill="#15130F">
          <path d="M351 372H493V428H379V484H493V652H323V596H437V540H323V400Z" />
          <rect x="533" y="372" width="56" height="56" />
          <rect x="533" y="464" width="56" height="188" />
        </g>
      </svg>
      <span class="name" data-tauri-drag-region>Silício</span>
    </div>

    <div class="spacer" data-tauri-drag-region></div>

    <button class="tool" onclick={() => (reportDialog.open = true)} title="{t().report.button} (Ctrl+E)">
      ↓ <span class="label">{t().report.button}</span>
    </button>

    <label class="lang">
      <span class="visually-hidden">{t().common.language}</span>
      <select value={i18n.locale} onchange={(e) => setLocale(e.currentTarget.value as Locale)}>
        {#each LOCALES as locale}<option value={locale.id}>{locale.label}</option>{/each}
      </select>
    </label>

    {#if custom}
      <div class="controls">
        <button class="wc" aria-label={t().window.minimize} title={t().window.minimize} onclick={() => win.minimize()}>
          <svg viewBox="0 0 14 14" aria-hidden="true"><path d="M2 11h10" /></svg>
        </button>
        <button class="wc" aria-label={t().window.maximize} title={t().window.maximize} onclick={() => win.toggleMaximize()}>
          <svg viewBox="0 0 14 14" aria-hidden="true">
            {#if maximized}
              <path d="M4 2h8v8M2 4h8v8H2z" />
            {:else}
              <rect x="2" y="2" width="10" height="10" />
            {/if}
          </svg>
        </button>
        <button class="wc close" aria-label={t().window.close} title={t().window.close} onclick={() => win.close()}>
          <svg viewBox="0 0 14 14" aria-hidden="true"><path d="M2 2l10 10M12 2L2 12" /></svg>
        </button>
      </div>
    {/if}
  </div>

  <nav aria-label={t().common.sections}>
    {#each visibleTabs() as tab, i}
      {@const label = t().tabs[tab.key]}
      {@const key = shortcut(i)}
      <a class="tab" class:active={page.url.pathname === tab.href} href={tab.href} title={key ? `${label} (Ctrl+${key})` : label}>
        {#if key}<span class="key num">{key}</span>{/if}
        <span class="label">{label}</span>
      </a>
    {/each}
  </nav>
</div>

<style>
  .top {
    border-bottom: 2px solid var(--border);
    background: var(--surface);
  }

  .titlebar {
    display: flex;
    align-items: stretch;
    height: 40px;
    border-bottom: 2px solid var(--border);
    user-select: none;
  }

  .logo {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 0 14px;
    border-right: 2px solid var(--border);
  }

  .mark {
    width: 24px;
    height: 24px;
    pointer-events: none;
  }

  .name {
    font-family: var(--font-display);
    font-size: 20px;
    font-weight: 900;
    letter-spacing: 0.08em;
    text-transform: uppercase;
  }

  .spacer {
    flex: 1;
  }

  .tool {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 0 14px;
    border: 0;
    border-left: 2px solid var(--border);
    background: none;
    color: var(--muted);
    font-size: 11px;
    font-weight: 700;
    letter-spacing: 0.08em;
    text-transform: uppercase;
  }

  .tool:hover {
    background: var(--surface-2);
    color: var(--text);
  }

  .lang {
    display: flex;
    align-items: center;
    padding: 0 10px;
    border-left: 2px solid var(--border);
  }

  .lang select {
    padding: 3px 6px;
    border: 1px solid var(--border);
    border-radius: 0;
    background: var(--bg);
    color: var(--text);
    font: inherit;
    font-size: 11px;
  }

  .controls {
    display: flex;
    border-left: 2px solid var(--border);
  }

  .wc {
    display: grid;
    place-items: center;
    width: 46px;
    border: 0;
    background: none;
    color: var(--muted);
  }

  .wc svg {
    width: 12px;
    height: 12px;
    fill: none;
    stroke: currentColor;
    stroke-width: 1.6;
  }

  .wc:hover {
    background: var(--surface-2);
    color: var(--text);
  }

  .wc.close:hover {
    background: var(--hot);
    color: #fff;
  }

  nav {
    display: flex;
    flex-wrap: wrap;
    gap: 2px;
    padding: 4px 6px;
  }

  .tab {
    display: flex;
    align-items: center;
    gap: 8px;
    min-height: 34px;
    padding: 0 10px;
    color: var(--muted);
    text-decoration: none;
    font-size: 11px;
    font-weight: 700;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    white-space: nowrap;
  }

  .tab:hover {
    background: var(--surface-2);
    color: var(--text);
  }

  .tab.active {
    background: var(--accent);
    color: var(--on-accent);
  }

  .key {
    padding: 0 5px;
    border: 1px solid currentColor;
    font-size: 10px;
  }

  .visually-hidden {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip: rect(0 0 0 0);
  }

  /* Com 13 abas, os números não cabem junto dos nomes em janelas médias;
     o atalho continua no título da aba. */
  @media (min-width: 861px) and (max-width: 1360px) {
    .tab .key {
      display: none;
    }

    .tab {
      padding: 0 8px;
    }
  }

  @media (max-width: 860px) {
    .tab .label,
    .tool .label {
      display: none;
    }
  }
</style>

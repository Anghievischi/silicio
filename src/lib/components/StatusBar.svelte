<script lang="ts">
  import { page } from "$app/state";
  import { openUrl } from "@tauri-apps/plugin-opener";
  import { t } from "$lib/i18n/index.svelte";
  import { clock } from "$lib/format";
  import { session } from "$lib/gaming.svelte";
  import { system } from "$lib/system.svelte";
  import { tabs } from "./Topbar.svelte";

  const RELEASES = "https://github.com/Anghievischi/silicio/releases";

  let current = $derived(tabs.find((tab) => tab.href === page.url.pathname));

  // A sessão de jogo grava no backend; aqui só conta o tempo para avisar em qualquer aba.
  let now = $state(Date.now());
  $effect(() => {
    if (!session.running) return;
    const timer = setInterval(() => (now = Date.now()), 1000);
    return () => clearInterval(timer);
  });
</script>

<footer>
  <span class="mode">{current ? t().tabs[current.key] : "Silício"}</span>
  <span class="live"><span class="dot"></span>{t().window.live}</span>
  {#if session.running}
    <a class="cell rec" href="/jogos">● {t().gaming.recording} {clock(Math.max(0, (now - session.startedAt) / 1000))}</a>
  {/if}
  {#if system.info?.os}<span class="cell muted">{system.info.os}</span>{/if}
  <span class="fill"></span>
  <span class="cell muted keys">{t().window.keys}</span>
  {#if system.info}
    {@const v = system.info.app_version}
    <button class="version num" title={t().app.releaseNotes} onclick={() => openUrl(`${RELEASES}/tag/v${v}`).catch(() => {})}>
      v{v}
    </button>
  {/if}
</footer>

<style>
  footer {
    display: flex;
    align-items: stretch;
    min-height: 26px;
    border-top: 2px solid var(--border);
    background: var(--surface);
    font-size: 11px;
    white-space: nowrap;
    overflow: hidden;
  }

  .mode {
    display: flex;
    align-items: center;
    padding: 0 12px;
    background: var(--accent);
    color: var(--on-accent);
    font-weight: 700;
    letter-spacing: 0.1em;
    text-transform: uppercase;
  }

  .live,
  .cell {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 0 12px;
    border-right: 2px solid var(--border);
  }

  .live {
    color: var(--good);
    text-transform: uppercase;
    letter-spacing: 0.08em;
  }

  .dot {
    width: 7px;
    height: 7px;
    background: currentColor;
    animation: blink 2s steps(2, jump-none) infinite;
  }

  @keyframes blink {
    50% {
      opacity: 0.25;
    }
  }

  .rec {
    background: var(--hot);
    color: #fff;
    font-weight: 700;
    text-decoration: none;
  }

  .fill {
    flex: 1;
  }

  .keys {
    border-right: 0;
    border-left: 2px solid var(--border);
  }

  .version {
    padding: 0 12px;
    border: 0;
    background: var(--surface-2);
    color: var(--text);
    font-size: 11px;
  }

  .version:hover {
    background: var(--accent);
    color: var(--on-accent);
  }

  @media (max-width: 760px) {
    .keys {
      display: none;
    }
  }
</style>

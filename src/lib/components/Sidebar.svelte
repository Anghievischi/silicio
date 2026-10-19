<script lang="ts" module>
  import type { Messages } from "$lib/i18n/pt-BR";

  // As abas seguem as do CPU-Z; `key` aponta para o nome traduzido.
  export const tabs: { href: string | null; key: keyof Messages["tabs"]; icon: string }[] = [
    { href: "/", key: "cpu", icon: "◧" },
    { href: "/placa-mae", key: "board", icon: "▦" },
    { href: "/memoria", key: "memory", icon: "▤" },
    { href: "/spd", key: "spd", icon: "▥" },
    { href: "/graficos", key: "gpu", icon: "◩" },
    { href: "/benchmark", key: "bench", icon: "◔" },
  ];
</script>

<script lang="ts">
  import { page } from "$app/state";
  import { i18n, LOCALES, setLocale, t, type Locale } from "$lib/i18n/index.svelte";
</script>

<nav>
  <div class="logo">
    <span class="mark">Si</span>
    <span class="name">Silício</span>
  </div>

  {#each tabs as tab, i}
    {@const label = t().tabs[tab.key]}
    {#if tab.href}
      <a class="tab" class:active={page.url.pathname === tab.href} href={tab.href} title="{label} (Ctrl+{i + 1})">
        <span class="icon">{tab.icon}</span>
        <span class="label">{label}</span>
      </a>
    {:else}
      <span class="tab disabled" title="{label} ({t().common.soon})">
        <span class="icon">{tab.icon}</span>
        <span class="label">{label}</span>
        <span class="soon">{t().common.soon}</span>
      </span>
    {/if}
  {/each}

  <label class="lang" title={t().common.language}>
    <span class="icon">🌐</span>
    <select
      aria-label={t().common.language}
      value={i18n.locale}
      onchange={(e) => setLocale(e.currentTarget.value as Locale)}
    >
      {#each LOCALES as locale}<option value={locale.id}>{locale.label}</option>{/each}
    </select>
  </label>
</nav>

<style>
  nav {
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 20px 12px;
    background: var(--surface);
    border-right: 1px solid var(--border);
  }

  .logo {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 0 8px 20px;
  }

  .mark {
    display: grid;
    place-items: center;
    width: 32px;
    height: 32px;
    border-radius: 9px;
    font-weight: 700;
    background: linear-gradient(135deg, var(--accent), color-mix(in srgb, var(--accent) 40%, #000));
    color: #fff;
  }

  .name {
    font-size: 17px;
    font-weight: 650;
  }

  .tab {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 9px 10px;
    border: 0;
    border-radius: 9px;
    color: inherit;
    text-decoration: none;
    white-space: nowrap;
  }

  .label {
    overflow: hidden;
    text-overflow: ellipsis;
  }

  a.tab:hover {
    background: var(--surface-2);
  }

  .tab.active {
    background: color-mix(in srgb, var(--accent) 16%, transparent);
    color: var(--accent);
    font-weight: 600;
  }

  .tab.disabled {
    color: var(--muted);
  }

  .lang {
    display: flex;
    align-items: center;
    gap: 10px;
    margin-top: auto;
    padding: 9px 10px;
    color: var(--muted);
  }

  .lang select {
    flex: 1;
    min-width: 0;
    padding: 5px 6px;
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--surface-2);
    color: var(--text);
    font: inherit;
    font-size: 13px;
  }

  .icon {
    width: 18px;
    text-align: center;
  }

  .soon {
    margin-left: auto;
    font-size: 10px;
    opacity: 0.7;
  }

  @media (max-width: 760px) {
    .name,
    .label,
    .soon {
      display: none;
    }
    .logo {
      padding-inline: 4px;
    }
  }
</style>

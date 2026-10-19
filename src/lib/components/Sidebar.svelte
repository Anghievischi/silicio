<script lang="ts" module>
  // As abas seguem as do CPU-Z.
  export const tabs = [
    { href: "/", label: "Processador", icon: "◧" },
    { href: "/placa-mae", label: "Placa-mãe", icon: "▦" },
    { href: "/memoria", label: "Memória", icon: "▤" },
    { href: "/spd", label: "Módulos (SPD)", icon: "▥" },
    { href: "/graficos", label: "Gráficos", icon: "◩" },
    { href: "/benchmark", label: "Benchmark", icon: "◔" },
  ];
</script>

<script lang="ts">
  import { page } from "$app/state";
</script>

<nav>
  <div class="logo">
    <span class="mark">Si</span>
    <span class="name">Silício</span>
  </div>

  {#each tabs as tab, i}
    {#if tab.href}
      <a class="tab" class:active={page.url.pathname === tab.href} href={tab.href} title="{tab.label} (Ctrl+{i + 1})">
        <span class="icon">{tab.icon}</span>
        <span class="label">{tab.label}</span>
      </a>
    {:else}
      <span class="tab disabled" title="{tab.label} (em breve)">
        <span class="icon">{tab.icon}</span>
        <span class="label">{tab.label}</span>
        <span class="soon">em breve</span>
      </span>
    {/if}
  {/each}
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

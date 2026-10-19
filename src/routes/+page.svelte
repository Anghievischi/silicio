<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { onMount } from "svelte";
  import Card from "$lib/components/Card.svelte";
  import Sparkline from "$lib/components/Sparkline.svelte";
  import { ghz, hex, size } from "$lib/format";
  import { privileged, unlock } from "$lib/privileged.svelte";
  import type { CpuInfo, CpuLive } from "$lib/types";

  const HISTORY = 60;
  // Clock de referência nominal das plataformas atuais, usado até o MSR ser lido.
  const NOMINAL_BUS_MHZ = 100;
  // O helper com privilégio usa pkexec; no Windows a leitura de MSR vai depender do driver PawnIO.
  const isWindows = navigator.userAgent.includes("Windows");

  let bus = $derived(privileged.data?.bus_mhz ?? null);
  let busMhz = $derived(bus ?? NOMINAL_BUS_MHZ);
  let vcore = $derived.by(() => {
    const volts = privileged.data?.threads.map((t) => t.voltage).filter((v): v is number => v != null) ?? [];
    return volts.length ? Math.max(...volts) : null;
  });

  let info = $state<CpuInfo | null>(null);
  let live = $state<CpuLive | null>(null);
  let history = $state<number[]>([]);
  let error = $state<string | null>(null);
  let copied = $state(false);

  let l3 = $derived(info?.caches.filter((c) => c.level === 3).reduce((t, c) => t + c.size_kib * c.instances, 0) ?? 0);
  let scaleMhz = $derived(Math.max(info?.clocks.max_mhz ?? 0, live?.peak_mhz ?? 0, 1000));

  onMount(() => {
    let timer: ReturnType<typeof setInterval>;

    invoke<CpuInfo>("cpu_info")
      .then((data) => {
        info = data;
      })
      .catch((e) => (error = String(e)));

    const tick = async () => {
      try {
        const sample = await invoke<CpuLive>("cpu_live");
        live = sample;
        history = [...history.slice(-(HISTORY - 1)), sample.avg_mhz];
      } catch (e) {
        error = String(e);
      }
    };
    tick();
    timer = setInterval(tick, 1000);
    return () => clearInterval(timer);
  });

  async function copyJson() {
    if (!info) return;
    await navigator.clipboard.writeText(JSON.stringify(info, null, 2));
    copied = true;
    setTimeout(() => (copied = false), 1500);
  }

  function loadColor(usage: number) {
    if (usage > 85) return "var(--hot)";
    if (usage > 50) return "var(--warn)";
    return "var(--good)";
  }

  function tempColor(t: number) {
    if (t >= 85) return "var(--hot)";
    if (t >= 70) return "var(--warn)";
    return "var(--good)";
  }
</script>

{#if error}
  <p class="error">Não foi possível ler o processador: {error}</p>
{:else if !info}
  <p class="muted">Lendo o processador…</p>
{:else}
  <header>
    <div>
      <span class="vendor">{info.vendor}</span>
      <h1>{info.brand}</h1>
      <p class="muted">
        {#if info.codename}
          {info.codename.name} · {info.codename.microarch} · {info.codename.process}
        {:else}
          Família {hex(info.signature.family)} · Modelo {hex(info.signature.model)}
        {/if}
      </p>
    </div>
    <button class="ghost" onclick={copyJson}>{copied ? "Copiado ✓" : "Copiar JSON"}</button>
  </header>

  <div class="stats">
    <div class="stat">
      <span class="muted">Núcleos / threads</span>
      <strong class="num">{info.cores} / {info.threads}</strong>
    </div>
    <div class="stat">
      <span class="muted">Clock médio</span>
      <strong class="num">{ghz(live?.avg_mhz)}</strong>
    </div>
    <div class="stat">
      <span class="muted">Clock máximo</span>
      <strong class="num">{ghz(info.clocks.max_mhz)}</strong>
    </div>
    <div class="stat">
      <span class="muted">Cache L3</span>
      <strong class="num">{l3 ? size(l3) : "—"}</strong>
    </div>
    <div class="stat">
      <span class="muted">Temperatura</span>
      {#if live?.temperature_c != null}
        <strong class="num" style:color={tempColor(live.temperature_c)}>{live.temperature_c.toFixed(0)} °C</strong>
      {:else}
        <strong class="muted">—</strong>
      {/if}
    </div>
  </div>

  <div class="grid">
    <Card title="Clock ao vivo" span={2}>
      <div class="clock-head">
        <div>
          <span class="big num">{ghz(live?.avg_mhz)}</span>
          <span class="muted">média · pico {ghz(live?.peak_mhz)}</span>
        </div>
        <div class="muted right">
          <div>Uso total <strong class="num">{live ? live.usage.toFixed(0) : "—"}%</strong></div>
          <div title={bus ? "Bus medido via TSC e MSR" : "Estimado a partir do clock de referência nominal de 100 MHz"}>
            Multiplicador {bus ? "" : "≈ "}<strong class="num">{live ? `x${(live.avg_mhz / busMhz).toFixed(1)}` : "—"}</strong>
            · bus {busMhz.toFixed(bus ? 2 : 0)} MHz
          </div>
          {#if info.clocks.driver}<div>{info.clocks.driver} · {info.clocks.governor}</div>{/if}
        </div>
      </div>
      <Sparkline values={history} max={scaleMhz} capacity={HISTORY} />
      <div class="axis muted">
        <span>{ghz(info.clocks.min_mhz)}</span>
        <span>últimos {HISTORY} s</span>
        <span>{ghz(scaleMhz)}</span>
      </div>
    </Card>

    <Card title="Especificação">
      <dl>
        <dt>Fabricante</dt><dd>{info.vendor_id}</dd>
        <dt>Família</dt><dd class="num">{hex(info.signature.family)} <span class="muted">(base {hex(info.signature.base_family)} + ext {hex(info.signature.ext_family)})</span></dd>
        <dt>Modelo</dt><dd class="num">{hex(info.signature.model)} <span class="muted">(base {hex(info.signature.base_model)} · ext {hex(info.signature.ext_model)})</span></dd>
        <dt>Stepping</dt><dd class="num">{info.signature.stepping}</dd>
        <dt>Microcódigo</dt><dd class="num">{info.microcode ?? "—"}</dd>
        {#if info.codename}
          <dt>Processo</dt><dd>{info.codename.process}</dd>
        {/if}
      </dl>
    </Card>

    <Card title="Núcleos" span={2}>
      <div class="cores">
        {#each info.topology as core}
          <div class="core">
            <span class="core-id muted">C{core.id}</span>
            {#each core.threads as t}
              {@const th = live?.threads[t]}
              <div class="thread" title="Thread {t}">
                <span class="num">{th ? (th.mhz / 1000).toFixed(2) : "—"}</span>
                <div class="bar"><div style:width="{th?.usage ?? 0}%" style:background={loadColor(th?.usage ?? 0)}></div></div>
              </div>
            {/each}
          </div>
        {/each}
      </div>
      <p class="hint muted">GHz por thread; a barra mostra o uso.</p>
    </Card>

    <Card title="Cache">
      <table>
        <tbody>
          {#each info.caches as c}
            <tr>
              <th>L{c.level} <span class="muted">{c.kind}</span></th>
              <td class="num">{c.instances > 1 ? `${c.instances} × ` : ""}{size(c.size_kib)}</td>
              <td class="num muted">{c.ways}-way · {c.line_bytes} B</td>
            </tr>
          {/each}
        </tbody>
      </table>
    </Card>

    <Card title="Leituras avançadas" span={3}>
      {#if privileged.data}
        {@const p = privileged.data}
        <div class="adv">
          <div><span class="muted">Soquete</span><strong>{p.processor?.socket ?? "—"}</strong></div>
          <div>
            <span class="muted">Tensão do núcleo (VID)</span>
            <strong class="num">{vcore ? `${vcore.toFixed(3)} V` : "—"}</strong>
          </div>
          <div><span class="muted">Bus real</span><strong class="num">{bus ? `${bus.toFixed(2)} MHz` : "—"}</strong></div>
          <div>
            <span class="muted">Multiplicador base</span>
            <strong class="num">{p.base_ratio ? `x${p.base_ratio.toFixed(1)}` : "—"}</strong>
          </div>
          <div>
            <span class="muted">Clock externo (SMBIOS)</span>
            <strong class="num">{p.processor?.external_clock_mhz ? `${p.processor.external_clock_mhz} MHz` : "—"}</strong>
          </div>
        </div>
        {#if p.errors.length}
          <p class="hint muted">Algumas leituras falharam: {p.errors.join("; ")}</p>
        {/if}
      {:else if isWindows}
        <p class="muted">No Windows, soquete, tensão e bus real ainda não estão disponíveis (vão usar o driver PawnIO).</p>
      {:else}
        <div class="unlock">
          <p class="muted">
            Soquete, tensão do núcleo e o bus real exigem acesso de administrador (registradores MSR e tabela SMBIOS).
            O Silício roda um pequeno helper só de leitura, e o sistema pede sua senha.
          </p>
          <button class="ghost" onclick={unlock} disabled={privileged.loading}>
            {privileged.loading ? "Aguardando autorização…" : "Desbloquear"}
          </button>
        </div>
        {#if privileged.error}<p class="error hint">{privileged.error}</p>{/if}
      {/if}
    </Card>

    <Card title="Instruções" span={3}>
      <div class="groups">
        {#each info.instructions as group}
          <div>
            <h3>{group.name}</h3>
            <div class="chips">
              {#each group.items as item}<span class="chip">{item}</span>{/each}
            </div>
          </div>
        {/each}
      </div>
    </Card>
  </div>
{/if}

<style>
  .clock-head {
    display: flex;
    justify-content: space-between;
    gap: 12px;
    margin-bottom: 10px;
  }
  .big {
    display: block;
    font-size: 32px;
    font-weight: 700;
    line-height: 1.1;
  }
  .right {
    text-align: right;
    font-size: 12px;
  }
  .axis {
    display: flex;
    justify-content: space-between;
    margin-top: 6px;
    font-size: 11px;
  }
  .cores {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(118px, 1fr));
    gap: 8px;
  }
  .core {
    display: flex;
    flex-direction: column;
    gap: 6px;
    padding: 10px;
    border-radius: 10px;
    background: var(--surface-2);
  }
  .core-id {
    font-size: 11px;
    font-weight: 600;
  }
  .thread {
    display: flex;
    flex-direction: column;
    gap: 3px;
    font-size: 13px;
  }
  .bar {
    height: 4px;
    border-radius: 2px;
    background: var(--border);
    overflow: hidden;
  }
  .bar div {
    height: 100%;
    transition: width 0.4s ease;
  }
  .hint {
    margin: 10px 0 0;
    font-size: 11px;
  }
  .groups {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(220px, 1fr));
    gap: 16px;
  }
  h3 {
    margin: 0 0 8px;
    font-size: 13px;
    font-weight: 600;
  }
  .chips {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }
  .chip {
    padding: 3px 9px;
    border-radius: 7px;
    font-size: 12px;
    background: var(--surface-2);
    border: 1px solid var(--border);
  }

  .adv {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(160px, 1fr));
    gap: 16px;
  }

  .adv div {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .adv span {
    font-size: 12px;
  }

  .adv strong {
    font-size: 18px;
    font-weight: 650;
  }

  .unlock {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
  }

  .unlock p {
    margin: 0;
    max-width: 70ch;
  }
</style>

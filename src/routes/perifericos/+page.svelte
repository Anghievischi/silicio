<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { availableMonitors, type Monitor } from "@tauri-apps/api/window";
  import { onMount } from "svelte";
  import Card from "$lib/components/Card.svelte";
  import DeviceIcon from "$lib/components/DeviceIcon.svelte";
  import CameraTest from "$lib/components/tests/CameraTest.svelte";
  import DeadPixelTest from "$lib/components/tests/DeadPixelTest.svelte";
  import KeyboardTest from "$lib/components/tests/KeyboardTest.svelte";
  import MouseTest from "$lib/components/tests/MouseTest.svelte";
  import SpeakerTest from "$lib/components/tests/SpeakerTest.svelte";
  import TestDialog from "$lib/components/tests/TestDialog.svelte";
  import { cameraSummary, displayName, usbSpeed } from "$lib/format";
  import { t } from "$lib/i18n/index.svelte";
  import type { AudioDevice, Peripherals } from "$lib/types";

  let data = $state<Peripherals | null>(null);
  let monitors = $state<Monitor[]>([]);
  let error = $state<string | null>(null);
  let test = $state<"keyboard" | "mouse" | "speakers" | "camera" | null>(null);
  let pixels = $state(false);

  // Uma janela por teste; fechar uma zera `test`.
  let keyboardOpen = $derived(test === "keyboard");
  let mouseOpen = $derived(test === "mouse");
  let speakersOpen = $derived(test === "speakers");
  let cameraOpen = $derived(test === "camera");

  onMount(() => {
    invoke<Peripherals>("peripherals_info")
      .then((d) => (data = d))
      .catch((e) => (error = String(e)));
    availableMonitors()
      .then((m) => (monitors = m))
      .catch(() => {});
  });

  const p = () => t().peripherals;

  function hz(n: number) {
    return Number.isInteger(n) ? `${n} Hz` : `${n.toFixed(2)} Hz`;
  }

  function audioFormat(a: AudioDevice) {
    if (!a.sample_rate) return p().idle;
    return p().format(`${a.sample_rate / 1000} kHz`, a.bits, a.channels);
  }

  const tests = [
    { id: "keyboard", icon: "keyboard" },
    { id: "mouse", icon: "mouse" },
    { id: "speakers", icon: "audio" },
    { id: "pixels", icon: "display" },
    { id: "camera", icon: "camera" },
  ] as const;

  function testInfo(id: (typeof tests)[number]["id"]) {
    const m = t().tests;
    return {
      keyboard: [m.keyboard, m.keyboardHint],
      mouse: [m.mouse, m.mouseHint],
      speakers: [m.speakers, m.speakersHint],
      pixels: [m.pixels, m.pixelsHint],
      camera: [m.camera, m.cameraHint],
    }[id];
  }
</script>

{#if error}
  <p class="error">{p().error}: {error}</p>
{:else if !data}
  <p class="muted">{p().loading}</p>
{:else}
  <header>
    <div>
      <h1>{t().tabs.peripherals}</h1>
      <p class="muted">
        {p().displays}: {data.displays.length} · {p().devices}: {data.devices.length} · {p().audio}: {data.audio.length} ·
        {p().cameras}: {data.cameras.length}
      </p>
    </div>
  </header>

  <div class="grid">
    <Card title={p().displays} span={3}>
      {#if !data.displays.length}
        <p class="muted">{p().noDisplays}</p>
      {/if}
      <div class="displays">
        {#each data.displays as d}
          {@const ratio = d.native ? d.native.width / d.native.height : 16 / 9}
          <div class="display">
            <div class="screen-art">
              <div class="panel" style:aspect-ratio={ratio}>
                <span class="num">{d.diagonal_in ? p().inches(d.diagonal_in.toFixed(1)) : "—"}</span>
                {#if d.native}<small class="num">{d.native.width} × {d.native.height}</small>{/if}
              </div>
              {#if d.internal === false || d.internal === null}<div class="stand"></div>{/if}
            </div>
            <div class="display-info">
              <div class="title">
                <strong>{displayName(d)}</strong>
                <span class="tag">{d.internal ? p().internal : p().external}{d.connector ? ` · ${d.connector}` : ""}</span>
                {#if d.hdr}<span class="tag accent">HDR</span>{/if}
                {#if d.bt2020}<span class="tag">BT.2020</span>{/if}
              </div>
              <dl>
                <dt>{p().native}</dt>
                <dd class="num">{d.native ? `${d.native.width} × ${d.native.height} @ ${hz(d.native.refresh_hz)}` : "—"}</dd>
                <dt>{p().refresh}</dt>
                <dd class="num">{d.refresh_max ? p().upTo(d.refresh_max) : "—"}{d.refresh_min ? ` (min. ${d.refresh_min} Hz)` : ""}</dd>
                <dt>{p().size}</dt>
                <dd class="num">{d.width_cm && d.height_cm ? `${d.width_cm} × ${d.height_cm} cm` : "—"}</dd>
                <dt>{p().color}</dt>
                <dd>{d.bit_depth ? p().bitsPerColor(d.bit_depth) : "—"}</dd>
                <dt>{p().connection}</dt>
                <dd>{[d.interface, `EDID ${d.version}`].filter(Boolean).join(" · ")}</dd>
                <dt>{p().manufactured}</dt>
                <dd>{d.year ? (d.week ? t().spd.week(d.week, d.year) : d.year) : "—"}</dd>
                <dt>{p().panel}</dt>
                <dd class="num">{d.vendor_code} {d.product_code.toString(16).toUpperCase().padStart(4, "0")}</dd>
              </dl>
              {#if d.modes.length > 1}
                <p class="modes muted">
                  {p().modes}: {d.modes.map((m) => `${m.width}×${m.height}@${Math.round(m.refresh_hz)}`).join(" · ")}
                </p>
              {/if}
            </div>
          </div>
        {/each}
      </div>
      {#if monitors.length}
        <div class="in-use">
          <span class="muted">{p().inUse}:</span>
          {#each monitors as m}
            <span class="tag num">{m.name ?? ""} {m.size.width} × {m.size.height} · {p().scale(Math.round(m.scaleFactor * 100))}</span>
          {/each}
        </div>
      {/if}
    </Card>

    <Card title={p().devices} span={2}>
      {#if !data.devices.length}
        <p class="muted">{p().noDevices}</p>
      {/if}
      <ul class="devices">
        {#each data.devices as dev}
          {@const speed = usbSpeed(dev.speed_mbps)}
          <li>
            <span class="badge" data-kind={dev.kinds[0] ?? "hid"}><DeviceIcon kind={dev.kinds[0] ?? "hid"} /></span>
            <div class="dev-main">
              <strong>{dev.name ?? p().kind[dev.kinds[0]] ?? "—"}</strong>
              <span class="muted">
                {[
                  dev.vendor,
                  dev.vendor_id != null
                    ? `${dev.vendor_id.toString(16).padStart(4, "0")}:${(dev.product_id ?? 0).toString(16).padStart(4, "0")}`
                    : null,
                ]
                  .filter(Boolean)
                  .join(" · ")}
              </span>
              <div class="kinds">
                <span class="tag">{dev.bus === "usb" ? "USB" : "Bluetooth"}</span>
                {#each dev.kinds as kind}<span class="tag">{p().kind[kind] ?? kind}</span>{/each}
              </div>
            </div>
            <div class="dev-side muted">
              {#if speed}<span title={p().speed}>{speed}</span>{/if}
              {#if dev.max_power_ma}<span title={p().power}>{dev.max_power_ma} mA</span>{/if}
              {#if dev.driver}<span title={p().driver} class="driver">{dev.driver}</span>{/if}
            </div>
          </li>
        {/each}
      </ul>
      {#if data.batteries.length}
        <h3>{p().batteries}</h3>
        <ul class="batteries">
          {#each data.batteries as b}
            <li>
              <span>{b.name}</span>
              <div class="bat"><div style:width="{b.capacity ?? 0}%"></div></div>
              <strong class="num">{b.capacity != null ? `${b.capacity}%` : "—"}</strong>
            </li>
          {/each}
        </ul>
      {/if}
    </Card>

    <Card title={t().tests.title}>
      <p class="muted small">{t().tests.hint}</p>
      <div class="tests">
        {#each tests as item}
          {@const [title, hint] = testInfo(item.id)}
          <button class="test" onclick={() => (item.id === "pixels" ? (pixels = true) : (test = item.id))} title={hint}>
            <span class="test-icon"><DeviceIcon kind={item.icon} /></span>
            <strong>{title}</strong>
            <span class="arrow">→</span>
          </button>
        {/each}
      </div>
    </Card>

    <Card title={p().audio} span={2}>
      {#if !data.audio.length}
        <p class="muted">{p().noAudio}</p>
      {/if}
      <div class="audio">
        {#each [["output", p().outputs], ["input", p().inputs]] as [dir, heading]}
          {@const list = data.audio.filter((a) => a.direction === dir)}
          {#if list.length}
            <div>
              <h3>{heading}</h3>
              <ul>
                {#each list as a}
                  <li>
                    <div>
                      <strong>{a.name}</strong>
                      {#if a.default}<span class="tag accent">{p().defaultDevice}</span>{/if}
                    </div>
                    <span class="muted">
                      {[a.card, a.form ? p().form[a.form] : null, a.bus && a.bus !== a.form ? p().bus[a.bus] : null]
                        .filter(Boolean)
                        .join(" · ")}
                    </span>
                    <span class="muted small num">{audioFormat(a)}</span>
                  </li>
                {/each}
              </ul>
            </div>
          {/if}
        {/each}
      </div>
    </Card>

    <Card title={p().cameras}>
      {#if !data.cameras.length}
        <p class="muted">{p().noCameras}</p>
      {/if}
      {#each data.cameras as cam}
        {@const best = cameraSummary(cam)}
        <div class="camera">
          <strong>{cam.name}</strong>
          {#if cam.bus}<span class="tag">USB</span>{/if}
          {#if best}
            <dl>
              <dt>{p().maxResolution}</dt>
              <dd class="num">{best}</dd>
            </dl>
          {/if}
          {#each cam.formats as f}
            <details>
              <summary><span class="num">{f.fourcc}</span> <span class="muted">· {f.description}</span></summary>
              <table>
                <tbody>
                  {#each f.modes as m}
                    <tr><td class="num">{m.width} × {m.height}</td><td class="num">{m.max_fps ? `${m.max_fps} fps` : "—"}</td></tr>
                  {/each}
                </tbody>
              </table>
            </details>
          {/each}
        </div>
      {/each}
      <p class="muted small">{p().privacyHint}</p>
    </Card>
  </div>
{/if}

<TestDialog title={t().tests.keyboard} bind:open={() => keyboardOpen, (v) => !v && (test = null)} closeOnEscape={false}>
  <KeyboardTest />
</TestDialog>
<TestDialog title={t().tests.mouse} bind:open={() => mouseOpen, (v) => !v && (test = null)}>
  <MouseTest />
</TestDialog>
<TestDialog title={t().tests.speakers} bind:open={() => speakersOpen, (v) => !v && (test = null)}>
  <SpeakerTest />
</TestDialog>
<TestDialog title={t().tests.camera} bind:open={() => cameraOpen, (v) => !v && (test = null)}>
  <CameraTest />
</TestDialog>
{#if pixels}<DeadPixelTest onclose={() => (pixels = false)} />{/if}

<style>
  .displays {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(420px, 1fr));
    gap: 20px;
  }

  .display {
    display: flex;
    gap: 20px;
    align-items: flex-start;
  }

  .screen-art {
    display: flex;
    flex-direction: column;
    align-items: center;
    width: 150px;
    flex-shrink: 0;
  }

  .panel {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    width: 100%;
    border: 5px solid color-mix(in srgb, var(--text) 75%, var(--surface));
    border-radius: 8px;
    background: linear-gradient(135deg, color-mix(in srgb, var(--accent) 55%, #000), color-mix(in srgb, var(--accent) 15%, #000));
    color: #fff;
    box-shadow: 0 0 24px color-mix(in srgb, var(--accent) 35%, transparent);
  }

  .panel span {
    font-size: 22px;
    font-weight: 700;
  }

  .panel small {
    font-size: 10px;
    opacity: 0.8;
  }

  .stand {
    width: 30px;
    height: 14px;
    background: color-mix(in srgb, var(--text) 60%, var(--surface));
    clip-path: polygon(30% 0, 70% 0, 100% 100%, 0 100%);
  }

  .display-info {
    flex: 1;
    min-width: 0;
  }

  .title {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 6px;
    margin-bottom: 10px;
  }

  .title strong {
    font-size: 16px;
    margin-right: 4px;
  }

  .tag {
    padding: 1px 8px;
    border-radius: 6px;
    font-size: 11px;
    font-weight: 600;
    background: var(--surface-2);
    border: 1px solid var(--border);
    white-space: nowrap;
  }

  .tag.accent {
    color: var(--accent);
    border-color: color-mix(in srgb, var(--accent) 45%, var(--border));
    background: color-mix(in srgb, var(--accent) 12%, transparent);
  }

  .modes,
  .small {
    font-size: 12px;
  }

  .modes {
    margin: 10px 0 0;
  }

  .in-use {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 6px;
    margin-top: 16px;
    padding-top: 12px;
    border-top: 1px solid var(--border);
    font-size: 12px;
  }

  ul {
    list-style: none;
    margin: 0;
    padding: 0;
  }

  .devices li {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 10px 0;
    border-bottom: 1px solid var(--border);
  }

  .devices li:last-child {
    border-bottom: 0;
  }

  .badge {
    display: grid;
    place-items: center;
    width: 36px;
    height: 36px;
    flex-shrink: 0;
    border-radius: 10px;
    font-weight: 700;
    color: var(--accent);
    background: color-mix(in srgb, var(--accent) 15%, transparent);
  }

  .badge[data-kind="camera"],
  .badge[data-kind="audio"] {
    color: var(--good);
    background: color-mix(in srgb, var(--good) 15%, transparent);
  }

  .badge[data-kind="bluetooth"],
  .badge[data-kind="network"],
  .badge[data-kind="storage"] {
    color: var(--warn);
    background: color-mix(in srgb, var(--warn) 15%, transparent);
  }

  .dev-main {
    display: flex;
    flex-direction: column;
    min-width: 0;
    flex: 1;
  }

  .dev-main > span {
    font-size: 12px;
  }

  .kinds {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
    margin-top: 4px;
  }

  .dev-side {
    display: flex;
    flex-direction: column;
    align-items: flex-end;
    gap: 2px;
    font-size: 12px;
    text-align: right;
  }

  .driver {
    font-family: ui-monospace, monospace;
    font-size: 11px;
  }

  h3 {
    margin: 16px 0 8px;
    font-size: 12px;
    color: var(--muted);
    text-transform: uppercase;
    letter-spacing: 0.06em;
  }

  .batteries li {
    display: grid;
    grid-template-columns: 1fr 120px auto;
    align-items: center;
    gap: 12px;
    padding: 4px 0;
  }

  .bat {
    height: 8px;
    border-radius: 4px;
    background: var(--surface-2);
    overflow: hidden;
  }

  .bat div {
    height: 100%;
    background: var(--good);
  }

  .tests {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .test {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 10px 12px;
    border: 1px solid var(--border);
    border-radius: 12px;
    background: var(--surface-2);
    text-align: left;
    transition:
      border-color 0.2s,
      transform 0.2s;
  }

  .test:hover {
    border-color: var(--accent);
    transform: translateX(2px);
  }

  .test-icon {
    display: grid;
    place-items: center;
    width: 34px;
    height: 34px;
    border-radius: 9px;
    color: var(--accent);
    background: color-mix(in srgb, var(--accent) 15%, transparent);
    font-weight: 700;
  }

  .arrow {
    margin-left: auto;
    color: var(--muted);
  }

  .audio {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(240px, 1fr));
    gap: 20px;
  }

  .audio h3 {
    margin-top: 0;
  }

  .audio li {
    display: flex;
    flex-direction: column;
    gap: 1px;
    padding: 8px 0;
    border-bottom: 1px solid var(--border);
  }

  .audio li:last-child {
    border-bottom: 0;
  }

  .audio li > span:first-of-type {
    font-size: 12px;
  }

  .audio li div {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .camera {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 8px;
    margin-bottom: 12px;
  }

  details {
    width: 100%;
  }

  summary {
    cursor: pointer;
    font-size: 13px;
  }

  details table {
    margin-top: 6px;
    font-size: 12px;
  }

  @media (max-width: 560px) {
    .displays {
      grid-template-columns: 1fr;
    }
    .display {
      flex-direction: column;
    }
  }
</style>

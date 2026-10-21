// Relatório com tudo o que o app coletou. Os dados viram uma lista de seções
// independente de formato, que depois é escrita em HTML, Markdown ou JSON.

import { invoke } from "@tauri-apps/api/core";
import { bytes, cameraSummary, displayName, ghz, hex, or, size, smbiosDate, usbSpeed } from "$lib/format";
import { i18n, t } from "$lib/i18n/index.svelte";
import { privileged } from "$lib/privileged.svelte";
import type {
  BoardInfo,
  CpuInfo,
  CpuLive,
  Gpu,
  GpuLive,
  MemoryInfo,
  MemoryLive,
  PcieLink,
  Peripherals,
  SpdModule,
  SystemInfo,
  Upgrades,
} from "$lib/types";
import { gpuPath, memoryAction, memorySpec } from "$lib/upgrades.svelte";

export type Format = "html" | "md" | "json";

type Block =
  | { kind: "kv"; title?: string; rows: [string, string][] }
  | { kind: "table"; title?: string; head: string[]; rows: string[][] }
  | { kind: "chips"; title: string; items: string[] };

interface Section {
  id: string;
  title: string;
  blocks: Block[];
}

interface Data {
  system: SystemInfo;
  cpu: CpuInfo | null;
  cpuLive: CpuLive | null;
  board: BoardInfo | null;
  memory: MemoryInfo | null;
  memoryLive: MemoryLive | null;
  spd: SpdModule[];
  gpus: Gpu[];
  gpuLive: GpuLive[];
  peripherals: Peripherals | null;
  upgrades: Upgrades | null;
  privileged: typeof privileged.data;
  bench: { date: string; single: number; multi: number; threads: number } | null;
  generatedAt: string;
}

/** Lê todos os componentes; o que falhar fica de fora em vez de interromper o relatório. */
export async function collect(): Promise<Data> {
  const get = <T>(cmd: string) => invoke<T>(cmd).catch(() => null);
  const [system, cpu, cpuLive, board, memory, memoryLive, spd, gpus, gpuLive, peripherals, upgrades] = await Promise.all([
    get<SystemInfo>("system_info"),
    get<CpuInfo>("cpu_info"),
    get<CpuLive>("cpu_live"),
    get<BoardInfo>("board_info"),
    get<MemoryInfo>("memory_info"),
    get<MemoryLive>("memory_live"),
    get<SpdModule[]>("spd_info"),
    get<Gpu[]>("gpu_info"),
    get<GpuLive[]>("gpu_live"),
    get<Peripherals>("peripherals_info"),
    get<Upgrades>("upgrade_info"),
  ]);
  let bench: Data["bench"] = null;
  try {
    bench = JSON.parse(localStorage.getItem("silicio.bench.history") ?? "[]")[0] ?? null;
  } catch {
    // Sem histórico salvo.
  }
  return {
    system: system ?? { app_version: "?", os: null, kernel: null, arch: "?" },
    cpu,
    cpuLive,
    board,
    memory,
    memoryLive,
    spd: spd ?? [],
    gpus: gpus ?? [],
    gpuLive: gpuLive ?? [],
    peripherals,
    upgrades,
    privileged: privileged.data,
    bench,
    generatedAt: new Date().toISOString(),
  };
}

const link = (l: PcieLink | null, current: boolean) => {
  const gen = current ? l?.current_gen : l?.max_gen;
  const width = current ? l?.current_width : l?.max_width;
  return gen ? `PCIe ${gen}.0${width ? ` x${width}` : ""}` : "—";
};

const mts = (n: number | null | undefined) => (n ? `${n} MT/s` : "—");

function sections(d: Data): Section[] {
  const m = t();
  const out: Section[] = [];
  const { cpu, board, memory } = d;

  // Resumo de uma linha por componente, para quem só quer o essencial.
  const summary: [string, string][] = [];
  const computer = [board?.system.vendor, board?.system.product].filter(Boolean).join(" ");
  if (computer) summary.push([m.report.computer, computer]);
  if (cpu) summary.push([m.tabs.cpu, `${cpu.brand} (${cpu.cores}C/${cpu.threads}T)`]);
  if (board) summary.push([m.tabs.board, [board.board.vendor, board.board.name].filter(Boolean).join(" ") || "—"]);
  if (memory) summary.push([m.tabs.memory, `${bytes(memory.total_bytes)} ${memory.kind ?? ""} ${mts(memory.speed_mts)} · ${m.memory.channels(memory.channels)}`]);
  for (const gpu of d.gpus) summary.push([m.gpu.card, `${gpu.name ?? gpu.vendor ?? "—"}${gpu.vram_bytes ? ` · ${bytes(gpu.vram_bytes)}` : ""}`]);
  out.push({
    id: "summary",
    title: m.report.summary,
    blocks: [
      { kind: "kv", rows: summary },
      {
        kind: "kv",
        title: m.report.os,
        rows: [
          [m.report.os, or(d.system.os)],
          [m.report.kernel, or(d.system.kernel)],
          [m.report.arch, d.system.arch],
        ],
      },
    ],
  });

  if (cpu) {
    const sig = cpu.signature;
    const blocks: Block[] = [
      {
        kind: "kv",
        rows: [
          [m.common.model, cpu.brand],
          [m.common.manufacturer, `${cpu.vendor} (${cpu.vendor_id})`],
          ...(cpu.codename
            ? ([
                ["Codename", `${cpu.codename.name} · ${cpu.codename.microarch}`],
                [m.cpu.process, cpu.codename.process],
              ] as [string, string][])
            : []),
          [m.common.family, `${hex(sig.family)} (base ${hex(sig.base_family)} + ext ${hex(sig.ext_family)})`],
          [m.common.model + " (CPUID)", hex(sig.model)],
          ["Stepping", String(sig.stepping)],
          [m.cpu.microcode, or(cpu.microcode)],
          [m.cpu.coresThreads, `${cpu.cores} / ${cpu.threads}`],
          [m.cpu.maxClock, ghz(cpu.clocks.max_mhz)],
          ...(cpu.clocks.driver ? ([["Driver", `${cpu.clocks.driver} · ${cpu.clocks.governor ?? ""}`]] as [string, string][]) : []),
        ],
      },
      {
        kind: "table",
        title: m.cpu.cache,
        head: [m.cpu.cache, m.memory.type, "Total", "Assoc."],
        rows: cpu.caches.map((c) => [
          `L${c.level}`,
          m.cpu.cacheKind[c.kind] ?? c.kind,
          `${c.instances > 1 ? `${c.instances} × ` : ""}${size(c.size_kib)}`,
          `${c.ways}-way · ${c.line_bytes} B`,
        ]),
      },
    ];
    const groups = new Set(cpu.topology.map((c) => c.cluster)).size;
    const perf = cpu.topology.filter((c) => c.kind === "performance").length;
    const eff = cpu.topology.filter((c) => c.kind === "efficiency").length;
    const main = blocks[0] as Extract<Block, { kind: "kv" }>;
    main.rows.push([m.report.topology, m.report.groups(groups)]);
    if (perf || eff) main.rows.push([m.coreMap.title, m.report.hybrid(perf, eff)]);
    if (d.privileged) {
      const p = d.privileged;
      const volts = p.threads.map((x) => x.voltage).filter((v): v is number => v != null);
      blocks.push({
        kind: "kv",
        title: m.cpu.advanced,
        rows: [
          [m.cpu.socket, or(p.processor?.socket)],
          [m.cpu.vid, volts.length ? `${Math.max(...volts).toFixed(3)} V` : "—"],
          [m.cpu.realBus, p.bus_mhz ? `${p.bus_mhz.toFixed(2)} MHz` : "—"],
          [m.cpu.baseMultiplier, p.base_ratio ? `x${p.base_ratio.toFixed(1)}` : "—"],
        ],
      });
    }
    for (const group of cpu.instructions) {
      blocks.push({ kind: "chips", title: `${m.cpu.instructions} · ${m.cpu.instructionGroup[group.name] ?? group.name}`, items: group.items });
    }
    out.push({ id: "cpu", title: m.tabs.cpu, blocks });
  }

  if (board) {
    const rows: [string, string][] = [
      [m.board.boardVendor, or(board.board.vendor)],
      [m.board.boardModel, or(board.board.name)],
      [m.board.revision, or(board.board.version)],
      [m.board.chipset, board.chipset ? `${or(board.chipset.vendor)} ${or(board.chipset.name)}` : "—"],
    ];
    if (board.southbridge) rows.push(["Southbridge", `${or(board.southbridge.vendor)} ${or(board.southbridge.name)}`]);
    out.push({
      id: "board",
      title: m.tabs.board,
      blocks: [
        {
          kind: "kv",
          title: m.board.system,
          rows: [
            [m.common.manufacturer, or(board.system.vendor)],
            [m.board.product, or(board.system.product)],
            [m.common.family, or(board.system.family)],
            [m.board.type, board.system.chassis ? (m.board.chassis[board.system.chassis] ?? board.system.chassis) : "—"],
          ],
        },
        { kind: "kv", title: m.board.board, rows },
        {
          kind: "kv",
          title: "BIOS",
          rows: [
            [m.board.brand, or(board.bios.vendor)],
            [m.common.version, or(board.bios.version)],
            [m.common.date, smbiosDate(board.bios.date, i18n.locale)],
          ],
        },
      ],
    });
  }

  if (memory) {
    const blocks: Block[] = [
      {
        kind: "kv",
        rows: [
          [m.memory.installed, bytes(memory.total_bytes)],
          [m.memory.type, or(memory.kind)],
          [m.memory.effectiveSpeed, mts(memory.speed_mts)],
          [m.memory.dramClock, memory.speed_mts ? `${memory.speed_mts / 2} MHz` : "—"],
          [m.report.topology, m.memory.channels(memory.channels)],
          [m.memory.maxCapacity, bytes(memory.max_capacity_bytes)],
        ],
      },
    ];
    if (memory.slots.length) {
      blocks.push({
        kind: "table",
        title: m.memory.modules,
        head: [m.report.slot, m.spd.size, m.memory.type, m.common.speed, m.common.manufacturer, m.common.model],
        rows: memory.slots.map((s) =>
          s.size_bytes
            ? [or(s.locator), bytes(s.size_bytes), [s.kind, s.form_factor].filter(Boolean).join(" ") || "—", `${mts(s.configured_mts)} (${mts(s.speed_mts)})`, or(s.manufacturer), or(s.part_number)]
            : [or(s.locator), m.memory.emptySlot, "", "", "", ""],
        ),
      });
    }
    out.push({ id: "memory", title: m.tabs.memory, blocks });
  }

  if (d.spd.length) {
    const blocks: Block[] = [];
    for (const mod of d.spd) {
      const x = mod.decoded;
      const title = `${m.spd.module} ${mod.slot} · ${mod.kind}${mod.module_type ? ` ${mod.module_type}` : ""}`;
      if (!x) {
        blocks.push({ kind: "kv", title, rows: [[m.spd.module, m.spd.notDecoded(mod.kind)]] });
        continue;
      }
      blocks.push({
        kind: "kv",
        title,
        rows: [
          [m.common.manufacturer, or(x.module_manufacturer)],
          [m.common.model, or(x.part_number)],
          [m.spd.size, bytes(x.size_mb * 1024 ** 2)],
          [m.spd.maxSpeed, `${x.max_speed_mts} MT/s (${x.pc_name})`],
          [m.spd.timings, x.table[0] ? `${x.table[0].cl}-${x.table[0].trcd}-${x.table[0].trp}-${x.table[0].tras}` : "—"],
          [m.common.voltage, x.voltage ? `${x.voltage.toFixed(2)} V` : "—"],
          [m.spd.dramChips, or(x.dram_manufacturer)],
          [m.spd.organization, `${x.ranks} rank(s) · x${x.device_width} · ${x.die_density_gbit} Gb`],
          [m.spd.xmp, x.xmp ? m.spd.xmpProfiles(x.xmp.version, x.xmp.profiles) : m.spd.no],
          [m.spd.manufactured, x.manufactured ? m.spd.week(x.manufactured[1], x.manufactured[0]) : "—"],
          [m.spd.integrity, x.crc_ok ? m.spd.crcOk : m.spd.crcBad],
        ],
      });
      blocks.push({
        kind: "table",
        title: `${m.spd.jedecTable} · ${m.spd.module} ${mod.slot}`,
        head: [m.common.speed, m.spd.clock, "CL", "tRCD", "tRP", "tRAS", "tRC"],
        rows: x.table.map((b) => [`${b.speed_mts} MT/s`, `${b.clock_mhz} MHz`, ...[b.cl, b.trcd, b.trp, b.tras, b.trc].map(String)]),
      });
    }
    out.push({ id: "spd", title: m.tabs.spd, blocks });
  }

  if (d.gpus.length) {
    out.push({
      id: "gpu",
      title: m.tabs.gpu,
      blocks: d.gpus.map((g) => ({
        kind: "kv" as const,
        title: g.name ?? g.vendor ?? g.id,
        rows: [
          [m.common.manufacturer, or(g.vendor)],
          [m.board.type, g.integrated == null ? "—" : g.integrated ? m.gpu.integrated : m.gpu.dedicated],
          [m.gpu.videoMemory, bytes(g.vram_bytes)],
          ["Driver", [g.driver, g.driver_version].filter(Boolean).join(" ") || "—"],
          [m.gpu.vbios, or(g.vbios)],
          [`${m.gpu.maxClocks} · ${m.gpu.core}`, g.max_core_mhz ? `${g.max_core_mhz} MHz` : "—"],
          [`${m.gpu.maxClocks} · ${m.gpu.memory}`, g.max_memory_mhz ? `${g.max_memory_mhz} MHz` : "—"],
          [m.common.pcieCurrent, link(g.link, true)],
          [m.common.pcieMax, link(g.link, false)],
        ] as [string, string][],
      })),
    });
  }

  const per = d.peripherals;
  if (per && (per.displays.length || per.devices.length || per.audio.length || per.cameras.length)) {
    const p = m.peripherals;
    const blocks: Block[] = per.displays.map((x) => ({
      kind: "kv" as const,
      title: `${p.displays} · ${displayName(x)}`,
      rows: [
        [m.board.type, `${x.internal ? p.internal : p.external}${x.connector ? ` (${x.connector})` : ""}`],
        [p.size, x.diagonal_in ? `${p.inches(x.diagonal_in.toFixed(1))} · ${x.width_cm} × ${x.height_cm} cm` : "—"],
        [p.native, x.native ? `${x.native.width} × ${x.native.height} @ ${x.native.refresh_hz} Hz` : "—"],
        [p.refresh, x.refresh_max ? p.upTo(x.refresh_max) : "—"],
        [p.color, [x.bit_depth ? p.bitsPerColor(x.bit_depth) : null, x.hdr ? "HDR" : null, x.bt2020 ? "BT.2020" : null].filter(Boolean).join(" · ") || "—"],
        [p.connection, [x.interface, `EDID ${x.version}`].filter(Boolean).join(" · ")],
        [p.manufactured, x.year ? (x.week ? m.spd.week(x.week, x.year) : String(x.year)) : "—"],
        [p.panel, `${x.vendor_code} ${x.product_code.toString(16).toUpperCase().padStart(4, "0")}`],
      ] as [string, string][],
    }));
    for (const cam of per.cameras) {
      blocks.push({
        kind: "kv",
        title: `${p.cameras} · ${cam.name}`,
        rows: [
          [p.maxResolution, cameraSummary(cam) ?? "—"],
          [p.formats, cam.formats.map((f) => f.fourcc).join(", ") || "—"],
        ],
      });
    }
    if (per.devices.length) {
      blocks.push({
        kind: "table",
        title: p.devices,
        head: [m.common.model, m.board.type, m.common.manufacturer, "ID", p.speed, p.power],
        rows: per.devices.map((x) => [
          or(x.name),
          [x.bus === "usb" ? "USB" : "Bluetooth", ...x.kinds.map((k) => p.kind[k] ?? k)].join(" · "),
          or(x.vendor),
          x.vendor_id != null ? `${x.vendor_id.toString(16).padStart(4, "0")}:${(x.product_id ?? 0).toString(16).padStart(4, "0")}` : "—",
          usbSpeed(x.speed_mbps) ?? "—",
          x.max_power_ma ? `${x.max_power_ma} mA` : "—",
        ]),
      });
    }
    if (per.batteries.length) {
      blocks.push({
        kind: "table",
        title: p.batteries,
        head: [m.common.model, "%", ""],
        rows: per.batteries.map((b) => [b.name, b.capacity != null ? `${b.capacity}%` : "—", or(b.status)]),
      });
    }
    if (per.audio.length) {
      blocks.push({
        kind: "table",
        title: p.audio,
        head: [m.common.model, m.board.type, p.card, m.report.value],
        rows: per.audio.map((a) => [
          `${a.name}${a.default ? ` (${p.defaultDevice})` : ""}`,
          [a.direction === "output" ? p.outputs : p.inputs, a.form ? p.form[a.form] : null, a.bus && a.bus !== a.form ? p.bus[a.bus] : null]
            .filter(Boolean)
            .join(" · "),
          or(a.card),
          a.sample_rate ? p.format(`${a.sample_rate / 1000} kHz`, a.bits, a.channels) : p.idle,
        ]),
      });
    }
    out.push({ id: "peripherals", title: m.tabs.peripherals, blocks });
  }

  const up = d.upgrades;
  if (up) {
    const u = m.upgrades;
    const memRows: [string, string][] = [[u.memory, memoryAction(up.memory)]];
    if (up.memory.module_count > 0) {
      memRows.push([u.memSpec, `${up.memory.module_count} × ${bytes(up.memory.module_bytes)} ${memorySpec(up.memory)}`]);
      memRows.push([u.memTarget, bytes(up.memory.target_bytes)]);
    }
    const cpuRows: [string, string][] =
      up.cpu.path === "socket"
        ? [
            [u.socket, or(up.cpu.socket)],
            [u.ceiling, up.cpu.ceiling.join(" · ")],
          ]
        : [[u.cpu, up.cpu.path === "soldered" ? u.cpuSoldered : u.cpuUnknown]];
    const gpuRows: [string, string][] = [[u.gpu, gpuPath(up.gpu)]];
    if (up.gpu.path !== "none") {
      gpuRows.push([u.tier, u.tiers[up.gpu.tier]]);
      gpuRows.push([u.suggestions, up.gpu.suggestions.map((s) => `${s.name} (${u.lanes(s.lanes)})`).join(" · ")]);
    }
    out.push({
      id: "upgrades",
      title: m.tabs.upgrades,
      blocks: [
        { kind: "kv", rows: [[u.bestMove, u.top[up.top]]] },
        { kind: "kv", title: u.gpu, rows: gpuRows },
        { kind: "kv", title: u.memory, rows: memRows },
        { kind: "kv", title: u.cpu, rows: cpuRows },
      ],
    });
  }

  const liveRows: [string, string][] = [];
  if (d.cpuLive) {
    liveRows.push([m.cpu.avgClock, ghz(d.cpuLive.avg_mhz)]);
    liveRows.push([m.overview.cpuLoad, `${d.cpuLive.usage.toFixed(0)}%`]);
    if (d.cpuLive.temperature_c != null) liveRows.push([m.common.temperature, `${d.cpuLive.temperature_c.toFixed(0)} °C`]);
  }
  if (d.memoryLive) {
    liveRows.push([m.overview.ram, `${bytes(d.memoryLive.used_bytes)} / ${bytes(d.memoryLive.total_bytes)}`]);
    liveRows.push([m.report.swap, `${bytes(d.memoryLive.swap_used_bytes)} / ${bytes(d.memoryLive.swap_total_bytes)}`]);
  }
  for (const g of d.gpuLive) {
    if (g.usage != null) liveRows.push([`${m.overview.gpuLoad} (${d.gpus.find((x) => x.id === g.id)?.name ?? g.id})`, `${g.usage.toFixed(0)}%`]);
    if (g.temperature_c != null) liveRows.push([`${m.common.temperature} GPU`, `${g.temperature_c.toFixed(0)} °C`]);
  }
  const blocks: Block[] = [{ kind: "kv", rows: liveRows }];
  if (d.bench) {
    blocks.push({
      kind: "kv",
      title: m.report.lastBench,
      rows: [
        [m.report.single, d.bench.single.toFixed(0)],
        [m.report.multi, `${d.bench.multi.toFixed(0)} (${d.bench.threads} threads)`],
        [m.common.date, new Date(d.bench.date).toLocaleString(i18n.locale)],
      ],
    });
  }
  out.push({ id: "live", title: m.report.liveNow, blocks });
  return out;
}

export function fileName(format: Format): string {
  const d = new Date();
  const pad = (n: number) => String(n).padStart(2, "0");
  const stamp = `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}-${pad(d.getHours())}${pad(d.getMinutes())}`;
  return `Silicio-${i18n.locale === "pt-BR" ? "relatorio" : "report"}-${stamp}.${format}`;
}

export function render(format: Format, d: Data): string {
  if (format === "json") return JSON.stringify(d, null, 2);
  return format === "md" ? markdown(d) : html(d);
}

function header(d: Data) {
  const m = t();
  const date = new Date(d.generatedAt).toLocaleString(i18n.locale);
  const title = d.cpu ? `${m.report.title} · ${d.cpu.brand}` : m.report.title;
  return { title, meta: `${m.report.generated(date, d.system.app_version)} ${m.report.privacy}` };
}

function markdown(d: Data): string {
  const cell = (s: string) => s.replace(/\|/g, "\\|") || " ";
  const { title, meta } = header(d);
  const lines = [`# ${title}`, "", `_${meta}_`, ""];
  for (const s of sections(d)) {
    lines.push(`## ${s.title}`, "");
    for (const b of s.blocks) {
      if (b.title) lines.push(`### ${b.title}`, "");
      if (b.kind === "kv") {
        lines.push(`| ${t().report.item} | ${t().report.value} |`, "|---|---|");
        for (const [k, v] of b.rows) lines.push(`| ${cell(k)} | ${cell(v)} |`);
      } else if (b.kind === "table") {
        lines.push(`| ${b.head.map(cell).join(" | ")} |`, `|${b.head.map(() => "---").join("|")}|`);
        for (const r of b.rows) lines.push(`| ${r.map(cell).join(" | ")} |`);
      } else {
        lines.push(b.items.map((i) => `\`${i}\``).join(" "));
      }
      lines.push("");
    }
  }
  return lines.join("\n");
}

const esc = (s: string) => s.replace(/[&<>"]/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" })[c]!);

function html(d: Data): string {
  const { title, meta } = header(d);
  const list = sections(d);
  const body = list
    .map((s) => {
      // Blocos de chave-valor lado a lado numa grade; tabelas e listas ocupam a largura toda.
      const block = (b: Block) => {
        const h = b.title ? `<h3>${esc(b.title)}</h3>` : "";
        if (b.kind === "kv") {
          return `<div class="block">${h}<dl>${b.rows.map(([k, v]) => `<dt>${esc(k)}</dt><dd>${esc(v)}</dd>`).join("")}</dl></div>`;
        }
        if (b.kind === "table") {
          const head = b.head.map((x) => `<th>${esc(x)}</th>`).join("");
          const rows = b.rows.map((r) => `<tr>${r.map((x) => `<td>${esc(x)}</td>`).join("")}</tr>`).join("");
          return `<div class="block">${h}<div class="table-wrap"><table><thead><tr>${head}</tr></thead><tbody>${rows}</tbody></table></div></div>`;
        }
        return `<div class="block">${h}<p class="chips">${b.items.map((i) => `<span>${esc(i)}</span>`).join("")}</p></div>`;
      };
      const kv = s.blocks.filter((b) => b.kind === "kv").map(block).join("");
      const wide = s.blocks.filter((b) => b.kind !== "kv").map(block).join("");
      const blocks = `${kv ? `<div class="blocks">${kv}</div>` : ""}${wide ? `<div class="stack">${wide}</div>` : ""}`;
      return `<section id="${s.id}"><h2>${esc(s.title)}</h2>${blocks}</section>`;
    })
    .join("\n");
  const toc = list.map((s) => `<a href="#${s.id}">${esc(s.title)}</a>`).join("");
  return `<!doctype html>
<html lang="${i18n.locale}">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>${esc(title)}</title>
<style>
:root{--bg:#f4f5f8;--surface:#fff;--border:#dfe2e9;--text:#161a22;--muted:#646b7a;--accent:#5b6cff;color-scheme:light dark}
@media (prefers-color-scheme:dark){:root{--bg:#0d0f13;--surface:#151820;--border:#262b37;--text:#e7e9ee;--muted:#8a91a1;--accent:#8a98ff}}
*{box-sizing:border-box}
body{margin:0;background:var(--bg);color:var(--text);font:14px/1.5 Inter,"Segoe UI",system-ui,sans-serif}
main{max-width:1000px;margin:0 auto;padding:32px 16px 64px}
header{padding:24px 28px;border-radius:16px;background:linear-gradient(135deg,var(--accent),#2b3290);color:#fff}
header h1{margin:0 0 6px;font-size:26px}
header p{margin:0;opacity:.85;font-size:13px}
nav{display:flex;flex-wrap:wrap;gap:8px;margin:16px 0 8px}
nav a{padding:5px 12px;border:1px solid var(--border);border-radius:999px;background:var(--surface);color:inherit;text-decoration:none;font-size:13px}
section{margin-top:28px}
h2{margin:0 0 12px;font-size:20px}
h3{margin:0 0 10px;font-size:12px;letter-spacing:.06em;text-transform:uppercase;color:var(--muted)}
.blocks{display:grid;grid-template-columns:repeat(auto-fit,minmax(300px,1fr));gap:12px}
.block{padding:16px 18px;border:1px solid var(--border);border-radius:12px;background:var(--surface);min-width:0}
.stack{display:flex;flex-direction:column;gap:12px;margin-top:12px}
dl{display:grid;grid-template-columns:minmax(120px,auto) 1fr;gap:6px 16px;margin:0}
dt{color:var(--muted)}dd{margin:0;overflow-wrap:anywhere}
.table-wrap{overflow-x:auto}
table{width:100%;border-collapse:collapse;font-variant-numeric:tabular-nums}
th,td{padding:6px 8px;text-align:left;border-bottom:1px solid var(--border)}
th{color:var(--muted);font-weight:600}
.chips{display:flex;flex-wrap:wrap;gap:6px;margin:0}
.chips span{padding:2px 9px;border:1px solid var(--border);border-radius:7px;font-size:12px}
footer{margin-top:40px;color:var(--muted);font-size:12px;text-align:center}
@media print{:root{--bg:#fff;--surface:#fff;--text:#000;--muted:#555;--border:#ccc}nav{display:none}header{background:none;color:#000;padding:0}section{break-inside:avoid-page}}
@media (max-width:560px){.blocks{grid-template-columns:1fr}dl{grid-template-columns:1fr}dd{margin-bottom:6px}}
</style>
</head>
<body>
<main>
<header><h1>${esc(title)}</h1><p>${esc(meta)}</p></header>
<nav aria-label="${esc(t().report.contents)}">${toc}</nav>
${body}
<footer>Silício ${esc(d.system.app_version)} · https://github.com/Anghievischi/silicio</footer>
</main>
</body>
</html>
`;
}

// Cartão de especificações para compartilhar: um SVG 1200×630 (tamanho de
// prévia em redes sociais) convertido em PNG.

import { bytes, displayName, ghz, size } from "$lib/format";
import { t } from "$lib/i18n/index.svelte";
import type { collect } from "$lib/report";

type Data = Awaited<ReturnType<typeof collect>>;

export const WIDTH = 1200;
export const HEIGHT = 630;

const ACCENTS: Record<string, string> = { AMD: "#ff5a4f", Intel: "#3b9dff" };

const THEMES = {
  dark: { bg: "#0d0f13", tile: "#151820", border: "#262b37", text: "#e7e9ee", muted: "#8a91a1" },
  light: { bg: "#f4f5f8", tile: "#ffffff", border: "#dfe2e9", text: "#161a22", muted: "#646b7a" },
};

export type Theme = keyof typeof THEMES;

const esc = (s: string) => s.replace(/[&<>"]/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" })[c]!);

/** SVG não quebra linha: corta o texto com reticências. */
const fit = (s: string, max: number) => (s.length > max ? `${s.slice(0, max - 1).trimEnd()}…` : s);

export function cardSvg(d: Data, theme: Theme, withBench: boolean): string {
  const m = t().share;
  const c = THEMES[theme];
  const cpu = d.cpu;
  const accent = ACCENTS[cpu?.vendor ?? ""] ?? "#7c8cff";
  const font = `font-family="Inter, 'Segoe UI Variable', 'Segoe UI', system-ui, sans-serif"`;

  const brand = cpu?.brand ?? "—";
  // O nome não pode invadir o desenho do chip à direita.
  const brandSize = brand.length > 30 ? 40 : 50;
  const sub = cpu?.codename ? [cpu.codename.name, cpu.codename.microarch, cpu.codename.process].join(" · ") : (cpu?.vendor ?? "");
  const l3 = cpu?.caches.filter((x) => x.level === 3).reduce((sum, x) => sum + x.size_kib * x.instances, 0) ?? 0;
  const pills = [
    cpu ? m.cores(cpu.cores, cpu.threads) : null,
    cpu?.clocks.max_mhz ? m.upTo(ghz(cpu.clocks.max_mhz)) : null,
    l3 ? `L3 ${size(l3)}` : null,
  ].filter((x): x is string => !!x);

  // Placa dedicada primeiro: é a que interessa num cartão de specs.
  const gpu = [...d.gpus].sort((a, b) => Number(a.integrated) - Number(b.integrated))[0];
  const display = d.peripherals?.displays.find((x) => x.internal === false) ?? d.peripherals?.displays[0];
  const tiles: [string, string, string][] = [
    [t().gpu.card, gpu?.name ?? "—", gpu ? [gpu.integrated ? t().gpu.integrated : t().gpu.dedicated, gpu.vram_bytes ? bytes(gpu.vram_bytes) : null].filter(Boolean).join(" · ") : ""],
    [
      t().tabs.memory,
      d.memory ? `${bytes(d.memory.total_bytes)} ${d.memory.kind ?? ""}`.trim() : "—",
      d.memory ? [d.memory.speed_mts ? `${d.memory.speed_mts} MT/s` : null, t().memory.channels(d.memory.channels)].filter(Boolean).join(" · ") : "",
    ],
    [t().tabs.board, d.board?.board.name ?? "—", d.board?.board.vendor ?? ""],
    display
      ? [
          m.display,
          displayName(display),
          [display.diagonal_in ? `${display.diagonal_in.toFixed(1)}″` : null, display.native ? `${display.native.width}×${display.native.height}` : null, display.refresh_max ? `${display.refresh_max} Hz` : null]
            .filter(Boolean)
            .join(" · "),
        ]
      : [m.system, d.system.os ?? "—", d.system.arch],
  ];

  const tileW = (WIDTH - 120 - 3 * 20) / 4;
  const tileSvg = tiles
    .map(([label, value, detail], i) => {
      const x = 60 + i * (tileW + 20);
      return `<g transform="translate(${x} 346)">
  <rect width="${tileW}" height="128" rx="18" fill="${c.tile}" stroke="${c.border}"/>
  <rect x="20" y="22" width="26" height="4" rx="2" fill="${accent}"/>
  <text x="20" y="52" ${font} font-size="13" font-weight="700" letter-spacing="1.5" fill="${c.muted}">${esc(label.toUpperCase())}</text>
  <text x="20" y="84" ${font} font-size="21" font-weight="700" fill="${c.text}">${esc(fit(value, 20))}</text>
  <text x="20" y="110" ${font} font-size="14" fill="${c.muted}">${esc(fit(detail, 30))}</text>
</g>`;
    })
    .join("\n");

  let pillX = 60;
  const pillY = 178 + brandSize + 58;
  const pillSvg = pills
    .map((p) => {
      const w = p.length * 9 + 32;
      const g = `<g transform="translate(${pillX} ${pillY})"><rect width="${w}" height="38" rx="19" fill="${accent}" fill-opacity="0.14" stroke="${accent}" stroke-opacity="0.45"/><text x="${w / 2}" y="25" text-anchor="middle" ${font} font-size="16" font-weight="650" fill="${c.text}">${esc(p)}</text></g>`;
      pillX += w + 12;
      return g;
    })
    .join("");

  const bench = withBench && d.bench;
  const bottom = bench
    ? `<g transform="translate(60 496)">
  <text y="22" ${font} font-size="13" font-weight="700" letter-spacing="1.5" fill="${c.muted}">${esc(m.bench.toUpperCase())}</text>
  <text y="62" ${font} font-size="34" font-weight="800" fill="${accent}">${Math.round(d.bench!.single)}</text>
  <text x="${String(Math.round(d.bench!.single)).length * 21 + 10}" y="62" ${font} font-size="16" fill="${c.muted}">${esc(m.single)}</text>
  <text x="300" y="62" ${font} font-size="34" font-weight="800" fill="${accent}">${Math.round(d.bench!.multi)}</text>
  <text x="${300 + String(Math.round(d.bench!.multi)).length * 21 + 10}" y="62" ${font} font-size="16" fill="${c.muted}">${esc(m.multi(d.bench!.threads))}</text>
</g>`
    : `<text x="60" y="548" ${font} font-size="18" fill="${c.muted}">${esc(fit([d.system.os, d.system.arch].filter(Boolean).join(" · "), 70))}</text>`;

  // Chip estilizado, o mesmo desenho da Visão geral.
  const pins = [0, 1, 2, 3, 4]
    .map((i) => {
      const p = 26 + i * 18;
      return `<line x1="${p}" y1="2" x2="${p}" y2="18"/><line x1="${p}" y1="146" x2="${p}" y2="162"/><line x1="2" y1="${p}" x2="18" y2="${p}"/><line x1="146" y1="${p}" x2="162" y2="${p}"/>`;
    })
    .join("");

  return `<svg xmlns="http://www.w3.org/2000/svg" width="${WIDTH}" height="${HEIGHT}" viewBox="0 0 ${WIDTH} ${HEIGHT}">
<defs>
  <radialGradient id="glow1" cx="0" cy="0" r="1" gradientUnits="userSpaceOnUse" gradientTransform="translate(120 40) scale(760 520)">
    <stop offset="0" stop-color="${accent}" stop-opacity="${theme === "dark" ? 0.32 : 0.2}"/><stop offset="1" stop-color="${accent}" stop-opacity="0"/>
  </radialGradient>
  <radialGradient id="glow2" cx="0" cy="0" r="1" gradientUnits="userSpaceOnUse" gradientTransform="translate(1060 220) scale(380)">
    <stop offset="0" stop-color="${accent}" stop-opacity="0.28"/><stop offset="1" stop-color="${accent}" stop-opacity="0"/>
  </radialGradient>
  <linearGradient id="mark" x1="0" y1="0" x2="1" y2="1">
    <stop offset="0" stop-color="${accent}"/><stop offset="1" stop-color="${accent}" stop-opacity="0.55"/>
  </linearGradient>
</defs>
<rect width="${WIDTH}" height="${HEIGHT}" fill="${c.bg}"/>
<rect width="${WIDTH}" height="${HEIGHT}" fill="url(#glow1)"/>
<rect width="${WIDTH}" height="${HEIGHT}" fill="url(#glow2)"/>
<g transform="translate(60 48)">
  <rect width="44" height="44" rx="12" fill="url(#mark)"/>
  <text x="22" y="29" text-anchor="middle" ${font} font-size="18" font-weight="800" fill="#fff">Si</text>
  <text x="58" y="30" ${font} font-size="22" font-weight="700" fill="${c.text}">Silício</text>
</g>
<g transform="translate(980 140)" stroke="${accent}" stroke-width="5" stroke-linecap="round" opacity="0.95">
  ${pins}
  <rect x="18" y="18" width="128" height="128" rx="16" fill="${accent}" fill-opacity="0.18" stroke-width="3"/>
  <rect x="44" y="44" width="76" height="76" rx="8" fill="${accent}" fill-opacity="0.55" stroke-width="2"/>
</g>
<text x="60" y="178" ${font} font-size="15" font-weight="800" letter-spacing="2" fill="${accent}">${esc((cpu?.vendor ?? "").toUpperCase())}</text>
<text x="60" y="${178 + brandSize}" ${font} font-size="${brandSize}" font-weight="800" letter-spacing="-1" fill="${c.text}">${esc(fit(brand, 38))}</text>
<text x="60" y="${178 + brandSize + 36}" ${font} font-size="20" fill="${c.muted}">${esc(fit(sub, 70))}</text>
${pillSvg}
${tileSvg}
${bottom}
<text x="${WIDTH - 60}" y="548" text-anchor="end" ${font} font-size="15" font-weight="600" fill="${c.muted}">github.com/Anghievischi/silicio</text>
<rect x="0" y="${HEIGHT - 8}" width="${WIDTH}" height="8" fill="${accent}"/>
</svg>`;
}

/** Rasteriza o SVG em PNG com o dobro da resolução, para ficar nítido em telas densas. */
export async function toPng(svg: string, scale = 2): Promise<Blob> {
  const url = URL.createObjectURL(new Blob([svg], { type: "image/svg+xml" }));
  try {
    const img = new Image();
    img.src = url;
    await img.decode();
    const canvas = document.createElement("canvas");
    canvas.width = WIDTH * scale;
    canvas.height = HEIGHT * scale;
    const ctx = canvas.getContext("2d")!;
    ctx.drawImage(img, 0, 0, canvas.width, canvas.height);
    return await new Promise((resolve, reject) =>
      canvas.toBlob((b) => (b ? resolve(b) : reject(new Error("canvas vazio"))), "image/png"),
    );
  } finally {
    URL.revokeObjectURL(url);
  }
}

// Cartão de especificações para compartilhar: um SVG 1200×630 (tamanho de
// prévia em redes sociais) convertido em PNG, no visual de ficha técnica do app.

import { bytes, displayName, ghz, size } from "$lib/format";
import { t } from "$lib/i18n/index.svelte";
import type { collect } from "$lib/report";
import display700 from "@fontsource/big-shoulders-display/files/big-shoulders-display-latin-700-normal.woff2?url";
import display900 from "@fontsource/big-shoulders-display/files/big-shoulders-display-latin-900-normal.woff2?url";
import mono400 from "@fontsource/jetbrains-mono/files/jetbrains-mono-latin-400-normal.woff2?url";
import mono700 from "@fontsource/jetbrains-mono/files/jetbrains-mono-latin-700-normal.woff2?url";

type Data = Awaited<ReturnType<typeof collect>>;

export const WIDTH = 1200;
export const HEIGHT = 630;

const STRIPES = ["#e5533d", "#f28c28", "#f2c14e", "#4fb3a9"];
const ACCENTS: Record<string, { bright: string; deep: string }> = {
  AMD: { bright: "#f28c28", deep: "#b3560d" },
  Intel: { bright: "#4fb3a9", deep: "#2f7f77" },
};

const THEMES = {
  dark: { bg: "#15130f", tile: "#1c1915", border: "#3a342a", text: "#ede4d3", muted: "#a39883", footer: "#0d0c09" },
  light: { bg: "#ede4d3", tile: "#f6f0e4", border: "#c9bba3", text: "#15130f", muted: "#5a4e3c", footer: "#15130f" },
};

export type Theme = keyof typeof THEMES;

const DISPLAY = `font-family="Big Shoulders Display, 'Arial Narrow', sans-serif"`;
const MONO = `font-family="JetBrains Mono, ui-monospace, monospace"`;

const esc = (s: string) => s.replace(/[&<>"]/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" })[c]!);

/** SVG não quebra linha: corta o texto com reticências. */
const fit = (s: string, max: number) => (s.length > max ? `${s.slice(0, max - 1).trimEnd()}…` : s);

// Largura média de um caractere em relação ao tamanho da fonte, para o layout.
const DISPLAY_EM = 0.46;
const MONO_EM = 0.6;

let fontCss: Promise<string> | null = null;

/**
 * Um SVG desenhado como imagem não enxerga as fontes da página, então as
 * fontes do app vão embutidas no próprio SVG.
 */
export function cardFonts(): Promise<string> {
  fontCss ??= Promise.all(
    [
      ["Big Shoulders Display", 700, display700],
      ["Big Shoulders Display", 900, display900],
      ["JetBrains Mono", 400, mono400],
      ["JetBrains Mono", 700, mono700],
    ].map(async ([family, weight, url]) => {
      const buffer = await (await fetch(url as string)).arrayBuffer();
      let binary = "";
      for (const byte of new Uint8Array(buffer)) binary += String.fromCharCode(byte);
      return `@font-face{font-family:"${family}";font-weight:${weight};src:url(data:font/woff2;base64,${btoa(binary)}) format("woff2")}`;
    }),
  )
    .then((faces) => faces.join(""))
    .catch(() => "");
  return fontCss;
}

/** Marca do app: o die do ícone com a faixa de quatro cores. */
const mark = (x: number, y: number, s: number) => `<svg x="${x}" y="${y}" width="${s}" height="${s}" viewBox="160 160 704 704">
  <rect x="160" y="160" width="704" height="704" fill="#1c1915"/>
  <path d="M330 260H764V764H260V330Z" fill="#ede4d3"/>
  ${STRIPES.map((c, i) => `<rect x="${652 + i * 28}" y="260" width="28" height="504" fill="${c}"/>`).join("")}
  <g fill="#15130f"><path d="M351 372H493V428H379V484H493V652H323V596H437V540H323V400Z"/><rect x="533" y="372" width="56" height="56"/><rect x="533" y="464" width="56" height="188"/></g>
</svg>`;

export function cardSvg(d: Data, theme: Theme, withBench: boolean, fonts = ""): string {
  const m = t().share;
  const c = THEMES[theme];
  const cpu = d.cpu;
  const vendorAccent = ACCENTS[cpu?.vendor ?? ""] ?? ACCENTS.AMD;
  const accent = theme === "dark" ? vendorAccent.bright : vendorAccent.deep;

  // "AMD Ryzen 7 5700U with Radeon Graphics": o complemento vai para a segunda linha.
  const [brand, extra] = (cpu?.brand ?? "—").split(/\s+(?=with\s)/i);
  const brandSize = Math.min(112, Math.floor(1080 / (brand.length * DISPLAY_EM)));

  const l3 = cpu?.caches.filter((x) => x.level === 3).reduce((sum, x) => sum + x.size_kib * x.instances, 0) ?? 0;
  const facts = [
    cpu?.vendor,
    cpu?.codename?.microarch,
    cpu ? `${cpu.cores}C / ${cpu.threads}T` : null,
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

  const tileW = (WIDTH - 120 - 3 * 16) / 4;
  const valueChars = Math.floor((tileW - 36) / (26 * DISPLAY_EM));
  const detailChars = Math.floor((tileW - 36) / (13 * MONO_EM));
  const tileSvg = tiles
    .map(([label, value, detail], i) => {
      const x = 60 + i * (tileW + 16);
      return `<g transform="translate(${x} 352)">
  <rect x="1" y="1" width="${tileW - 2}" height="122" fill="${c.tile}" stroke="${c.border}" stroke-width="2"/>
  <rect x="18" y="20" width="10" height="10" fill="${accent}"/>
  <text x="36" y="30" ${MONO} font-size="12" font-weight="700" letter-spacing="1.5" fill="${c.muted}">${esc(fit(label.toUpperCase(), 22))}</text>
  <text x="18" y="74" ${DISPLAY} font-size="30" font-weight="900" fill="${c.text}">${esc(fit(value.toUpperCase(), valueChars))}</text>
  <text x="18" y="102" ${MONO} font-size="13" fill="${c.muted}">${esc(fit(detail, detailChars))}</text>
</g>`;
    })
    .join("\n");

  // Faixa de quatro cores sobre o rodapé, a assinatura do app.
  const stripes = STRIPES.map((s, i) => `<rect x="0" y="${514 + i * 9}" width="${WIDTH}" height="9" fill="${s}"/>`).join("");

  // Rodapé: pontuação do benchmark (ou o sistema), montado da esquerda para a direita.
  let x = 60;
  const run: string[] = [];
  const put = (text: string, attrs: string, em: number, fontSize: number, gap: number) => {
    run.push(`<text x="${x}" ${attrs}>${esc(text)}</text>`);
    x += text.length * fontSize * em + gap;
  };
  if (withBench && d.bench) {
    const score = `y="600" ${DISPLAY} font-size="44" font-weight="900" fill="${vendorAccent.bright}"`;
    const caption = `y="598" ${MONO} font-size="13" fill="#a39883"`;
    put(m.bench.toUpperCase(), `y="590" ${MONO} font-size="12" font-weight="700" letter-spacing="1.5" fill="#a39883"`, MONO_EM + 0.12, 12, 28);
    put(String(Math.round(d.bench.single)), score, DISPLAY_EM, 44, 10);
    put(m.single, caption, MONO_EM, 13, 30);
    put(String(Math.round(d.bench.multi)), score, DISPLAY_EM, 44, 10);
    put(m.multi(d.bench.threads), caption, MONO_EM, 13, 0);
  } else {
    put(fit(["> ", d.system.os, d.system.arch].filter(Boolean).join(" "), 60), `y="596" ${MONO} font-size="15" fill="#a39883"`, MONO_EM, 15, 0);
  }
  const footerLeft = run.join("\n");

  return `<svg xmlns="http://www.w3.org/2000/svg" width="${WIDTH}" height="${HEIGHT}" viewBox="0 0 ${WIDTH} ${HEIGHT}">
<style>${fonts}</style>
<rect width="${WIDTH}" height="${HEIGHT}" fill="${c.bg}"/>
${mark(60, 44, 40)}
<text x="114" y="76" ${DISPLAY} font-size="32" font-weight="900" letter-spacing="3" fill="${c.text}">SILÍCIO</text>
<g transform="translate(${WIDTH - 60} 44)">
  <rect x="${-(m.specSheet.length * (13 * MONO_EM + 1) + 34)}" y="1" width="${m.specSheet.length * (13 * MONO_EM + 1) + 33}" height="38" fill="none" stroke="${c.text}" stroke-width="2"/>
  <text x="-17" y="26" text-anchor="end" ${MONO} font-size="13" font-weight="700" letter-spacing="1" fill="${c.text}">${esc(m.specSheet.toUpperCase())}</text>
</g>
<text x="60" y="150" ${MONO} font-size="14" font-weight="700" letter-spacing="1.5" fill="${c.muted}">${esc(fit(facts.join(" · ").toUpperCase(), 70))}</text>
<text x="56" y="${150 + brandSize * 0.92 + 12}" ${DISPLAY} font-size="${brandSize}" font-weight="900" fill="${c.text}">${esc(brand.toUpperCase())}</text>
${extra ? `<text x="60" y="${150 + brandSize * 0.92 + 56}" ${DISPLAY} font-size="34" font-weight="700" letter-spacing="1" fill="${c.muted}">${esc(fit(extra.toUpperCase(), 60))}</text>` : ""}
${tileSvg}
${stripes}
<rect x="0" y="550" width="${WIDTH}" height="${HEIGHT - 550}" fill="${c.footer}"/>
${footerLeft}
<text x="${WIDTH - 60}" y="596" text-anchor="end" ${MONO} font-size="14" fill="#a39883">github.com/Anghievischi/silicio</text>
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

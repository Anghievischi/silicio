import type { Messages } from "./i18n/pt-BR";
import type { Disk, MemProfile, SpdDecoded, SystemBattery } from "./types";

export const hex = (n: number) => "0x" + n.toString(16).toUpperCase();

export function ghz(mhz: number | null | undefined): string {
  if (!mhz) return "—";
  return (mhz / 1000).toFixed(2) + " GHz";
}

export function size(kib: number): string {
  if (kib >= 1024) return `${+(kib / 1024).toFixed(1)} MB`;
  return `${kib} KB`;
}

export function bytes(n: number | null | undefined): string {
  if (!n) return "—";
  const gib = n / 1024 ** 3;
  if (gib >= 1) return `${+gib.toFixed(1)} GB`;
  return `${Math.round(n / 1024 ** 2)} MB`;
}

/** SMBIOS usa MM/DD/AAAA; mostramos a data no formato do idioma. */
export function smbiosDate(date: string | null, locale: string): string {
  const m = date?.match(/^(\d{2})\/(\d{2})\/(\d{4})$/);
  if (!m) return date ?? "—";
  return new Date(Number(m[3]), Number(m[1]) - 1, Number(m[2])).toLocaleDateString(locale);
}

export const or = (v: string | number | null | undefined) => (v === null || v === undefined || v === "" ? "—" : String(v));

/** Geração e taxa de um link USB a partir da velocidade em Mbit/s. */
export function usbSpeed(mbps: number | null | undefined): string | null {
  if (!mbps) return null;
  if (mbps <= 1.5) return "USB 1.x · 1,5 Mbps";
  if (mbps <= 12) return "USB 1.x · 12 Mbps";
  if (mbps <= 480) return "USB 2.0 · 480 Mbps";
  if (mbps <= 5000) return "USB 3.2 Gen 1 · 5 Gbps";
  if (mbps <= 10000) return "USB 3.2 Gen 2 · 10 Gbps";
  if (mbps <= 20000) return "USB 3.2 Gen 2×2 · 20 Gbps";
  return `USB4 · ${mbps / 1000} Gbps`;
}

/** Maior resolução da câmera, com o maior FPS disponível nela. */
export function cameraSummary(cam: { formats: { modes: { width: number; height: number; max_fps: number | null }[] }[] }): string | null {
  const modes = cam.formats.flatMap((f) => f.modes);
  if (!modes.length) return null;
  const area = Math.max(...modes.map((m) => m.width * m.height));
  const top = modes.filter((m) => m.width * m.height === area);
  const fps = Math.max(0, ...top.map((m) => m.max_fps ?? 0));
  return `${top[0].width} × ${top[0].height}${fps ? ` @ ${fps} fps` : ""}`;
}

/** "LG ULTRAGEAR" em vez de "LG LG ULTRAGEAR": o nome do monitor às vezes já traz a marca. */
export function displayName(d: { vendor: string | null; vendor_code: string; name: string | null }): string {
  const vendor = d.vendor ?? d.vendor_code;
  if (!d.name) return vendor;
  return d.name.toLowerCase().startsWith(vendor.toLowerCase()) ? d.name : `${vendor} ${d.name}`;
}

/** "XMP 3.0 · 2 perfis + EXPO · 1 perfil", ou "Não". */
export function memProfilesSummary(d: SpdDecoded, m: Messages): string {
  const parts = [
    d.xmp ? m.spd.xmpProfiles(d.xmp.version, d.xmp.profiles.length) : null,
    d.expo ? m.spd.expoProfiles(d.expo.length) : null,
  ].filter(Boolean);
  return parts.length ? parts.join(" + ") : m.spd.no;
}

/** Perfis XMP e EXPO de um módulo, com o rótulo que o BIOS mostra ("XMP 1", "EXPO 2"). */
export function memProfiles(d: SpdDecoded): { label: string; profile: MemProfile }[] {
  return [
    ...(d.xmp?.profiles ?? []).map((p) => ({ label: `XMP ${p.number}`, profile: p })),
    ...(d.expo ?? []).map((p) => ({ label: `EXPO ${p.number}`, profile: p })),
  ];
}

/** Tamanho de disco em unidades decimais, como vem na caixa ("512 GB", "2 TB"). */
export function diskSize(n: number | null | undefined): string {
  if (!n) return "—";
  if (n >= 1e12) return `${+(n / 1e12).toFixed(1)} TB`;
  if (n >= 1e9) return `${Math.round(n / 1e9)} GB`;
  return `${Math.round(n / 1e6)} MB`;
}

export const diskName = (d: Disk) => [d.vendor, d.model].filter(Boolean).join(" ") || d.id;

const BUS: Record<string, string> = { nvme: "NVMe", sata: "SATA", usb: "USB", mmc: "SD/eMMC", scsi: "SCSI", virtual: "Virtual" };

/** "NVMe · PCIe 4.0 x4", "SATA · 6.0 Gbps". */
export function diskInterface(d: Disk): string {
  const bus = d.bus ? BUS[d.bus] : null;
  const link = d.link?.max_gen ? `PCIe ${d.link.max_gen}.0${d.link.max_width ? ` x${d.link.max_width}` : ""}` : d.sata_speed;
  return [bus, link].filter(Boolean).join(" · ") || "—";
}

/** Capacidade que a bateria ainda segura, em % da original. */
export function batteryHealth(b: SystemBattery): number | null {
  if (!b.full_wh || !b.design_wh) return null;
  return Math.min(100, Math.round((b.full_wh / b.design_wh) * 100));
}

/** Tempo até esvaziar ou até encher, pela potência do momento. */
export function batteryTime(b: SystemBattery): { hours: number; minutes: number } | null {
  if (!b.power_w || b.now_wh == null) return null;
  const wh = b.status === "discharging" ? b.now_wh : b.status === "charging" && b.full_wh ? b.full_wh - b.now_wh : null;
  if (wh == null || wh <= 0) return null;
  const total = Math.round((wh / b.power_w) * 60);
  if (total > 48 * 60) return null;
  return { hours: Math.floor(total / 60), minutes: total % 60 };
}

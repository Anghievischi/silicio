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

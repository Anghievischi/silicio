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

/** SMBIOS usa MM/DD/AAAA; mostramos DD/MM/AAAA. */
export function smbiosDate(date: string | null): string {
  const m = date?.match(/^(\d{2})\/(\d{2})\/(\d{4})$/);
  return m ? `${m[2]}/${m[1]}/${m[3]}` : (date ?? "—");
}

export const or = (v: string | number | null | undefined) => (v === null || v === undefined || v === "" ? "—" : String(v));

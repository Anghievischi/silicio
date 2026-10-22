import { invoke } from "@tauri-apps/api/core";
import { t } from "$lib/i18n/index.svelte";
import type { SessionFinding, SessionReport, SessionSample, SessionStatus } from "$lib/types";

/** Sessão de jogo: compartilhada entre a aba Jogos e a barra de status. */
export const session = $state<{
  running: boolean;
  /** Date.now() do início, para a barra de status contar o tempo sem consultar o backend. */
  startedAt: number;
  report: SessionReport | null;
}>({ running: false, startedAt: 0, report: null });

export function applyStatus(status: SessionStatus) {
  session.running = status.running;
  if (status.running) session.startedAt = Date.now() - status.seconds * 1000;
}

export async function startSession() {
  applyStatus(await invoke<SessionStatus>("session_start"));
}

export async function stopSession() {
  session.report = await invoke<SessionReport | null>("session_stop");
  session.running = false;
}

/** Laudo da última sessão, lido do backend ao abrir a aba. */
export async function loadSession() {
  const status = await invoke<SessionStatus>("session_status");
  applyStatus(status);
  if (status.has_report && !session.report) {
    session.report = await invoke<SessionReport | null>("session_report");
  }
  return status;
}

/** Título e explicação de um achado da sessão no idioma atual. */
export function sessionFindingText(f: SessionFinding, report: SessionReport): { title: string; text: string } {
  const describe = t().gaming.findings[f.code] as (f: SessionFinding, r: SessionReport) => { title: string; text: string };
  return describe(f, report);
}

/** Reduz a série a `points` valores (média de cada trecho) para caber no gráfico. */
export function downsample(samples: SessionSample[], get: (s: SessionSample) => number | null, points = 120): number[] {
  const values = samples.map(get);
  const size = Math.max(1, Math.ceil(values.length / points));
  const out: number[] = [];
  for (let i = 0; i < values.length; i += size) {
    const chunk = values.slice(i, i + size).filter((v): v is number => v != null);
    out.push(chunk.length ? chunk.reduce((a, b) => a + b, 0) / chunk.length : 0);
  }
  return out;
}

/** Amostras em CSV, uma linha por segundo. */
export function toCsv(samples: SessionSample[]): string {
  const mb = (n: number | null) => (n == null ? "" : Math.round(n / 2 ** 20));
  const v = (n: number | boolean | null) => (n == null ? "" : typeof n === "number" ? +n.toFixed(1) : n);
  const head = "t,cpu_usage,thread_max,cpu_mhz,cpu_temp,gpu_usage,gpu_mhz,gpu_temp,gpu_power_w,vram_used_mb,igpu_usage,ram_used_mb,swap_used_mb,on_battery";
  const rows = samples.map((s) =>
    [s.t, v(s.cpu_usage), v(s.thread_max), s.cpu_mhz, v(s.cpu_temp), v(s.gpu_usage), v(s.gpu_mhz), v(s.gpu_temp), v(s.gpu_power), mb(s.vram_used), v(s.igpu_usage), mb(s.ram_used), mb(s.swap_used), v(s.on_battery)].join(","),
  );
  return [head, ...rows].join("\n") + "\n";
}

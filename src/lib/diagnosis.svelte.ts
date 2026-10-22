import { invoke } from "@tauri-apps/api/core";
import { t } from "$lib/i18n/index.svelte";
import type { Diagnosis, Finding } from "$lib/types";

/** Diagnóstico, calculado uma vez e usado na Visão geral, na aba própria e na Memória. */
export const diagnosis = $state<{ data: Diagnosis | null; error: string | null }>({ data: null, error: null });

export function loadDiagnosis() {
  if (diagnosis.data) return;
  invoke<Diagnosis>("diagnosis_info")
    .then((data) => {
      diagnosis.data = data;
    })
    .catch((e) => {
      diagnosis.error = String(e);
    });
}

/** Título e explicação do achado no idioma atual. */
export function findingText(f: Finding): { title: string; text: string } {
  const describe = t().diagnosis.findings[f.code] as (f: Finding) => { title: string; text: string };
  return describe(f);
}

/** "1 problema · 2 dicas". */
export function findingCounts(d: Diagnosis): string {
  const count = (s: Finding["severity"]) => d.findings.filter((f) => f.severity === s).length;
  return t().diagnosis.counts(count("problem"), count("warning"), count("info"));
}

/** Aba onde o componente do achado aparece em detalhe. */
export const AREA_TAB: Record<Finding["area"], string> = {
  memory: "/memoria",
  cpu: "/processador",
  gpu: "/graficos",
  storage: "/armazenamento",
  battery: "/bateria",
  board: "/placa-mae",
  peripherals: "/perifericos",
};

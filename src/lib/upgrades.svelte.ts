import { invoke } from "@tauri-apps/api/core";
import { bytes } from "$lib/format";
import { t } from "$lib/i18n/index.svelte";
import type { Upgrades } from "$lib/types";

/** Sugestões de upgrade, calculadas uma vez e usadas em várias abas. */
export const upgrades = $state<{ data: Upgrades | null; error: string | null }>({ data: null, error: null });

export function loadUpgrades() {
  if (upgrades.data) return;
  invoke<Upgrades>("upgrade_info")
    .then((data) => (upgrades.data = data))
    .catch((e) => (upgrades.error = String(e)));
}

/** Frase com a ação sugerida para a memória. */
export function memoryAction(memory: Upgrades["memory"]): string {
  const u = t().upgrades;
  const module = bytes(memory.module_bytes);
  switch (memory.action) {
    case "dual-channel":
      return u.memDual(module);
    case "add":
      return u.memAdd(memory.module_count, module);
    case "replace":
      return u.memReplace(memory.module_count, module);
    case "maxed":
      return u.memMaxed;
    case "soldered":
      return u.memSoldered;
    default:
      return u.memUnknown;
  }
}

/** "DDR4 SODIMM · 3200 MT/s" */
export function memorySpec(memory: Upgrades["memory"]): string {
  const kind = [memory.kind, memory.form_factor].filter(Boolean).join(" ");
  return kind + (memory.speed_mts ? ` · ${memory.speed_mts} MT/s` : "");
}

/** Por onde entra uma placa de vídeo nova. */
export function gpuPath(gpu: Upgrades["gpu"]): string {
  const u = t().upgrades;
  if (gpu.path === "slot") return u.gpuSlot(gpu.slot_gen);
  if (gpu.path === "external") return u.gpuExternal(u.port[gpu.external_port ?? ""] ?? "");
  return u.gpuNone;
}

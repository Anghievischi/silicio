import { invoke } from "@tauri-apps/api/core";
import type { SystemBattery, SystemInfo } from "$lib/types";

/** Versão do app e sistema operacional, lidos uma vez e usados em várias telas. */
export const system = $state<{ info: SystemInfo | null; hasBattery: boolean }>({ info: null, hasBattery: false });

/** Controla a janela de exportação, aberta pelo menu ou pela Visão geral. */
export const reportDialog = $state({ open: false });

/** Controla a janela do cartão para compartilhar. */
export const shareDialog = $state({ open: false });

export function loadSystem() {
  if (system.info) return;
  invoke<SystemInfo>("system_info")
    .then((info) => (system.info = info))
    .catch(() => {});
  // A aba Bateria só aparece em notebooks.
  invoke<SystemBattery[]>("battery_info")
    .then((list) => (system.hasBattery = list.length > 0))
    .catch(() => {});
}

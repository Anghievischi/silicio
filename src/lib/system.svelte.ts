import { invoke } from "@tauri-apps/api/core";
import type { SystemInfo } from "$lib/types";

/** Versão do app e sistema operacional, lidos uma vez e usados em várias telas. */
export const system = $state<{ info: SystemInfo | null }>({ info: null });

/** Controla a janela de exportação, aberta pelo menu ou pela Visão geral. */
export const reportDialog = $state({ open: false });

/** Controla a janela do cartão para compartilhar. */
export const shareDialog = $state({ open: false });

export function loadSystem() {
  if (system.info) return;
  invoke<SystemInfo>("system_info")
    .then((info) => (system.info = info))
    .catch(() => {});
}

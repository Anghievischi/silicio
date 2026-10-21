import { invoke } from "@tauri-apps/api/core";
import type { PrivilegedInfo } from "$lib/types";

// Compartilhado entre as abas: a senha é pedida uma vez por sessão.
export const privileged = $state<{ data: PrivilegedInfo | null; loading: boolean; error: string | null }>({
  data: null,
  loading: false,
  error: null,
});

const isWindows = navigator.userAgent.includes("Windows");
// No Windows a temperatura depende da autorização; lembra que o usuário já a deu.
const AUTO_KEY = "silicio.sensors";

function remember(on: boolean) {
  if (!isWindows) return;
  try {
    if (on) localStorage.setItem(AUTO_KEY, "1");
    else localStorage.removeItem(AUTO_KEY);
  } catch {
    // Sem armazenamento, só não pede sozinho na próxima vez.
  }
}

export async function unlock() {
  privileged.loading = true;
  privileged.error = null;
  try {
    privileged.data = await invoke<PrivilegedInfo>("privileged_info");
    remember(true);
  } catch (e) {
    privileged.error = String(e);
    remember(false);
  } finally {
    privileged.loading = false;
  }
}

/** Ao abrir o app no Windows, pede a autorização de novo se ela já foi dada antes. */
export function autoUnlock() {
  try {
    if (isWindows && localStorage.getItem(AUTO_KEY) === "1") unlock();
  } catch {
    // Sem armazenamento: o usuário desbloqueia pela aba Processador.
  }
}

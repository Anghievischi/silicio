import { invoke } from "@tauri-apps/api/core";
import { isWindows } from "$lib/system.svelte";
import type { PrivilegedInfo } from "$lib/types";

// Compartilhado entre as abas: a senha é pedida uma vez por sessão.
export const privileged = $state<{ data: PrivilegedInfo | null; loading: boolean; error: string | null }>({
  data: null,
  loading: false,
  error: null,
});

export async function unlock() {
  privileged.loading = true;
  privileged.error = null;
  try {
    privileged.data = await invoke<PrivilegedInfo>("privileged_info");
  } catch (e) {
    privileged.error = String(e);
  } finally {
    privileged.loading = false;
  }
}

/** No Windows só o SMBIOS é lido, sem pedir permissão: carrega ao abrir o app. */
export function autoUnlock() {
  if (isWindows) unlock();
}

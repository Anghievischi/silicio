import { en } from "./en";
import { ptBR, type Messages } from "./pt-BR";

export type Locale = "pt-BR" | "en";

export const LOCALES: { id: Locale; label: string }[] = [
  { id: "pt-BR", label: "Português" },
  { id: "en", label: "English" },
];

const dictionaries: Record<Locale, Messages> = { "pt-BR": ptBR, en };
const STORAGE_KEY = "silicio.locale";

/** Idioma salvo pelo usuário ou, na falta dele, o do sistema. */
function detect(): Locale {
  try {
    const saved = localStorage.getItem(STORAGE_KEY);
    if (saved === "pt-BR" || saved === "en") return saved;
  } catch {
    // Sem armazenamento: segue para o idioma do sistema.
  }
  return navigator.language.toLowerCase().startsWith("pt") ? "pt-BR" : "en";
}

export const i18n = $state({ locale: detect() });

/** Textos do idioma atual. Ler dentro do template mantém a tela reativa à troca de idioma. */
export const t = (): Messages => dictionaries[i18n.locale];

export function setLocale(locale: Locale) {
  i18n.locale = locale;
  document.documentElement.lang = locale;
  try {
    localStorage.setItem(STORAGE_KEY, locale);
  } catch {
    // A escolha vale só para esta sessão.
  }
}

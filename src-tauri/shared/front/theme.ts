import type { MessageKey } from "@front-shared/i18n";

export type ThemeId = "light" | "dark" | "system";

export interface ThemeOption {
  id: ThemeId;
  nameKey: MessageKey;
  descriptionKey: MessageKey;
}

const STORAGE_KEY = "codex-switch-theme";
const DEFAULT_THEME: ThemeId = "light";

export const themeOptions: readonly ThemeOption[] = [
  {
    id: "system",
    nameKey: "themeSystemName",
    descriptionKey: "themeSystemDescription",
  },
  {
    id: "light",
    nameKey: "themeLightName",
    descriptionKey: "themeLightDescription",
  },
  {
    id: "dark",
    nameKey: "themeDarkName",
    descriptionKey: "themeDarkDescription",
  },
] as const;

export function isThemeId(value: string | undefined): value is ThemeId {
  return themeOptions.some((theme) => theme.id === value);
}

export function getThemeOption(themeId: ThemeId): ThemeOption {
  return themeOptions.find((theme) => theme.id === themeId) ?? themeOptions[0];
}

export function resolveInitialTheme(): ThemeId {
  const stored = globalThis.localStorage?.getItem(STORAGE_KEY) ?? undefined;
  if (stored === "classic" || stored === "mica" || stored === "pine" || stored === "sea-salt") {
    return "light";
  }
  if (stored === "jade-night") {
    return "dark";
  }
  return isThemeId(stored) ? stored : DEFAULT_THEME;
}

export function persistTheme(theme: ThemeId): void {
  globalThis.localStorage?.setItem(STORAGE_KEY, theme);
}

export function resolveEffectiveTheme(theme: ThemeId): "classic" | "jade-night" {
  if (theme === "light") {
    return "classic";
  }
  if (theme === "dark") {
    return "jade-night";
  }

  const prefersDark = globalThis.matchMedia?.("(prefers-color-scheme: dark)").matches ?? false;
  return prefersDark ? "jade-night" : "classic";
}

export function applyTheme(theme: ThemeId): void {
  document.documentElement.dataset.themeChoice = theme;
  document.documentElement.dataset.theme = resolveEffectiveTheme(theme);
}

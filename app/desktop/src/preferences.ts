export type Theme = "dark" | "light";

export type Preferences = {
  avatarDataUrl: string;
  displayName: string;
  theme: Theme;
};

export const preferencesStorageKey = "ic-workbench-preferences-v1";

export const defaultPreferences: Preferences = {
  avatarDataUrl: "",
  displayName: "本地工作区",
  theme: "dark",
};

export function loadPreferences(): Preferences {
  try {
    const stored = window.localStorage.getItem(preferencesStorageKey);
    if (!stored) return defaultPreferences;

    const parsed = JSON.parse(stored) as Partial<Preferences>;
    return {
      avatarDataUrl:
        typeof parsed.avatarDataUrl === "string" ? parsed.avatarDataUrl : "",
      displayName:
        typeof parsed.displayName === "string" && parsed.displayName.trim()
          ? parsed.displayName.trim().slice(0, 24)
          : defaultPreferences.displayName,
      theme: parsed.theme === "light" ? "light" : "dark",
    };
  } catch {
    return defaultPreferences;
  }
}

export function savePreferences(preferences: Preferences): void {
  try {
    window.localStorage.setItem(
      preferencesStorageKey,
      JSON.stringify(preferences),
    );
  } catch {
    // The application can still run when local storage is unavailable or full.
  }
}

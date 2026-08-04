import { useCallback, useSyncExternalStore } from "react";

export type ThemePreference = "dark" | "light" | "system";

/** Persisted preference key — must stay in sync with the boot script in `__root.tsx`. */
export const THEME_STORAGE_KEY = "slipstream-theme";

export const DARK_THEME_MEDIA_QUERY = "(prefers-color-scheme: dark)";

const listeners = new Set<() => void>();

/**
 * Tab-local preference when `localStorage` is missing or `setItem` throws.
 * Only used on the default (browser) read/write path — explicit store args in
 * tests never touch this, so suites stay isolated.
 */
let sessionPreference: ThemePreference | null = null;

/** Clears tab-local fallback so unit tests do not leak across cases. */
export function resetThemeSessionPreferenceForTests(): void {
	sessionPreference = null;
}

function emit() {
	for (const listener of listeners) listener();
}

function browserStorage(): Pick<Storage, "getItem" | "setItem"> | null {
	try {
		if (typeof globalThis.localStorage === "undefined") return null;
		return globalThis.localStorage;
	} catch {
		return null;
	}
}

/** Map a raw storage value to a preference; unknown/null becomes `system`. */
export function parseThemePreference(stored: string | null): ThemePreference {
	if (stored === "dark" || stored === "light" || stored === "system") {
		return stored;
	}
	return "system";
}

/** Resolve whether the dark palette should apply for a preference + system flag. */
export function resolveIsDark(
	preference: ThemePreference,
	systemPrefersDark: boolean,
): boolean {
	if (preference === "dark") return true;
	if (preference === "light") return false;
	return systemPrefersDark;
}

/**
 * Read the theme preference.
 * Omit `store` to use browser storage + the in-memory session fallback.
 * Pass an explicit store (or `null`) for tests — that path ignores session state.
 */
export function readThemePreference(
	store?: Pick<Storage, "getItem"> | null,
): ThemePreference {
	if (arguments.length >= 1) {
		if (!store) return "system";
		try {
			return parseThemePreference(store.getItem(THEME_STORAGE_KEY));
		} catch {
			return "system";
		}
	}
	if (sessionPreference !== null) return sessionPreference;
	const browser = browserStorage();
	if (!browser) return "system";
	try {
		return parseThemePreference(browser.getItem(THEME_STORAGE_KEY));
	} catch {
		return "system";
	}
}

/**
 * Persist (or session-cache) the theme preference and notify subscribers.
 * Omit `store` for the browser path: failed/`null` storage still updates this tab.
 * Pass an explicit store (or `null`) for tests — that path never mutates session state.
 */
export function writeThemePreference(
	preference: ThemePreference,
	store?: Pick<Storage, "setItem"> | null,
): void {
	if (arguments.length >= 2) {
		if (!store) return;
		try {
			store.setItem(THEME_STORAGE_KEY, preference);
		} catch {
			return;
		}
		emit();
		return;
	}

	const browser = browserStorage();
	if (browser) {
		try {
			browser.setItem(THEME_STORAGE_KEY, preference);
			sessionPreference = null;
		} catch {
			sessionPreference = preference;
		}
	} else {
		sessionPreference = preference;
	}
	emit();
}

/** Inline boot script so first paint matches the stored preference (SSR-safe string). */
export function themeBootScriptSource(): string {
	return `(() => {
  let stored = null;
  try {
    stored = window.localStorage.getItem("${THEME_STORAGE_KEY}");
  } catch {}

  const preference =
    stored === "dark" || stored === "light" || stored === "system"
      ? stored
      : "system";
  const dark =
    preference === "dark" ||
    (preference === "system" &&
      (typeof window.matchMedia !== "function" ||
        window.matchMedia("${DARK_THEME_MEDIA_QUERY}").matches));

  document.documentElement.classList.toggle("dark", dark);
  document.documentElement.style.colorScheme = dark ? "dark" : "light";
})();`;
}

function readSystemPrefersDark(): boolean {
	if (typeof window === "undefined") return true;
	return (
		typeof window.matchMedia !== "function" ||
		window.matchMedia(DARK_THEME_MEDIA_QUERY).matches
	);
}

function resolveDarkTheme(): boolean {
	return resolveIsDark(readThemePreference(), readSystemPrefersDark());
}

function subscribeToTheme(onStoreChange: () => void) {
	listeners.add(onStoreChange);
	if (typeof window === "undefined") {
		return () => {
			listeners.delete(onStoreChange);
		};
	}

	const media = window.matchMedia?.(DARK_THEME_MEDIA_QUERY);
	media?.addEventListener("change", onStoreChange);
	window.addEventListener("storage", onStoreChange);

	return () => {
		listeners.delete(onStoreChange);
		media?.removeEventListener("change", onStoreChange);
		window.removeEventListener("storage", onStoreChange);
	};
}

/** SSR-safe dark resolution used by the document root. */
export function useDarkTheme(): boolean {
	return useSyncExternalStore(subscribeToTheme, resolveDarkTheme, () => true);
}

/** Preference + setter for Settings; applies immediately via the shared store. */
export function useThemePreference(): {
	preference: ThemePreference;
	setPreference: (next: ThemePreference) => void;
	isDark: boolean;
} {
	const preference = useSyncExternalStore(
		subscribeToTheme,
		readThemePreference,
		() => "system" as ThemePreference,
	);
	const isDark = useSyncExternalStore(
		subscribeToTheme,
		resolveDarkTheme,
		() => true,
	);
	const setPreference = useCallback((next: ThemePreference) => {
		writeThemePreference(next);
	}, []);
	return { preference, setPreference, isDark };
}

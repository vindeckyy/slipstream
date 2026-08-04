import { strictEqual } from "node:assert/strict";
import { describe, test } from "node:test";
import {
	parseThemePreference,
	resolveIsDark,
	THEME_STORAGE_KEY,
	themeBootScriptSource,
	readThemePreference,
	resetThemeSessionPreferenceForTests,
	writeThemePreference,
} from "./theme";

function memoryStore(initial: Record<string, string> = {}) {
	const data = { ...initial };
	return {
		getItem(key: string): string | null {
			return Object.hasOwn(data, key) ? (data[key] ?? null) : null;
		},
		setItem(key: string, value: string) {
			data[key] = value;
		},
		dump: () => data,
	};
}

function throwingStore() {
	return {
		getItem(): string | null {
			return null;
		},
		setItem(_key: string, _value: string) {
			throw new Error("quota exceeded");
		},
	};
}

describe("theme preference storage", () => {
	test("parseThemePreference accepts dark, light, and system", () => {
		strictEqual(parseThemePreference("dark"), "dark");
		strictEqual(parseThemePreference("light"), "light");
		strictEqual(parseThemePreference("system"), "system");
	});

	test("parseThemePreference falls back to system for unknown values", () => {
		strictEqual(parseThemePreference(null), "system");
		strictEqual(parseThemePreference(""), "system");
		strictEqual(parseThemePreference("sepia"), "system");
	});

	test("resolveIsDark follows preference over system", () => {
		strictEqual(resolveIsDark("dark", false), true);
		strictEqual(resolveIsDark("light", true), false);
		strictEqual(resolveIsDark("system", true), true);
		strictEqual(resolveIsDark("system", false), false);
	});

	test("persists preference under the shared storage key", () => {
		const store = memoryStore();
		writeThemePreference("light", store);
		strictEqual(store.dump()[THEME_STORAGE_KEY], "light");
		strictEqual(readThemePreference(store), "light");
	});

	test("readThemePreference treats a missing store as system", () => {
		strictEqual(readThemePreference(null), "system");
	});

	test("explicit null write does not pollute the session fallback", () => {
		resetThemeSessionPreferenceForTests();
		writeThemePreference("dark", null);
		strictEqual(readThemePreference(), "system");
		strictEqual(readThemePreference(null), "system");
	});

	test("boot script embeds the shared storage key", () => {
		const script = themeBootScriptSource();
		strictEqual(script.includes(THEME_STORAGE_KEY), true);
		strictEqual(script.includes("prefers-color-scheme: dark"), true);
	});

	test("default write with unavailable storage keeps preference in session", () => {
		resetThemeSessionPreferenceForTests();
		const previous = Object.getOwnPropertyDescriptor(
			globalThis,
			"localStorage",
		);
		Object.defineProperty(globalThis, "localStorage", {
			configurable: true,
			get() {
				throw new Error("blocked");
			},
		});
		try {
			writeThemePreference("light");
			strictEqual(readThemePreference(), "light");
			// Injected stores stay independent of session state.
			strictEqual(readThemePreference(null), "system");
			strictEqual(readThemePreference(memoryStore()), "system");
		} finally {
			if (previous) {
				Object.defineProperty(globalThis, "localStorage", previous);
			} else {
				Reflect.deleteProperty(globalThis, "localStorage");
			}
			resetThemeSessionPreferenceForTests();
		}
	});

	test("default write still applies when setItem throws", () => {
		resetThemeSessionPreferenceForTests();
		const previous = Object.getOwnPropertyDescriptor(
			globalThis,
			"localStorage",
		);
		Object.defineProperty(globalThis, "localStorage", {
			configurable: true,
			value: throwingStore(),
		});
		try {
			writeThemePreference("dark");
			strictEqual(readThemePreference(), "dark");
			strictEqual(readThemePreference(memoryStore()), "system");
		} finally {
			if (previous) {
				Object.defineProperty(globalThis, "localStorage", previous);
			} else {
				Reflect.deleteProperty(globalThis, "localStorage");
			}
			resetThemeSessionPreferenceForTests();
		}
	});
});

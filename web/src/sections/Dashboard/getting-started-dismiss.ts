import { useCallback, useSyncExternalStore } from "react";

/** Browser-local only: hide the Dashboard "Getting started" card until storage is cleared. */
export const GETTING_STARTED_DISMISS_KEY = "slipstream.getting-started.dismissed";

const listeners = new Set<() => void>();

function emit() {
	for (const listener of listeners) listener();
}

function subscribe(onStoreChange: () => void): () => void {
	listeners.add(onStoreChange);
	if (typeof window !== "undefined") {
		window.addEventListener("storage", onStoreChange);
	}
	return () => {
		listeners.delete(onStoreChange);
		if (typeof window !== "undefined") {
			window.removeEventListener("storage", onStoreChange);
		}
	};
}

function browserStorage(): Pick<Storage, "getItem" | "setItem"> | null {
	try {
		if (typeof globalThis.localStorage === "undefined") return null;
		return globalThis.localStorage;
	} catch {
		return null;
	}
}

export function readGettingStartedDismissed(
	store: Pick<Storage, "getItem"> | null = browserStorage(),
): boolean {
	if (!store) return false;
	try {
		return store.getItem(GETTING_STARTED_DISMISS_KEY) === "1";
	} catch {
		return false;
	}
}

export function writeGettingStartedDismissed(
	store: Pick<Storage, "setItem"> | null = browserStorage(),
): void {
	if (!store) return;
	try {
		store.setItem(GETTING_STARTED_DISMISS_KEY, "1");
	} catch {
		// Private mode / quota: keep the card visible rather than crash.
		return;
	}
	emit();
}

/** SSR-safe dismissal flag: server and first hydration paint as not dismissed. */
export function useGettingStartedDismissed(): {
	dismissed: boolean;
	dismiss: () => void;
} {
	const dismissed = useSyncExternalStore(
		subscribe,
		readGettingStartedDismissed,
		() => false,
	);
	const dismiss = useCallback(() => writeGettingStartedDismissed(), []);
	return { dismissed, dismiss };
}

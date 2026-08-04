import { strictEqual } from "node:assert/strict";
import { describe, test } from "node:test";
import {
	GETTING_STARTED_DISMISS_KEY,
	readGettingStartedDismissed,
	writeGettingStartedDismissed,
} from "./getting-started-dismiss";

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

describe("getting-started dismiss marker", () => {
	test("treats a missing store as not dismissed", () => {
		strictEqual(readGettingStartedDismissed(null), false);
	});

	test("persists a local dismiss flag", () => {
		const store = memoryStore();
		writeGettingStartedDismissed(store);
		strictEqual(store.dump()[GETTING_STARTED_DISMISS_KEY], "1");
		strictEqual(readGettingStartedDismissed(store), true);
	});
});

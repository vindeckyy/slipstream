// Connection/config resolution helpers for the supervised runner's Linux state directories.
import { afterEach, beforeEach, describe, expect, test } from "bun:test";
import * as path from "node:path";
import { pluginIngestDir, pluginStateDir } from "../src/config.js";

describe("pluginStateDir", () => {
	let saved: string | undefined;
	beforeEach(() => {
		saved = process.env.SLIPSTREAM_CONFIG_DIR;
	});
	afterEach(() => {
		if (saved === undefined) delete process.env.SLIPSTREAM_CONFIG_DIR;
		else process.env.SLIPSTREAM_CONFIG_DIR = saved;
	});

	test("resolves <config_dir>/plugin-state[/name] and honors the config-dir override", () => {
		process.env.SLIPSTREAM_CONFIG_DIR = path.join("/tmp", "ss-cfg");
		expect(pluginStateDir()).toBe(path.join("/tmp", "ss-cfg", "plugin-state"));
		expect(pluginStateDir("rom-manager")).toBe(
			path.join("/tmp", "ss-cfg", "plugin-state", "rom-manager"),
		);
	});

	test("the per-plugin dir is nested under the shared root", () => {
		process.env.SLIPSTREAM_CONFIG_DIR = path.join("/tmp", "ss-cfg2");
		expect(pluginStateDir("x").startsWith(pluginStateDir())).toBe(true);
	});
});

describe("pluginIngestDir", () => {
	let saved: string | undefined;
	beforeEach(() => {
		saved = process.env.SLIPSTREAM_CONFIG_DIR;
	});
	afterEach(() => {
		if (saved === undefined) delete process.env.SLIPSTREAM_CONFIG_DIR;
		else process.env.SLIPSTREAM_CONFIG_DIR = saved;
	});

	test("resolves <config_dir>/ingest[/name], distinct from plugin-state", () => {
		process.env.SLIPSTREAM_CONFIG_DIR = path.join("/tmp", "ss-cfg3");
		expect(pluginIngestDir()).toBe(path.join("/tmp", "ss-cfg3", "ingest"));
		expect(pluginIngestDir("catalog-sync")).toBe(
			path.join("/tmp", "ss-cfg3", "ingest", "catalog-sync"),
		);
		// The inbox is a different tree from plugin state.
		expect(pluginIngestDir("catalog-sync")).not.toBe(pluginStateDir("catalog-sync"));
	});
});

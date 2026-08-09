import { strictEqual } from "node:assert/strict";
import { describe, test } from "node:test";
import { isPluginUiEmbedPath } from "./auth";
import { parsePluginUiPath } from "../routes/plugin-ui/[...]";

const capability = "a".repeat(64);

describe("plugin UI origin boundary", () => {
	test("recognizes only capability paths as public plugin UI paths", () => {
		strictEqual(
			isPluginUiEmbedPath(`/plugin-ui/demo-ui/_embed/${capability}/`),
			true,
		);
		strictEqual(isPluginUiEmbedPath(`/plugin-ui/demo-ui/${capability}/`), false);
		strictEqual(isPluginUiEmbedPath("/plugin-ui/demo-ui/"), false);
		strictEqual(
			isPluginUiEmbedPath(`/plugin-ui/demo-ui/_embed/${"g".repeat(64)}/`),
			false,
		);
		strictEqual(
			isPluginUiEmbedPath(`/plugin-ui/demo-ui/_embed/${capability}/../api`),
			false,
		);
	});

	test("strips the capability segment before dialing the plugin", () => {
		strictEqual(
			parsePluginUiPath(`/plugin-ui/demo-ui/_embed/${capability}/events`)
				?.rest,
			"/events",
		);
		strictEqual(
			parsePluginUiPath(`/plugin-ui/demo-ui/_embed/${capability}/events`)
				?.prefix,
			`/plugin-ui/demo-ui/_embed/${capability}`,
		);
		strictEqual(
			parsePluginUiPath(`/plugin-ui/demo-ui/_embed/${"g".repeat(64)}/events`),
			null,
		);
		strictEqual(parsePluginUiPath("/plugin-ui/demo-ui/_embed/short"), null);
	});
});

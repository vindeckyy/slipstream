import { strictEqual } from "node:assert/strict";
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { afterEach, describe, test } from "node:test";
import { configureUiPassword, uiPassword } from "./auth";

const originalPassword = process.env.SLIPSTREAM_UI_PASSWORD;
const originalPasswordFile = process.env.SLIPSTREAM_UI_PASSWORD_FILE;
let tempDir: string | undefined;

afterEach(() => {
	if (originalPassword === undefined) delete process.env.SLIPSTREAM_UI_PASSWORD;
	else process.env.SLIPSTREAM_UI_PASSWORD = originalPassword;
	if (originalPasswordFile === undefined)
		delete process.env.SLIPSTREAM_UI_PASSWORD_FILE;
	else process.env.SLIPSTREAM_UI_PASSWORD_FILE = originalPasswordFile;
	if (tempDir) rmSync(tempDir, { recursive: true, force: true });
	tempDir = undefined;
});

describe("web console password setup", () => {
	test("persists a password and makes it available immediately", () => {
		tempDir = mkdtempSync(join(tmpdir(), "slipstream-auth-"));
		process.env.SLIPSTREAM_UI_PASSWORD_FILE = join(tempDir, "web-password");
		delete process.env.SLIPSTREAM_UI_PASSWORD;

		strictEqual(uiPassword(), "");
		strictEqual(configureUiPassword("correct horse battery staple"), true);
		strictEqual(uiPassword(), "correct horse battery staple");
		delete process.env.SLIPSTREAM_UI_PASSWORD;
		strictEqual(uiPassword(), "correct horse battery staple");
		strictEqual(
			readFileSync(process.env.SLIPSTREAM_UI_PASSWORD_FILE, "utf8"),
			"SLIPSTREAM_UI_PASSWORD=correct horse battery staple\n",
		);
	});

	test("never overwrites an existing password", () => {
		tempDir = mkdtempSync(join(tmpdir(), "slipstream-auth-"));
		const passwordFile = join(tempDir, "web-password");
		process.env.SLIPSTREAM_UI_PASSWORD_FILE = passwordFile;
		delete process.env.SLIPSTREAM_UI_PASSWORD;
		writeFileSync(passwordFile, "SLIPSTREAM_UI_PASSWORD=existing-password\n");

		strictEqual(uiPassword(), "existing-password");
		strictEqual(configureUiPassword("replacement-password"), false);
		strictEqual(
			readFileSync(passwordFile, "utf8"),
			"SLIPSTREAM_UI_PASSWORD=existing-password\n",
		);
	});

	test("can recover an empty password file", () => {
		tempDir = mkdtempSync(join(tmpdir(), "slipstream-auth-"));
		const passwordFile = join(tempDir, "web-password");
		process.env.SLIPSTREAM_UI_PASSWORD_FILE = passwordFile;
		delete process.env.SLIPSTREAM_UI_PASSWORD;
		writeFileSync(passwordFile, "");

		strictEqual(configureUiPassword("correct horse battery staple"), true);
		strictEqual(uiPassword(), "correct horse battery staple");
	});
});

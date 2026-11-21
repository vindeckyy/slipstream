// First-run setup is gated by a filesystem secret (web-setup-token), not by the caller's address.
//
// The defect this pins down: setup used to require `isLoopbackAddress(peerAddress(event))`. Under a
// loopback-terminating reverse proxy (`tailscale serve`, nginx `proxy_pass http://127.0.0.1:PORT`)
// every remote caller arrives as 127.0.0.1, so that check passed for the whole tailnet. A test that
// only exercised `isLoopbackAddress` would therefore "pass" while the console was claimable by
// anyone on the tailnet — so these tests drive the token, which is what actually decides.
import { strictEqual } from "node:assert/strict";
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { afterEach, beforeEach, describe, test } from "node:test";
import { isLoopbackAddress, setupToken, verifySetupToken } from "./auth";

const ENV_KEYS = [
	"SLIPSTREAM_UI_SETUP_TOKEN",
	"SLIPSTREAM_UI_SETUP_TOKEN_FILE",
	"SLIPSTREAM_CONFIG_DIR",
	"SLIPSTREAM_UI_PASSWORD",
	"SLIPSTREAM_UI_PASSWORD_FILE",
] as const;

let saved: Record<string, string | undefined> = {};
let dirs: string[] = [];

beforeEach(() => {
	saved = {};
	for (const key of ENV_KEYS) {
		saved[key] = process.env[key];
		delete process.env[key];
	}
});

afterEach(() => {
	for (const key of ENV_KEYS) {
		if (saved[key] === undefined) delete process.env[key];
		else process.env[key] = saved[key];
	}
	for (const dir of dirs) rmSync(dir, { recursive: true, force: true });
	dirs = [];
});

function freshConfigDir(): string {
	const dir = mkdtempSync(join(tmpdir(), "ss-setup-token-"));
	dirs.push(dir);
	process.env.SLIPSTREAM_CONFIG_DIR = dir;
	process.env.SLIPSTREAM_UI_SETUP_TOKEN_FILE = join(dir, "web-setup-token");
	return dir;
}

describe("first-run setup token", () => {
	test("is created in the private config dir, 64 hex chars, and is stable", () => {
		const dir = freshConfigDir();
		const token = setupToken();
		strictEqual(/^[0-9a-f]{64}$/.test(token), true, `unexpected token shape: ${token}`);
		strictEqual(readFileSync(join(dir, "web-setup-token"), "utf8").trim(), token);
		strictEqual(setupToken(), token, "the token must not rotate between reads");
	});

	test("accepts the on-disk token and rejects everything else", () => {
		freshConfigDir();
		const token = setupToken();
		strictEqual(verifySetupToken(token), true);
		strictEqual(verifySetupToken(` ${token}\n`), true, "surrounding whitespace is tolerated");
		strictEqual(verifySetupToken(""), false, "an empty submission never passes");
		strictEqual(verifySetupToken("0".repeat(64)), false, "a guessed token does not pass");
		strictEqual(verifySetupToken(token.slice(0, -1)), false, "a truncated token does not pass");
	});

	test("a remote caller behind a loopback proxy still cannot pass without the token", () => {
		freshConfigDir();
		const token = setupToken();
		// This is exactly the confusing state the old gate could not distinguish: the peer address
		// says "local" because the proxy is local, while the real caller is on the tailnet.
		strictEqual(isLoopbackAddress("127.0.0.1"), true);
		strictEqual(verifySetupToken(""), false, "proxy-forwarded caller has no token -> refused");
		strictEqual(
			verifySetupToken("not-the-token"),
			false,
			"proxy-forwarded caller guessing -> refused",
		);
		// ...and the permitted path (someone who read the host's file) still works:
		strictEqual(verifySetupToken(token), true);
	});

	test("a different host's token does not unlock this console", () => {
		freshConfigDir();
		const otherDir = mkdtempSync(join(tmpdir(), "ss-setup-token-other-"));
		dirs.push(otherDir);
		writeFileSync(join(otherDir, "web-setup-token"), `${"a".repeat(64)}\n`);
		const otherToken = readFileSync(join(otherDir, "web-setup-token"), "utf8").trim();
		strictEqual(verifySetupToken(otherToken), false);
		strictEqual(verifySetupToken(setupToken()), true);
	});

	test("the setup route no longer authorizes on peer address", () => {
		// Static regression guard: reintroducing `isLoopbackAddress` into the setup handler would
		// silently reopen the tailnet claim window on every proxied install.
		const source = readFileSync(join(__dirname, "..", "routes", "_auth", "setup.post.ts"), "utf8");
		strictEqual(source.includes("isLoopbackAddress"), false);
		strictEqual(source.includes("verifySetupToken"), true);
	});
});

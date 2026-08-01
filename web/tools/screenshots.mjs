// Capture marketing/console screenshots from the built Storybook.
//
// Mirrors the iOS harness (clients/apple/tools/screenshots.sh): one "scene" per
// story, a mock-populated REAL view, captured by the platform's own renderer —
// here headless Chromium over `storybook-static`. No display, GPU, login, or live
// mgmt backend: the page stories render entirely from fixtures (src/stories/lib).
//
//   bun run build-storybook            # produce ./storybook-static
//   node tools/screenshots.mjs         # → ./screenshots/<story-id>.png
//
// Env knobs: OUT (output dir), STORYBOOK_STATIC (input dir), SETTLE (ms after the
// page looks ready, default 600), WIDTH/HEIGHT/SCALE (desktop viewport, default
// 1440x900@2x), ONLY (comma-separated story-id substring filter), VIEWPORTS
// (desktop,mobile), THEMES (dark,light), and MOTIONS (full,reduced).

import { existsSync } from "node:fs";
import { mkdir, readFile } from "node:fs/promises";
import { createServer } from "node:http";
import { extname, join, normalize, resolve } from "node:path";
import { chromium } from "playwright";

const ROOT = resolve(process.env.STORYBOOK_STATIC ?? "storybook-static");
const OUT = resolve(process.env.OUT ?? "screenshots");
const SETTLE = envNumber("SETTLE", 600, 0);
const WIDTH = envNumber("WIDTH", 1440, 1, true);
const HEIGHT = envNumber("HEIGHT", 900, 1, true);
const SCALE = envNumber("SCALE", 2, 0.1);
const ONLY = (process.env.ONLY ?? "")
	.split(",")
	.map((s) => s.trim())
	.filter(Boolean);
const VIEWPORT_NAMES = parseList(
	process.env.VIEWPORTS ?? process.env.VIEWPORT ?? "desktop",
);
const THEMES = parseList(process.env.THEMES ?? process.env.THEME ?? "dark");
const MOTIONS = parseList(process.env.MOTIONS ?? process.env.MOTION ?? "full");

function envNumber(name, fallback, minimum, integer = false) {
	const raw = process.env[name];
	if (raw === undefined) return fallback;
	const value = Number(raw);
	if (
		!Number.isFinite(value) ||
		value < minimum ||
		(integer && !Number.isInteger(value))
	) {
		throw new Error(
			`${name} must be a ${integer ? "whole number" : "number"} >= ${minimum}`,
		);
	}
	return value;
}

function parseList(value) {
	return value
		.split(",")
		.map((s) => s.trim().toLowerCase())
		.filter(Boolean);
}

const VIEWPORTS = VIEWPORT_NAMES.map((name) => {
	const preset =
		name === "mobile"
			? { width: 390, height: 844, scale: SCALE }
			: name === "desktop"
				? { width: WIDTH, height: HEIGHT, scale: SCALE }
				: null;
	if (!preset) {
		throw new Error(`unknown viewport "${name}" (expected desktop or mobile)`);
	}
	return { name, ...preset };
});

for (const theme of THEMES) {
	if (theme !== "dark" && theme !== "light") {
		throw new Error(`unknown theme "${theme}" (expected dark or light)`);
	}
}
for (const motion of MOTIONS) {
	if (motion !== "full" && motion !== "reduced") {
		throw new Error(`unknown motion "${motion}" (expected full or reduced)`);
	}
}

const VARIANTS = VIEWPORTS.flatMap((viewport) =>
	THEMES.flatMap((theme) =>
		MOTIONS.map((motion) => ({ viewport, theme, motion })),
	),
);
if (VARIANTS.length === 0) {
	throw new Error("select at least one viewport, theme, and motion variant");
}

// Only the page-level + shell stories make sense as console screenshots — skip the
// component-library stories (Button, Badge, …).
const TITLE_PREFIXES = ["Pages/", "Shell/"];

const MIME = {
	".html": "text/html",
	".js": "text/javascript",
	".mjs": "text/javascript",
	".css": "text/css",
	".json": "application/json",
	".svg": "image/svg+xml",
	".png": "image/png",
	".jpg": "image/jpeg",
	".woff": "font/woff",
	".woff2": "font/woff2",
	".ttf": "font/ttf",
	".map": "application/json",
	".ico": "image/x-icon",
};

function staticServer(rootDir) {
	return createServer(async (req, res) => {
		try {
			const url = new URL(req.url, "http://localhost");
			let path = decodeURIComponent(url.pathname);
			if (path.endsWith("/")) path += "index.html";
			// Contain the path to rootDir (no traversal).
			const filePath = normalize(join(rootDir, path));
			if (!filePath.startsWith(rootDir)) {
				res.writeHead(403).end();
				return;
			}
			const body = await readFile(filePath);
			res.writeHead(200, {
				"content-type": MIME[extname(filePath)] ?? "application/octet-stream",
			});
			res.end(body);
		} catch {
			res.writeHead(404).end();
		}
	});
}

async function listStories(rootDir) {
	const indexPath = join(rootDir, "index.json");
	if (!existsSync(indexPath)) {
		throw new Error(
			`${indexPath} not found — run \`bun run build-storybook\` first`,
		);
	}
	const index = JSON.parse(await readFile(indexPath, "utf8"));
	const entries = Object.values(index.entries ?? index.stories ?? {});
	return entries
		.filter((e) => e.type === "story" || e.type === undefined)
		.filter((e) => TITLE_PREFIXES.some((p) => (e.title ?? "").startsWith(p)))
		.filter((e) => ONLY.length === 0 || ONLY.some((f) => e.id.includes(f)))
		.sort((a, b) => a.id.localeCompare(b.id));
}

async function waitForStableRender(page) {
	await page.waitForSelector("#storybook-root > *", {
		state: "visible",
		timeout: 20_000,
	});
	await page.evaluate(async () => {
		await document.fonts.ready;
		await Promise.all(
			Array.from(document.images, (image) => {
				if (image.complete) return Promise.resolve();
				return new Promise((resolve) => {
					const finish = () => {
						window.clearTimeout(timeout);
						resolve();
					};
					const timeout = window.setTimeout(finish, 10_000);
					image.addEventListener("load", finish, { once: true });
					image.addEventListener("error", finish, { once: true });
				});
			}),
		);
	});
	await page.waitForFunction(
		() => {
			const root = document.querySelector("#storybook-root");
			if (!root) return false;
			const rect = root.getBoundingClientRect();
			return (
				rect.width > 0 && rect.height > 0 && document.fonts.status === "loaded"
			);
		},
		undefined,
		{ timeout: 20_000 },
	);
	// Recharts mounts behind a client-only guard. Wait for it when this story has a chart before
	// checking the layout signature.
	const chart = page.locator(".recharts-surface").first();
	if (await chart.count()) {
		await chart.waitFor({ state: "visible", timeout: 4_000 }).catch(() => {});
	}
	await page.waitForFunction(
		({ sampleMs }) => {
			const root = document.querySelector("#storybook-root");
			if (!root) return false;
			const signature = () => {
				const rect = root.getBoundingClientRect();
				return [
					rect.x,
					rect.y,
					rect.width,
					rect.height,
					root.scrollWidth,
					root.scrollHeight,
				].join(":");
			};
			const first = signature();
			return new Promise((resolve) => {
				window.setTimeout(() => resolve(first === signature()), sampleMs);
			});
		},
		{ sampleMs: Math.max(50, Math.min(250, SETTLE)) },
		{ timeout: 20_000 },
	);
	await page.waitForTimeout(SETTLE);
}

function outputPath(storyId, variant) {
	const defaultVariant =
		VARIANTS.length === 1 &&
		variant.viewport.name === "desktop" &&
		variant.theme === "dark" &&
		variant.motion === "full";
	const suffix = defaultVariant
		? ""
		: `--${variant.viewport.name}-${variant.theme}-${variant.motion}`;
	return join(OUT, `${storyId}${suffix}.png`);
}

async function main() {
	if (!existsSync(ROOT)) {
		throw new Error(
			`${ROOT} not found — run \`bun run build-storybook\` first`,
		);
	}
	const stories = await listStories(ROOT);
	if (stories.length === 0)
		throw new Error("no Pages/* or Shell/* stories found");
	await mkdir(OUT, { recursive: true });

	const server = staticServer(ROOT);
	await new Promise((r) => server.listen(0, "127.0.0.1", r));
	const port = server.address().port;

	const browser = await chromium.launch({
		args: ["--force-color-profile=srgb"],
	});

	let ok = 0;
	for (const variant of VARIANTS) {
		const context = await browser.newContext({
			viewport: {
				width: variant.viewport.width,
				height: variant.viewport.height,
			},
			deviceScaleFactor: variant.viewport.scale,
			colorScheme: variant.theme,
			reducedMotion: variant.motion === "reduced" ? "reduce" : "no-preference",
		});
		for (const story of stories) {
			const page = await context.newPage();
			const globals = encodeURIComponent(
				`theme:${variant.theme};motion:${variant.motion}`,
			);
			const url = `http://127.0.0.1:${port}/iframe.html?id=${encodeURIComponent(
				story.id,
			)}&viewMode=story&globals=${globals}`;
			try {
				await page.goto(url, {
					waitUntil: "domcontentloaded",
					timeout: 30_000,
				});
				await page
					.waitForLoadState("networkidle", { timeout: 5_000 })
					.catch(() => {});
				await waitForStableRender(page);
				const file = outputPath(story.id, variant);
				await page.screenshot({
					path: file,
					animations: "disabled",
					caret: "hide",
				});
				console.log(
					`✓ ${story.id} [${variant.viewport.name}, ${variant.theme}, ${variant.motion}] → ${file}`,
				);
				ok++;
			} catch (e) {
				console.warn(
					`✗ ${story.id} [${variant.viewport.name}, ${variant.theme}, ${variant.motion}]: ${e.message}`,
				);
			} finally {
				await page.close();
			}
		}
		await context.close();
	}

	await browser.close();
	await new Promise((r) => server.close(r));
	const total = stories.length * VARIANTS.length;
	console.log(`\n${ok}/${total} captures written → ${OUT}`);
	if (ok === 0) process.exit(1);
}

main().catch((e) => {
	console.error(e);
	process.exit(1);
});

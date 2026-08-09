// `slipstream-host plugins …` package operations, run on the vendored bun. The host CLI forwards
// add/remove/list here (crates/slipstream-host/src/plugins.rs) and the runner-cli exposes them as
// subcommands. Everything a plugin needs to be installed — the plugins dir, the `@slipstream`
// registry scope in bunfig.toml, and the right bun — is handled here so the operator types one line
// instead of the old create-dir / write-bunfig / `bun add` ritual.
import * as fs from "node:fs";
import * as path from "node:path";
import { configDir } from "./config.js";

/**
 * Default `@slipstream` package registry for plugin installs (`bunfig.toml` scope map).
 * Operators override with `SLIPSTREAM_PLUGIN_REGISTRY` (GitHub Packages, a local verdaccio,
 * etc.). Placeholder default is GitHub Packages until a public Slipstream feed exists.
 */
export const REGISTRY =
	process.env.SLIPSTREAM_PLUGIN_REGISTRY?.trim() || "https://npm.pkg.github.com";

/** Where plugin packages install: `<config_dir>/plugins` (matches runner.ts discovery). */
export const pluginsDirDefault = (): string => path.join(configDir(), "plugins");

export interface ResolveOptions {
	/**
	 * Allow names that resolve on the PUBLIC npm registry (unscoped `slipstream-plugin-*`, foreign
	 * scopes, arbitrary paths). Off by default: only the `@slipstream` scope — pinned to
	 * [`REGISTRY`] by [`ensureBunfig`] — installs without it, so a typo or a squatted look-alike
	 * package can't silently pull operator-privileged code from npmjs.org (the CLI flag is
	 * `--allow-public-registry`).
	 */
	allowPublicRegistry?: boolean;
}

/**
 * Resolve a friendly plugin name to its npm package. A bare first-party name maps into the
 * `@slipstream` scope (`catalog-sync` maps to `@slipstream/plugin-catalog-sync`, `rom-manager` maps to
 * `@slipstream/plugin-rom-manager`); an `@slipstream/…` name is used verbatim. Anything else —
 * the unscoped `slipstream-plugin-…` convention, foreign scopes, registry paths — resolves on
 * the public registry and is refused unless [`ResolveOptions.allowPublicRegistry`] is set.
 */
export const resolvePackage = (
	name: string,
	opts: ResolveOptions = {},
): string => {
	const n = name.trim();
	if (!n) throw new Error("empty plugin name");
	if (!n.startsWith("@") && !n.includes("/") && !n.startsWith("slipstream-plugin-")) {
		return `@slipstream/plugin-${n}`; // bare first-party name
	}
	if (n.startsWith("@slipstream/")) return n; // first-party scope, pinned to our registry
	if (!opts.allowPublicRegistry) {
		throw new Error(
			`'${n}' would install from the PUBLIC npm registry, not Slipstream's. Plugins run ` +
				"with operator privileges - install only code you trust. If you mean it, re-run " +
				"with --allow-public-registry.",
		);
	}
	return n;
};

/** Does this resolved package name install from Slipstream's own scoped registry? */
const isFirstParty = (pkg: string): boolean => pkg.startsWith("@slipstream/");

/**
 * Create the plugins dir (and parents) if needed, and make it bun's install root.
 *
 * The `package.json` is load-bearing, not decoration: `bun add` installs into the nearest ancestor
 * `package.json`, not into its working directory. Without one here, a stray `~/package.json` — one
 * old `bun add`/`npm init` in a home dir — silently captures every plugin install. bun reports
 * success and exits 0, the packages land in that tree, and the plugins dir stays empty (reproduced
 * on-glass 2026-07-31; it presented as a plugin store that installs nothing).
 *
 * Only seeds a tree with no `node_modules`. A dir with packages but no `package.json` is
 * hand-assembled or an older layout, and both this module's [`listInstalled`] and the host's
 * installed-package scan fall back to the naming convention there; an empty `dependencies` would
 * make the host report every plugin already installed as gone.
 */
export const ensurePluginsDir = (dir = pluginsDirDefault()): string => {
	fs.mkdirSync(dir, { recursive: true });
	const manifest = path.join(dir, "package.json");
	if (!fs.existsSync(manifest) && !fs.existsSync(path.join(dir, "node_modules"))) {
		fs.writeFileSync(manifest, '{\n  "name": "slipstream-plugins",\n  "private": true\n}\n');
	}
	return dir;
};

/**
 * Ensure `<dir>/bunfig.toml` maps every scope we need to its registry, so `bun add` resolves
 * plugins from the right place. `@slipstream` → Slipstream's own registry is always mapped;
 * `extraScopes` adds others — a plugin-store catalog entry carries its own registry, and the scope
 * is what binds a package name to it (design D8, which is why catalog entries must be scoped).
 *
 * Idempotent and non-destructive: a scope already mapped to the same URL is left alone, a scope
 * mapped to a *different* URL is rewritten, and any unrelated bunfig content is preserved.
 */
export const ensureBunfig = (
	dir = pluginsDirDefault(),
	extraScopes: Record<string, string> = {},
): void => {
	const file = path.join(dir, "bunfig.toml");
	const wanted: Record<string, string> = { "@slipstream": REGISTRY, ...extraScopes };
	let existing = "";
	try {
		existing = fs.readFileSync(file, "utf8");
	} catch {
		// no bunfig yet — write a fresh one below
	}

	let out = existing;
	const missing: string[] = [];
	for (const [scope, url] of Object.entries(wanted)) {
		// Match `"@scope" = "…"` (quoted or bare key) anywhere in the file.
		const line = new RegExp(`^\\s*"?${escapeRe(scope)}"?\\s*=\\s*".*"\\s*$`, "m");
		const replacement = `"${scope}" = "${url}"`;
		if (line.test(out)) {
			const current = out.match(line)?.[0] ?? "";
			if (current.includes(`"${url}"`)) continue; // already correct
			out = out.replace(line, replacement);
		} else {
			missing.push(replacement);
		}
	}
	if (missing.length === 0) {
		if (out !== existing) fs.writeFileSync(file, out);
		return;
	}
	const block = missing.join("\n");
	if (!out.trim()) {
		fs.writeFileSync(file, `[install.scopes]\n${block}\n`);
	} else if (/^\[install\.scopes\][^\n]*$/m.test(out)) {
		// Insert under the existing table header.
		fs.writeFileSync(
			file,
			out.replace(/^\[install\.scopes\][^\n]*$/m, (m) => `${m}\n${block}`),
		);
	} else {
		const sep = out.endsWith("\n") ? "" : "\n";
		fs.writeFileSync(file, `${out}${sep}\n[install.scopes]\n${block}\n`);
	}
};

const escapeRe = (s: string): string => s.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");

export interface PkgOpts extends ResolveOptions {
	/** Plugins dir. Default `<config_dir>/plugins`. */
	dir?: string;
	/** Line sink for progress. Default stdout. */
	log?: (line: string) => void;
	/**
	 * Record the resolved version exactly (`bun add --exact`) instead of a caret range. The plugin
	 * store always sets this: a catalog entry pins one reviewed version, and a caret range in
	 * `package.json` would let a later `bun install` in this tree drift off it.
	 */
	exact?: boolean;
	/** Extra `scope → registry URL` mappings to write into `bunfig.toml` before installing. */
	registries?: Record<string, string>;
}

/** Run `bun add`/`bun remove` in the plugins dir on the current (vendored) bun. */
const runBun = (action: "add" | "remove", pkgs: string[], opts: PkgOpts): void => {
	const dir = opts.dir ?? pluginsDirDefault();
	const log = opts.log ?? ((l: string) => console.log(l));
	ensurePluginsDir(dir);
	if (action === "add") ensureBunfig(dir, opts.registries);
	log(`${action === "add" ? "installing" : "removing"} ${pkgs.join(", ")} in ${dir}`);
	// `process.execPath` is the bun running this file (the vendored one under the package), so a
	// system-wide bun on PATH is not required. Inherit stdio so `bun`'s progress reaches the user.
	const args = [process.execPath, action, ...pkgs];
	if (action === "add") {
		// NEVER run install lifecycle scripts. A plugin is code we chose to run under the runner,
		// and a postinstall script runs immediately with the installer's privileges. bun already
		// declines untrusted scripts by default; this makes it explicit and unconditional. A plugin
		// that needs a native build step is a review rejection, not a case to support.
		args.push("--ignore-scripts");
		if (opts.exact) args.push("--exact");
	}
	const res = Bun.spawnSync(args, {
		cwd: dir,
		stdio: ["inherit", "inherit", "inherit"],
	});
	if (!res.success) {
		throw new Error(`bun ${action} exited ${res.exitCode ?? "?"} — see output above`);
	}
};

/** Install one or more plugins by friendly name or package. */
export const addPlugins = (names: string[], opts: PkgOpts = {}): void => {
	const pkgs = names.map((n) => resolvePackage(n, opts));
	const log = opts.log ?? ((l: string) => console.log(l));
	for (const pkg of pkgs.filter((p) => !isFirstParty(p))) {
		log(
			`[plugins] WARNING: ${pkg} installs from the public npm registry - it is not ` +
				"published by Slipstream. It will run with operator privileges.",
		);
	}
	runBun("add", pkgs, opts);
};

/** Uninstall one or more plugins by friendly name or package. Removal is always safe — a name
 * never gates on the registry it once came from. */
export const removePlugins = (names: string[], opts: PkgOpts = {}): void =>
	runBun(
		"remove",
		names.map((n) => resolvePackage(n, { allowPublicRegistry: true })),
		opts,
	);

export interface InstalledPlugin {
	/** npm package name, e.g. `@slipstream/plugin-catalog-sync` or `slipstream-plugin-foo`. */
	pkg: string;
	/** Installed version from the package's package.json, if readable. */
	version?: string;
}

/**
 * Enumerate installed plugin packages under `<dir>/node_modules` — the unscoped convention
 * (`slipstream-plugin-*`) and **any** scope's `plugin-*` (`@slipstream/plugin-rom-manager`,
 * `@retro-hub/plugin-x`). Mirrors the discovery in runner.ts so `list` shows exactly what the
 * runner would supervise.
 */
export const listInstalled = (dir = pluginsDirDefault()): InstalledPlugin[] => {
	const modules = path.join(dir, "node_modules");
	const out: InstalledPlugin[] = [];
	const versionOf = (pkgDir: string): string | undefined => {
		try {
			const m = JSON.parse(
				fs.readFileSync(path.join(pkgDir, "package.json"), "utf8"),
			) as { version?: string };
			return m.version;
		} catch {
			return undefined;
		}
	};
	let entries: string[];
	try {
		entries = fs.readdirSync(modules).sort();
	} catch {
		return out; // no plugins installed yet
	}
	for (const entry of entries) {
		if (entry.startsWith("slipstream-plugin-")) {
			out.push({ pkg: entry, version: versionOf(path.join(modules, entry)) });
		} else if (entry.startsWith("@")) {
			let scoped: string[] = [];
			try {
				scoped = fs.readdirSync(path.join(modules, entry)).sort();
			} catch {
				scoped = [];
			}
			for (const s of scoped) {
				if (s.startsWith("plugin-")) {
					out.push({
						pkg: `${entry}/${s}`,
						version: versionOf(path.join(modules, entry, s)),
					});
				}
			}
		}
	}
	return out;
};

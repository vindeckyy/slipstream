import * as nodeHttp from "node:http";
import * as nodeHttps from "node:https";
import { fileURLToPath } from "node:url";
import { paraglideVitePlugin } from "@inlang/paraglide-js";
import tailwindcss from "@tailwindcss/vite";
import { nitroV2Plugin } from "@tanstack/nitro-v2-vite-plugin";
import { tanstackStart } from "@tanstack/react-start/plugin/vite";
import viteReact from "@vitejs/plugin-react";
import {
	createApp,
	createError,
	createRouter,
	defineEventHandler,
	getRequestHeader,
	useSession as getSession,
	toNodeListener,
} from "h3";
import { defineConfig, loadEnv, type Plugin } from "vite";
import viteTsConfigPaths from "vite-tsconfig-paths";
import loginHandler from "./server/routes/_auth/login.post";
import logoutHandler from "./server/routes/_auth/logout.post";
import setupHandler from "./server/routes/_auth/setup.post";
import {
	type SessionData,
	sessionConfig,
	sessionEpoch,
	uiPassword,
} from "./server/util/auth";

// Absolute path to our Nitro server source (middleware + routes). Passed as a scanDir
// because the TanStack Nitro plugin doesn't auto-scan a server/ dir.
const serverDir = fileURLToPath(new URL("./server", import.meta.url));

// Dev-only `/plugin-ui/<id>/**` reverse proxy — the vite-dev counterpart of the Bun/Nitro route
// (server/routes/plugin-ui/[...].ts), which can't run in dev because it uses Bun's `tls` fetch
// option. Same contract: look up the plugin's {port, secret} from the management API server-side,
// inject the secret, strip the cookie, dial 127.0.0.1 only, stream the response (SSE included).
// Needs SLIPSTREAM_MGMT_TOKEN in the dev environment (like talking to any token-required host).
function pluginUiDevProxy(
	mgmtUrl: string,
	mgmtToken: string | undefined,
): Plugin {
	const fetchCred = (
		id: string,
		token: string,
	): Promise<{ port: number; secret: string } | null> =>
		new Promise((resolve) => {
			const u = new URL(`${mgmtUrl}/api/v1/plugins/${id}/ui-credential`);
			const mod = u.protocol === "https:" ? nodeHttps : nodeHttp;
			const r = mod.request(
				u,
				{
					method: "GET",
					headers: { authorization: `Bearer ${token}` },
					rejectUnauthorized: false, // host's self-signed loopback cert
				} as nodeHttps.RequestOptions,
				(resp) => {
					let data = "";
					resp.on("data", (c) => {
						data += c;
					});
					resp.on("end", () => {
						if (resp.statusCode === 200) {
							try {
								resolve(JSON.parse(data));
							} catch {
								resolve(null);
							}
						} else resolve(null);
					});
				},
			);
			r.on("error", () => resolve(null));
			r.end();
		});

	return {
		name: "slipstream-plugin-ui-dev-proxy",
		configureServer(server) {
			server.middlewares.use("/plugin-ui", async (req, res) => {
				const raw = req.url ?? "/"; // connect strips the /plugin-ui mount prefix
				const m = raw.match(/^\/([a-z][a-z0-9-]*)(\/[^?]*)?(\?.*)?$/);
				const id = m?.[1];
				if (!id) {
					res.statusCode = 404;
					res.end("bad plugin-ui path");
					return;
				}
				const rest = m?.[2] ?? "/";
				const search = m?.[3] ?? "";
				const token = mgmtToken;
				if (!token) {
					res.statusCode = 503;
					res.end("dev plugin-ui proxy: set SLIPSTREAM_MGMT_TOKEN");
					return;
				}
				const cred = await fetchCred(id, token);
				if (!cred) {
					res.statusCode = 502;
					res.end(`plugin "${id}" is not running`);
					return;
				}
				const headers = { ...req.headers } as Record<string, string | string[]>;
				delete headers.host;
				delete headers.cookie;
				delete headers.authorization;
				headers["x-forwarded-prefix"] = `/plugin-ui/${id}`;
				const proxyReq = nodeHttp.request(
					{
						host: "127.0.0.1",
						port: cred.port,
						method: req.method,
						path: rest + search,
						headers: { ...headers, authorization: `Bearer ${cred.secret}` },
					},
					(pr) => {
						res.statusCode = pr.statusCode ?? 502;
						for (const [k, v] of Object.entries(pr.headers)) {
							if (v !== undefined) res.setHeader(k, v);
						}
						pr.pipe(res); // stream (SSE included)
					},
				);
				proxyReq.on("error", () => {
					res.statusCode = 502;
					res.end("plugin unreachable");
				});
				req.pipe(proxyReq);
			});
		},
	};
}

/**
 * Vite dev does not run Nitro's scanned `server/` routes, so mount the auth handlers explicitly.
 * Without this bridge the dev login form posts to a 404, which looks like a bad password even
 * though the production Bun bundle has the routes.
 */
function authDevMiddleware(): Plugin {
	const withCsrf = (
		handler: Parameters<typeof defineEventHandler>[0],
		requiresSession = false,
	) =>
		defineEventHandler(async (event) => {
			const site = getRequestHeader(event, "sec-fetch-site")?.toLowerCase();
			if (site && site !== "same-origin" && site !== "none") {
				throw createError({
					statusCode: 403,
					statusMessage: "cross-site request refused",
				});
			}
			if (requiresSession) {
				if (!uiPassword()) {
					throw createError({
						statusCode: 503,
						statusMessage: "auth not configured",
					});
				}
				const session = await getSession<SessionData>(event, sessionConfig());
				if (
					!session.data.authenticated ||
					session.data.epoch !== sessionEpoch()
				) {
					throw createError({
						statusCode: 401,
						statusMessage: "unauthorized",
					});
				}
			}
			return handler(event);
		});

	return {
		name: "slipstream-auth-dev",
		configureServer(server) {
			const router = createRouter()
				.post("/login", withCsrf(loginHandler))
				.post("/setup", withCsrf(setupHandler))
				.post("/logout", withCsrf(logoutHandler, true));
			const app = createApp();
			app.use(router.handler);
			server.middlewares.use("/_auth", toNodeListener(app));
			server.middlewares.use((req, res, next) => {
				if (req.method !== "GET" && req.method !== "HEAD") {
					next();
					return;
				}
				const url = new URL(req.url ?? "/", "http://slipstream.local");
				const { pathname } = url;
				// Let Vite handle assets, HMR, the API proxy, and source modules.
				if (
					pathname.startsWith("/api") ||
					pathname.startsWith("/_auth") ||
					pathname.startsWith("/@") ||
					pathname.startsWith("/src/") ||
					pathname.startsWith("/node_modules/") ||
					pathname.startsWith("/assets/") ||
					pathname.startsWith("/__") ||
					pathname.includes(".")
				) {
					next();
					return;
				}
				const configured = Boolean(uiPassword());
				if (pathname === "/setup") {
					if (configured) res.writeHead(302, { Location: "/login" }).end();
					else next();
					return;
				}
				if (pathname === "/login") {
					if (configured) next();
					else res.writeHead(302, { Location: `/setup${url.search}` }).end();
					return;
				}
				if (!configured) {
					res
						.writeHead(302, {
							Location: `/setup?next=${encodeURIComponent(pathname)}`,
						})
						.end();
					return;
				}
				next();
			});
		},
	};
}

/**
 * Drop @unom/ui's game-UI sound sprites from the build.
 *
 * `@unom/ui/button` pulls in `sound/defaults.js`, which resolves two sprite sheets with
 * `new URL(…, import.meta.url)` at module scope — a 4.8 MB .wav and a 2.2 MB .mp3. Vite therefore
 * emits both into `.output/public/assets/`, where they were ~7 MB of an 8.2 MB asset payload, and
 * they ride along into the Windows installer and the .deb.
 *
 * The console never mounts `UnomProviders`, so no sound player is bundled and not one byte of that
 * can ever be played. Stub the two files to an empty URL instead of shipping them.
 *
 * If the console ever DOES want click sounds, delete this plugin — that is the whole revert.
 */
function dropUnomSoundSprites(): Plugin {
	// The module that names them, and the `new URL(<sprite>, import.meta.url).href` expressions
	// inside it. Rewriting the EXPRESSION is what works: Vite emits these assets from its own
	// `new URL(…, import.meta.url)` transform, so intercepting the .wav/.mp3 module id never fires.
	const DEFAULTS = /@unom[\\/]ui[\\/].*sound[\\/]defaults\.(?:js|mjs)$/;
	const SPRITE_URL =
		/new URL\(\s*(["'])[^"']*\.(?:wav|mp3)\1\s*,\s*import\.meta\.url\s*\)\.href/g;
	return {
		name: "slipstream-drop-unom-sound-sprites",
		enforce: "pre",
		transform(code, id) {
			if (!DEFAULTS.test(id)) return null;
			const out = code.replace(SPRITE_URL, '""');
			return out === code ? null : { code: out, map: null };
		},
	};
}

export default defineConfig(({ mode }) => {
	const env = loadEnv(mode, process.cwd(), "");
	const mgmtUrl =
		env.SLIPSTREAM_MGMT_URL ||
		process.env.SLIPSTREAM_MGMT_URL ||
		"https://127.0.0.1:47990";
	const mgmtToken =
		env.SLIPSTREAM_MGMT_TOKEN ||
		process.env.SLIPSTREAM_MGMT_TOKEN ||
		undefined;
	if (env.SLIPSTREAM_UI_PASSWORD && !process.env.SLIPSTREAM_UI_PASSWORD) {
		process.env.SLIPSTREAM_UI_PASSWORD = env.SLIPSTREAM_UI_PASSWORD;
	}
	if (!mgmtToken) {
		console.warn(
			"[slipstream-web] SLIPSTREAM_MGMT_TOKEN unset — /api proxy will 401. Put the host token in web/.env.local",
		);
	}

	return {
		server: {
			proxy: {
				// `secure: false`: the host serves its own self-signed identity cert on loopback.
				"/api": {
					target: mgmtUrl,
					changeOrigin: true,
					secure: false,
					// Inject the management bearer, exactly as the deployed BFF does
					// (server/routes/api/[...].ts). The host requires a token on every route now, so
					// without this `bun run dev` 401s on every call and `apiFetch` bounces the developer
					// to /login — where logging in doesn't help, because dev has no login gate at all.
					configure(proxy) {
						const token = mgmtToken;
						if (!token) return;
						proxy.on("proxyReq", (proxyReq) => {
							proxyReq.setHeader("authorization", `Bearer ${token}`);
						});
					},
				},
			},
		},
		plugins: [
			authDevMiddleware(),
			// First, so it intercepts /plugin-ui before the SSR catch-all in dev.
			pluginUiDevProxy(mgmtUrl, mgmtToken),
			dropUnomSoundSprites(),
			viteTsConfigPaths({ projects: ["./tsconfig.json"] }),
			tailwindcss(),
			paraglideVitePlugin({
				project: "./project.inlang",
				outdir: "./src/paraglide",
				strategy: ["localStorage", "preferredLanguage", "baseLocale"],
			}),
			// Full SSR on the TanStack Start runtime (the management console's data queries run
			// client-side after hydration — React Query doesn't fetch during SSR — so the server
			// renders a data-free shell that hydrates in the browser).
			tanstackStart(),
			// Nitro v2 is the deployment target: the `bun` preset bundles a Bun-runnable server to
			// .output/ (`bun run .output/server/index.mjs`). Auth + the /api proxy live in the
			// scanned `server/` dir (middleware/auth.ts gates every request; routes/api/[...].ts
			// proxies to the management host injecting the bearer token server-side) — NOT a static
			// routeRule, so the proxy runs behind the login gate and reads env at runtime.
			nitroV2Plugin({
				// bun + a CUSTOM entry: Nitro's `bun` preset bundles the handler, and `entry` swaps the
				// stock self-listening entry for ours (`nitro-entry/bun-https.mjs`), which calls
				// `Bun.serve({ tls })` so the console is served over HTTPS (HTTP/1.1 over TLS) with the
				// host's own identity cert. (No HTTP/2 — Bun.serve has no h2 server — and no HTTP/3, which a
				// browser won't speak against this self-signed, no-SAN host cert.) Bun is the runtime
				// everywhere now — the Windows installer already bundles it, and the slipstream-web .deb
				// vendors it (it can't be `node`: `Bun.serve` is a bun API). (dev `vite dev` is unaffected.)
				preset: "bun",
				entry: fileURLToPath(
					new URL("./nitro-entry/bun-https.mjs", import.meta.url),
				),
				// BUNDLE every dependency into the server output (no externalized node_modules). Three wins:
				// (1) the .output tree drops from ~47k files / 730 MB (the whole untree-shaken @unom/ui dep
				// tree — payload, lexical, date-fns…) to a handful of tree-shaken chunks; (2) the output is a
				// self-contained ~75-file `.output` the bundled `bun` runs directly (the Windows installer
				// ships bun + that `.output`, not node + a node_modules forest); (3) it removes the
				// bare external imports (`srvx`, `seroval`…) bun couldn't resolve at runtime — the reason we
				// used to need node. node still runs the same self-contained output for the Linux .deb.
				noExternals: true,
				compatibilityDate: "2026-06-10",
				// Scan server/{middleware,routes} for the auth gate + the /api proxy.
				scanDirs: [serverDir],
			}),
			// Must come AFTER tanstackStart — provides the React JSX transform + Refresh runtime
			// that Start's dev mode requires (omitting it leaves the client JS unable to load).
			viteReact(),
		],
	};
});

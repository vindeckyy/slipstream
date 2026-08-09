// /plugin-ui/<id>/** is an opaque-origin iframe surface. The authenticated parent first probes
// __health with a capability, then every iframe request carries that capability in its path. The
// proxy looks up the plugin's {port, secret} server-side, injects the secret as a bearer, strips
// browser credentials, and streams the response through. The plugin is dialed on 127.0.0.1 only.
//
// This route runs in the built Bun/Nitro server. In `vite dev` a small middleware in vite.config.ts
// handles `/plugin-ui` instead (it intercepts before this route, like the /api dev proxy).
import {
	defineEventHandler,
	getProxyRequestHeaders,
	getRequestHeader,
	getRequestURL,
	readRawBody,
	sendWebResponse,
	setResponseStatus,
	type H3Event,
} from "h3";
import { isPluginUiEmbedPath, sessionEpoch } from "../../util/auth";
import {
	bustCredential,
	fetchUiCredential,
	PLUGIN_ID_RE,
} from "../../util/pluginProxy";

export const PLUGIN_UI_CAPABILITY_HEADER =
	"x-slipstream-plugin-ui-capability";
const PLUGIN_UI_CAPABILITY_RE = /^[0-9a-f]{64}$/;
const EMBED_SEGMENT = "/_embed/";
const CAPABILITY_TTL_MS = 15 * 60 * 1000;
const MAX_CAPABILITIES = 256;
const capabilities = new Map<string, { epoch: number; expiresAt: number }>();

export interface ParsedPluginUiPath {
	id: string;
	rest: string;
	capability: string | null;
	prefix: string;
}

export function parsePluginUiPath(pathname: string): ParsedPluginUiPath | null {
	const match = pathname.match(/^\/plugin-ui\/([^/]+)(\/.*)?$/);
	const id = match?.[1];
	if (!id || !PLUGIN_ID_RE.test(id)) return null;

	const suffix = match?.[2] ?? "/";
	const embed = suffix.match(
		new RegExp(`^${EMBED_SEGMENT}([0-9a-f]{64})(\/.*)?$`),
	);
	if (embed) {
		const capability = embed[1];
		if (!capability) return null;
		return {
			id,
			rest: embed[2] ?? "/",
			capability,
			prefix: `/plugin-ui/${id}${EMBED_SEGMENT}${capability}`,
		};
	}
	if (suffix.startsWith(EMBED_SEGMENT)) return null;
	return { id, rest: suffix, capability: null, prefix: `/plugin-ui/${id}` };
}

function capabilityKey(id: string, capability: string): string {
	return `${id}\0${capability}`;
}

function pruneCapabilities(now: number): void {
	for (const [key, entry] of capabilities) {
		if (entry.expiresAt <= now) capabilities.delete(key);
	}
}

function rememberCapability(id: string, capability: string): void {
	const now = Date.now();
	pruneCapabilities(now);
	capabilities.delete(capabilityKey(id, capability));
	while (capabilities.size >= MAX_CAPABILITIES) {
		const oldest = capabilities.keys().next().value as string | undefined;
		if (!oldest) break;
		capabilities.delete(oldest);
	}
	capabilities.set(capabilityKey(id, capability), {
		epoch: sessionEpoch(),
		expiresAt: now + CAPABILITY_TTL_MS,
	});
}

function hasCapability(id: string, capability: string): boolean {
	const key = capabilityKey(id, capability);
	const entry = capabilities.get(key);
	if (!entry || entry.epoch !== sessionEpoch()) return false;
	const now = Date.now();
	if (entry.expiresAt <= now) {
		capabilities.delete(key);
		return false;
	}
	entry.expiresAt = now + CAPABILITY_TTL_MS;
	return true;
}

export default defineEventHandler(async (event) => {
	const { pathname, search } = getRequestURL(event);
	const parsed = parsePluginUiPath(pathname);
	if (!parsed) {
		setResponseStatus(event, 404);
		return { error: "not a valid plugin-ui path" };
	}
	const { id, rest, capability, prefix } = parsed;
	const opaqueOrigin = capability !== null;
	const origin = getRequestHeader(event, "origin");
	if (opaqueOrigin) {
		if (!isPluginUiEmbedPath(pathname) || (origin && origin !== "null")) {
			return sendWebResponse(event, errorResponse(403, "plugin UI origin refused", true));
		}
		if (!hasCapability(id, capability)) {
			return sendWebResponse(event, errorResponse(404, "plugin UI capability expired", true));
		}
	}

	const requestedCapability = getRequestHeader(
		event,
		PLUGIN_UI_CAPABILITY_HEADER,
	)?.trim();
	if (
		requestedCapability &&
		(!PLUGIN_UI_CAPABILITY_RE.test(requestedCapability) ||
			pathname !== `/plugin-ui/${id}/__health`)
	) {
		return sendWebResponse(event, errorResponse(400, "invalid plugin UI capability", false));
	}
	if (!capability && rest !== "/__health") {
		return sendWebResponse(event, errorResponse(403, "plugin UI embedding requires a capability", false));
	}

	if (opaqueOrigin && event.method === "OPTIONS") {
		return sendWebResponse(event, preflightResponse(event));
	}

	// Forwardable request headers (h3 strips hop-by-hop + host); we set our own auth and drop the
	// session cookie and capability so plugin code never sees either browser credential.
	const headers = getProxyRequestHeaders(event) as Record<string, string>;
	for (const key of Object.keys(headers)) {
		if (
			["cookie", "authorization", PLUGIN_UI_CAPABILITY_HEADER].includes(
				key.toLowerCase(),
			)
		) {
			delete headers[key];
		}
	}
	headers["x-forwarded-prefix"] = prefix;
	const method = event.method;
	// Only read a body for the methods that can carry one. `readRawBody` asserts a payload method,
	// so calling it for OPTIONS (a plugin UI's CORS preflight, or any client probing Allow) threw
	// 405 out of the CONSOLE before the plugin was ever dialed.
	const body = BODY_METHODS.has(method)
		? ((await readRawBody(event, false)) as Uint8Array | undefined)
		: undefined;

	// One proxied attempt; `null` means the plugin is unreachable (unregistered, or its port died).
	const attempt = async (bustCache: boolean): Promise<Response | null> => {
		const cred = await fetchUiCredential(id, { bustCache });
		if (!cred) return null;
		const target = `http://127.0.0.1:${cred.port}${rest}${search}`;
		try {
			return await fetch(target, {
				method,
				headers: { ...headers, authorization: `Bearer ${cred.secret}` },
				body: body as BodyInit | undefined,
				redirect: "manual",
			});
		} catch {
			// The port is dead (plugin crashed/restarted on a new port): drop the stale credential so
			// the next request re-resolves it.
			bustCredential(id);
			return null;
		}
	};

	let resp = await attempt(false);
	// Stale secret after a plugin restart (S7): the plugin rejects our cached secret — re-fetch once.
	if (resp?.status === 401) {
		const retry = await attempt(true);
		if (retry) resp = retry;
	}
	if (!resp) {
		setResponseStatus(event, 502);
		return { error: `plugin "${id}" is not running` };
	}
	if (requestedCapability && resp.ok) rememberCapability(id, requestedCapability);
	return sendWebResponse(event, sanitize(resp, opaqueOrigin, rest, prefix));
});

/** Methods that may carry a request body. Anything else (GET, HEAD, OPTIONS, TRACE) must not be
 * handed to `readRawBody`. */
const BODY_METHODS = new Set(["POST", "PUT", "PATCH", "DELETE"]);

/**
 * Rebuild a plugin's response before it goes out on the console's own origin.
 *
 * Only content headers needed by a plugin page are forwarded. Security, cookie, and framing
 * headers belong to the console. Plugin redirects are rewritten into the capability path below.
 */
const PLUGIN_HEADER_ALLOWLIST = new Set([
	"content-type",
	"cache-control",
	"etag",
	"last-modified",
	"expires",
	"vary",
	"content-language",
	"content-disposition",
	"accept-ranges",
	"content-range",
	"link", // preload hints for its own assets
]);

function sanitize(
	resp: Response,
	opaqueOrigin: boolean,
	rest: string,
	prefix: string,
): Response {
	const headers = new Headers();
	for (const [k, v] of resp.headers) {
		if (PLUGIN_HEADER_ALLOWLIST.has(k.toLowerCase())) headers.set(k, v);
	}
	const location = resp.headers.get("location");
	const safeLocation = location && rewritePluginLocation(location, rest, prefix);
	if (safeLocation) headers.set("location", safeLocation);
	headers.set(
		"content-security-policy",
		"sandbox allow-scripts allow-forms allow-popups allow-modals; object-src 'none'; base-uri 'none'",
	);
	headers.set("x-content-type-options", "nosniff");
	headers.set("referrer-policy", "no-referrer");
	if (opaqueOrigin) {
		for (const [key, value] of corsHeaders().entries()) headers.set(key, value);
		headers.set("cache-control", "no-store");
	} else if (rest === "/__health") {
		headers.set("cache-control", "no-store");
	}
	// 204/304 must not carry a body — passing one through throws in the Response constructor.
	const bodyless = resp.status === 204 || resp.status === 304;
	return new Response(bodyless ? null : resp.body, {
		status: resp.status,
		statusText: resp.statusText,
		headers,
	});
}

function rewritePluginLocation(
	location: string,
	rest: string,
	prefix: string,
): string | null {
	try {
		const base = new URL(`http://plugin.invalid${rest}`);
		const target = new URL(location, base);
		if (target.origin !== base.origin) return null;
		return `${prefix}${target.pathname}${target.search}${target.hash}`;
	} catch {
		return null;
	}
}

const CORS_METHODS = "GET, HEAD, POST, PUT, PATCH, DELETE, OPTIONS";
const FORBIDDEN_CORS_HEADERS = new Set([
	"authorization",
	"cookie",
	"host",
	"origin",
	"x-forwarded-prefix",
	PLUGIN_UI_CAPABILITY_HEADER,
]);

function corsHeaders(requestedHeaders?: string): Headers {
	const headers = new Headers({
		"access-control-allow-origin": "null",
		"access-control-allow-methods": CORS_METHODS,
		"access-control-max-age": "300",
		vary: "Origin",
	});
	const allowed = requestedHeaders
		?.split(",")
		.map((header) => header.trim().toLowerCase())
		.filter(
			(header) =>
				/^[!#$%&'*+.^_`|~0-9a-z-]+$/.test(header) &&
				!FORBIDDEN_CORS_HEADERS.has(header),
		)
		.join(", ");
	if (allowed) headers.set("access-control-allow-headers", allowed);
	return headers;
}

function preflightResponse(event: H3Event): Response {
	return new Response(null, {
		status: 204,
		headers: corsHeaders(getRequestHeader(event, "access-control-request-headers")),
	});
}

function errorResponse(status: number, message: string, opaqueOrigin: boolean): Response {
	const headers = new Headers({ "content-type": "application/json; charset=utf-8" });
	if (opaqueOrigin) {
		for (const [key, value] of corsHeaders().entries()) headers.set(key, value);
	}
	return new Response(JSON.stringify({ error: message }), { status, headers });
}

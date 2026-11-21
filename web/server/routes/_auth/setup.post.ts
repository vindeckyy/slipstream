// POST /_auth/setup {setupToken, password, confirmation} - create the console credential on first run.
// This route is gate-free only in the sense that no PASSWORD is needed: it requires the one-time
// setup token, a secret from the host's private config directory (see `setupToken` in util/auth.ts).
// The peer address is deliberately NOT used for authorization: behind `tailscale serve` or any
// loopback-terminating proxy every remote caller arrives as 127.0.0.1, so loopback proves nothing.
// The helper writes with exclusive create semantics, so a second visitor cannot replace the first
// password.
import {
	createError,
	defineEventHandler,
	getRequestHeader,
	readBody,
	setResponseHeader,
	useSession,
} from "h3";
import {
	configureUiPassword,
	MIN_UI_PASSWORD_LENGTH,
	peerAddress,
	type SessionData,
	sessionConfig,
	sessionEpoch,
	uiPassword,
	verifySetupToken,
} from "../../util/auth";
import {
	recordLoginFailure,
	recordLoginSuccess,
	throttleRetryAfterMs,
} from "../../util/loginThrottle";

export default defineEventHandler(async (event) => {
	if (uiPassword()) {
		throw createError({
			statusCode: 409,
			statusMessage: "console password is already configured",
		});
	}

	// Throttle bucket only. The peer address is never an authorization decision.
	const ip = peerAddress(event);
	const wait = throttleRetryAfterMs(ip);
	if (wait > 0) {
		setResponseHeader(event, "Retry-After", Math.ceil(wait / 1000));
		throw createError({
			statusCode: 429,
			statusMessage: "too many setup attempts - try again shortly",
		});
	}

	const body = await readBody<{
		password?: unknown;
		confirmation?: unknown;
		setupToken?: unknown;
	}>(event);

	// The token is checked BEFORE any password validation, so an unauthorized caller learns nothing
	// about the password rules and cannot use this route as an oracle.
	const submittedToken =
		typeof body?.setupToken === "string"
			? body.setupToken
			: (getRequestHeader(event, "x-slipstream-setup-token") ?? "");
	if (!verifySetupToken(submittedToken)) {
		recordLoginFailure(ip);
		throw createError({
			statusCode: 403,
			statusMessage:
				"first-time setup requires the setup token from the host's config directory (web-setup-token, or SLIPSTREAM_UI_SETUP_TOKEN)",
		});
	}
	const password = typeof body?.password === "string" ? body.password : "";
	const confirmation =
		typeof body?.confirmation === "string" ? body.confirmation : "";

	if (password.trim().length < MIN_UI_PASSWORD_LENGTH) {
		recordLoginFailure(ip);
		throw createError({
			statusCode: 400,
			statusMessage: `password must be at least ${MIN_UI_PASSWORD_LENGTH} characters`,
		});
	}
	if (/[\r\n]/.test(password)) {
		recordLoginFailure(ip);
		throw createError({
			statusCode: 400,
			statusMessage: "password contains an unsupported character",
		});
	}
	if (password !== confirmation) {
		recordLoginFailure(ip);
		throw createError({
			statusCode: 400,
			statusMessage: "passwords do not match",
		});
	}

	try {
		if (!configureUiPassword(password)) {
			throw createError({
				statusCode: 409,
				statusMessage: "console password is already configured",
			});
		}
	} catch (error) {
		if (error && typeof error === "object" && "statusCode" in error)
			throw error;
		throw createError({
			statusCode: 503,
			statusMessage: "could not save the console password",
		});
	}

	recordLoginSuccess(ip);
	const session = await useSession<SessionData>(event, sessionConfig());
	await session.update({ authenticated: true, epoch: sessionEpoch() });
	return { ok: true };
});

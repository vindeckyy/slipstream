// POST /_auth/setup {password, confirmation} - create the console credential on first run.
// This route is public only while the console has no configured password. The helper writes with
// exclusive create semantics, so a second first visitor cannot replace the first password.
import {
	createError,
	defineEventHandler,
	readBody,
	setResponseHeader,
	useSession,
} from "h3";
import {
	configureUiPassword,
	isLoopbackAddress,
	MIN_UI_PASSWORD_LENGTH,
	peerAddress,
	type SessionData,
	sessionConfig,
	sessionEpoch,
	uiPassword,
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

	const ip = peerAddress(event);
	if (!isLoopbackAddress(ip)) {
		throw createError({
			statusCode: 403,
			statusMessage: "first-time setup must be completed from the host",
		});
	}
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
	}>(event);
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

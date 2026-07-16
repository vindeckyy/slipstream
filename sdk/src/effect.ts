// `@slipstream/host/effect` — the Effect-native surface (RFC §7): the `SlipstreamHost` service
// tag + live layer, the wire schemas, typed errors, and Stream accessors. The package root
// (`@slipstream/host`) is the Promise facade; this entry is for plugins and Effect programs.
//
//   import { Effect, Stream } from "effect";
//   import { SlipstreamHost, SlipstreamHostLive, events } from "@slipstream/host/effect";
//
//   const program = Effect.gen(function* () {
//     yield* events().pipe(
//       Stream.filter((e) => e.kind === "stream.started"),
//       Stream.runForEach((e) => Effect.log(`stream started: ${e.stream.mode}`)),
//     );
//   });
//   Effect.runPromise(program.pipe(Effect.provide(SlipstreamHostLive())));
import { Effect, type Schema as S, Stream } from "effect";
import {
	EventStreamError,
	SlipstreamHost,
	type RequestError,
	type VersionSkew,
} from "./client.js";
import type { EventStreamOptions, SseFrame } from "./sse.js";
import type { HostEvent } from "./wire.js";

export {
	ApiError,
	AuthError,
	EventStreamError,
	layer,
	makeService,
	SlipstreamHost,
	type SlipstreamHostService,
	SlipstreamHostLive,
	type RequestError,
	SseAuthError,
	TransportError,
	VersionSkew,
} from "./client.js";
export { type ConnectOptions, configDir, resolveConfig } from "./config.js";
export type { EventStreamOptions, SseFrame } from "./sse.js";
export * from "./wire.js";
/** The generated REST wire schemas (orval `client: 'effect'` over `api/openapi.json`). */
export * as api from "./gen/schemas.js";

/** The decoded lifecycle-event stream of the ambient [`SlipstreamHost`]. */
export const events = (
	opts?: EventStreamOptions,
): Stream.Stream<HostEvent, EventStreamError, SlipstreamHost> =>
	Stream.unwrap(Effect.map(SlipstreamHost, (s) => s.events(opts)));

/** Every SSE frame verbatim (the `dropped` marker + unknown kinds included). */
export const eventsRaw = (
	opts?: EventStreamOptions,
): Stream.Stream<SseFrame, EventStreamError, SlipstreamHost> =>
	Stream.unwrap(Effect.map(SlipstreamHost, (s) => s.eventsRaw(opts)));

/** One management-API request under `/api/v1` on the ambient service. */
export const request = (
	method: string,
	path: string,
	body?: unknown,
): Effect.Effect<unknown, RequestError, SlipstreamHost> =>
	Effect.flatMap(SlipstreamHost, (s) => s.request(method, path, body));

/** GET + schema-validate on the ambient service (schemas from [`api`]). */
export const get = <A, I>(
	path: string,
	schema: S.Schema<A, I>,
): Effect.Effect<A, RequestError | VersionSkew, SlipstreamHost> =>
	Effect.flatMap(SlipstreamHost, (s) => s.get(path, schema));

import * as Data from "effect/Data"
import * as Effect from "effect/Effect"
import type { SchemaError } from "effect/Schema"
import * as Schema from "effect/Schema"
import * as Stream from "effect/Stream"
import * as Sse from "effect/unstable/encoding/Sse"
import * as HttpClient from "effect/unstable/http/HttpClient"
import * as HttpClientError from "effect/unstable/http/HttpClientError"
import * as HttpClientRequest from "effect/unstable/http/HttpClientRequest"
import * as HttpClientResponse from "effect/unstable/http/HttpClientResponse"
// non-recursive definitions
export type ApiActiveGpu = { readonly "backend": string, readonly "id": string, readonly "name": string, readonly "sessions": number, readonly "vendor": string }
export const ApiActiveGpu = Schema.Struct({ "backend": Schema.String.annotate({ "description": "The encode backend in use (`nvenc` | `amf` | `qsv` | `vaapi` | `software`)." }), "id": Schema.String.annotate({ "description": "Stable id matching an entry of `gpus` (empty for the CPU/software encoder)." }), "name": Schema.String, "sessions": Schema.Number.annotate({ "description": "Number of live encode sessions on it.", "format": "int32" }).check(Schema.isInt().annotate({ "expected": "an integer" })).check(Schema.isGreaterThanOrEqualTo(0).annotate({ "expected": "a value greater than or equal to 0" })), "vendor": Schema.String.annotate({ "description": "`nvidia` | `amd` | `intel` | `other`." }) }).annotate({ "description": "The GPU live sessions are encoding on right now.", "identifier": "ApiActiveGpu" })
export type ApiCodec = "h264" | "hevc" | "av1" | "pyrowave"
export const ApiCodec = Schema.Literals(["h264", "hevc", "av1", "pyrowave"]).annotate({ "description": "Video codec identifier. The wire token matches the codec's canonical name used across the\nstack (SDP/GameStream advertisement, the stats-capture `CaptureMeta.codec`, and the encoder's\n[`Codec::label`]) — notably `H.265` serializes as `\"hevc\"`, not `\"h265\"`, so the same codec\nreads identically on every console page.", "identifier": "ApiCodec" })
export type ApiDisplayInfo = { readonly "backend": string, readonly "client"?: string | null, readonly "display_index": number, readonly "expires_in_ms"?: number | null, readonly "group": number, readonly "identity_slot"?: number | null, readonly "mode": string, readonly "sessions": number, readonly "slot": number, readonly "state": string, readonly "topology": string, readonly "x": number, readonly "y": number }
export const ApiDisplayInfo = Schema.Struct({ "backend": Schema.String.annotate({ "description": "Backend name (`ss-vdisplay`, `kwin`, …)." }), "client": Schema.optionalKey(Schema.Union([Schema.String, Schema.Null]).annotate({ "description": "Short client label, when the owner tracks it." })), "display_index": Schema.Number.annotate({ "description": "This display's ordinal within its group, in acquire order (0-based).", "format": "int32" }).check(Schema.isInt().annotate({ "expected": "an integer" })).check(Schema.isGreaterThanOrEqualTo(0).annotate({ "expected": "a value greater than or equal to 0" })), "expires_in_ms": Schema.optionalKey(Schema.Union([Schema.Number.check(Schema.isInt().annotate({ "expected": "an integer" })).check(Schema.isGreaterThanOrEqualTo(0).annotate({ "expected": "a value greater than or equal to 0" })), Schema.Null]).annotate({ "description": "Milliseconds until a lingering display is torn down (absent when active/pinned).", "format": "int64" })), "group": Schema.Number.annotate({ "description": "Display group (shared desktop) id — several displays with the same group form one desktop (§6A).", "format": "int32" }).check(Schema.isInt().annotate({ "expected": "an integer" })).check(Schema.isGreaterThanOrEqualTo(0).annotate({ "expected": "a value greater than or equal to 0" })), "identity_slot": Schema.optionalKey(Schema.Union([Schema.Number.check(Schema.isInt().annotate({ "expected": "an integer" })).check(Schema.isGreaterThanOrEqualTo(0).annotate({ "expected": "a value greater than or equal to 0" })), Schema.Null]).annotate({ "description": "Stable per-client identity slot keying persistent config + manual layout (absent = shared/anonymous).", "format": "int32" })), "mode": Schema.String.annotate({ "description": "`WIDTHxHEIGHT@HZ`." }), "sessions": Schema.Number.annotate({ "description": "Live sessions holding the display.", "format": "int32" }).check(Schema.isInt().annotate({ "expected": "an integer" })).check(Schema.isGreaterThanOrEqualTo(0).annotate({ "expected": "a value greater than or equal to 0" })), "slot": Schema.Number.annotate({ "description": "Stable-enough id for the `/display/release` `slot` argument.", "format": "int64" }).check(Schema.isInt().annotate({ "expected": "an integer" })).check(Schema.isGreaterThanOrEqualTo(0).annotate({ "expected": "a value greater than or equal to 0" })), "state": Schema.String.annotate({ "description": "`active` | `lingering` | `pinned`." }), "topology": Schema.String.annotate({ "description": "Effective topology for this display's group (`extend` | `primary` | `exclusive`)." }), "x": Schema.Number.annotate({ "description": "Desktop-space top-left `x` (auto-row or the console's manual arrangement, §6.2).", "format": "int32" }).check(Schema.isInt().annotate({ "expected": "an integer" })), "y": Schema.Number.annotate({ "description": "Desktop-space top-left `y`.", "format": "int32" }).check(Schema.isInt().annotate({ "expected": "an integer" })) }).annotate({ "description": "One live or kept virtual display.", "identifier": "ApiDisplayInfo" })
export type ApiError = { readonly "error": string }
export const ApiError = Schema.Struct({ "error": Schema.String }).annotate({ "description": "Error envelope for every non-2xx response.", "identifier": "ApiError" })
export type ApiGpu = { readonly "id": string, readonly "name": string, readonly "vendor": string, readonly "vram_mb": number }
export const ApiGpu = Schema.Struct({ "id": Schema.String.annotate({ "description": "Stable identifier (`vendorid-deviceid-occurrence`, hex PCI ids) — pass to `setGpuPreference`.\nStable across reboots and driver updates, unlike an adapter index or LUID." }), "name": Schema.String.annotate({ "description": "Adapter/marketing name." }), "vendor": Schema.String.annotate({ "description": "`nvidia` | `amd` | `intel` | `other`." }), "vram_mb": Schema.Number.annotate({ "description": "Dedicated VRAM in MiB (0 where the platform doesn't expose it).", "format": "int64" }).check(Schema.isInt().annotate({ "expected": "an integer" })).check(Schema.isGreaterThanOrEqualTo(0).annotate({ "expected": "a value greater than or equal to 0" })) }).annotate({ "description": "One hardware GPU on the host (software/WARP adapters are never listed).", "identifier": "ApiGpu" })
export type ApiMonitorInfo = { readonly "connector": string, readonly "description": string, readonly "enabled": boolean, readonly "managed": boolean, readonly "mode": string, readonly "primary": boolean, readonly "scale": number, readonly "selected": boolean, readonly "x": number, readonly "y": number }
export const ApiMonitorInfo = Schema.Struct({ "connector": Schema.String.annotate({ "description": "Connector name (`DP-1`, `HDMI-A-2`) — the value `SLIPSTREAM_CAPTURE_MONITOR` takes." }), "description": Schema.String.annotate({ "description": "Human label for a picker (`make model`, else the connector)." }), "enabled": Schema.Boolean.annotate({ "description": "Driven right now. A disabled head is still listed, so it can be explained rather than missing." }), "managed": Schema.Boolean.annotate({ "description": "Best-effort: this is one of OUR virtual displays, not a real head (reliable on KWin only)." }), "mode": Schema.String.annotate({ "description": "`WIDTHxHEIGHT@HZ` of the current mode (size only when the refresh is unknown)." }), "primary": Schema.Boolean.annotate({ "description": "The compositor's primary/focused head." }), "scale": Schema.Number.annotate({ "description": "Logical scale factor.", "format": "double" }).check(Schema.isFinite().annotate({ "expected": "a finite number" })), "selected": Schema.Boolean.annotate({ "description": "True when `SLIPSTREAM_CAPTURE_MONITOR` currently names this monitor." }), "x": Schema.Number.annotate({ "description": "Desktop-space top-left — what makes a head identifiable when two share a size.", "format": "int32" }).check(Schema.isInt().annotate({ "expected": "an integer" })), "y": Schema.Number.annotate({ "format": "int32" }).check(Schema.isInt().annotate({ "expected": "an integer" })) }).annotate({ "description": "One physical monitor this host has, as the compositor reports it.", "identifier": "ApiMonitorInfo" })
export type ApiSelectedGpu = { readonly "id": string, readonly "name": string, readonly "source": string, readonly "vendor": string }
export const ApiSelectedGpu = Schema.Struct({ "id": Schema.String, "name": Schema.String, "source": Schema.String.annotate({ "description": "Why this GPU was selected: `preference` (the manual choice), `env`\n(`SLIPSTREAM_RENDER_ADAPTER`), `auto` (max dedicated VRAM / platform default), or\n`preference_missing` (a manual choice is set but that GPU is absent — auto-selected\ninstead so the host keeps streaming)." }), "vendor": Schema.String.annotate({ "description": "`nvidia` | `amd` | `intel` | `other`." }) }).annotate({ "description": "The GPU the **next** session's pipeline will be created on, and why. (A preference change\napplies to the next session; a running session keeps the GPU it opened on.)", "identifier": "ApiSelectedGpu" })
export type ApplyRequest = { readonly "force"?: boolean }
export const ApplyRequest = Schema.Struct({ "force": Schema.optionalKey(Schema.Boolean.annotate({ "description": "Proceed even while a streaming session is live (the stream will drop when the host\nrestarts — the console warns before sending this)." })) }).annotate({ "identifier": "ApplyRequest" })
export type ApprovePending = { readonly "name"?: string | null }
export const ApprovePending = Schema.Struct({ "name": Schema.optionalKey(Schema.Union([Schema.String, Schema.Null]).annotate({ "description": "Operator-chosen label for the device (defaults to the name it knocked with)." })) }).annotate({ "description": "Approve-pending-device request body. Send `{}` to keep the device's own name.", "identifier": "ApprovePending" })
export type ArmNativePairing = { readonly "fingerprint"?: string | null, readonly "ttl_secs"?: number | null }
export const ArmNativePairing = Schema.Struct({ "fingerprint": Schema.optionalKey(Schema.Union([Schema.String, Schema.Null]).annotate({ "description": "Optional: bind the window to ONE device fingerprint (hex SHA-256, e.g. from a pending knock).\nWhen set, only a pairing attempt from that fingerprint consumes the window — so an unpaired\nLAN peer can neither pair nor burn a window armed for a specific device (security-review #9).\nOmit for an unbound window (any device may use the PIN — trusted-LAN only)." })), "ttl_secs": Schema.optionalKey(Schema.Union([Schema.Number.check(Schema.isInt().annotate({ "expected": "an integer" })).check(Schema.isGreaterThanOrEqualTo(0).annotate({ "expected": "a value greater than or equal to 0" })), Schema.Null]).annotate({ "description": "Window length in seconds (default 120; clamped to 15–600).", "format": "int32" })) }).annotate({ "description": "Arm-native-pairing request body.", "identifier": "ArmNativePairing" })
export type Artwork = { readonly "header"?: string | null, readonly "hero"?: string | null, readonly "logo"?: string | null, readonly "portrait"?: string | null }
export const Artwork = Schema.Struct({ "header": Schema.optionalKey(Schema.Union([Schema.String, Schema.Null]).annotate({ "description": "Horizontal header (Steam `header.jpg`) — the universal fallback." })), "hero": Schema.optionalKey(Schema.Union([Schema.String, Schema.Null]).annotate({ "description": "Wide background (Steam `library_hero.jpg`)." })), "logo": Schema.optionalKey(Schema.Union([Schema.String, Schema.Null]).annotate({ "description": "Transparent title logo (Steam `logo.png`)." })), "portrait": Schema.optionalKey(Schema.Union([Schema.String, Schema.Null]).annotate({ "description": "Vertical capsule / poster (Steam `library_600x900.jpg`). Best for a grid." })) }).annotate({ "description": "Cover art for a title. All fields are URLs (the Steam CDN for Steam titles, user-supplied for\ncustom). The client prefers `portrait` for a grid and falls back to `header` when a title has\nno 600×900 capsule (common for older Steam apps).", "identifier": "Artwork" })
export type AudioVideoConfig = { readonly "audio_capture"?: string | null, readonly "audio_fec"?: boolean, readonly "audio_gain"?: number | null, readonly "capture_max_age_ms"?: number | null, readonly "capture_method"?: string | null, readonly "compositor"?: string | null, readonly "four_four_four"?: boolean, readonly "gamescope_hdr"?: boolean, readonly "gamescope_sdr_nits"?: number | null, readonly "gamescope_splash"?: boolean, readonly "headless_compositor"?: string | null, readonly "max_fps"?: number | null, readonly "pipewire_latency_ms"?: number | null, readonly "ten_bit"?: boolean, readonly "vdisplay_hz_mult"?: number, readonly "video_source"?: string | null }
export const AudioVideoConfig = Schema.Struct({ "audio_capture": Schema.optionalKey(Schema.Union([Schema.String, Schema.Null]).annotate({ "description": "Linux audio capture source (`SLIPSTREAM_STREAM_SINK`): `stream-sink` (default, a\nhost-owned sink apps play into) or `monitor` (record the default sink)." })), "audio_fec": Schema.optionalKey(Schema.Boolean.annotate({ "description": "Audio FEC over the native plane (`SLIPSTREAM_AUDIO_FEC`). Default on: RS parity over\ngroups of 5 ms Opus frames so a lost packet is rebuilt instead of clicking. Off only\nas an escape hatch." })), "audio_gain": Schema.optionalKey(Schema.Union([Schema.Number.check(Schema.isFinite().annotate({ "expected": "a finite number" })).check(Schema.isGreaterThanOrEqualTo(0).annotate({ "expected": "a value greater than or equal to 0" })).check(Schema.isLessThanOrEqualTo(4).annotate({ "expected": "a value less than or equal to 4" })), Schema.Null]).annotate({ "description": "Linear audio gain applied to captured samples (`SLIPSTREAM_AUDIO_GAIN`, default 1.0).\nFor quiet sources; 1.0 = unchanged, 0.5 = half, 2.0 = double.", "format": "float" })), "capture_max_age_ms": Schema.optionalKey(Schema.Union([Schema.Number.check(Schema.isInt().annotate({ "expected": "an integer" })).check(Schema.isGreaterThanOrEqualTo(1).annotate({ "expected": "a value greater than or equal to 1" })).check(Schema.isLessThanOrEqualTo(500).annotate({ "expected": "a value less than or equal to 500" })), Schema.Null]).annotate({ "description": "Capture-frame age threshold used by the Linux latency diagnostics.", "format": "int32" })), "capture_method": Schema.optionalKey(Schema.Union([Schema.String, Schema.Null]).annotate({ "description": "Desktop capture backend (`SLIPSTREAM_CAPTURE_METHOD`):\n`auto` | `portal` | `kwin` | `wlr` | `kms` | `x11` | `nvfbc`." })), "compositor": Schema.optionalKey(Schema.Union([Schema.String, Schema.Null]).annotate({ "description": "Virtual-display compositor preference (`SLIPSTREAM_COMPOSITOR`):\n`kwin` | `mutter` | `wlroots` | `hyprland` | `gamescope`." })), "four_four_four": Schema.optionalKey(Schema.Boolean.annotate({ "description": "Prefer 4:4:4 when supported (`SLIPSTREAM_444`). Default on." })), "gamescope_hdr": Schema.optionalKey(Schema.Boolean.annotate({ "description": "Gamescope HDR. Default on." })), "gamescope_sdr_nits": Schema.optionalKey(Schema.Union([Schema.Number.check(Schema.isInt().annotate({ "expected": "an integer" })).check(Schema.isGreaterThanOrEqualTo(1).annotate({ "expected": "a value greater than or equal to 1" })).check(Schema.isLessThanOrEqualTo(10000).annotate({ "expected": "a value less than or equal to 10000" })), Schema.Null]).annotate({ "description": "SDR luminance inside an HDR Gamescope session (`SLIPSTREAM_GAMESCOPE_SDR_NITS`).", "format": "int32" })), "gamescope_splash": Schema.optionalKey(Schema.Boolean.annotate({ "description": "Keep a bare Gamescope session painting during application startup.\nDefault on because a blank Gamescope session produces no capture buffers." })), "headless_compositor": Schema.optionalKey(Schema.Union([Schema.String, Schema.Null]).annotate({ "description": "Headless session spawner (`SLIPSTREAM_HEADLESS_COMPOSITOR`):\n`off` | `auto` | `labwc` | `krfb` | `gamescope`." })), "max_fps": Schema.optionalKey(Schema.Union([Schema.Number.check(Schema.isInt().annotate({ "expected": "an integer" })).check(Schema.isGreaterThanOrEqualTo(0).annotate({ "expected": "a value greater than or equal to 0" })), Schema.Null]).annotate({ "description": "Cap encode FPS (`SLIPSTREAM_MAX_FPS`).", "format": "int32" })), "pipewire_latency_ms": Schema.optionalKey(Schema.Union([Schema.Number.check(Schema.isInt().annotate({ "expected": "an integer" })).check(Schema.isGreaterThanOrEqualTo(1).annotate({ "expected": "a value greater than or equal to 1" })).check(Schema.isLessThanOrEqualTo(40).annotate({ "expected": "a value less than or equal to 40" })), Schema.Null]).annotate({ "description": "Requested PipeWire video-node latency in milliseconds. This is a scheduling hint, not a\nguarantee from the compositor.", "format": "int32" })), "ten_bit": Schema.optionalKey(Schema.Boolean.annotate({ "description": "Prefer 10-bit encode when the client asks (`SLIPSTREAM_10BIT`). Default on." })), "vdisplay_hz_mult": Schema.optionalKey(Schema.Number.annotate({ "description": "Virtual-display refresh multiplier (`SLIPSTREAM_VDISPLAY_HZ_MULT`), from 1x to 4x.", "format": "int32" }).check(Schema.isInt().annotate({ "expected": "an integer" })).check(Schema.isGreaterThanOrEqualTo(0).annotate({ "expected": "a value greater than or equal to 0" }))), "video_source": Schema.optionalKey(Schema.Union([Schema.String, Schema.Null]).annotate({ "description": "`virtual` | `portal` (`SLIPSTREAM_VIDEO_SOURCE`)." })) }).annotate({ "identifier": "AudioVideoConfig" })
export type AvailableCaptureMethod = { readonly "available": boolean, readonly "id": string, readonly "label": string }
export const AvailableCaptureMethod = Schema.Struct({ "available": Schema.Boolean.annotate({ "description": "Best-effort: env / binary / protocol looks usable on this host right now." }), "id": Schema.String.annotate({ "description": "Stable identifier (`\"auto\"` | `\"portal\"` | `\"kwin\"` | `\"wlr\"` | `\"kms\"` | `\"x11\"` | `\"nvfbc\"`)." }), "label": Schema.String.annotate({ "description": "Human-readable label for UIs." }) }).annotate({ "description": "A desktop-mirror capture method the host can use, and whether it's usable now.", "identifier": "AvailableCaptureMethod" })
export type AvailableCompositor = { readonly "available": boolean, readonly "default": boolean, readonly "id": string, readonly "label": string }
export const AvailableCompositor = Schema.Struct({ "available": Schema.Boolean.annotate({ "description": "Usable on this host right now: the live session's own compositor, or gamescope wherever\nits binary is installed." }), "default": Schema.Boolean.annotate({ "description": "True for the backend an `Auto` (unspecified) request resolves to right now." }), "id": Schema.String.annotate({ "description": "Stable identifier (`\"kwin\"` | `\"wlroots\"` | `\"mutter\"` | `\"gamescope\"`) — pass this to a\nclient's `--compositor` flag." }), "label": Schema.String.annotate({ "description": "Human-readable label for UIs." }) }).annotate({ "description": "A compositor backend the host can drive a virtual output on, and whether it's usable now.", "identifier": "AvailableCompositor" })
export type AvailableHeadlessCompositor = { readonly "available": boolean, readonly "id": string, readonly "label": string }
export const AvailableHeadlessCompositor = Schema.Struct({ "available": Schema.Boolean.annotate({ "description": "True when the matching binary is on `PATH`." }), "id": Schema.String.annotate({ "description": "Stable identifier (`\"auto\"` | `\"labwc\"` | `\"krfb\"` | `\"gamescope\"`)." }), "label": Schema.String.annotate({ "description": "Human-readable label for UIs." }) }).annotate({ "description": "A headless compositor the host can spawn when no live session is present.", "identifier": "AvailableHeadlessCompositor" })
export type CaptureMeta = { readonly "client": string, readonly "codec": string, readonly "duration_ms": number, readonly "encoder_backend"?: string, readonly "fps": number, readonly "gpu"?: string, readonly "height": number, readonly "id": string, readonly "kind": string, readonly "sample_count": number, readonly "started_unix_ms": number, readonly "width": number }
export const CaptureMeta = Schema.Struct({ "client": Schema.String.annotate({ "description": "Short label / fingerprint prefix, or `\"\"` if unknown." }), "codec": Schema.String.annotate({ "description": "`\"h264\" | \"hevc\" | \"av1\"`." }), "duration_ms": Schema.Number.annotate({ "format": "int64" }).check(Schema.isInt().annotate({ "expected": "an integer" })).check(Schema.isGreaterThanOrEqualTo(0).annotate({ "expected": "a value greater than or equal to 0" })), "encoder_backend": Schema.optionalKey(Schema.String.annotate({ "description": "The encode backend that ACTUALLY opened for this session — `\"nvenc\"`, `\"vaapi\"`,\n`\"vulkan\"`, `\"amf\"`, `\"qsv\"`, `\"software\"`, … — and the GPU it runs on.\n\nRecorded because the stage split alone can't be read without them. A p50 `submit` of 10 ms\nmeans \"the GPU's CSC+encode throughput is the ceiling\" on one backend and something else\nentirely on another, and every fps-shortfall report so far has cost a round-trip asking\nwhich one it was. Both come from `ss_gpu::active()`, the record the encoder open itself\nwrites, so they name the branch that really opened rather than a re-derived guess.\n\n`\"\"` when nothing was streaming at registration (or on a build without the record)." })), "fps": Schema.Number.annotate({ "format": "int32" }).check(Schema.isInt().annotate({ "expected": "an integer" })).check(Schema.isGreaterThanOrEqualTo(0).annotate({ "expected": "a value greater than or equal to 0" })), "gpu": Schema.optionalKey(Schema.String.annotate({ "description": "Human-readable GPU name (`\"NVIDIA GeForce RTX 4090\"`, `\"CPU (openh264)\"`), or `\"\"`." })), "height": Schema.Number.annotate({ "format": "int32" }).check(Schema.isInt().annotate({ "expected": "an integer" })).check(Schema.isGreaterThanOrEqualTo(0).annotate({ "expected": "a value greater than or equal to 0" })), "id": Schema.String.annotate({ "description": "e.g. `\"2026-06-26T20-14-03Z_5120x1440\"` — also the filename stem." }), "kind": Schema.String.annotate({ "description": "`\"native\" | \"gamestream\"`." }), "sample_count": Schema.Number.annotate({ "format": "int32" }).check(Schema.isInt().annotate({ "expected": "an integer" })).check(Schema.isGreaterThanOrEqualTo(0).annotate({ "expected": "a value greater than or equal to 0" })), "started_unix_ms": Schema.Number.annotate({ "format": "int64" }).check(Schema.isInt().annotate({ "expected": "an integer" })).check(Schema.isGreaterThanOrEqualTo(0).annotate({ "expected": "a value greater than or equal to 0" })), "width": Schema.Number.annotate({ "format": "int32" }).check(Schema.isInt().annotate({ "expected": "an integer" })).check(Schema.isGreaterThanOrEqualTo(0).annotate({ "expected": "a value greater than or equal to 0" })) }).annotate({ "description": "Capture summary — the filename stem plus the negotiated mode/codec/client. Stored at the head\nof each on-disk recording and listed standalone (without the sample body) by\n[`StatsRecorder::list`].", "identifier": "CaptureMeta" })
export type CatalogEntry = { readonly "author": string, readonly "blocked"?: string | null, readonly "compatible": boolean, readonly "description": string, readonly "homepage"?: string | null, readonly "icon"?: string | null, readonly "id": string, readonly "incompatible_reason"?: string | null, readonly "installed_version"?: string | null, readonly "license"?: string | null, readonly "min_host"?: string | null, readonly "pkg": string, readonly "platforms": ReadonlyArray<string>, readonly "reviewed_at"?: string | null, readonly "source": string, readonly "tier": string, readonly "title": string, readonly "update_available": boolean, readonly "version": string }
export const CatalogEntry = Schema.Struct({ "author": Schema.String, "blocked": Schema.optionalKey(Schema.Union([Schema.String, Schema.Null]).annotate({ "description": "A revocation covering the catalogued version — do not offer this without shouting." })), "compatible": Schema.Boolean.annotate({ "description": "Can this host install it?" }), "description": Schema.String, "homepage": Schema.optionalKey(Schema.Union([Schema.String, Schema.Null])), "icon": Schema.optionalKey(Schema.Union([Schema.String, Schema.Null])), "id": Schema.String, "incompatible_reason": Schema.optionalKey(Schema.Union([Schema.String, Schema.Null])), "installed_version": Schema.optionalKey(Schema.Union([Schema.String, Schema.Null]).annotate({ "description": "The version installed right now, if any." })), "license": Schema.optionalKey(Schema.Union([Schema.String, Schema.Null])), "min_host": Schema.optionalKey(Schema.Union([Schema.String, Schema.Null])), "pkg": Schema.String, "platforms": Schema.Array(Schema.String), "reviewed_at": Schema.optionalKey(Schema.Union([Schema.String, Schema.Null]).annotate({ "description": "When Slipstream maintainers reviewed this exact tarball (built-in source only)." })), "source": Schema.String.annotate({ "description": "Which source listed it." }), "tier": Schema.String.annotate({ "description": "`verified` (built-in source) or `external` (an operator-added source). Never `unverified`:\nunverified installs come from a raw spec and are never listed (D7)." }), "title": Schema.String, "update_available": Schema.Boolean.annotate({ "description": "Installed, but at a different version than the catalog pins." }), "version": Schema.String.annotate({ "description": "The one installable version this entry pins." }) }).annotate({ "description": "One row on the shelf.", "identifier": "CatalogEntry" })
export type CheckStatus = "pass" | "warn" | "fail" | "skip"
export const CheckStatus = Schema.Literals(["pass", "warn", "fail", "skip"]).annotate({ "identifier": "CheckStatus" })
export type ClipboardPolicy = "off" | "text-only" | "on"
export const ClipboardPolicy = Schema.Literals(["off", "text-only", "on"]).annotate({ "identifier": "ClipboardPolicy" })
export type DetectHint = { readonly "exe"?: string | null, readonly "install_dir"?: string | null, readonly "process_name"?: string | null }
export const DetectHint = Schema.Struct({ "exe": Schema.optionalKey(Schema.Union([Schema.String, Schema.Null]).annotate({ "description": "The game's own executable, as an absolute path." })), "install_dir": Schema.optionalKey(Schema.Union([Schema.String, Schema.Null]).annotate({ "description": "Where the title is installed. Any process running from under this directory is part of the\ngame — the universal recipe, and the one worth supplying if you supply only one." })), "process_name": Schema.optionalKey(Schema.Union([Schema.String, Schema.Null]).annotate({ "description": "The executable's file name (`Hades.exe`), when its location isn't fixed. Weakest of the three\n— see [`DetectSpec::process_name`]." })) }).annotate({ "description": "What an operator (or a provider plugin) can tell the host about recognizing a title — the wire\nhalf of [`DetectSpec`], and the only part of it that is ever accepted from outside.\n\nDeliberately a **subset**: the store-derived signals (a Steam appid, a launcher's environment\nmarker) are things the host discovers for itself and would be meaningless — or dangerous — to take\non someone's word. What is left is what a provider genuinely knows and the host cannot guess: where\nthe title is installed, which executable is the game, what the process is called. All three are\noptional; supplying none is the same as supplying no hint at all.\n\nNever returned by the catalog API — see the module docs on why detect data does not cross the wire\noutbound.", "identifier": "DetectHint" })
export type DisconnectReason = "quit" | "timeout" | "error"
export const DisconnectReason = Schema.Literals(["quit", "timeout", "error"]).annotate({ "description": "Why a client went away. `Quit` is a deliberate user \"stop\" (the typed close code);\n`Timeout` is a transport idle timeout (the client vanished); `Error` is everything else.", "identifier": "DisconnectReason" })
export type EncoderConfig = { readonly "encoder"?: string, readonly "render_adapter"?: string | null, readonly "zerocopy"?: boolean | null }
export const EncoderConfig = Schema.Struct({ "encoder": Schema.optionalKey(Schema.String.annotate({ "description": "`auto` | `nvenc` | `amf` | `qsv` | `vaapi` | `software` (`SLIPSTREAM_ENCODER`)." })), "render_adapter": Schema.optionalKey(Schema.Union([Schema.String, Schema.Null]).annotate({ "description": "Substring pin for the render adapter (`SLIPSTREAM_RENDER_ADAPTER`)." })), "zerocopy": Schema.optionalKey(Schema.Union([Schema.Boolean, Schema.Null]).annotate({ "description": "Tri-state zero-copy override (`SLIPSTREAM_ZEROCOPY`). `null` = vendor default." })) }).annotate({ "identifier": "EncoderConfig" })
export type EndGameRequest = { readonly "app_id"?: string | null }
export const EndGameRequest = Schema.Struct({ "app_id": Schema.optionalKey(Schema.Union([Schema.String, Schema.Null]).annotate({ "description": "Store-qualified library id (`steam:570`) to end; omit to end every waiting game." })) }).annotate({ "description": "Request body for `endGame`.", "identifier": "EndGameRequest" })
export type EndGameResult = { readonly "ended": number }
export const EndGameResult = Schema.Struct({ "ended": Schema.Number.annotate({ "description": "How many waiting games were ended." }).check(Schema.isInt().annotate({ "expected": "an integer" })).check(Schema.isGreaterThanOrEqualTo(0).annotate({ "expected": "a value greater than or equal to 0" })) }).annotate({ "description": "Result of an `endGame`.", "identifier": "EndGameResult" })
export type GameEndReason = "exited" | "terminated"
export const GameEndReason = Schema.Literals(["exited", "terminated"]).annotate({ "description": "Why a launched game is no longer running.", "identifier": "GameEndReason" })
export type GameMeta = { readonly "description"?: string | null, readonly "developer"?: string | null, readonly "genres"?: ReadonlyArray<string>, readonly "platform"?: string | null, readonly "players"?: number | null, readonly "publisher"?: string | null, readonly "region"?: string | null, readonly "release_year"?: number | null, readonly "tags"?: ReadonlyArray<string> }
export const GameMeta = Schema.Struct({ "description": Schema.optionalKey(Schema.Union([Schema.String, Schema.Null]).annotate({ "description": "Short blurb for a details pane." })), "developer": Schema.optionalKey(Schema.Union([Schema.String, Schema.Null])), "genres": Schema.optionalKey(Schema.Array(Schema.String).annotate({ "description": "Genre taxonomy from the metadata source (`\"RPG\"`, `\"Platformer\"`, …)." })), "platform": Schema.optionalKey(Schema.Union([Schema.String, Schema.Null]).annotate({ "description": "The system the title runs on — `\"PS2\"`, `\"Xbox 360\"`, `\"SNES\"`, … Installed-store\nscanners stamp `\"PC\"`; `GET /library?platform=` filters on it (case-insensitive)." })), "players": Schema.optionalKey(Schema.Union([Schema.Number.check(Schema.isInt().annotate({ "expected": "an integer" })).check(Schema.isGreaterThanOrEqualTo(0).annotate({ "expected": "a value greater than or equal to 0" })), Schema.Null]).annotate({ "description": "Maximum simultaneous (local) players.", "format": "int32" })), "publisher": Schema.optionalKey(Schema.Union([Schema.String, Schema.Null])), "region": Schema.optionalKey(Schema.Union([Schema.String, Schema.Null]).annotate({ "description": "Release region — emulation-relevant (`\"NTSC-U\"`, `\"PAL\"`, `\"NTSC-J\"`)." })), "release_year": Schema.optionalKey(Schema.Union([Schema.Number.check(Schema.isInt().annotate({ "expected": "an integer" })).check(Schema.isGreaterThanOrEqualTo(0).annotate({ "expected": "a value greater than or equal to 0" })), Schema.Null]).annotate({ "description": "Year of first release — the granularity metadata sources reliably agree on.", "format": "int32" })), "tags": Schema.optionalKey(Schema.Array(Schema.String).annotate({ "description": "Free-form organizational labels (`\"co-op\"`, `\"kids\"`, `\"finished\"`, …)." })) }).annotate({ "description": "Descriptive metadata for a title — everything a richer library UI (details pane, platform\nfilter, couch-pick badges) renders beyond the poster. Every field is optional and defaults to\nabsent, so pre-metadata catalogs, providers, and clients keep working unchanged. The struct is\n`#[serde(flatten)]`-ed into [`GameEntry`] / the custom-store shapes: one definition, a flat\nwire shape everywhere.\n\nValues are free-form display strings, not enums — emulation sources (RomM, EmuDeck, Playnite)\neach have their own vocabulary and the host has no business normalizing it.", "identifier": "GameMeta" })
export type GameOnSessionEnd = "keep" | "on_quit" | "always"
export const GameOnSessionEnd = Schema.Literals(["keep", "on_quit", "always"]).annotate({ "description": "What to do with the launched game when its session ends.", "identifier": "GameOnSessionEnd" })
export type GameSession = "auto" | "dedicated"
export const GameSession = Schema.Literals(["auto", "dedicated"]).annotate({ "description": "How a session that **launches a game** (a library id on the Hello / apps.json / Decky pin) is\nserved (`design/gamemode-and-dedicated-sessions.md` §5.2). Orthogonal to the preset/lifecycle axes\n- a top-level [`DisplayPolicy`] field, NOT part of [`EffectivePolicy`], so a preset never clobbers\nit. Linux-only in effect.", "identifier": "GameSession" })
export type GeneralConfig = { readonly "host_name"?: string | null, readonly "perf"?: boolean }
export const GeneralConfig = Schema.Struct({ "host_name": Schema.optionalKey(Schema.Union([Schema.String, Schema.Null]).annotate({ "description": "Display name for Moonlight / mDNS (`SLIPSTREAM_HOST_NAME`)." })), "perf": Schema.optionalKey(Schema.Boolean.annotate({ "description": "Verbose perf logging (`SLIPSTREAM_PERF`)." })) }).annotate({ "identifier": "GeneralConfig" })
export type Health = { readonly "abi_version": number, readonly "status": string, readonly "version": string }
export const Health = Schema.Struct({ "abi_version": Schema.Number.annotate({ "description": "`slipstream-core` C ABI version.", "format": "int32" }).check(Schema.isInt().annotate({ "expected": "an integer" })).check(Schema.isGreaterThanOrEqualTo(0).annotate({ "expected": "a value greater than or equal to 0" })), "status": Schema.String.annotate({ "description": "Always `\"ok\"` when the host responds." }), "version": Schema.String.annotate({ "description": "`slipstream-host` crate version." }) }).annotate({ "description": "Liveness + version probe.", "identifier": "Health" })
export type HostFacts = { readonly "platform": string, readonly "version": string }
export const HostFacts = Schema.Struct({ "platform": Schema.String.annotate({ "description": "`linux`." }), "version": Schema.String }).annotate({ "description": "Facts about this host, so the console can grey out rows it can't install.", "identifier": "HostFacts" })
export type Identity = "shared" | "per-client" | "per-client-mode"
export const Identity = Schema.Literals(["shared", "per-client", "per-client-mode"]).annotate({ "description": "Stable display identity, so desktop environments persist per-display config (KDE scaling). Stored\nat Stage 0; carriers wired from the identity stage.", "identifier": "Identity" })
export type InputConfig = { readonly "gamepad"?: string | null, readonly "gamescope_grab_cursor"?: boolean, readonly "hide_host_cursor"?: boolean, readonly "pen"?: boolean }
export const InputConfig = Schema.Struct({ "gamepad": Schema.optionalKey(Schema.Union([Schema.String, Schema.Null]).annotate({ "description": "Gamepad backend preference (`SLIPSTREAM_GAMEPAD`)." })), "gamescope_grab_cursor": Schema.optionalKey(Schema.Boolean.annotate({ "description": "Gamescope: grab cursor into the nested session.\nDefault off (matches runtime `SLIPSTREAM_GAMESCOPE_GRAB_CURSOR`)." })), "hide_host_cursor": Schema.optionalKey(Schema.Boolean.annotate({ "description": "Hide the host's local OS cursor while clients stream (`SLIPSTREAM_HIDE_HOST_CURSOR`).\nDefault on." })), "pen": Schema.optionalKey(Schema.Boolean.annotate({ "description": "Advertise full-fidelity pen/stylus input (`SLIPSTREAM_PEN`). Default on." })) }).annotate({ "identifier": "InputConfig" })
export type InstallRequest = { readonly "accept_unverified"?: boolean, readonly "id"?: string | null, readonly "source"?: string | null, readonly "spec"?: string | null }
export const InstallRequest = Schema.Struct({ "accept_unverified": Schema.optionalKey(Schema.Boolean.annotate({ "description": "Required with [`Self::spec`]: the operator's explicit acknowledgement that this installs\nunreviewed code with operator privileges. The console collects it behind a typed\nconfirmation; the API refuses without it so no other caller can skip the decision." })), "id": Schema.optionalKey(Schema.Union([Schema.String, Schema.Null]).annotate({ "description": "Catalog entry id (with [`Self::source`])." })), "source": Schema.optionalKey(Schema.Union([Schema.String, Schema.Null]).annotate({ "description": "Catalog source name (with [`Self::id`])." })), "spec": Schema.optionalKey(Schema.Union([Schema.String, Schema.Null]).annotate({ "description": "A raw package spec (`@scope/name`, `@scope/name@1.2.3`, an https tarball or git+https URL).\nNothing reviewed it and nothing pins it." })) }).annotate({ "description": "`POST /store/install` — either a catalogued entry, or a raw spec the operator owns.", "identifier": "InstallRequest" })
export type InstalledView = { readonly "blocked"?: string | null, readonly "entry_id"?: string | null, readonly "installed_at"?: string | null, readonly "pkg": string, readonly "plugin_id"?: string | null, readonly "running": boolean, readonly "source"?: string | null, readonly "tier": string, readonly "title"?: string | null, readonly "update_available"?: string | null, readonly "version"?: string | null }
export const InstalledView = Schema.Struct({ "blocked": Schema.optionalKey(Schema.Union([Schema.String, Schema.Null]).annotate({ "description": "A revocation covering the *installed* version. Reported, never auto-removed: silently\ndeleting running code is its own hazard, so the operator decides." })), "entry_id": Schema.optionalKey(Schema.Union([Schema.String, Schema.Null]).annotate({ "description": "The catalog entry this maps to, when it is on a shelf we know." })), "installed_at": Schema.optionalKey(Schema.Union([Schema.String, Schema.Null])), "pkg": Schema.String, "plugin_id": Schema.optionalKey(Schema.Union([Schema.String, Schema.Null]).annotate({ "description": "The plugin id it registers under — the key into `GET /plugins`." })), "running": Schema.Boolean.annotate({ "description": "Is it registered in the live lease registry right now?" }), "source": Schema.optionalKey(Schema.Union([Schema.String, Schema.Null])), "tier": Schema.String.annotate({ "description": "`verified` / `external` / `unverified` / `cli` — remembered from install time, so an\nunverified plugin stays visibly unverified long after the dialog is forgotten." }), "title": Schema.optionalKey(Schema.Union([Schema.String, Schema.Null])), "update_available": Schema.optionalKey(Schema.Union([Schema.String, Schema.Null]).annotate({ "description": "The catalog's version, when it's newer than what's installed." })), "version": Schema.optionalKey(Schema.Union([Schema.String, Schema.Null])) }).annotate({ "description": "An installed plugin package, joined with its provenance and whether it's actually running.", "identifier": "InstalledView" })
export type JobRef = { readonly "job": string }
export const JobRef = Schema.Struct({ "job": Schema.String }).annotate({ "description": "202 body: where to watch the work.", "identifier": "JobRef" })
export type Objects_ = { readonly "mode": "off" }
export const Objects_ = Schema.Struct({ "mode": Schema.Literal("off") }).annotate({ "description": "Tear the display down at session end." })
export type Objects_1 = { readonly "mode": "duration", readonly "seconds": number }
export const Objects_1 = Schema.Struct({ "mode": Schema.Literal("duration"), "seconds": Schema.Number.annotate({ "description": "Linger window in seconds.", "format": "int32" }).check(Schema.isInt().annotate({ "expected": "an integer" })).check(Schema.isGreaterThanOrEqualTo(0).annotate({ "expected": "a value greater than or equal to 0" })) }).annotate({ "description": "Keep the display for `seconds` after the last session leaves, then tear it down; a reconnect\ninside the window reuses it." })
export type Objects_2 = { readonly "mode": "forever" }
export const Objects_2 = Schema.Struct({ "mode": Schema.Literal("forever") }).annotate({ "description": "Keep the display until host shutdown or an explicit release (the `Pinned` lifecycle state).\n**Not honored until the display-lifecycle stage** — rejected by the mgmt PUT at Stage 0." })
export type LatencyProfile = "balanced" | "low_latency"
export const LatencyProfile = Schema.Literals(["balanced", "low_latency"]).annotate({ "identifier": "LatencyProfile" })
export type LaunchSpec = { readonly "kind": string, readonly "value": string }
export const LaunchSpec = Schema.Struct({ "kind": Schema.String.annotate({ "description": "`\"steam_appid\"` or `\"command\"`." }), "value": Schema.String.annotate({ "description": "The appid (for `steam_appid`) or the shell command (for `command`)." }) }).annotate({ "description": "How the host would launch a title (consumed by the session launcher in a later step). Kept\nopen-ended so new stores slot in: `steam_appid` → `steam steam://rungameid/<value>`;\n`command` → run `<value>` nested in a gamescope session.", "identifier": "LaunchSpec" })
export type LayoutMode = "auto-row" | "manual"
export const LayoutMode = Schema.Literals(["auto-row", "manual"]).annotate({ "description": "How group members are arranged in the desktop coordinate space. Stored at Stage 0; applied from\nthe multi-monitor stage.", "identifier": "LayoutMode" })
export type LogEntry = { readonly "level": string, readonly "msg": string, readonly "seq": number, readonly "target": string, readonly "ts_ms": number }
export const LogEntry = Schema.Struct({ "level": Schema.String.annotate({ "description": "`ERROR` | `WARN` | `INFO` | `DEBUG` | `TRACE`." }), "msg": Schema.String.annotate({ "description": "The formatted message, structured fields appended as `key=value`." }), "seq": Schema.Number.annotate({ "description": "Monotonic sequence number (1-based) — pass the last one back as the `after` cursor.", "format": "int64" }).check(Schema.isInt().annotate({ "expected": "an integer" })).check(Schema.isGreaterThanOrEqualTo(0).annotate({ "expected": "a value greater than or equal to 0" })), "target": Schema.String.annotate({ "description": "The emitting module path (tracing target)." }), "ts_ms": Schema.Number.annotate({ "description": "Unix timestamp in milliseconds.", "format": "int64" }).check(Schema.isInt().annotate({ "expected": "an integer" })).check(Schema.isGreaterThanOrEqualTo(0).annotate({ "expected": "a value greater than or equal to 0" })) }).annotate({ "description": "One captured log event.", "identifier": "LogEntry" })
export type ModeConflict = "separate" | "steal" | "join" | "reject"
export const ModeConflict = Schema.Literals(["separate", "steal", "join", "reject"]).annotate({ "description": "Admission when a *different* client connects while a display/session is already live and asks for\na different mode. Stored at Stage 0; enforced from the mode-conflict admission stage.", "identifier": "ModeConflict" })
export type MoonlightBroadcastRequest = { readonly "enabled": boolean }
export const MoonlightBroadcastRequest = Schema.Struct({ "enabled": Schema.Boolean.annotate({ "description": "Whether to run and advertise the GameStream/Moonlight compatibility plane." }) }).annotate({ "identifier": "MoonlightBroadcastRequest" })
export type NativeClient = { readonly "fingerprint": string, readonly "name": string }
export const NativeClient = Schema.Struct({ "fingerprint": Schema.String.annotate({ "description": "Hex SHA-256 of the client certificate — its stable id here." }), "name": Schema.String.annotate({ "description": "The name the client supplied when pairing." }) }).annotate({ "description": "A paired native (slipstream/1) client.", "identifier": "NativeClient" })
export type NativePairStatus = { readonly "armed": boolean, readonly "enabled": boolean, readonly "expires_in_secs"?: number | null, readonly "paired_clients": number, readonly "pin"?: string | null }
export const NativePairStatus = Schema.Struct({ "armed": Schema.Boolean.annotate({ "description": "True while a pairing window is open." }), "enabled": Schema.Boolean.annotate({ "description": "Whether the native host is running (the unified host started with `--native`)." }), "expires_in_secs": Schema.optionalKey(Schema.Union([Schema.Number.check(Schema.isInt().annotate({ "expected": "an integer" })).check(Schema.isGreaterThanOrEqualTo(0).annotate({ "expected": "a value greater than or equal to 0" })), Schema.Null]).annotate({ "description": "Seconds left in the window (null = disarmed, or armed with no expiry via the CLI flag).", "format": "int64" })), "paired_clients": Schema.Number.annotate({ "description": "Number of paired native clients.", "format": "int32" }).check(Schema.isInt().annotate({ "expected": "an integer" })).check(Schema.isGreaterThanOrEqualTo(0).annotate({ "expected": "a value greater than or equal to 0" })), "pin": Schema.optionalKey(Schema.Union([Schema.String, Schema.Null]).annotate({ "description": "The PIN to display while armed (null when disarmed)." })) }).annotate({ "description": "Native (slipstream/1) pairing status. Unlike GameStream, the **host** mints the PIN (the SPAKE2\nceremony needs it client-side first), so the console **displays** `pin` for the user to enter on\ntheir device — armed on demand for a short window.", "identifier": "NativePairStatus" })
export type NetworkConfig = { readonly "chacha20"?: boolean, readonly "fec_pct"?: number | null, readonly "gamestream"?: boolean | null, readonly "mdns"?: boolean }
export const NetworkConfig = Schema.Struct({ "chacha20": Schema.optionalKey(Schema.Boolean.annotate({ "description": "Prefer ChaCha20-Poly1305 for soft-AES clients (`SLIPSTREAM_CHACHA20`). Default on." })), "fec_pct": Schema.optionalKey(Schema.Union([Schema.Number.check(Schema.isInt().annotate({ "expected": "an integer" })).check(Schema.isGreaterThanOrEqualTo(0).annotate({ "expected": "a value greater than or equal to 0" })), Schema.Null]).annotate({ "description": "FEC percentage for the native plane (`SLIPSTREAM_FEC_PCT`), when set.", "format": "int32" })), "gamestream": Schema.optionalKey(Schema.Union([Schema.Boolean, Schema.Null]).annotate({ "description": "Run and advertise the GameStream/Moonlight compatibility plane on the next host start." })), "mdns": Schema.optionalKey(Schema.Boolean.annotate({ "description": "Advertise over mDNS (`SLIPSTREAM_MDNS`). Default on." })) }).annotate({ "identifier": "NetworkConfig" })
export type NetworkPolicy = "auto" | "lan" | "wan"
export const NetworkPolicy = Schema.Literals(["auto", "lan", "wan"]).annotate({ "identifier": "NetworkPolicy" })
export type PairedClient = { readonly "fingerprint": string, readonly "not_after_unix"?: number | null, readonly "not_before_unix"?: number | null, readonly "subject"?: string | null }
export const PairedClient = Schema.Struct({ "fingerprint": Schema.String.annotate({ "description": "Lowercase hex SHA-256 of the client certificate DER — the client's stable id here." }), "not_after_unix": Schema.optionalKey(Schema.Union([Schema.Number.check(Schema.isInt().annotate({ "expected": "an integer" })), Schema.Null]).annotate({ "description": "Certificate validity end (unix seconds).", "format": "int64" })), "not_before_unix": Schema.optionalKey(Schema.Union([Schema.Number.check(Schema.isInt().annotate({ "expected": "an integer" })), Schema.Null]).annotate({ "description": "Certificate validity start (unix seconds).", "format": "int64" })), "subject": Schema.optionalKey(Schema.Union([Schema.String, Schema.Null]).annotate({ "description": "Certificate subject (e.g. `CN=NVIDIA GameStream Client`), if the DER parses." })) }).annotate({ "description": "A paired (certificate-pinned) Moonlight client.", "identifier": "PairedClient" })
export type PairingStatus = { readonly "pin_pending": boolean }
export const PairingStatus = Schema.Struct({ "pin_pending": Schema.Boolean.annotate({ "description": "True while a pairing handshake is parked waiting for the user's PIN." }) }).annotate({ "description": "Pairing-flow status.", "identifier": "PairingStatus" })
export type PendingDevice = { readonly "age_secs": number, readonly "fingerprint": string, readonly "id": number, readonly "name": string }
export const PendingDevice = Schema.Struct({ "age_secs": Schema.Number.annotate({ "description": "Seconds since the device last knocked.", "format": "int64" }).check(Schema.isInt().annotate({ "expected": "an integer" })).check(Schema.isGreaterThanOrEqualTo(0).annotate({ "expected": "a value greater than or equal to 0" })), "fingerprint": Schema.String.annotate({ "description": "Hex SHA-256 of the device's certificate — what approval pins." }), "id": Schema.Number.annotate({ "description": "Id to address approve/deny (per-process; entries expire after ~10 minutes).", "format": "int32" }).check(Schema.isInt().annotate({ "expected": "an integer" })).check(Schema.isGreaterThanOrEqualTo(0).annotate({ "expected": "a value greater than or equal to 0" })), "name": Schema.String.annotate({ "description": "Best-effort device label (the client's own name, else fingerprint-derived)." }) }).annotate({ "description": "An unpaired device that tried to connect while the host requires pairing — awaiting\n**delegated approval** (approve it here instead of fetching the host PIN out of band).", "identifier": "PendingDevice" })
export type PerformanceProfile = "balanced" | "low_latency"
export const PerformanceProfile = Schema.Literals(["balanced", "low_latency"]).annotate({ "identifier": "PerformanceProfile" })
export type Plane = "native" | "gamestream"
export const Plane = Schema.Literals(["native", "gamestream"]).annotate({ "description": "Which protocol plane an event originated from. Hooks and scripts filter on it — a hook\nthat fires for native clients but not Moonlight clients is a bug, not a v2 feature.", "identifier": "Plane" })
export type PluginUi = { readonly "icon"?: string | null, readonly "port": number, readonly "secret": string }
export const PluginUi = Schema.Struct({ "icon": Schema.optionalKey(Schema.Union([Schema.String, Schema.Null]).annotate({ "description": "Optional lucide icon name for the console nav entry (`^[a-z0-9-]{1,48}$`)." })), "port": Schema.Number.annotate({ "description": "The **loopback** port the plugin serves its UI on. The host and console only ever dial\n`127.0.0.1:<port>`; a registration can never carry a hostname.", "format": "int32" }).check(Schema.isInt().annotate({ "expected": "an integer" })).check(Schema.isGreaterThanOrEqualTo(0).annotate({ "expected": "a value greater than or equal to 0" })), "secret": Schema.String.annotate({ "description": "Per-boot shared secret the console proxy must present (as `Authorization: Bearer`) on every\nrequest to the plugin's UI server. Rotated whenever the plugin restarts." }) }).annotate({ "description": "A plugin's UI surface as it registers it. Carries the secret — this shape is only ever a request\nbody, never a response ([`PluginUiPublic`] is the secret-free view).", "identifier": "PluginUi" })
export type PluginUiPublic = { readonly "icon"?: string | null, readonly "port": number }
export const PluginUiPublic = Schema.Struct({ "icon": Schema.optionalKey(Schema.Union([Schema.String, Schema.Null])), "port": Schema.Number.annotate({ "format": "int32" }).check(Schema.isInt().annotate({ "expected": "an integer" })).check(Schema.isGreaterThanOrEqualTo(0).annotate({ "expected": "a value greater than or equal to 0" })) }).annotate({ "description": "The secret-free view of a plugin's UI surface — what [`list_plugins`] returns to the browser.", "identifier": "PluginUiPublic" })
export type PortMap = { readonly "audio": number, readonly "control": number, readonly "http": number, readonly "https": number, readonly "mgmt": number, readonly "rtsp": number, readonly "video": number }
export const PortMap = Schema.Struct({ "audio": Schema.Number.annotate({ "format": "int32" }).check(Schema.isInt().annotate({ "expected": "an integer" })).check(Schema.isGreaterThanOrEqualTo(0).annotate({ "expected": "a value greater than or equal to 0" })), "control": Schema.Number.annotate({ "format": "int32" }).check(Schema.isInt().annotate({ "expected": "an integer" })).check(Schema.isGreaterThanOrEqualTo(0).annotate({ "expected": "a value greater than or equal to 0" })), "http": Schema.Number.annotate({ "description": "nvhttp plain HTTP (serverinfo, pairing).", "format": "int32" }).check(Schema.isInt().annotate({ "expected": "an integer" })).check(Schema.isGreaterThanOrEqualTo(0).annotate({ "expected": "a value greater than or equal to 0" })), "https": Schema.Number.annotate({ "description": "nvhttp mutual-TLS HTTPS (post-pairing).", "format": "int32" }).check(Schema.isInt().annotate({ "expected": "an integer" })).check(Schema.isGreaterThanOrEqualTo(0).annotate({ "expected": "a value greater than or equal to 0" })), "mgmt": Schema.Number.annotate({ "description": "This management API.", "format": "int32" }).check(Schema.isInt().annotate({ "expected": "an integer" })).check(Schema.isGreaterThanOrEqualTo(0).annotate({ "expected": "a value greater than or equal to 0" })), "rtsp": Schema.Number.annotate({ "format": "int32" }).check(Schema.isInt().annotate({ "expected": "an integer" })).check(Schema.isGreaterThanOrEqualTo(0).annotate({ "expected": "a value greater than or equal to 0" })), "video": Schema.Number.annotate({ "format": "int32" }).check(Schema.isInt().annotate({ "expected": "an integer" })).check(Schema.isGreaterThanOrEqualTo(0).annotate({ "expected": "a value greater than or equal to 0" })) }).annotate({ "description": "Every port a client integration may need (Moonlight derives the stream ports from the\nHTTP base; a control pane should not have to).", "identifier": "PortMap" })
export type Position = { readonly "x": number, readonly "y": number }
export const Position = Schema.Struct({ "x": Schema.Number.annotate({ "format": "int32" }).check(Schema.isInt().annotate({ "expected": "an integer" })), "y": Schema.Number.annotate({ "format": "int32" }).check(Schema.isInt().annotate({ "expected": "an integer" })) }).annotate({ "description": "A desktop-space offset for a display (top-left origin).", "identifier": "Position" })
export type PrepCmd = { readonly "do": string, readonly "undo"?: string | null }
export const PrepCmd = Schema.Struct({ "do": Schema.String.annotate({ "description": "Command run before launch. Same execution recipe and ownership checks as hook `run`\ncommands (event-less: stdin is empty JSON, env carries the `PF_APP_*` context)." }), "undo": Schema.optionalKey(Schema.Union([Schema.String, Schema.Null]).annotate({ "description": "Command run after the session ends. Skipped when its `do` failed (it never took effect)." })) }).annotate({ "description": "One per-app preparation step (RFC §6 — deliberate Sunshine `prep-cmd` parity): `do` runs\n**synchronously before the app launches** (an HDR toggle or a MangoHud env change must land\nfirst), `undo` runs at session end — reverse order across steps, best-effort, on every exit\npath including a crash-unwind (RAII via [`PrepGuard`]).", "identifier": "PrepCmd" })
export type Preset = "custom" | "default" | "gaming-rig" | "shared-desktop" | "hotdesk" | "workstation"
export const Preset = Schema.Literals(["custom", "default", "gaming-rig", "shared-desktop", "hotdesk", "workstation"]).annotate({ "description": "A named bundle of the fields below. `Custom` (the default) means the explicit fields rule; any\nother preset ignores the stored fields and expands to its own ([`DisplayPolicy::effective`]).", "identifier": "Preset" })
export type ProviderRemoved = { readonly "removed": number }
export const ProviderRemoved = Schema.Struct({ "removed": Schema.Number.annotate({ "description": "How many entries the provider owned (and were removed)." }).check(Schema.isInt().annotate({ "expected": "an integer" })).check(Schema.isGreaterThanOrEqualTo(0).annotate({ "expected": "a value greater than or equal to 0" })) }).annotate({ "description": "The count envelope a provider uninstall returns.", "identifier": "ProviderRemoved" })
export type ReleaseDisplayRequest = { readonly "slot"?: number | null }
export const ReleaseDisplayRequest = Schema.Struct({ "slot": Schema.optionalKey(Schema.Union([Schema.Number.check(Schema.isInt().annotate({ "expected": "an integer" })).check(Schema.isGreaterThanOrEqualTo(0).annotate({ "expected": "a value greater than or equal to 0" })), Schema.Null]).annotate({ "description": "Slot to release (see `state`); omit to release **all** kept displays.", "format": "int64" })) }).annotate({ "description": "Request body for `releaseDisplay`.", "identifier": "ReleaseDisplayRequest" })
export type ReleaseDisplayResult = { readonly "released": number }
export const ReleaseDisplayResult = Schema.Struct({ "released": Schema.Number.annotate({ "description": "Number of kept displays torn down." }).check(Schema.isInt().annotate({ "expected": "an integer" })).check(Schema.isGreaterThanOrEqualTo(0).annotate({ "expected": "a value greater than or equal to 0" })) }).annotate({ "description": "Result of a `/display/release`.", "identifier": "ReleaseDisplayResult" })
export type RuntimeRequest = { readonly "enabled": boolean }
export const RuntimeRequest = Schema.Struct({ "enabled": Schema.Boolean }).annotate({ "identifier": "RuntimeRequest" })
export type RuntimeView = { readonly "detail"?: string | null, readonly "enabled": boolean, readonly "installed": boolean, readonly "principal"?: string | null, readonly "running": boolean, readonly "unit": string }
export const RuntimeView = Schema.Struct({ "detail": Schema.optionalKey(Schema.Union([Schema.String, Schema.Null])), "enabled": Schema.Boolean, "installed": Schema.Boolean.annotate({ "description": "Is the runner payload/unit present at all?" }), "principal": Schema.optionalKey(Schema.Union([Schema.String, Schema.Null])), "running": Schema.Boolean, "unit": Schema.String.annotate({ "description": "systemd unit or scheduled-task name." }) }).annotate({ "identifier": "RuntimeView" })
export type ScannerInfo = { readonly "enabled": boolean, readonly "id": string, readonly "label": string }
export const ScannerInfo = Schema.Struct({ "enabled": Schema.Boolean.annotate({ "description": "Whether this host runs the scanner (default true)." }), "id": Schema.String.annotate({ "description": "Stable scanner id — the same string the scanner's entries carry in their `store` field." }), "label": Schema.String.annotate({ "description": "Human-facing name for the console toggle." }) }).annotate({ "description": "One installed-store scanner this host build supports, with its enable state — the unit the\nconsole renders a toggle for. The list is platform-gated at compile time (the scanners are),\nso the console never shows a toggle that cannot do anything on this host.", "identifier": "ScannerInfo" })
export type ScannerToggle = { readonly "enabled": boolean }
export const ScannerToggle = Schema.Struct({ "enabled": Schema.Boolean.annotate({ "description": "Whether the scanner should run on this host." }) }).annotate({ "description": "Request body for `setLibraryScanner`.", "identifier": "ScannerToggle" })
export type SessionInfo = { readonly "fps": number, readonly "height": number, readonly "width": number }
export const SessionInfo = Schema.Struct({ "fps": Schema.Number.annotate({ "format": "int32" }).check(Schema.isInt().annotate({ "expected": "an integer" })).check(Schema.isGreaterThanOrEqualTo(0).annotate({ "expected": "a value greater than or equal to 0" })), "height": Schema.Number.annotate({ "format": "int32" }).check(Schema.isInt().annotate({ "expected": "an integer" })).check(Schema.isGreaterThanOrEqualTo(0).annotate({ "expected": "a value greater than or equal to 0" })), "width": Schema.Number.annotate({ "format": "int32" }).check(Schema.isInt().annotate({ "expected": "an integer" })).check(Schema.isGreaterThanOrEqualTo(0).annotate({ "expected": "a value greater than or equal to 0" })) }).annotate({ "description": "Client-requested launch parameters (key material is never exposed here).", "identifier": "SessionInfo" })
export type SessionRef = { readonly "client": string, readonly "hdr": boolean, readonly "id": number, readonly "mode": string }
export const SessionRef = Schema.Struct({ "client": Schema.String.annotate({ "description": "Short client label (cert-fingerprint prefix, or peer IP for an anonymous client)." }), "hdr": Schema.Boolean, "id": Schema.Number.annotate({ "description": "Host-local session id (unique within this host process).", "format": "int64" }).check(Schema.isInt().annotate({ "expected": "an integer" })).check(Schema.isGreaterThanOrEqualTo(0).annotate({ "expected": "a value greater than or equal to 0" })), "mode": Schema.String.annotate({ "description": "Negotiated mode, `WxH@Hz` (e.g. `\"3840x2160@120\"`)." }) }).annotate({ "description": "A live A/V session (the plane-neutral notion the Dashboard shows).", "identifier": "SessionRef" })
export type SetGpuPreference = { readonly "gpu_id"?: string | null, readonly "mode": string }
export const SetGpuPreference = Schema.Struct({ "gpu_id": Schema.optionalKey(Schema.Union([Schema.String, Schema.Null]).annotate({ "description": "Required when `mode` is `manual`: the stable `id` of a currently listed GPU\n(see `listGpus`)." })), "mode": Schema.String.annotate({ "description": "`auto` (env pin, else max dedicated VRAM — the default) or `manual`." }) }).annotate({ "description": "Request body for `setGpuPreference`.", "identifier": "SetGpuPreference" })
export type SourceInput = { readonly "public_key"?: string | null, readonly "url": string }
export const SourceInput = Schema.Struct({ "public_key": Schema.optionalKey(Schema.Union([Schema.String, Schema.Null]).annotate({ "description": "`ed25519:<base64>`. Omitted ⇒ an unsigned source (accepted, flagged everywhere)." })), "url": Schema.String }).annotate({ "identifier": "SourceInput" })
export type SourceView = { readonly "builtin": boolean, readonly "entry_count": number, readonly "error"?: string | null, readonly "fetched_at"?: number | null, readonly "name": string, readonly "public_key"?: string | null, readonly "signed": boolean, readonly "stale": boolean, readonly "url": string }
export const SourceView = Schema.Struct({ "builtin": Schema.Boolean.annotate({ "description": "The built-in `slipstream` source: not editable, not removable, and the only source whose entries\nmay carry the \"verified\" tier." }), "entry_count": Schema.Number.annotate({ "format": "int32" }).check(Schema.isInt().annotate({ "expected": "an integer" })).check(Schema.isGreaterThanOrEqualTo(0).annotate({ "expected": "a value greater than or equal to 0" })), "error": Schema.optionalKey(Schema.Union([Schema.String, Schema.Null]).annotate({ "description": "Why the last refresh failed, if it did." })), "fetched_at": Schema.optionalKey(Schema.Union([Schema.Number.check(Schema.isInt().annotate({ "expected": "an integer" })).check(Schema.isGreaterThanOrEqualTo(0).annotate({ "expected": "a value greater than or equal to 0" })), Schema.Null]).annotate({ "description": "Unix seconds of the data we hold, when we hold any.", "format": "int64" })), "name": Schema.String, "public_key": Schema.optionalKey(Schema.Union([Schema.String, Schema.Null])), "signed": Schema.Boolean.annotate({ "description": "Whether we check a signature on this source's index. An unsigned source still works; the\nconsole marks it." }), "stale": Schema.Boolean.annotate({ "description": "The catalog we're serving is older than the last refresh attempt (offline, or the last\nfetch failed) — entries still install, because the pin travelled with the entry." }), "url": Schema.String }).annotate({ "description": "A configured catalog source and how its last refresh went.", "identifier": "SourceView" })
export type StageTiming = { readonly "name": string, readonly "p50_us": number, readonly "p99_us": number }
export const StageTiming = Schema.Struct({ "name": Schema.String.annotate({ "description": "`\"capture\" | \"submit\" | \"encode\" | \"packetize\" | \"send\"` (path-dependent)." }), "p50_us": Schema.Number.annotate({ "format": "float" }).check(Schema.isFinite().annotate({ "expected": "a finite number" })), "p99_us": Schema.Number.annotate({ "format": "float" }).check(Schema.isFinite().annotate({ "expected": "a finite number" })) }).annotate({ "description": "One pipeline stage's latency in an aggregation window (microseconds).", "identifier": "StageTiming" })
export type State = "running" | "done" | "failed"
export const State = Schema.Literals(["running", "done", "failed"]).annotate({ "identifier": "State" })
export type StatsStatus = { readonly "armed": boolean, readonly "elapsed_ms": number, readonly "kind": string, readonly "sample_count": number, readonly "started_unix_ms": number }
export const StatsStatus = Schema.Struct({ "armed": Schema.Boolean.annotate({ "description": "Capture currently running." }), "elapsed_ms": Schema.Number.annotate({ "description": "Host-measured elapsed time of the in-progress capture, in ms (`0` if idle). Computed from the\nhost's MONOTONIC clock, so a console can show elapsed time without subtracting `started_unix_ms`\nfrom its own (possibly skewed) wall clock.", "format": "int64" }).check(Schema.isInt().annotate({ "expected": "an integer" })).check(Schema.isGreaterThanOrEqualTo(0).annotate({ "expected": "a value greater than or equal to 0" })), "kind": Schema.String.annotate({ "description": "Path of the in-progress capture (`\"\"` if idle)." }), "sample_count": Schema.Number.annotate({ "description": "Samples in the in-progress capture.", "format": "int32" }).check(Schema.isInt().annotate({ "expected": "an integer" })).check(Schema.isGreaterThanOrEqualTo(0).annotate({ "expected": "a value greater than or equal to 0" })), "started_unix_ms": Schema.Number.annotate({ "description": "Unix start time of the in-progress capture (`0` if idle).", "format": "int64" }).check(Schema.isInt().annotate({ "expected": "an integer" })).check(Schema.isGreaterThanOrEqualTo(0).annotate({ "expected": "a value greater than or equal to 0" })) }).annotate({ "description": "Snapshot of the in-progress capture for the management API.", "identifier": "StatsStatus" })
export type SubmitPin = { readonly "pin": string }
export const SubmitPin = Schema.Struct({ "pin": Schema.String.annotate({ "description": "1–16 ASCII digits (Moonlight shows 4)." }) }).annotate({ "description": "The PIN Moonlight displays during pairing.", "identifier": "SubmitPin" })
export type SupportHost = { readonly "abi_version": number, readonly "gamestream": boolean, readonly "os": string, readonly "os_name": string, readonly "version": string }
export const SupportHost = Schema.Struct({ "abi_version": Schema.Number.annotate({ "format": "int32" }).check(Schema.isInt().annotate({ "expected": "an integer" })).check(Schema.isGreaterThanOrEqualTo(0).annotate({ "expected": "a value greater than or equal to 0" })), "gamestream": Schema.Boolean, "os": Schema.String, "os_name": Schema.String, "version": Schema.String }).annotate({ "identifier": "SupportHost" })
export type Topology = "auto" | "extend" | "primary" | "exclusive"
export const Topology = Schema.Literals(["auto", "extend", "primary", "exclusive"]).annotate({ "description": "What the host does to the box's display topology while managed virtual displays are up.", "identifier": "Topology" })
export type UiCredential = { readonly "port": number, readonly "secret": string }
export const UiCredential = Schema.Struct({ "port": Schema.Number.annotate({ "format": "int32" }).check(Schema.isInt().annotate({ "expected": "an integer" })).check(Schema.isGreaterThanOrEqualTo(0).annotate({ "expected": "a value greater than or equal to 0" })), "secret": Schema.String }).annotate({ "description": "`GET /plugins/{id}/ui-credential` — the console proxy's server-side lookup (bearer + loopback).\nThis is the only endpoint that returns a secret; the console BFF denylists it from the browser.", "identifier": "UiCredential" })
export type UninstallRequest = { readonly "pkg": string }
export const UninstallRequest = Schema.Struct({ "pkg": Schema.String }).annotate({ "identifier": "UninstallRequest" })
export type UpdateJobInfo = { readonly "received_bytes": number, readonly "stage": string, readonly "started_unix": number, readonly "target_version": string, readonly "total_bytes"?: number | null }
export const UpdateJobInfo = Schema.Struct({ "received_bytes": Schema.Number.annotate({ "format": "int64" }).check(Schema.isInt().annotate({ "expected": "an integer" })).check(Schema.isGreaterThanOrEqualTo(0).annotate({ "expected": "a value greater than or equal to 0" })), "stage": Schema.String.annotate({ "description": "`downloading` | `verifying` | `applying` | `restarting`." }), "started_unix": Schema.Number.annotate({ "format": "int64" }).check(Schema.isInt().annotate({ "expected": "an integer" })).check(Schema.isGreaterThanOrEqualTo(0).annotate({ "expected": "a value greater than or equal to 0" })), "target_version": Schema.String.annotate({ "description": "The version being installed." }), "total_bytes": Schema.optionalKey(Schema.Union([Schema.Number.check(Schema.isInt().annotate({ "expected": "an integer" })).check(Schema.isGreaterThanOrEqualTo(0).annotate({ "expected": "a value greater than or equal to 0" })), Schema.Null]).annotate({ "format": "int64" })) }).annotate({ "description": "A running apply job.", "identifier": "UpdateJobInfo" })
export type UpdateManifestInfo = { readonly "notes_url": string, readonly "published_at": string, readonly "serial": number, readonly "stale": boolean, readonly "version": string }
export const UpdateManifestInfo = Schema.Struct({ "notes_url": Schema.String.annotate({ "description": "Release-notes link (pinned to our forge by the manifest validator)." }), "published_at": Schema.String.annotate({ "description": "RFC-3339 publish time (display only)." }), "serial": Schema.Number.annotate({ "description": "Publish serial (unix seconds) — monotonic per channel.", "format": "int64" }).check(Schema.isInt().annotate({ "expected": "an integer" })).check(Schema.isGreaterThanOrEqualTo(0).annotate({ "expected": "a value greater than or equal to 0" })), "stale": Schema.Boolean.annotate({ "description": "The last verified manifest is suspiciously old (>45 days) — the freeze/stale hint." }), "version": Schema.String.annotate({ "description": "The released version this manifest announces." }) }).annotate({ "description": "One channel's manifest facts, as much as the console renders.", "identifier": "UpdateManifestInfo" })
export type UpdateResultInfo = { readonly "error"?: string | null, readonly "finished_unix": number, readonly "from": string, readonly "ok": boolean, readonly "stage"?: string | null, readonly "staged"?: boolean, readonly "to": string }
export const UpdateResultInfo = Schema.Struct({ "error": Schema.optionalKey(Schema.Union([Schema.String, Schema.Null])), "finished_unix": Schema.Number.annotate({ "format": "int64" }).check(Schema.isInt().annotate({ "expected": "an integer" })).check(Schema.isGreaterThanOrEqualTo(0).annotate({ "expected": "a value greater than or equal to 0" })), "from": Schema.String, "ok": Schema.Boolean, "stage": Schema.optionalKey(Schema.Union([Schema.String, Schema.Null]).annotate({ "description": "The stage that failed; absent on success." })), "staged": Schema.optionalKey(Schema.Boolean.annotate({ "description": "Applied but activates on the next reboot (rpm-ostree)." })), "to": Schema.String }).annotate({ "description": "Durable outcome of the most recent apply attempt (survives the host's own restart).", "identifier": "UpdateResultInfo" })
export type StreamInfo = { readonly "bitrate_kbps": number, readonly "codec": ApiCodec, readonly "fps": number, readonly "height": number, readonly "last_resize_ms"?: number | null, readonly "min_fec": number, readonly "packet_size": number, readonly "time_to_first_frame_ms"?: number | null, readonly "width": number }
export const StreamInfo = Schema.Struct({ "bitrate_kbps": Schema.Number.annotate({ "format": "int32" }).check(Schema.isInt().annotate({ "expected": "an integer" })).check(Schema.isGreaterThanOrEqualTo(0).annotate({ "expected": "a value greater than or equal to 0" })), "codec": ApiCodec, "fps": Schema.Number.annotate({ "format": "int32" }).check(Schema.isInt().annotate({ "expected": "an integer" })).check(Schema.isGreaterThanOrEqualTo(0).annotate({ "expected": "a value greater than or equal to 0" })), "height": Schema.Number.annotate({ "format": "int32" }).check(Schema.isInt().annotate({ "expected": "an integer" })).check(Schema.isGreaterThanOrEqualTo(0).annotate({ "expected": "a value greater than or equal to 0" })), "last_resize_ms": Schema.optionalKey(Schema.Union([Schema.Number.check(Schema.isInt().annotate({ "expected": "an integer" })).check(Schema.isGreaterThanOrEqualTo(0).annotate({ "expected": "a value greater than or equal to 0" })), Schema.Null]).annotate({ "description": "Most recent mid-stream resize total, reconfigure → pipeline rebuilt, in ms (native sessions;\n`null` when no resize happened / GameStream).", "format": "int32" })), "min_fec": Schema.Number.annotate({ "description": "Client's parity floor per FEC block (`minRequiredFecPackets`).", "format": "int32" }).check(Schema.isInt().annotate({ "expected": "an integer" })).check(Schema.isGreaterThanOrEqualTo(0).annotate({ "expected": "a value greater than or equal to 0" })), "packet_size": Schema.Number.annotate({ "description": "Video payload size per packet (bytes).", "format": "int32" }).check(Schema.isInt().annotate({ "expected": "an integer" })).check(Schema.isGreaterThanOrEqualTo(0).annotate({ "expected": "a value greater than or equal to 0" })), "time_to_first_frame_ms": Schema.optionalKey(Schema.Union([Schema.Number.check(Schema.isInt().annotate({ "expected": "an integer" })).check(Schema.isGreaterThanOrEqualTo(0).annotate({ "expected": "a value greater than or equal to 0" })), Schema.Null]).annotate({ "description": "Session bring-up total, hello → first video packet, in ms (native sessions; `null` on the\nGameStream plane or while the session is still bringing up).", "format": "int32" })), "width": Schema.Number.annotate({ "format": "int32" }).check(Schema.isInt().annotate({ "expected": "an integer" })).check(Schema.isGreaterThanOrEqualTo(0).annotate({ "expected": "a value greater than or equal to 0" })) }).annotate({ "description": "RTSP-negotiated stream parameters.", "identifier": "StreamInfo" })
export type DisplayStateResponse = { readonly "displays": ReadonlyArray<ApiDisplayInfo> }
export const DisplayStateResponse = Schema.Struct({ "displays": Schema.Array(ApiDisplayInfo) }).annotate({ "description": "The host's managed virtual displays right now.", "identifier": "DisplayStateResponse" })
export type MonitorsResponse = { readonly "compositor"?: string | null, readonly "error"?: string | null, readonly "monitors": ReadonlyArray<ApiMonitorInfo>, readonly "pin_supported": boolean, readonly "pinned"?: string | null }
export const MonitorsResponse = Schema.Struct({ "compositor": Schema.optionalKey(Schema.Union([Schema.String, Schema.Null]).annotate({ "description": "Compositor backend the enumeration came from (`kwin`, `mutter`, …), when one was resolved." })), "error": Schema.optionalKey(Schema.Union([Schema.String, Schema.Null]).annotate({ "description": "Why the list is empty, when enumeration failed (compositor unreachable, unsupported\nplatform). `None` with an empty list means \"asked, and there are none\"." })), "monitors": Schema.Array(ApiMonitorInfo).annotate({ "description": "The heads, ordered left-to-right by desktop position." }), "pin_supported": Schema.Boolean.annotate({ "description": "Whether this build can actually STREAM one of these monitors.\n\nLinux can enumerate and capture one of these monitors through the selected compositor." }), "pinned": Schema.optionalKey(Schema.Union([Schema.String, Schema.Null]).annotate({ "description": "The configured `SLIPSTREAM_CAPTURE_MONITOR`, if any — reported even when it matches nothing,\nso the console can show \"pinned to DP-2, which this host doesn't have\"." })) }).annotate({ "description": "The host's physical monitors + which one capture is pinned to.", "identifier": "MonitorsResponse" })
export type GpuState = { readonly "active"?: null | ApiActiveGpu, readonly "encoder_pin"?: string | null, readonly "env_override"?: string | null, readonly "gpus": ReadonlyArray<ApiGpu>, readonly "mode": string, readonly "preferred_available": boolean, readonly "preferred_id"?: string | null, readonly "preferred_name"?: string | null, readonly "selected"?: null | ApiSelectedGpu }
export const GpuState = Schema.Struct({ "active": Schema.optionalKey(Schema.Union([Schema.Null, Schema.suspend((): Schema.Codec<ApiActiveGpu> => ApiActiveGpu).annotate({ "description": "The GPU live sessions use right now (absent while nothing is streaming)." })], { mode: "oneOf" })), "encoder_pin": Schema.optionalKey(Schema.Union([Schema.String, Schema.Null]).annotate({ "description": "`SLIPSTREAM_ENCODER` (the host.env encoder pin), when set to something other than `auto`\n(e.g. `qsv`, `nvenc`, `amf`, `software`). A pin whose vendor contradicts the selected\nGPU is overridden at session open — the adapter wins — so the console can warn that the\npin is stale rather than letting the selection look broken." })), "env_override": Schema.optionalKey(Schema.Union([Schema.String, Schema.Null]).annotate({ "description": "`SLIPSTREAM_RENDER_ADAPTER` (the host.env pin), when set — it applies while `mode` is\n`auto`; a manual preference overrides it." })), "gpus": Schema.Array(ApiGpu).annotate({ "description": "The host's hardware GPUs." }), "mode": Schema.String.annotate({ "description": "`auto` or `manual`." }), "preferred_available": Schema.Boolean.annotate({ "description": "Whether the preferred GPU is currently present." }), "preferred_id": Schema.optionalKey(Schema.Union([Schema.String, Schema.Null]).annotate({ "description": "The manually preferred GPU's stable id, when one is stored (kept while `mode` is `auto` so\na console can offer returning to it). May reference a GPU that is currently absent." })), "preferred_name": Schema.optionalKey(Schema.Union([Schema.String, Schema.Null]).annotate({ "description": "The stored name of the preferred GPU (a usable label even when it is absent)." })), "selected": Schema.optionalKey(Schema.Union([Schema.Null, Schema.suspend((): Schema.Codec<ApiSelectedGpu> => ApiSelectedGpu).annotate({ "description": "The GPU the next session will use." })], { mode: "oneOf" })) }).annotate({ "description": "Full GPU-selection state for the console: inventory, the persisted preference, what the next\nsession will use, and what is in use right now.", "identifier": "GpuState" })
export type PreflightCheck = { readonly "detail": string, readonly "id": string, readonly "label": string, readonly "remediation"?: string | null, readonly "status": CheckStatus }
export const PreflightCheck = Schema.Struct({ "detail": Schema.String, "id": Schema.String.annotate({ "description": "Stable identifier suitable for UI filtering and support reports." }), "label": Schema.String, "remediation": Schema.optionalKey(Schema.Union([Schema.String, Schema.Null])), "status": CheckStatus }).annotate({ "identifier": "PreflightCheck" })
export type SessionSettings = { readonly "disconnect_grace_seconds"?: number, readonly "game_on_session_end"?: GameOnSessionEnd, readonly "session_on_game_exit"?: boolean, readonly "version"?: number }
export const SessionSettings = Schema.Struct({ "disconnect_grace_seconds": Schema.optionalKey(Schema.Number.annotate({ "description": "How long a vanished client has to reconnect before `Always` ends its game. Ignored by the\nother two policies.", "format": "int32" }).check(Schema.isInt().annotate({ "expected": "an integer" })).check(Schema.isGreaterThanOrEqualTo(0).annotate({ "expected": "a value greater than or equal to 0" }))), "game_on_session_end": Schema.optionalKey(Schema.suspend((): Schema.Codec<GameOnSessionEnd> => GameOnSessionEnd).annotate({ "description": "End the launched game when the session ends. See [`GameOnSessionEnd`]." })), "session_on_game_exit": Schema.optionalKey(Schema.Boolean.annotate({ "description": "End the streaming session when the launched game exits." })), "version": Schema.optionalKey(Schema.Number.annotate({ "format": "int32" }).check(Schema.isInt().annotate({ "expected": "an integer" })).check(Schema.isGreaterThanOrEqualTo(0).annotate({ "expected": "a value greater than or equal to 0" }))) }).annotate({ "description": "The persisted settings.", "identifier": "SessionSettings" })
export type KeepAlive = Objects_ | Objects_1 | Objects_2
export const KeepAlive = Schema.Union([Objects_, Objects_1, Objects_2], { mode: "oneOf" }).annotate({ "description": "How long a virtual display (and, on gamescope's bare spawn, the nested session + its game)\nsurvives after the last client session detaches. Serialized as an object tagged on `mode`\n(`{\"mode\":\"off\"}` / `{\"mode\":\"duration\",\"seconds\":300}` / `{\"mode\":\"forever\"}`) so the web form\nand the OpenAPI schema stay simple.", "identifier": "KeepAlive" })
export type GameEntry = { readonly "description"?: string | null, readonly "developer"?: string | null, readonly "genres"?: ReadonlyArray<string>, readonly "platform"?: string | null, readonly "players"?: number | null, readonly "publisher"?: string | null, readonly "region"?: string | null, readonly "release_year"?: number | null, readonly "tags"?: ReadonlyArray<string>, readonly "art": Artwork, readonly "id": string, readonly "launch"?: null | LaunchSpec, readonly "provider"?: string | null, readonly "store": string, readonly "title": string }
export const GameEntry = Schema.Struct({ "description": Schema.optionalKey(Schema.Union([Schema.String, Schema.Null]).annotate({ "description": "Short blurb for a details pane." })), "developer": Schema.optionalKey(Schema.Union([Schema.String, Schema.Null])), "genres": Schema.optionalKey(Schema.Array(Schema.String).annotate({ "description": "Genre taxonomy from the metadata source (`\"RPG\"`, `\"Platformer\"`, …)." })), "platform": Schema.optionalKey(Schema.Union([Schema.String, Schema.Null]).annotate({ "description": "The system the title runs on — `\"PS2\"`, `\"Xbox 360\"`, `\"SNES\"`, … Installed-store\nscanners stamp `\"PC\"`; `GET /library?platform=` filters on it (case-insensitive)." })), "players": Schema.optionalKey(Schema.Union([Schema.Number.check(Schema.isInt().annotate({ "expected": "an integer" })).check(Schema.isGreaterThanOrEqualTo(0).annotate({ "expected": "a value greater than or equal to 0" })), Schema.Null]).annotate({ "description": "Maximum simultaneous (local) players.", "format": "int32" })), "publisher": Schema.optionalKey(Schema.Union([Schema.String, Schema.Null])), "region": Schema.optionalKey(Schema.Union([Schema.String, Schema.Null]).annotate({ "description": "Release region — emulation-relevant (`\"NTSC-U\"`, `\"PAL\"`, `\"NTSC-J\"`)." })), "release_year": Schema.optionalKey(Schema.Union([Schema.Number.check(Schema.isInt().annotate({ "expected": "an integer" })).check(Schema.isGreaterThanOrEqualTo(0).annotate({ "expected": "a value greater than or equal to 0" })), Schema.Null]).annotate({ "description": "Year of first release — the granularity metadata sources reliably agree on.", "format": "int32" })), "tags": Schema.optionalKey(Schema.Array(Schema.String).annotate({ "description": "Free-form organizational labels (`\"co-op\"`, `\"kids\"`, `\"finished\"`, …)." })), "art": Artwork, "id": Schema.String.annotate({ "description": "Stable, store-qualified id: `steam:<appid>` or `custom:<id>`." }), "launch": Schema.optionalKey(Schema.Union([Schema.Null, Schema.suspend((): Schema.Codec<LaunchSpec> => LaunchSpec).annotate({ "description": "How the host would launch it, when known." })], { mode: "oneOf" })), "provider": Schema.optionalKey(Schema.Union([Schema.String, Schema.Null]).annotate({ "description": "The external provider owning this entry (custom-store entries synced by a provider\nplugin, RFC §8) — `None` for installed-store titles and manual custom entries. The\nconsole uses it for attribution; `GET /library?provider=` filters on it." })), "store": Schema.String.annotate({ "description": "Which store surfaced it: `\"steam\"` or `\"custom\"`." }), "title": Schema.String }).annotate({ "description": "Descriptive metadata, flattened — see [`GameMeta`].", "identifier": "GameEntry" })
export type LogPage = { readonly "dropped": boolean, readonly "entries": ReadonlyArray<LogEntry>, readonly "next": number }
export const LogPage = Schema.Struct({ "dropped": Schema.Boolean.annotate({ "description": "True when entries between `after` and the first returned one were already evicted." }), "entries": Schema.Array(LogEntry), "next": Schema.Number.annotate({ "description": "Cursor for the next poll (the last returned seq, or the request's `after` when empty).", "format": "int64" }).check(Schema.isInt().annotate({ "expected": "an integer" })).check(Schema.isGreaterThanOrEqualTo(0).annotate({ "expected": "a value greater than or equal to 0" })) }).annotate({ "description": "One poll's worth of log entries.", "identifier": "LogPage" })
export type HostConfigFile = { readonly "audio_video"?: AudioVideoConfig, readonly "clipboard"?: ClipboardPolicy, readonly "encoders"?: EncoderConfig, readonly "general"?: GeneralConfig, readonly "input"?: InputConfig, readonly "latency_profile"?: LatencyProfile, readonly "network"?: NetworkConfig, readonly "network_policy"?: NetworkPolicy, readonly "performance_profile"?: PerformanceProfile, readonly "version"?: number }
export const HostConfigFile = Schema.Struct({ "audio_video": Schema.optionalKey(AudioVideoConfig), "clipboard": Schema.optionalKey(Schema.suspend((): Schema.Codec<ClipboardPolicy> => ClipboardPolicy).annotate({ "description": "Host clipboard policy. The client must also enable clipboard sharing for a session." })), "encoders": Schema.optionalKey(EncoderConfig), "general": Schema.optionalKey(GeneralConfig), "input": Schema.optionalKey(InputConfig), "latency_profile": Schema.optionalKey(Schema.suspend((): Schema.Codec<LatencyProfile> => LatencyProfile).annotate({ "description": "Named encoder latency profile (`SLIPSTREAM_LATENCY_PROFILE`)." })), "network": Schema.optionalKey(NetworkConfig), "network_policy": Schema.optionalKey(Schema.suspend((): Schema.Codec<NetworkPolicy> => NetworkPolicy).annotate({ "description": "Named transport starting policy (`SLIPSTREAM_NETWORK_POLICY`)." })), "performance_profile": Schema.optionalKey(Schema.suspend((): Schema.Codec<PerformanceProfile> => PerformanceProfile).annotate({ "description": "Named worker scheduling profile (`SLIPSTREAM_PERFORMANCE_PROFILE`)." })), "version": Schema.optionalKey(Schema.Number.annotate({ "format": "int32" }).check(Schema.isInt().annotate({ "expected": "an integer" })).check(Schema.isGreaterThanOrEqualTo(0).annotate({ "expected": "a value greater than or equal to 0" }))) }).annotate({ "description": "Sunshine-shaped host settings the console can toggle without editing env files by hand.", "identifier": "HostConfigFile" })
export type ActiveGame = { readonly "app_id"?: string | null, readonly "client": string, readonly "grace_remaining_s"?: number | null, readonly "plane": Plane, readonly "session_id"?: number | null, readonly "state": string, readonly "store"?: string | null, readonly "title": string }
export const ActiveGame = Schema.Struct({ "app_id": Schema.optionalKey(Schema.Union([Schema.String, Schema.Null]).annotate({ "description": "Store-qualified library id (`steam:570`) — the key the console matches against `GET /library`\nto show box art. Absent for an operator-typed GameStream command." })), "client": Schema.String.annotate({ "description": "Client-supplied device name of the session that launched it; may be empty." }), "grace_remaining_s": Schema.optionalKey(Schema.Union([Schema.Number.check(Schema.isInt().annotate({ "expected": "an integer" })).check(Schema.isGreaterThanOrEqualTo(0).annotate({ "expected": "a value greater than or equal to 0" })), Schema.Null]).annotate({ "description": "Seconds until this game is ended — only present on a `grace` row.", "format": "int64" })), "plane": Schema.suspend((): Schema.Codec<Plane> => Plane).annotate({ "description": "`native` or `gamestream`." }), "session_id": Schema.optionalKey(Schema.Union([Schema.Number.check(Schema.isInt().annotate({ "expected": "an integer" })).check(Schema.isGreaterThanOrEqualTo(0).annotate({ "expected": "a value greater than or equal to 0" })), Schema.Null]).annotate({ "description": "The session streaming it; `null` for a game waiting out its reconnect window.", "format": "int64" })), "state": Schema.String.annotate({ "description": "`launching` (launched, not seen running yet), `running`, `exited`, or `grace` (its session is\ngone and it will be ended when the reconnect window closes)." }), "store": Schema.optionalKey(Schema.Union([Schema.String, Schema.Null]).annotate({ "description": "Which store surfaced it (`steam`, `heroic`, `custom`, …), when known." })), "title": Schema.String.annotate({ "description": "Display title." }) }).annotate({ "description": "One launched game, for the console's running-game card.", "identifier": "ActiveGame" })
export type ClientRef = { readonly "fingerprint"?: string | null, readonly "name": string, readonly "plane": Plane }
export const ClientRef = Schema.Struct({ "fingerprint": Schema.optionalKey(Schema.Union([Schema.String, Schema.Null]).annotate({ "description": "Hex SHA-256 certificate fingerprint, when the client presented one." })), "name": Schema.String.annotate({ "description": "Client-supplied device name; may be empty (an anonymous or compat-plane client)." }), "plane": Plane }).annotate({ "description": "The connecting/disconnecting client's identity.", "identifier": "ClientRef" })
export type DeviceRef = { readonly "fingerprint": string, readonly "name": string, readonly "plane": Plane }
export const DeviceRef = Schema.Struct({ "fingerprint": Schema.String.annotate({ "description": "Hex certificate fingerprint." }), "name": Schema.String.annotate({ "description": "Sanitized device name (the pairing store's copy)." }), "plane": Plane }).annotate({ "description": "A device in the pairing flow.", "identifier": "DeviceRef" })
export type GameRefPayload = { readonly "app"?: string | null, readonly "client": string, readonly "plane": Plane, readonly "store"?: string | null, readonly "title": string }
export const GameRefPayload = Schema.Struct({ "app": Schema.optionalKey(Schema.Union([Schema.String, Schema.Null]).annotate({ "description": "Store-qualified library id (`steam:570`). Absent for an operator-typed GameStream\n`apps.json` command, which has no library entry behind it." })), "client": Schema.String.annotate({ "description": "Client-supplied device name of the session that launched it; may be empty." }), "plane": Plane, "store": Schema.optionalKey(Schema.Union([Schema.String, Schema.Null]).annotate({ "description": "Which store surfaced it (`steam`, `heroic`, `custom`, …), when known." })), "title": Schema.String.annotate({ "description": "Display title." }) }).annotate({ "description": "A launched game, as the `game.*` events see it.", "identifier": "GameRefPayload" })
export type HookFilter = { readonly "app"?: string | null, readonly "client"?: string | null, readonly "fingerprint"?: string | null, readonly "plane"?: null | Plane }
export const HookFilter = Schema.Struct({ "app": Schema.optionalKey(Schema.Union([Schema.String, Schema.Null]).annotate({ "description": "Launched app id/title (`stream.*` events)." })), "client": Schema.optionalKey(Schema.Union([Schema.String, Schema.Null]).annotate({ "description": "Client/device name (for `session.*`: the short client label the Dashboard shows)." })), "fingerprint": Schema.optionalKey(Schema.Union([Schema.String, Schema.Null]).annotate({ "description": "Certificate fingerprint (hex, case-insensitive)." })), "plane": Schema.optionalKey(Schema.Union([Schema.Null, Schema.suspend((): Schema.Codec<Plane> => Plane).annotate({ "description": "Protocol plane (`native` / `gamestream`)." })], { mode: "oneOf" })) }).annotate({ "description": "Exact-match filters against an event's identity fields (RFC open-question 3: exact match\nonly — anything richer is what the SDK is for). Absent fields don't constrain; a filter\nfield set on an event kind that doesn't carry it (e.g. `client` on `host.started`) never\nmatches.", "identifier": "HookFilter" })
export type StreamRef = { readonly "app"?: string | null, readonly "client": string, readonly "hdr": boolean, readonly "mode": string, readonly "plane": Plane }
export const StreamRef = Schema.Struct({ "app": Schema.optionalKey(Schema.Union([Schema.String, Schema.Null]).annotate({ "description": "The launched app/title for this stream, when one was requested (store-qualified id on\nthe native plane, app title on the GameStream plane)." })), "client": Schema.String.annotate({ "description": "Client-supplied device name; may be empty." }), "hdr": Schema.Boolean, "mode": Schema.String.annotate({ "description": "Negotiated mode, `WxH@Hz`." }), "plane": Plane }).annotate({ "description": "A live video stream (what the stream marker file reflects).", "identifier": "StreamRef" })
export type PluginRegistration = { readonly "title": string, readonly "ui"?: null | PluginUi, readonly "version"?: string | null }
export const PluginRegistration = Schema.Struct({ "title": Schema.String.annotate({ "description": "Human-readable title for the console nav entry (1–64 chars; control chars stripped)." }), "ui": Schema.optionalKey(Schema.Union([Schema.Null, Schema.suspend((): Schema.Codec<PluginUi> => PluginUi).annotate({ "description": "Present iff the plugin serves a UI surface. A registration with no `ui` is a liveness/phone-book\nentry only (e.g. a future runner-management listing) and grows no nav entry." })], { mode: "oneOf" })), "version": Schema.optionalKey(Schema.Union([Schema.String, Schema.Null]).annotate({ "description": "Optional plugin version, purely informational (≤32 chars)." })) }).annotate({ "description": "Register/renew body for `PUT /plugins/{id}`.", "identifier": "PluginRegistration" })
export type PluginSummary = { readonly "id": string, readonly "title": string, readonly "ui"?: null | PluginUiPublic, readonly "version"?: string | null }
export const PluginSummary = Schema.Struct({ "id": Schema.String, "title": Schema.String, "ui": Schema.optionalKey(Schema.Union([Schema.Null, PluginUiPublic], { mode: "oneOf" })), "version": Schema.optionalKey(Schema.Union([Schema.String, Schema.Null])) }).annotate({ "description": "One entry in `GET /plugins`. **Never carries the secret** — the browser learns a plugin exists\nand has a UI, nothing that lets it reach the plugin directly (it goes through the console proxy).", "identifier": "PluginSummary" })
export type HostInfo = { readonly "abi_version": number, readonly "app_version": string, readonly "codecs": ReadonlyArray<ApiCodec>, readonly "gamestream": boolean, readonly "gfe_version": string, readonly "hostname": string, readonly "local_ip": string, readonly "os": string, readonly "os_name": string, readonly "ports": PortMap, readonly "uniqueid": string, readonly "version": string }
export const HostInfo = Schema.Struct({ "abi_version": Schema.Number.annotate({ "description": "`slipstream-core` C ABI version.", "format": "int32" }).check(Schema.isInt().annotate({ "expected": "an integer" })).check(Schema.isGreaterThanOrEqualTo(0).annotate({ "expected": "a value greater than or equal to 0" })), "app_version": Schema.String.annotate({ "description": "GameStream host version advertised to Moonlight clients." }), "codecs": Schema.Array(ApiCodec).annotate({ "description": "Codecs the host can encode (NVENC)." }), "gamestream": Schema.Boolean.annotate({ "description": "Whether the GameStream/Moonlight-compat planes are running (`--gamestream`). `false` on the\nsecure default (native slipstream/1 only) — a console can hide Moonlight-only UI (e.g. the\nMoonlight PIN pairing card, which could never receive a PIN when this is `false`)." }), "gfe_version": Schema.String.annotate({ "description": "GFE version advertised to Moonlight clients." }), "hostname": Schema.String, "local_ip": Schema.String.annotate({ "description": "Best-effort primary LAN IP." }), "os": Schema.String.annotate({ "description": "Linux OS identity chain, generic → most specific, slash-separated\n(`linux[/<family>][/<id>]`). A client walks it most-specific-first and shows the first\ntoken it has an icon for, so an unknown distro still degrades to its family's mark." }), "os_name": Schema.String.annotate({ "description": "Human-readable OS name from os-release `PRETTY_NAME`." }), "ports": PortMap, "uniqueid": Schema.String.annotate({ "description": "Stable per-host id (persisted across restarts), matched on pairing." }), "version": Schema.String.annotate({ "description": "`slipstream-host` crate version." }) }).annotate({ "description": "Host identity and advertised capabilities (static for the life of the process).", "identifier": "HostInfo" })
export type DisplayLayoutRequest = { readonly "positions"?: { readonly [x: string]: Position } }
export const DisplayLayoutRequest = Schema.Struct({ "positions": Schema.optionalKey(Schema.Record(Schema.String, Position).annotate({ "description": "`{\"<identity_slot>\": {\"x\": …, \"y\": …}}` — where each arranged display's top-left sits." }).check(Schema.isPropertyNames(Schema.String).annotate({ "expected": "an object with property names matching the schema" }))) }).annotate({ "description": "Request body for `setDisplayLayout`: per-identity-slot desktop offsets, keyed by the identity-slot\nid as a string (the same id `/display/state` reports as `identity_slot`).", "identifier": "DisplayLayoutRequest" })
export type Objects_3 = { readonly [x: string]: Position }
export const Objects_3 = Schema.Record(Schema.String, Position).check(Schema.isPropertyNames(Schema.String).annotate({ "expected": "an object with property names matching the schema" }))
export type CustomEntry = { readonly "description"?: string | null, readonly "developer"?: string | null, readonly "genres"?: ReadonlyArray<string>, readonly "platform"?: string | null, readonly "players"?: number | null, readonly "publisher"?: string | null, readonly "region"?: string | null, readonly "release_year"?: number | null, readonly "tags"?: ReadonlyArray<string>, readonly "art"?: Artwork, readonly "detect"?: DetectHint, readonly "external_id"?: string | null, readonly "id": string, readonly "launch"?: null | LaunchSpec, readonly "prep"?: ReadonlyArray<PrepCmd>, readonly "provider"?: string | null, readonly "title": string }
export const CustomEntry = Schema.Struct({ "description": Schema.optionalKey(Schema.Union([Schema.String, Schema.Null]).annotate({ "description": "Short blurb for a details pane." })), "developer": Schema.optionalKey(Schema.Union([Schema.String, Schema.Null])), "genres": Schema.optionalKey(Schema.Array(Schema.String).annotate({ "description": "Genre taxonomy from the metadata source (`\"RPG\"`, `\"Platformer\"`, …)." })), "platform": Schema.optionalKey(Schema.Union([Schema.String, Schema.Null]).annotate({ "description": "The system the title runs on — `\"PS2\"`, `\"Xbox 360\"`, `\"SNES\"`, … Installed-store\nscanners stamp `\"PC\"`; `GET /library?platform=` filters on it (case-insensitive)." })), "players": Schema.optionalKey(Schema.Union([Schema.Number.check(Schema.isInt().annotate({ "expected": "an integer" })).check(Schema.isGreaterThanOrEqualTo(0).annotate({ "expected": "a value greater than or equal to 0" })), Schema.Null]).annotate({ "description": "Maximum simultaneous (local) players.", "format": "int32" })), "publisher": Schema.optionalKey(Schema.Union([Schema.String, Schema.Null])), "region": Schema.optionalKey(Schema.Union([Schema.String, Schema.Null]).annotate({ "description": "Release region — emulation-relevant (`\"NTSC-U\"`, `\"PAL\"`, `\"NTSC-J\"`)." })), "release_year": Schema.optionalKey(Schema.Union([Schema.Number.check(Schema.isInt().annotate({ "expected": "an integer" })).check(Schema.isGreaterThanOrEqualTo(0).annotate({ "expected": "a value greater than or equal to 0" })), Schema.Null]).annotate({ "description": "Year of first release — the granularity metadata sources reliably agree on.", "format": "int32" })), "tags": Schema.optionalKey(Schema.Array(Schema.String).annotate({ "description": "Free-form organizational labels (`\"co-op\"`, `\"kids\"`, `\"finished\"`, …)." })), "art": Schema.optionalKey(Artwork), "detect": Schema.optionalKey(Schema.suspend((): Schema.Codec<DetectHint> => DetectHint).annotate({ "description": "How to recognize this title's process once it is running (design §9) — the one thing a\nprovider knows that the host cannot work out for itself.\n\nOptional: without it the entry is still tracked by the child the host spawns for it, which\ncovers every command that stays in the foreground. It earns its keep for a command that hands\noff and exits — a launcher script, a `flatpak run`, a front-end that starts an emulator — where\nthe host would otherwise lose the game the moment the shim returns." })), "external_id": Schema.optionalKey(Schema.Union([Schema.String, Schema.Null]).annotate({ "description": "The provider's own stable key for this title — the reconcile diff key, so the\nhost-assigned `id` stays stable across reconciles. Present iff `provider` is." })), "id": Schema.String.annotate({ "description": "Host-assigned, stable for the life of the entry (the `{id}` in the CRUD path)." }), "launch": Schema.optionalKey(Schema.Union([Schema.Null, LaunchSpec], { mode: "oneOf" })), "prep": Schema.optionalKey(Schema.Array(PrepCmd).annotate({ "description": "Per-title prep/undo steps (RFC §6): each `do` runs before this title launches, each\n`undo` at session end in reverse order (see [`crate::hooks::run_prep`])." })), "provider": Schema.optionalKey(Schema.Union([Schema.String, Schema.Null]).annotate({ "description": "The external provider owning this entry (RFC §8), set ONLY by the provider reconcile\nAPI — `None` = a manual entry, which no provider operation ever touches, and which the\nmanual CRUD alone may edit (the converse holds too: manual CRUD refuses provider-owned\nentries, so ownership is never ambiguous)." })), "title": Schema.String }).annotate({ "description": "Descriptive metadata (platform, description, …), flattened — see [`GameMeta`].", "identifier": "CustomEntry" })
export type CustomInput = { readonly "description"?: string | null, readonly "developer"?: string | null, readonly "genres"?: ReadonlyArray<string>, readonly "platform"?: string | null, readonly "players"?: number | null, readonly "publisher"?: string | null, readonly "region"?: string | null, readonly "release_year"?: number | null, readonly "tags"?: ReadonlyArray<string>, readonly "art"?: Artwork, readonly "detect"?: DetectHint, readonly "launch"?: null | LaunchSpec, readonly "prep"?: ReadonlyArray<PrepCmd>, readonly "title": string }
export const CustomInput = Schema.Struct({ "description": Schema.optionalKey(Schema.Union([Schema.String, Schema.Null]).annotate({ "description": "Short blurb for a details pane." })), "developer": Schema.optionalKey(Schema.Union([Schema.String, Schema.Null])), "genres": Schema.optionalKey(Schema.Array(Schema.String).annotate({ "description": "Genre taxonomy from the metadata source (`\"RPG\"`, `\"Platformer\"`, …)." })), "platform": Schema.optionalKey(Schema.Union([Schema.String, Schema.Null]).annotate({ "description": "The system the title runs on — `\"PS2\"`, `\"Xbox 360\"`, `\"SNES\"`, … Installed-store\nscanners stamp `\"PC\"`; `GET /library?platform=` filters on it (case-insensitive)." })), "players": Schema.optionalKey(Schema.Union([Schema.Number.check(Schema.isInt().annotate({ "expected": "an integer" })).check(Schema.isGreaterThanOrEqualTo(0).annotate({ "expected": "a value greater than or equal to 0" })), Schema.Null]).annotate({ "description": "Maximum simultaneous (local) players.", "format": "int32" })), "publisher": Schema.optionalKey(Schema.Union([Schema.String, Schema.Null])), "region": Schema.optionalKey(Schema.Union([Schema.String, Schema.Null]).annotate({ "description": "Release region — emulation-relevant (`\"NTSC-U\"`, `\"PAL\"`, `\"NTSC-J\"`)." })), "release_year": Schema.optionalKey(Schema.Union([Schema.Number.check(Schema.isInt().annotate({ "expected": "an integer" })).check(Schema.isGreaterThanOrEqualTo(0).annotate({ "expected": "a value greater than or equal to 0" })), Schema.Null]).annotate({ "description": "Year of first release — the granularity metadata sources reliably agree on.", "format": "int32" })), "tags": Schema.optionalKey(Schema.Array(Schema.String).annotate({ "description": "Free-form organizational labels (`\"co-op\"`, `\"kids\"`, `\"finished\"`, …)." })), "art": Schema.optionalKey(Artwork), "detect": Schema.optionalKey(Schema.suspend((): Schema.Codec<DetectHint> => DetectHint).annotate({ "description": "How to recognize this title's process — see [`CustomEntry::detect`]." })), "launch": Schema.optionalKey(Schema.Union([Schema.Null, LaunchSpec], { mode: "oneOf" })), "prep": Schema.optionalKey(Schema.Array(PrepCmd).annotate({ "description": "Per-title prep/undo steps — commands run as the host user; operator-privileged config." })), "title": Schema.String }).annotate({ "description": "Descriptive metadata (platform, description, …), flattened — see [`GameMeta`]. Replaced\nwholesale on update, like `art`: an edit must round-trip every field it wants kept.", "identifier": "CustomInput" })
export type ProviderEntryInput = { readonly "description"?: string | null, readonly "developer"?: string | null, readonly "genres"?: ReadonlyArray<string>, readonly "platform"?: string | null, readonly "players"?: number | null, readonly "publisher"?: string | null, readonly "region"?: string | null, readonly "release_year"?: number | null, readonly "tags"?: ReadonlyArray<string>, readonly "art"?: Artwork, readonly "detect"?: DetectHint, readonly "external_id": string, readonly "launch"?: null | LaunchSpec, readonly "prep"?: ReadonlyArray<PrepCmd>, readonly "title": string }
export const ProviderEntryInput = Schema.Struct({ "description": Schema.optionalKey(Schema.Union([Schema.String, Schema.Null]).annotate({ "description": "Short blurb for a details pane." })), "developer": Schema.optionalKey(Schema.Union([Schema.String, Schema.Null])), "genres": Schema.optionalKey(Schema.Array(Schema.String).annotate({ "description": "Genre taxonomy from the metadata source (`\"RPG\"`, `\"Platformer\"`, …)." })), "platform": Schema.optionalKey(Schema.Union([Schema.String, Schema.Null]).annotate({ "description": "The system the title runs on — `\"PS2\"`, `\"Xbox 360\"`, `\"SNES\"`, … Installed-store\nscanners stamp `\"PC\"`; `GET /library?platform=` filters on it (case-insensitive)." })), "players": Schema.optionalKey(Schema.Union([Schema.Number.check(Schema.isInt().annotate({ "expected": "an integer" })).check(Schema.isGreaterThanOrEqualTo(0).annotate({ "expected": "a value greater than or equal to 0" })), Schema.Null]).annotate({ "description": "Maximum simultaneous (local) players.", "format": "int32" })), "publisher": Schema.optionalKey(Schema.Union([Schema.String, Schema.Null])), "region": Schema.optionalKey(Schema.Union([Schema.String, Schema.Null]).annotate({ "description": "Release region — emulation-relevant (`\"NTSC-U\"`, `\"PAL\"`, `\"NTSC-J\"`)." })), "release_year": Schema.optionalKey(Schema.Union([Schema.Number.check(Schema.isInt().annotate({ "expected": "an integer" })).check(Schema.isGreaterThanOrEqualTo(0).annotate({ "expected": "a value greater than or equal to 0" })), Schema.Null]).annotate({ "description": "Year of first release — the granularity metadata sources reliably agree on.", "format": "int32" })), "tags": Schema.optionalKey(Schema.Array(Schema.String).annotate({ "description": "Free-form organizational labels (`\"co-op\"`, `\"kids\"`, `\"finished\"`, …)." })), "art": Schema.optionalKey(Artwork), "detect": Schema.optionalKey(Schema.suspend((): Schema.Codec<DetectHint> => DetectHint).annotate({ "description": "How to recognize this title's process — see [`CustomEntry::detect`]. A provider that knows its\ntitles' install directories (Playnite does) should send them: it is what lets a game launched\nthrough the provider's own client still end its session when the player quits." })), "external_id": Schema.String.annotate({ "description": "The provider's stable id for this title (the reconcile diff key)." }), "launch": Schema.optionalKey(Schema.Union([Schema.Null, LaunchSpec], { mode: "oneOf" })), "prep": Schema.optionalKey(Schema.Array(PrepCmd).annotate({ "description": "Per-title prep/undo steps — commands run as the host user; operator-privileged config." })), "title": Schema.String }).annotate({ "description": "Descriptive metadata (platform, description, …), flattened — see [`GameMeta`].", "identifier": "ProviderEntryInput" })
export type LocalSummary = { readonly "audio_streaming": boolean, readonly "client_name"?: string | null, readonly "conflicts"?: ReadonlyArray<string>, readonly "games"?: ReadonlyArray<string>, readonly "kept_displays": number, readonly "native_paired_clients": number, readonly "paired_clients": number, readonly "pending_approvals": number, readonly "pin_pending": boolean, readonly "session"?: null | SessionInfo, readonly "version": string, readonly "video_streaming": boolean }
export const LocalSummary = Schema.Struct({ "audio_streaming": Schema.Boolean.annotate({ "description": "True while audio is streaming on either plane (same rule as `video_streaming`)." }), "client_name": Schema.optionalKey(Schema.Union([Schema.String, Schema.Null]).annotate({ "description": "Display name of the (first) streaming native client — the trust store's name for it, else\nthe name the device sent at connect. `null` when idle, for a nameless client, or for a\nGameStream session (that plane carries no device name)." })), "conflicts": Schema.optionalKey(Schema.Array(Schema.String).annotate({ "description": "Other Moonlight-compatible hosts (Sunshine/Apollo/...) whose process is running now.\nSide-by-side is unsupported. Compact labels (e.g. `Sunshine (running)`); install-only /\nservice-registered hits stay out of this field (see `detect-conflicts` / `render_report`)." })), "games": Schema.optionalKey(Schema.Array(Schema.String).annotate({ "description": "Launched games the host is tracking, as compact labels (`Hades`, `Hades (closing in 4:12)`).\n\nThe countdown form is the one that matters: it means the game's client is gone and the host\nwill end the game when the window closes — something a user at the machine should be able to\nsee (and stop) without opening the console. Empty when nothing was launched." })), "kept_displays": Schema.Number.annotate({ "description": "Virtual displays being KEPT with no live session — lingering (keep-alive window) or pinned\n(`keep_alive: forever`). Non-zero means a display (and, exclusive, your physical monitors) is\nheld; the tray surfaces it + a one-click release. Active (in-use) displays are not counted.", "format": "int32" }).check(Schema.isInt().annotate({ "expected": "an integer" })).check(Schema.isGreaterThanOrEqualTo(0).annotate({ "expected": "a value greater than or equal to 0" })), "native_paired_clients": Schema.Number.annotate({ "description": "Number of paired native (slipstream/1) devices.", "format": "int32" }).check(Schema.isInt().annotate({ "expected": "an integer" })).check(Schema.isGreaterThanOrEqualTo(0).annotate({ "expected": "a value greater than or equal to 0" })), "paired_clients": Schema.Number.annotate({ "description": "Number of pinned (paired) GameStream client certificates.", "format": "int32" }).check(Schema.isInt().annotate({ "expected": "an integer" })).check(Schema.isGreaterThanOrEqualTo(0).annotate({ "expected": "a value greater than or equal to 0" })), "pending_approvals": Schema.Number.annotate({ "description": "Native pairing knocks awaiting the operator's approval (count only).", "format": "int32" }).check(Schema.isInt().annotate({ "expected": "an integer" })).check(Schema.isGreaterThanOrEqualTo(0).annotate({ "expected": "a value greater than or equal to 0" })), "pin_pending": Schema.Boolean.annotate({ "description": "True while a GameStream pairing handshake is parked waiting for the user's PIN." }), "session": Schema.optionalKey(Schema.Union([Schema.Null, Schema.suspend((): Schema.Codec<SessionInfo> => SessionInfo).annotate({ "description": "The active session: GameStream's launch (Moonlight `/launch`) when present, else the first\nlive native session. `null` when nothing is streaming." })], { mode: "oneOf" })), "version": Schema.String.annotate({ "description": "Host version (mirrors `/health`)." }), "video_streaming": Schema.Boolean.annotate({ "description": "True while video is streaming on EITHER plane: the GameStream media pipeline, or a live\nnative (slipstream/1) session — the default plane, invisible in the GameStream flag alone." }) }).annotate({ "description": "Non-sensitive host status for the local tray icon: counts and booleans — no PIN values, no\nfingerprints. The ONE name exposed is `client_name`, the streaming client's display label\n(deliberate loosening for the tray's \"client connected\" toast: it tells the local user who is\non their machine, which is disclosure in the user's favor — and any local process could\nalready infer a session exists from the booleans here). Served unauthenticated to LOOPBACK\npeers only (see `require_auth`): this narrow read-only route is intended for local status\ndisplays.", "identifier": "LocalSummary" })
export type CatalogResponse = { readonly "busy": boolean, readonly "host": HostFacts, readonly "plugins": ReadonlyArray<CatalogEntry>, readonly "sources": ReadonlyArray<SourceView> }
export const CatalogResponse = Schema.Struct({ "busy": Schema.Boolean.annotate({ "description": "True while a package operation is in flight — the console disables install buttons." }), "host": HostFacts, "plugins": Schema.Array(CatalogEntry), "sources": Schema.Array(SourceView) }).annotate({ "identifier": "CatalogResponse" })
export type StatsSample = { readonly "bitrate_kbps": number, readonly "capture_age_over_limit"?: boolean, readonly "capture_age_us"?: number, readonly "capture_backend"?: string, readonly "capture_buffers_drained"?: number, readonly "capture_frames_overwritten"?: number, readonly "capture_frames_published"?: number, readonly "capture_height"?: number, readonly "capture_modifier"?: number, readonly "capture_width"?: number, readonly "fec_recovered": number, readonly "fps": number, readonly "frames_dropped": number, readonly "mbps": number, readonly "packets_dropped": number, readonly "repeat_fps": number, readonly "send_dropped": number, readonly "session_id": number, readonly "stages": ReadonlyArray<StageTiming>, readonly "t_ms": number }
export const StatsSample = Schema.Struct({ "bitrate_kbps": Schema.Number.annotate({ "description": "Configured target bitrate.", "format": "int32" }).check(Schema.isInt().annotate({ "expected": "an integer" })).check(Schema.isGreaterThanOrEqualTo(0).annotate({ "expected": "a value greater than or equal to 0" })), "capture_age_over_limit": Schema.optionalKey(Schema.Boolean.annotate({ "description": "Whether `capture_age_us` exceeded `SLIPSTREAM_CAPTURE_MAX_AGE_MS` at this boundary." })), "capture_age_us": Schema.optionalKey(Schema.Number.annotate({ "description": "Age of the newest source frame when the statistics boundary was recorded. This is the\ncapture-side age only, before network and client presentation latency.", "format": "int32" }).check(Schema.isInt().annotate({ "expected": "an integer" })).check(Schema.isGreaterThanOrEqualTo(0).annotate({ "expected": "a value greater than or equal to 0" }))), "capture_backend": Schema.optionalKey(Schema.String.annotate({ "description": "Stable capture adapter label, for example `pipewire`, `wlr-screencopy`, or `x11-getimage`." })), "capture_buffers_drained": Schema.optionalKey(Schema.Number.annotate({ "description": "Cumulative extra PipeWire buffers discarded while selecting the newest buffer.", "format": "int64" }).check(Schema.isInt().annotate({ "expected": "an integer" })).check(Schema.isGreaterThanOrEqualTo(0).annotate({ "expected": "a value greater than or equal to 0" }))), "capture_frames_overwritten": Schema.optionalKey(Schema.Number.annotate({ "description": "Cumulative source-side frame overwrites at the statistics boundary. A rising value means\nthe encoder is not consuming the one-deep newest-frame slot quickly enough.", "format": "int64" }).check(Schema.isInt().annotate({ "expected": "an integer" })).check(Schema.isGreaterThanOrEqualTo(0).annotate({ "expected": "a value greater than or equal to 0" }))), "capture_frames_published": Schema.optionalKey(Schema.Number.annotate({ "description": "Frames published by the source since the capturer opened.", "format": "int64" }).check(Schema.isInt().annotate({ "expected": "an integer" })).check(Schema.isGreaterThanOrEqualTo(0).annotate({ "expected": "a value greater than or equal to 0" }))), "capture_height": Schema.optionalKey(Schema.Number.annotate({ "format": "int32" }).check(Schema.isInt().annotate({ "expected": "an integer" })).check(Schema.isGreaterThanOrEqualTo(0).annotate({ "expected": "a value greater than or equal to 0" }))), "capture_modifier": Schema.optionalKey(Schema.Number.annotate({ "description": "Negotiated source modifier, or zero for linear/unknown.", "format": "int64" }).check(Schema.isInt().annotate({ "expected": "an integer" })).check(Schema.isGreaterThanOrEqualTo(0).annotate({ "expected": "a value greater than or equal to 0" }))), "capture_width": Schema.optionalKey(Schema.Number.annotate({ "description": "Negotiated source dimensions. Zero means the backend has not reported them.", "format": "int32" }).check(Schema.isInt().annotate({ "expected": "an integer" })).check(Schema.isGreaterThanOrEqualTo(0).annotate({ "expected": "a value greater than or equal to 0" }))), "fec_recovered": Schema.Number.annotate({ "description": "FEC shards recovered this window (delta).", "format": "int32" }).check(Schema.isInt().annotate({ "expected": "an integer" })).check(Schema.isGreaterThanOrEqualTo(0).annotate({ "expected": "a value greater than or equal to 0" })), "fps": Schema.Number.annotate({ "description": "Genuine NEW frames/s from the source.", "format": "float" }).check(Schema.isFinite().annotate({ "expected": "a finite number" })), "frames_dropped": Schema.Number.annotate({ "description": "Frames dropped this window (delta).", "format": "int32" }).check(Schema.isInt().annotate({ "expected": "an integer" })).check(Schema.isGreaterThanOrEqualTo(0).annotate({ "expected": "a value greater than or equal to 0" })), "mbps": Schema.Number.annotate({ "description": "Attempted sealed wire bytes/s (Mb/s): full UDP payloads at seal time — video AU bytes\nplus shard framing (header + AEAD) plus FEC parity, and for PyroWave's datagram-aligned\nmode the zero-padded window tails. NOT goodput, and NOT reduced by socket send drops.", "format": "float" }).check(Schema.isFinite().annotate({ "expected": "a finite number" })), "packets_dropped": Schema.Number.annotate({ "description": "Packets dropped this window (receiver-side / reassembler, where known).", "format": "int32" }).check(Schema.isInt().annotate({ "expected": "an integer" })).check(Schema.isGreaterThanOrEqualTo(0).annotate({ "expected": "a value greater than or equal to 0" })), "repeat_fps": Schema.Number.annotate({ "description": "Re-encoded holds/s (source-starvation indicator).", "format": "float" }).check(Schema.isFinite().annotate({ "expected": "a finite number" })), "send_dropped": Schema.Number.annotate({ "description": "Host send-buffer overflow / EAGAIN this window (delta).", "format": "int32" }).check(Schema.isInt().annotate({ "expected": "an integer" })).check(Schema.isGreaterThanOrEqualTo(0).annotate({ "expected": "a value greater than or equal to 0" })), "session_id": Schema.Number.annotate({ "description": "Disambiguates concurrent sessions (usually constant).", "format": "int32" }).check(Schema.isInt().annotate({ "expected": "an integer" })).check(Schema.isGreaterThanOrEqualTo(0).annotate({ "expected": "a value greater than or equal to 0" })), "stages": Schema.Array(StageTiming).annotate({ "description": "Ordered pipeline stages for this path." }), "t_ms": Schema.Number.annotate({ "description": "Milliseconds since capture start (monotonic; stamped by [`StatsRecorder::push_sample`]).", "format": "int64" }).check(Schema.isInt().annotate({ "expected": "an integer" })).check(Schema.isGreaterThanOrEqualTo(0).annotate({ "expected": "a value greater than or equal to 0" })) }).annotate({ "description": "One aggregated sample (~ every 2 s native, ~ every 1 s GameStream).", "identifier": "StatsSample" })
export type Job = { readonly "error"?: string | null, readonly "finished_at"?: number | null, readonly "id": string, readonly "kind": string, readonly "log": ReadonlyArray<string>, readonly "phase": string, readonly "started_at": number, readonly "state": State, readonly "target": string }
export const Job = Schema.Struct({ "error": Schema.optionalKey(Schema.Union([Schema.String, Schema.Null])), "finished_at": Schema.optionalKey(Schema.Union([Schema.Number.check(Schema.isInt().annotate({ "expected": "an integer" })).check(Schema.isGreaterThanOrEqualTo(0).annotate({ "expected": "a value greater than or equal to 0" })), Schema.Null]).annotate({ "format": "int64" })), "id": Schema.String, "kind": Schema.String.annotate({ "description": "`install` or `uninstall`." }), "log": Schema.Array(Schema.String).annotate({ "description": "Tail of the runner's combined stdout/stderr." }), "phase": Schema.String.annotate({ "description": "Coarse step name, for a progress line the operator can read." }), "started_at": Schema.Number.annotate({ "format": "int64" }).check(Schema.isInt().annotate({ "expected": "an integer" })).check(Schema.isGreaterThanOrEqualTo(0).annotate({ "expected": "a value greater than or equal to 0" })), "state": State, "target": Schema.String.annotate({ "description": "What the operator asked for — a package name, or the raw spec they typed." }) }).annotate({ "description": "A job as the console sees it. Field names are snake_case like the rest of the management API\n(the *file* formats — index, sources, manifest — follow npm's camelCase instead).", "identifier": "Job" })
export type SupportRuntime = { readonly "active_game": boolean, readonly "audio_streaming": boolean, readonly "conflicts": ReadonlyArray<string>, readonly "stats": StatsStatus, readonly "video_streaming": boolean }
export const SupportRuntime = Schema.Struct({ "active_game": Schema.Boolean, "audio_streaming": Schema.Boolean, "conflicts": Schema.Array(Schema.String), "stats": StatsStatus, "video_streaming": Schema.Boolean }).annotate({ "identifier": "SupportRuntime" })
export type UpdateStatus = { readonly "apply": string, readonly "available": boolean, readonly "channel": string, readonly "channel_hint": string, readonly "check_disabled": boolean, readonly "current_version": string, readonly "install_kind": string, readonly "job"?: null | UpdateJobInfo, readonly "last_checked_unix"?: number | null, readonly "last_error"?: string | null, readonly "last_result"?: null | UpdateResultInfo, readonly "manifest"?: null | UpdateManifestInfo, readonly "not_published": boolean, readonly "opt_in_hint"?: string | null }
export const UpdateStatus = Schema.Struct({ "apply": Schema.String.annotate({ "description": "What the console may offer for this install: `notify` (show the command) — later\nphases add `full` (one-click apply) and `staged` (apply + reboot to finish)." }), "available": Schema.Boolean.annotate({ "description": "A newer release than `current_version` exists for this channel (definitive\ncomparisons only — an unparseable version pair never flags)." }), "channel": Schema.String.annotate({ "description": "Release channel this install follows: `stable` | `canary`." }), "channel_hint": Schema.String.annotate({ "description": "The copy-pastable update command for this install kind." }), "check_disabled": Schema.Boolean.annotate({ "description": "Update checks are disabled on this host (`SLIPSTREAM_UPDATE_CHECK=0`)." }), "current_version": Schema.String.annotate({ "description": "The running host version." }), "install_kind": Schema.String.annotate({ "description": "How this host was installed: `sysext` | `rpm-ostree` | `apt` | `dnf` | `pacman` |\n`steamos-source` | `nix` | `source`." }), "job": Schema.optionalKey(Schema.Union([Schema.Null, Schema.suspend((): Schema.Codec<UpdateJobInfo> => UpdateJobInfo).annotate({ "description": "The apply in flight, if any." })], { mode: "oneOf" })), "last_checked_unix": Schema.optionalKey(Schema.Union([Schema.Number.check(Schema.isInt().annotate({ "expected": "an integer" })).check(Schema.isGreaterThanOrEqualTo(0).annotate({ "expected": "a value greater than or equal to 0" })), Schema.Null]).annotate({ "description": "When the last successful check happened (unix seconds).", "format": "int64" })), "last_error": Schema.optionalKey(Schema.Union([Schema.String, Schema.Null]).annotate({ "description": "Why the last check failed, verbatim, if it did." })), "last_result": Schema.optionalKey(Schema.Union([Schema.Null, Schema.suspend((): Schema.Codec<UpdateResultInfo> => UpdateResultInfo).annotate({ "description": "Outcome of the most recent apply attempt." })], { mode: "oneOf" })), "manifest": Schema.optionalKey(Schema.Union([Schema.Null, Schema.suspend((): Schema.Codec<UpdateManifestInfo> => UpdateManifestInfo).annotate({ "description": "The last verified manifest, if any check has succeeded." })], { mode: "oneOf" })), "not_published": Schema.Boolean.annotate({ "description": "The check reached the feed and found this channel has **no release published yet** —\nan expected state (a channel nobody has announced to answers with a 404), not a\nfailure. Mutually exclusive with `last_error`, so a UI can say \"nothing published yet\"\ninstead of painting an empty feed as a broken host. Never set once a manifest has been\nseen for this channel: a feed that loses a document it used to serve stays an error." }), "opt_in_hint": Schema.optionalKey(Schema.Union([Schema.String, Schema.Null]).annotate({ "description": "This install could one-click apply, but the operator hasn't opted in yet — the\ncommand to run (Linux: join the `slipstream-update` group)." })) }).annotate({ "description": "The full update-check state for this host.", "identifier": "UpdateStatus" })
export type PreflightReport = { readonly "checks": ReadonlyArray<PreflightCheck>, readonly "generated_unix_ms": number, readonly "ready": boolean, readonly "schema": number }
export const PreflightReport = Schema.Struct({ "checks": Schema.Array(PreflightCheck), "generated_unix_ms": Schema.Number.annotate({ "format": "int64" }).check(Schema.isInt().annotate({ "expected": "an integer" })).check(Schema.isGreaterThanOrEqualTo(0).annotate({ "expected": "a value greater than or equal to 0" })), "ready": Schema.Boolean, "schema": Schema.Number.annotate({ "format": "int32" }).check(Schema.isInt().annotate({ "expected": "an integer" })).check(Schema.isGreaterThanOrEqualTo(0).annotate({ "expected": "a value greater than or equal to 0" })) }).annotate({ "identifier": "PreflightReport" })
export type SessionSettingsState = { readonly "configured": boolean, readonly "enforced": ReadonlyArray<string>, readonly "settings": SessionSettings }
export const SessionSettingsState = Schema.Struct({ "configured": Schema.Boolean.annotate({ "description": "Whether an operator has ever saved these settings (`false` ⇒ `settings` are the defaults)." }), "enforced": Schema.Array(Schema.String).annotate({ "description": "Which fields this build actually enforces. Empty on a platform with no launch path (macOS),\nso the console can say so instead of offering a switch that does nothing." }), "settings": Schema.suspend((): Schema.Codec<SessionSettings> => SessionSettings).annotate({ "description": "The stored settings (or the built-in defaults when this host has never been configured)." }) }).annotate({ "description": "The session⇄game lifetime settings, plus which axes this build acts on.", "identifier": "SessionSettingsState" })
export type HostConfigState = { readonly "configured": boolean, readonly "env_path": string, readonly "requires_restart": boolean, readonly "settings": HostConfigFile }
export const HostConfigState = Schema.Struct({ "configured": Schema.Boolean.annotate({ "description": "Whether an operator has ever saved host-config.json." }), "env_path": Schema.String.annotate({ "description": "Absolute path of the dual-written host.env file." }), "requires_restart": Schema.Boolean.annotate({ "description": "Env-backed knobs need a host restart to take effect in the running process." }), "settings": HostConfigFile }).annotate({ "description": "Host configuration the console can edit (Sunshine-style toggles).", "identifier": "HostConfigState" })
export type RuntimeStatus = { readonly "active_sessions": number, readonly "audio_streaming": boolean, readonly "games": ReadonlyArray<ActiveGame>, readonly "native_paired_clients": number, readonly "paired_clients": number, readonly "pin_pending": boolean, readonly "session"?: null | SessionInfo, readonly "stream"?: null | StreamInfo, readonly "video_streaming": boolean }
export const RuntimeStatus = Schema.Struct({ "active_sessions": Schema.Number.annotate({ "description": "Number of live streaming sessions across BOTH planes (GameStream + native slipstream/1). The\nnative server admits concurrent sessions, so this can exceed 1; `session`/`stream` below\ndescribe a single representative session for the detail card.", "format": "int32" }).check(Schema.isInt().annotate({ "expected": "an integer" })).check(Schema.isGreaterThanOrEqualTo(0).annotate({ "expected": "a value greater than or equal to 0" })), "audio_streaming": Schema.Boolean.annotate({ "description": "True while the audio stream thread is running." }), "games": Schema.Array(ActiveGame).annotate({ "description": "Every launched game the host is tracking: one row per live session that launched a title, plus\nany game whose session has ended and which is waiting out its reconnect window before being\nended (`state: \"grace\"`). Empty when nothing was launched — a plain desktop stream has no game." }), "native_paired_clients": Schema.Number.annotate({ "description": "Number of paired native (slipstream/1) devices — the default plane, so on a host that has\nnever been touched by Moonlight this is the only non-zero one of the pair.", "format": "int32" }).check(Schema.isInt().annotate({ "expected": "an integer" })).check(Schema.isGreaterThanOrEqualTo(0).annotate({ "expected": "a value greater than or equal to 0" })), "paired_clients": Schema.Number.annotate({ "description": "Number of pinned (paired) GameStream client certificates. Native (slipstream/1) devices pair\nagainst a separate store and are counted in `native_paired_clients` — sum the two for\n\"how many clients are paired with this host\".", "format": "int32" }).check(Schema.isInt().annotate({ "expected": "an integer" })).check(Schema.isGreaterThanOrEqualTo(0).annotate({ "expected": "a value greater than or equal to 0" })), "pin_pending": Schema.Boolean.annotate({ "description": "True while a pairing handshake is parked waiting for the user's PIN\n(submit it via `POST /api/v1/pair/pin`)." }), "session": Schema.optionalKey(Schema.Union([Schema.Null, Schema.suspend((): Schema.Codec<SessionInfo> => SessionInfo).annotate({ "description": "A representative active session. GameStream's launch (Moonlight `/launch`) when present, else\nthe first live native session. `null` when nothing is streaming." })], { mode: "oneOf" })), "stream": Schema.optionalKey(Schema.Union([Schema.Null, Schema.suspend((): Schema.Codec<StreamInfo> => StreamInfo).annotate({ "description": "The active stream's parameters — RTSP-negotiated for GameStream, or the live native session's\nmode/codec/bitrate. `null` when nothing is streaming." })], { mode: "oneOf" })), "video_streaming": Schema.Boolean.annotate({ "description": "True while the video stream thread is running." }) }).annotate({ "description": "Live host status (changes as clients launch/end sessions).", "identifier": "RuntimeStatus" })
export type HookEntry = { readonly "debounce_ms"?: number, readonly "filter"?: null | HookFilter, readonly "hmac_secret_file"?: string | null, readonly "on": string, readonly "run"?: string | null, readonly "timeout_s"?: number, readonly "webhook"?: string | null }
export const HookEntry = Schema.Struct({ "debounce_ms": Schema.optionalKey(Schema.Number.annotate({ "description": "Minimum interval between firings of this hook, in milliseconds. 0 = fire every time.", "format": "int64" }).check(Schema.isInt().annotate({ "expected": "an integer" })).check(Schema.isGreaterThanOrEqualTo(0).annotate({ "expected": "a value greater than or equal to 0" }))), "filter": Schema.optionalKey(Schema.Union([Schema.Null, Schema.suspend((): Schema.Codec<HookFilter> => HookFilter).annotate({ "description": "Exact-match constraints on the event's fields; every present field must match." })], { mode: "oneOf" })), "hmac_secret_file": Schema.optionalKey(Schema.Union([Schema.String, Schema.Null]).annotate({ "description": "File holding the webhook HMAC secret (`X-Slipstream-Signature: sha256=<hex>`). The file\nshould be operator-owned and private; a world-readable secret is warned about." })), "on": Schema.String.annotate({ "description": "Which events fire this hook: an exact kind (`stream.started`) or a `domain.*` prefix\n(`pairing.*`) — the same vocabulary as the SSE `?kinds=` filter." }), "run": Schema.optionalKey(Schema.Union([Schema.String, Schema.Null]).annotate({ "description": "Shell command to execute (detached, event JSON on stdin + `PF_EVENT_*` env)." })), "timeout_s": Schema.optionalKey(Schema.Number.annotate({ "description": "Exec timeout in seconds (1–600, default 30); the process group is killed on expiry.", "format": "int32" }).check(Schema.isInt().annotate({ "expected": "an integer" })).check(Schema.isGreaterThanOrEqualTo(0).annotate({ "expected": "a value greater than or equal to 0" }))), "webhook": Schema.optionalKey(Schema.Union([Schema.String, Schema.Null]).annotate({ "description": "URL to POST the event JSON to." })) }).annotate({ "description": "One hook: fire `run` and/or `webhook` when an event matching `on` (+ `filter`) occurs.", "identifier": "HookEntry" })
export type EventKind = { readonly "client": ClientRef, readonly "kind": "client.connected" } | { readonly "client": ClientRef, readonly "kind": "client.disconnected", readonly "reason": DisconnectReason } | { readonly "kind": "session.started", readonly "session": SessionRef } | { readonly "kind": "session.ended", readonly "session": SessionRef } | { readonly "kind": "stream.started", readonly "stream": StreamRef } | { readonly "kind": "stream.stopped", readonly "stream": StreamRef } | { readonly "game": GameRefPayload, readonly "kind": "game.running" } | { readonly "game": GameRefPayload, readonly "kind": "game.exited", readonly "reason": GameEndReason } | { readonly "device": DeviceRef, readonly "kind": "pairing.pending" } | { readonly "device": DeviceRef, readonly "kind": "pairing.completed" } | { readonly "device": DeviceRef, readonly "kind": "pairing.denied" } | { readonly "backend": string, readonly "kind": "display.created", readonly "mode": string } | { readonly "count": number, readonly "kind": "display.released" } | { readonly "kind": "library.changed", readonly "source": string } | { readonly "channel": string, readonly "install_kind": string, readonly "kind": "update.available", readonly "version": string } | { readonly "from": string, readonly "kind": "update.applied", readonly "to": string } | { readonly "id": string, readonly "kind": "plugins.changed" } | { readonly "kind": "store.changed" } | { readonly "gamestream": boolean, readonly "kind": "host.started", readonly "version": string } | { readonly "kind": "host.stopping" }
export const EventKind = Schema.Union([Schema.Struct({ "client": ClientRef, "kind": Schema.Literal("client.connected") }), Schema.Struct({ "client": ClientRef, "kind": Schema.Literal("client.disconnected"), "reason": DisconnectReason }), Schema.Struct({ "kind": Schema.Literal("session.started"), "session": SessionRef }), Schema.Struct({ "kind": Schema.Literal("session.ended"), "session": SessionRef }), Schema.Struct({ "kind": Schema.Literal("stream.started"), "stream": StreamRef }), Schema.Struct({ "kind": Schema.Literal("stream.stopped"), "stream": StreamRef }), Schema.Struct({ "game": GameRefPayload, "kind": Schema.Literal("game.running") }).annotate({ "description": "A launched game was confirmed running — fires once per launch, after the host has actually\nseen the game's process (not merely spawned its launcher)." }), Schema.Struct({ "game": GameRefPayload, "kind": Schema.Literal("game.exited"), "reason": GameEndReason }).annotate({ "description": "A launched game is gone. `reason` distinguishes the player quitting from the host ending it\nper the lifetime policy." }), Schema.Struct({ "device": DeviceRef, "kind": Schema.Literal("pairing.pending") }), Schema.Struct({ "device": DeviceRef, "kind": Schema.Literal("pairing.completed") }), Schema.Struct({ "device": DeviceRef, "kind": Schema.Literal("pairing.denied") }), Schema.Struct({ "backend": Schema.String.annotate({ "description": "The virtual-display backend that minted it (`VirtualDisplay::name`)." }), "kind": Schema.Literal("display.created"), "mode": Schema.String.annotate({ "description": "`WxH@Hz`." }) }), Schema.Struct({ "count": Schema.Number.annotate({ "description": "How many kept displays this release retired.", "format": "int32" }).check(Schema.isInt().annotate({ "expected": "an integer" })).check(Schema.isGreaterThanOrEqualTo(0).annotate({ "expected": "a value greater than or equal to 0" })), "kind": Schema.Literal("display.released") }), Schema.Struct({ "kind": Schema.Literal("library.changed"), "source": Schema.String.annotate({ "description": "What mutated the library: `\"manual\"` today; a provider id once the provider\nAPI (RFC §8) lands." }) }), Schema.Struct({ "channel": Schema.String.annotate({ "description": "The channel it was announced on (`stable` | `canary`)." }), "install_kind": Schema.String.annotate({ "description": "This host's install kind (`apt`, `deb`, or another Linux package source), so a hook\ncan render the update hint without another call." }), "kind": Schema.Literal("update.available"), "version": Schema.String.annotate({ "description": "The newer release's version string." }) }).annotate({ "description": "A verified update manifest announced a release newer than the running host. Emitted\nonce per discovered version (a steady-state \"newer exists\" doesn't re-fire on every\nrefresh)." }), Schema.Struct({ "from": Schema.String, "kind": Schema.Literal("update.applied"), "to": Schema.String }).annotate({ "description": "A host update completed: emitted by boot-time reconciliation, i.e. by the NEW binary's\nfirst start after a successful apply." }), Schema.Struct({ "id": Schema.String.annotate({ "description": "The plugin whose registration changed (registered, restarted, deregistered, or\nlease-expired). A consumer re-reads `GET /api/v1/plugins` for the new set." }), "kind": Schema.Literal("plugins.changed") }), Schema.Struct({ "kind": Schema.Literal("store.changed") }).annotate({ "description": "The set of installed plugins, or what the store knows about them, changed — an install or\nuninstall finished, or a catalog refresh brought in new rows. A consumer re-reads\n`GET /api/v1/store/catalog` / `…/installed`. Deliberately payload-free: the store's answer\nis a join over several sources of truth, so \"go look again\" is the only honest signal." }), Schema.Struct({ "gamestream": Schema.Boolean.annotate({ "description": "Whether the GameStream/Moonlight compat plane is enabled." }), "kind": Schema.Literal("host.started"), "version": Schema.String }), Schema.Struct({ "kind": Schema.Literal("host.stopping") })], { mode: "oneOf" }).annotate({ "description": "The event catalog (RFC §4). Serialized internally tagged as `\"kind\": \"<domain>.<verb>\"`,\nflattened into [`HostEvent`]. **Additive-only** within [`SCHEMA_VERSION`].", "identifier": "EventKind" })
export type HostEvent = { readonly "client": ClientRef, readonly "kind": "client.connected", readonly "schema": number, readonly "seq": number, readonly "ts_ms": number } | { readonly "client": ClientRef, readonly "kind": "client.disconnected", readonly "reason": DisconnectReason, readonly "schema": number, readonly "seq": number, readonly "ts_ms": number } | { readonly "kind": "session.started", readonly "session": SessionRef, readonly "schema": number, readonly "seq": number, readonly "ts_ms": number } | { readonly "kind": "session.ended", readonly "session": SessionRef, readonly "schema": number, readonly "seq": number, readonly "ts_ms": number } | { readonly "kind": "stream.started", readonly "stream": StreamRef, readonly "schema": number, readonly "seq": number, readonly "ts_ms": number } | { readonly "kind": "stream.stopped", readonly "stream": StreamRef, readonly "schema": number, readonly "seq": number, readonly "ts_ms": number } | { readonly "game": GameRefPayload, readonly "kind": "game.running", readonly "schema": number, readonly "seq": number, readonly "ts_ms": number } | { readonly "game": GameRefPayload, readonly "kind": "game.exited", readonly "reason": GameEndReason, readonly "schema": number, readonly "seq": number, readonly "ts_ms": number } | { readonly "device": DeviceRef, readonly "kind": "pairing.pending", readonly "schema": number, readonly "seq": number, readonly "ts_ms": number } | { readonly "device": DeviceRef, readonly "kind": "pairing.completed", readonly "schema": number, readonly "seq": number, readonly "ts_ms": number } | { readonly "device": DeviceRef, readonly "kind": "pairing.denied", readonly "schema": number, readonly "seq": number, readonly "ts_ms": number } | { readonly "backend": string, readonly "kind": "display.created", readonly "mode": string, readonly "schema": number, readonly "seq": number, readonly "ts_ms": number } | { readonly "count": number, readonly "kind": "display.released", readonly "schema": number, readonly "seq": number, readonly "ts_ms": number } | { readonly "kind": "library.changed", readonly "source": string, readonly "schema": number, readonly "seq": number, readonly "ts_ms": number } | { readonly "channel": string, readonly "install_kind": string, readonly "kind": "update.available", readonly "version": string, readonly "schema": number, readonly "seq": number, readonly "ts_ms": number } | { readonly "from": string, readonly "kind": "update.applied", readonly "to": string, readonly "schema": number, readonly "seq": number, readonly "ts_ms": number } | { readonly "id": string, readonly "kind": "plugins.changed", readonly "schema": number, readonly "seq": number, readonly "ts_ms": number } | { readonly "kind": "store.changed", readonly "schema": number, readonly "seq": number, readonly "ts_ms": number } | { readonly "gamestream": boolean, readonly "kind": "host.started", readonly "version": string, readonly "schema": number, readonly "seq": number, readonly "ts_ms": number } | { readonly "kind": "host.stopping", readonly "schema": number, readonly "seq": number, readonly "ts_ms": number }
export const HostEvent = Schema.Union([Schema.Struct({ "client": ClientRef, "kind": Schema.Literal("client.connected"), "schema": Schema.Number.annotate({ "description": "Wire-shape version ([`SCHEMA_VERSION`]).", "format": "int32" }).check(Schema.isInt().annotate({ "expected": "an integer" })).check(Schema.isGreaterThanOrEqualTo(0).annotate({ "expected": "a value greater than or equal to 0" })), "seq": Schema.Number.annotate({ "description": "Monotonic sequence number (1-based) — a consumer resumes with `since = last seen`.", "format": "int64" }).check(Schema.isInt().annotate({ "expected": "an integer" })).check(Schema.isGreaterThanOrEqualTo(0).annotate({ "expected": "a value greater than or equal to 0" })), "ts_ms": Schema.Number.annotate({ "description": "Unix timestamp in milliseconds (the [`crate::log_capture::LogEntry`] convention).", "format": "int64" }).check(Schema.isInt().annotate({ "expected": "an integer" })).check(Schema.isGreaterThanOrEqualTo(0).annotate({ "expected": "a value greater than or equal to 0" })) }), Schema.Struct({ "client": ClientRef, "kind": Schema.Literal("client.disconnected"), "reason": DisconnectReason, "schema": Schema.Number.annotate({ "description": "Wire-shape version ([`SCHEMA_VERSION`]).", "format": "int32" }).check(Schema.isInt().annotate({ "expected": "an integer" })).check(Schema.isGreaterThanOrEqualTo(0).annotate({ "expected": "a value greater than or equal to 0" })), "seq": Schema.Number.annotate({ "description": "Monotonic sequence number (1-based) — a consumer resumes with `since = last seen`.", "format": "int64" }).check(Schema.isInt().annotate({ "expected": "an integer" })).check(Schema.isGreaterThanOrEqualTo(0).annotate({ "expected": "a value greater than or equal to 0" })), "ts_ms": Schema.Number.annotate({ "description": "Unix timestamp in milliseconds (the [`crate::log_capture::LogEntry`] convention).", "format": "int64" }).check(Schema.isInt().annotate({ "expected": "an integer" })).check(Schema.isGreaterThanOrEqualTo(0).annotate({ "expected": "a value greater than or equal to 0" })) }), Schema.Struct({ "kind": Schema.Literal("session.started"), "session": SessionRef, "schema": Schema.Number.annotate({ "description": "Wire-shape version ([`SCHEMA_VERSION`]).", "format": "int32" }).check(Schema.isInt().annotate({ "expected": "an integer" })).check(Schema.isGreaterThanOrEqualTo(0).annotate({ "expected": "a value greater than or equal to 0" })), "seq": Schema.Number.annotate({ "description": "Monotonic sequence number (1-based) — a consumer resumes with `since = last seen`.", "format": "int64" }).check(Schema.isInt().annotate({ "expected": "an integer" })).check(Schema.isGreaterThanOrEqualTo(0).annotate({ "expected": "a value greater than or equal to 0" })), "ts_ms": Schema.Number.annotate({ "description": "Unix timestamp in milliseconds (the [`crate::log_capture::LogEntry`] convention).", "format": "int64" }).check(Schema.isInt().annotate({ "expected": "an integer" })).check(Schema.isGreaterThanOrEqualTo(0).annotate({ "expected": "a value greater than or equal to 0" })) }), Schema.Struct({ "kind": Schema.Literal("session.ended"), "session": SessionRef, "schema": Schema.Number.annotate({ "description": "Wire-shape version ([`SCHEMA_VERSION`]).", "format": "int32" }).check(Schema.isInt().annotate({ "expected": "an integer" })).check(Schema.isGreaterThanOrEqualTo(0).annotate({ "expected": "a value greater than or equal to 0" })), "seq": Schema.Number.annotate({ "description": "Monotonic sequence number (1-based) — a consumer resumes with `since = last seen`.", "format": "int64" }).check(Schema.isInt().annotate({ "expected": "an integer" })).check(Schema.isGreaterThanOrEqualTo(0).annotate({ "expected": "a value greater than or equal to 0" })), "ts_ms": Schema.Number.annotate({ "description": "Unix timestamp in milliseconds (the [`crate::log_capture::LogEntry`] convention).", "format": "int64" }).check(Schema.isInt().annotate({ "expected": "an integer" })).check(Schema.isGreaterThanOrEqualTo(0).annotate({ "expected": "a value greater than or equal to 0" })) }), Schema.Struct({ "kind": Schema.Literal("stream.started"), "stream": StreamRef, "schema": Schema.Number.annotate({ "description": "Wire-shape version ([`SCHEMA_VERSION`]).", "format": "int32" }).check(Schema.isInt().annotate({ "expected": "an integer" })).check(Schema.isGreaterThanOrEqualTo(0).annotate({ "expected": "a value greater than or equal to 0" })), "seq": Schema.Number.annotate({ "description": "Monotonic sequence number (1-based) — a consumer resumes with `since = last seen`.", "format": "int64" }).check(Schema.isInt().annotate({ "expected": "an integer" })).check(Schema.isGreaterThanOrEqualTo(0).annotate({ "expected": "a value greater than or equal to 0" })), "ts_ms": Schema.Number.annotate({ "description": "Unix timestamp in milliseconds (the [`crate::log_capture::LogEntry`] convention).", "format": "int64" }).check(Schema.isInt().annotate({ "expected": "an integer" })).check(Schema.isGreaterThanOrEqualTo(0).annotate({ "expected": "a value greater than or equal to 0" })) }), Schema.Struct({ "kind": Schema.Literal("stream.stopped"), "stream": StreamRef, "schema": Schema.Number.annotate({ "description": "Wire-shape version ([`SCHEMA_VERSION`]).", "format": "int32" }).check(Schema.isInt().annotate({ "expected": "an integer" })).check(Schema.isGreaterThanOrEqualTo(0).annotate({ "expected": "a value greater than or equal to 0" })), "seq": Schema.Number.annotate({ "description": "Monotonic sequence number (1-based) — a consumer resumes with `since = last seen`.", "format": "int64" }).check(Schema.isInt().annotate({ "expected": "an integer" })).check(Schema.isGreaterThanOrEqualTo(0).annotate({ "expected": "a value greater than or equal to 0" })), "ts_ms": Schema.Number.annotate({ "description": "Unix timestamp in milliseconds (the [`crate::log_capture::LogEntry`] convention).", "format": "int64" }).check(Schema.isInt().annotate({ "expected": "an integer" })).check(Schema.isGreaterThanOrEqualTo(0).annotate({ "expected": "a value greater than or equal to 0" })) }), Schema.Struct({ "game": GameRefPayload, "kind": Schema.Literal("game.running"), "schema": Schema.Number.annotate({ "description": "Wire-shape version ([`SCHEMA_VERSION`]).", "format": "int32" }).check(Schema.isInt().annotate({ "expected": "an integer" })).check(Schema.isGreaterThanOrEqualTo(0).annotate({ "expected": "a value greater than or equal to 0" })), "seq": Schema.Number.annotate({ "description": "Monotonic sequence number (1-based) — a consumer resumes with `since = last seen`.", "format": "int64" }).check(Schema.isInt().annotate({ "expected": "an integer" })).check(Schema.isGreaterThanOrEqualTo(0).annotate({ "expected": "a value greater than or equal to 0" })), "ts_ms": Schema.Number.annotate({ "description": "Unix timestamp in milliseconds (the [`crate::log_capture::LogEntry`] convention).", "format": "int64" }).check(Schema.isInt().annotate({ "expected": "an integer" })).check(Schema.isGreaterThanOrEqualTo(0).annotate({ "expected": "a value greater than or equal to 0" })) }).annotate({ "description": "A launched game was confirmed running — fires once per launch, after the host has actually\nseen the game's process (not merely spawned its launcher)." }), Schema.Struct({ "game": GameRefPayload, "kind": Schema.Literal("game.exited"), "reason": GameEndReason, "schema": Schema.Number.annotate({ "description": "Wire-shape version ([`SCHEMA_VERSION`]).", "format": "int32" }).check(Schema.isInt().annotate({ "expected": "an integer" })).check(Schema.isGreaterThanOrEqualTo(0).annotate({ "expected": "a value greater than or equal to 0" })), "seq": Schema.Number.annotate({ "description": "Monotonic sequence number (1-based) — a consumer resumes with `since = last seen`.", "format": "int64" }).check(Schema.isInt().annotate({ "expected": "an integer" })).check(Schema.isGreaterThanOrEqualTo(0).annotate({ "expected": "a value greater than or equal to 0" })), "ts_ms": Schema.Number.annotate({ "description": "Unix timestamp in milliseconds (the [`crate::log_capture::LogEntry`] convention).", "format": "int64" }).check(Schema.isInt().annotate({ "expected": "an integer" })).check(Schema.isGreaterThanOrEqualTo(0).annotate({ "expected": "a value greater than or equal to 0" })) }).annotate({ "description": "A launched game is gone. `reason` distinguishes the player quitting from the host ending it\nper the lifetime policy." }), Schema.Struct({ "device": DeviceRef, "kind": Schema.Literal("pairing.pending"), "schema": Schema.Number.annotate({ "description": "Wire-shape version ([`SCHEMA_VERSION`]).", "format": "int32" }).check(Schema.isInt().annotate({ "expected": "an integer" })).check(Schema.isGreaterThanOrEqualTo(0).annotate({ "expected": "a value greater than or equal to 0" })), "seq": Schema.Number.annotate({ "description": "Monotonic sequence number (1-based) — a consumer resumes with `since = last seen`.", "format": "int64" }).check(Schema.isInt().annotate({ "expected": "an integer" })).check(Schema.isGreaterThanOrEqualTo(0).annotate({ "expected": "a value greater than or equal to 0" })), "ts_ms": Schema.Number.annotate({ "description": "Unix timestamp in milliseconds (the [`crate::log_capture::LogEntry`] convention).", "format": "int64" }).check(Schema.isInt().annotate({ "expected": "an integer" })).check(Schema.isGreaterThanOrEqualTo(0).annotate({ "expected": "a value greater than or equal to 0" })) }), Schema.Struct({ "device": DeviceRef, "kind": Schema.Literal("pairing.completed"), "schema": Schema.Number.annotate({ "description": "Wire-shape version ([`SCHEMA_VERSION`]).", "format": "int32" }).check(Schema.isInt().annotate({ "expected": "an integer" })).check(Schema.isGreaterThanOrEqualTo(0).annotate({ "expected": "a value greater than or equal to 0" })), "seq": Schema.Number.annotate({ "description": "Monotonic sequence number (1-based) — a consumer resumes with `since = last seen`.", "format": "int64" }).check(Schema.isInt().annotate({ "expected": "an integer" })).check(Schema.isGreaterThanOrEqualTo(0).annotate({ "expected": "a value greater than or equal to 0" })), "ts_ms": Schema.Number.annotate({ "description": "Unix timestamp in milliseconds (the [`crate::log_capture::LogEntry`] convention).", "format": "int64" }).check(Schema.isInt().annotate({ "expected": "an integer" })).check(Schema.isGreaterThanOrEqualTo(0).annotate({ "expected": "a value greater than or equal to 0" })) }), Schema.Struct({ "device": DeviceRef, "kind": Schema.Literal("pairing.denied"), "schema": Schema.Number.annotate({ "description": "Wire-shape version ([`SCHEMA_VERSION`]).", "format": "int32" }).check(Schema.isInt().annotate({ "expected": "an integer" })).check(Schema.isGreaterThanOrEqualTo(0).annotate({ "expected": "a value greater than or equal to 0" })), "seq": Schema.Number.annotate({ "description": "Monotonic sequence number (1-based) — a consumer resumes with `since = last seen`.", "format": "int64" }).check(Schema.isInt().annotate({ "expected": "an integer" })).check(Schema.isGreaterThanOrEqualTo(0).annotate({ "expected": "a value greater than or equal to 0" })), "ts_ms": Schema.Number.annotate({ "description": "Unix timestamp in milliseconds (the [`crate::log_capture::LogEntry`] convention).", "format": "int64" }).check(Schema.isInt().annotate({ "expected": "an integer" })).check(Schema.isGreaterThanOrEqualTo(0).annotate({ "expected": "a value greater than or equal to 0" })) }), Schema.Struct({ "backend": Schema.String.annotate({ "description": "The virtual-display backend that minted it (`VirtualDisplay::name`)." }), "kind": Schema.Literal("display.created"), "mode": Schema.String.annotate({ "description": "`WxH@Hz`." }), "schema": Schema.Number.annotate({ "description": "Wire-shape version ([`SCHEMA_VERSION`]).", "format": "int32" }).check(Schema.isInt().annotate({ "expected": "an integer" })).check(Schema.isGreaterThanOrEqualTo(0).annotate({ "expected": "a value greater than or equal to 0" })), "seq": Schema.Number.annotate({ "description": "Monotonic sequence number (1-based) — a consumer resumes with `since = last seen`.", "format": "int64" }).check(Schema.isInt().annotate({ "expected": "an integer" })).check(Schema.isGreaterThanOrEqualTo(0).annotate({ "expected": "a value greater than or equal to 0" })), "ts_ms": Schema.Number.annotate({ "description": "Unix timestamp in milliseconds (the [`crate::log_capture::LogEntry`] convention).", "format": "int64" }).check(Schema.isInt().annotate({ "expected": "an integer" })).check(Schema.isGreaterThanOrEqualTo(0).annotate({ "expected": "a value greater than or equal to 0" })) }), Schema.Struct({ "count": Schema.Number.annotate({ "description": "How many kept displays this release retired.", "format": "int32" }).check(Schema.isInt().annotate({ "expected": "an integer" })).check(Schema.isGreaterThanOrEqualTo(0).annotate({ "expected": "a value greater than or equal to 0" })), "kind": Schema.Literal("display.released"), "schema": Schema.Number.annotate({ "description": "Wire-shape version ([`SCHEMA_VERSION`]).", "format": "int32" }).check(Schema.isInt().annotate({ "expected": "an integer" })).check(Schema.isGreaterThanOrEqualTo(0).annotate({ "expected": "a value greater than or equal to 0" })), "seq": Schema.Number.annotate({ "description": "Monotonic sequence number (1-based) — a consumer resumes with `since = last seen`.", "format": "int64" }).check(Schema.isInt().annotate({ "expected": "an integer" })).check(Schema.isGreaterThanOrEqualTo(0).annotate({ "expected": "a value greater than or equal to 0" })), "ts_ms": Schema.Number.annotate({ "description": "Unix timestamp in milliseconds (the [`crate::log_capture::LogEntry`] convention).", "format": "int64" }).check(Schema.isInt().annotate({ "expected": "an integer" })).check(Schema.isGreaterThanOrEqualTo(0).annotate({ "expected": "a value greater than or equal to 0" })) }), Schema.Struct({ "kind": Schema.Literal("library.changed"), "source": Schema.String.annotate({ "description": "What mutated the library: `\"manual\"` today; a provider id once the provider\nAPI (RFC §8) lands." }), "schema": Schema.Number.annotate({ "description": "Wire-shape version ([`SCHEMA_VERSION`]).", "format": "int32" }).check(Schema.isInt().annotate({ "expected": "an integer" })).check(Schema.isGreaterThanOrEqualTo(0).annotate({ "expected": "a value greater than or equal to 0" })), "seq": Schema.Number.annotate({ "description": "Monotonic sequence number (1-based) — a consumer resumes with `since = last seen`.", "format": "int64" }).check(Schema.isInt().annotate({ "expected": "an integer" })).check(Schema.isGreaterThanOrEqualTo(0).annotate({ "expected": "a value greater than or equal to 0" })), "ts_ms": Schema.Number.annotate({ "description": "Unix timestamp in milliseconds (the [`crate::log_capture::LogEntry`] convention).", "format": "int64" }).check(Schema.isInt().annotate({ "expected": "an integer" })).check(Schema.isGreaterThanOrEqualTo(0).annotate({ "expected": "a value greater than or equal to 0" })) }), Schema.Struct({ "channel": Schema.String.annotate({ "description": "The channel it was announced on (`stable` | `canary`)." }), "install_kind": Schema.String.annotate({ "description": "This host's install kind (`apt`, `deb`, or another Linux package source), so a hook\ncan render the update hint without another call." }), "kind": Schema.Literal("update.available"), "version": Schema.String.annotate({ "description": "The newer release's version string." }), "schema": Schema.Number.annotate({ "description": "Wire-shape version ([`SCHEMA_VERSION`]).", "format": "int32" }).check(Schema.isInt().annotate({ "expected": "an integer" })).check(Schema.isGreaterThanOrEqualTo(0).annotate({ "expected": "a value greater than or equal to 0" })), "seq": Schema.Number.annotate({ "description": "Monotonic sequence number (1-based) — a consumer resumes with `since = last seen`.", "format": "int64" }).check(Schema.isInt().annotate({ "expected": "an integer" })).check(Schema.isGreaterThanOrEqualTo(0).annotate({ "expected": "a value greater than or equal to 0" })), "ts_ms": Schema.Number.annotate({ "description": "Unix timestamp in milliseconds (the [`crate::log_capture::LogEntry`] convention).", "format": "int64" }).check(Schema.isInt().annotate({ "expected": "an integer" })).check(Schema.isGreaterThanOrEqualTo(0).annotate({ "expected": "a value greater than or equal to 0" })) }).annotate({ "description": "A verified update manifest announced a release newer than the running host. Emitted\nonce per discovered version (a steady-state \"newer exists\" doesn't re-fire on every\nrefresh)." }), Schema.Struct({ "from": Schema.String, "kind": Schema.Literal("update.applied"), "to": Schema.String, "schema": Schema.Number.annotate({ "description": "Wire-shape version ([`SCHEMA_VERSION`]).", "format": "int32" }).check(Schema.isInt().annotate({ "expected": "an integer" })).check(Schema.isGreaterThanOrEqualTo(0).annotate({ "expected": "a value greater than or equal to 0" })), "seq": Schema.Number.annotate({ "description": "Monotonic sequence number (1-based) — a consumer resumes with `since = last seen`.", "format": "int64" }).check(Schema.isInt().annotate({ "expected": "an integer" })).check(Schema.isGreaterThanOrEqualTo(0).annotate({ "expected": "a value greater than or equal to 0" })), "ts_ms": Schema.Number.annotate({ "description": "Unix timestamp in milliseconds (the [`crate::log_capture::LogEntry`] convention).", "format": "int64" }).check(Schema.isInt().annotate({ "expected": "an integer" })).check(Schema.isGreaterThanOrEqualTo(0).annotate({ "expected": "a value greater than or equal to 0" })) }).annotate({ "description": "A host update completed: emitted by boot-time reconciliation, i.e. by the NEW binary's\nfirst start after a successful apply." }), Schema.Struct({ "id": Schema.String.annotate({ "description": "The plugin whose registration changed (registered, restarted, deregistered, or\nlease-expired). A consumer re-reads `GET /api/v1/plugins` for the new set." }), "kind": Schema.Literal("plugins.changed"), "schema": Schema.Number.annotate({ "description": "Wire-shape version ([`SCHEMA_VERSION`]).", "format": "int32" }).check(Schema.isInt().annotate({ "expected": "an integer" })).check(Schema.isGreaterThanOrEqualTo(0).annotate({ "expected": "a value greater than or equal to 0" })), "seq": Schema.Number.annotate({ "description": "Monotonic sequence number (1-based) — a consumer resumes with `since = last seen`.", "format": "int64" }).check(Schema.isInt().annotate({ "expected": "an integer" })).check(Schema.isGreaterThanOrEqualTo(0).annotate({ "expected": "a value greater than or equal to 0" })), "ts_ms": Schema.Number.annotate({ "description": "Unix timestamp in milliseconds (the [`crate::log_capture::LogEntry`] convention).", "format": "int64" }).check(Schema.isInt().annotate({ "expected": "an integer" })).check(Schema.isGreaterThanOrEqualTo(0).annotate({ "expected": "a value greater than or equal to 0" })) }), Schema.Struct({ "kind": Schema.Literal("store.changed"), "schema": Schema.Number.annotate({ "description": "Wire-shape version ([`SCHEMA_VERSION`]).", "format": "int32" }).check(Schema.isInt().annotate({ "expected": "an integer" })).check(Schema.isGreaterThanOrEqualTo(0).annotate({ "expected": "a value greater than or equal to 0" })), "seq": Schema.Number.annotate({ "description": "Monotonic sequence number (1-based) — a consumer resumes with `since = last seen`.", "format": "int64" }).check(Schema.isInt().annotate({ "expected": "an integer" })).check(Schema.isGreaterThanOrEqualTo(0).annotate({ "expected": "a value greater than or equal to 0" })), "ts_ms": Schema.Number.annotate({ "description": "Unix timestamp in milliseconds (the [`crate::log_capture::LogEntry`] convention).", "format": "int64" }).check(Schema.isInt().annotate({ "expected": "an integer" })).check(Schema.isGreaterThanOrEqualTo(0).annotate({ "expected": "a value greater than or equal to 0" })) }).annotate({ "description": "The set of installed plugins, or what the store knows about them, changed — an install or\nuninstall finished, or a catalog refresh brought in new rows. A consumer re-reads\n`GET /api/v1/store/catalog` / `…/installed`. Deliberately payload-free: the store's answer\nis a join over several sources of truth, so \"go look again\" is the only honest signal." }), Schema.Struct({ "gamestream": Schema.Boolean.annotate({ "description": "Whether the GameStream/Moonlight compat plane is enabled." }), "kind": Schema.Literal("host.started"), "version": Schema.String, "schema": Schema.Number.annotate({ "description": "Wire-shape version ([`SCHEMA_VERSION`]).", "format": "int32" }).check(Schema.isInt().annotate({ "expected": "an integer" })).check(Schema.isGreaterThanOrEqualTo(0).annotate({ "expected": "a value greater than or equal to 0" })), "seq": Schema.Number.annotate({ "description": "Monotonic sequence number (1-based) — a consumer resumes with `since = last seen`.", "format": "int64" }).check(Schema.isInt().annotate({ "expected": "an integer" })).check(Schema.isGreaterThanOrEqualTo(0).annotate({ "expected": "a value greater than or equal to 0" })), "ts_ms": Schema.Number.annotate({ "description": "Unix timestamp in milliseconds (the [`crate::log_capture::LogEntry`] convention).", "format": "int64" }).check(Schema.isInt().annotate({ "expected": "an integer" })).check(Schema.isGreaterThanOrEqualTo(0).annotate({ "expected": "a value greater than or equal to 0" })) }), Schema.Struct({ "kind": Schema.Literal("host.stopping"), "schema": Schema.Number.annotate({ "description": "Wire-shape version ([`SCHEMA_VERSION`]).", "format": "int32" }).check(Schema.isInt().annotate({ "expected": "an integer" })).check(Schema.isGreaterThanOrEqualTo(0).annotate({ "expected": "a value greater than or equal to 0" })), "seq": Schema.Number.annotate({ "description": "Monotonic sequence number (1-based) — a consumer resumes with `since = last seen`.", "format": "int64" }).check(Schema.isInt().annotate({ "expected": "an integer" })).check(Schema.isGreaterThanOrEqualTo(0).annotate({ "expected": "a value greater than or equal to 0" })), "ts_ms": Schema.Number.annotate({ "description": "Unix timestamp in milliseconds (the [`crate::log_capture::LogEntry`] convention).", "format": "int64" }).check(Schema.isInt().annotate({ "expected": "an integer" })).check(Schema.isGreaterThanOrEqualTo(0).annotate({ "expected": "a value greater than or equal to 0" })) })], { mode: "oneOf" }).annotate({ "description": "The event kind + payload, flattened: `\"kind\": \"stream.started\", …payload…`.", "identifier": "HostEvent" })
export type Layout = { readonly "mode"?: LayoutMode, readonly "positions"?: Objects_3 }
export const Layout = Schema.Struct({ "mode": Schema.optionalKey(LayoutMode), "positions": Schema.optionalKey(Objects_3) }).annotate({ "description": "Group layout: the arrangement mode plus, for [`LayoutMode::Manual`], per-slot offsets keyed by\nidentity-slot id (string keys for stable JSON).", "identifier": "Layout" })
export type Capture = { readonly "meta": CaptureMeta, readonly "samples": ReadonlyArray<StatsSample> }
export const Capture = Schema.Struct({ "meta": CaptureMeta, "samples": Schema.Array(StatsSample) }).annotate({ "description": "A full capture: summary + the sample time-series. The wire + on-disk shape.", "identifier": "Capture" })
export type SupportBundle = { readonly "configuration": HostConfigFile, readonly "generated_unix_ms": number, readonly "host": SupportHost, readonly "id": string, readonly "logs": ReadonlyArray<LogEntry>, readonly "recordings": ReadonlyArray<CaptureMeta>, readonly "redactions": ReadonlyArray<string>, readonly "runtime": SupportRuntime, readonly "schema": number }
export const SupportBundle = Schema.Struct({ "configuration": HostConfigFile, "generated_unix_ms": Schema.Number.annotate({ "format": "int64" }).check(Schema.isInt().annotate({ "expected": "an integer" })).check(Schema.isGreaterThanOrEqualTo(0).annotate({ "expected": "a value greater than or equal to 0" })), "host": SupportHost, "id": Schema.String, "logs": Schema.Array(LogEntry), "recordings": Schema.Array(CaptureMeta), "redactions": Schema.Array(Schema.String), "runtime": SupportRuntime, "schema": Schema.Number.annotate({ "format": "int32" }).check(Schema.isInt().annotate({ "expected": "an integer" })).check(Schema.isGreaterThanOrEqualTo(0).annotate({ "expected": "a value greater than or equal to 0" })) }).annotate({ "identifier": "SupportBundle" })
export type HooksConfig = { readonly "hooks"?: ReadonlyArray<HookEntry> }
export const HooksConfig = Schema.Struct({ "hooks": Schema.optionalKey(Schema.Array(HookEntry)) }).annotate({ "description": "The operator's hook configuration — the `hooks.json` document and the `/api/v1/hooks` body.", "identifier": "HooksConfig" })
export type DisplayPolicy = { readonly "capture_monitor"?: string | null, readonly "ddc_power_off"?: boolean, readonly "game_session"?: GameSession, readonly "identity"?: Identity, readonly "keep_alive"?: KeepAlive, readonly "layout"?: Layout, readonly "max_displays"?: number, readonly "mode_conflict"?: ModeConflict, readonly "pnp_disable_monitors"?: boolean, readonly "preset"?: Preset, readonly "topology"?: Topology, readonly "version"?: number }
export const DisplayPolicy = Schema.Struct({ "capture_monitor": Schema.optionalKey(Schema.Union([Schema.String, Schema.Null]).annotate({ "description": "**Mirror a physical monitor instead of creating a virtual display**: the connector name\n(`DP-1`, `HDMI-A-2`) sessions should stream, or `None` for the normal virtual-display path.\n\nOrthogonal to `preset`/lifecycle (like `game_session`): a preset change never clears it, and\n`#[serde(default)]` leaves existing `display-settings.json` files untouched. It is a\n**host-wide** setting, not per-client — the host-pinned decision of record in\n`design/per-monitor-portal-capture.md` §5.3. `SLIPSTREAM_CAPTURE_MONITOR` overrides it (see\n[`capture_monitor`]), so an appliance can pin in `host.env` without the console fighting it." })), "ddc_power_off": Schema.optionalKey(Schema.Boolean.annotate({ "description": "EXPERIMENTAL: command physical monitors' panels off over DDC/CI (VCP 0xD6 → DPMS off)\nright before an `Exclusive` isolate deactivates them, and back on at restore. Targets the\n\"connected-but-dark head\" periodic-stutter class (monitor standby auto-input-scan / DP link\nchurn while the virtual display is the sole active display) at the monitor-firmware level.\nLinux uses `ddcutil` when available.\nBest-effort - monitors without DDC/CI (or with it disabled in the OSD) are skipped.\nOrthogonal to `preset` (like `game_session`): preserved across preset changes;\n`#[serde(default)]` = off so existing `display-settings.json` files are untouched." })), "game_session": Schema.optionalKey(Schema.suspend((): Schema.Codec<GameSession> => GameSession).annotate({ "description": "How a game-launching session is served (`design/gamemode-and-dedicated-sessions.md` §5.2).\nOrthogonal to `preset`/lifecycle — preserved across preset changes; `#[serde(default)]` = `Auto`\nso existing `display-settings.json` files are untouched." })), "identity": Schema.optionalKey(Identity), "keep_alive": Schema.optionalKey(KeepAlive), "layout": Schema.optionalKey(Layout), "max_displays": Schema.optionalKey(Schema.Number.annotate({ "description": "Upper bound on simultaneously-live virtual displays (clamped to `1..=16` on write).", "format": "int32" }).check(Schema.isInt().annotate({ "expected": "an integer" })).check(Schema.isGreaterThanOrEqualTo(0).annotate({ "expected": "a value greater than or equal to 0" }))), "mode_conflict": Schema.optionalKey(ModeConflict), "pnp_disable_monitors": Schema.optionalKey(Schema.Boolean.annotate({ "description": "EXPERIMENTAL: silence idle / standby monitors for the stream's duration and restore them\nat teardown. On Linux this force-offs connected external DRM connectors via sysfs\n(`/sys/class/drm/*/status`). Targets the\nsame \"connected-but-dark head\" periodic-stutter class as [`Self::ddc_power_off`], but at the\nOS reaction level (HPD / auto-input scan no longer wakes the desktop stack). A crash-recovery\njournal restores leftovers on host startup. Orthogonal to `preset` (like `game_session`);\n`#[serde(default)]` = off." })), "preset": Schema.optionalKey(Preset), "topology": Schema.optionalKey(Topology), "version": Schema.optionalKey(Schema.Number.annotate({ "description": "Schema version (currently 1) — lets a future field addition migrate rather than reject.", "format": "int32" }).check(Schema.isInt().annotate({ "expected": "an integer" })).check(Schema.isGreaterThanOrEqualTo(0).annotate({ "expected": "a value greater than or equal to 0" }))) }).annotate({ "description": "The user-facing display-management policy — what `display-settings.json` holds and what the mgmt\nAPI GETs/PUTs. When [`preset`](Self::preset) is not [`Preset::Custom`] the explicit fields are\nignored (the console writes one or the other); [`effective`](Self::effective) resolves both to a\nsingle [`EffectivePolicy`].", "identifier": "DisplayPolicy" })
export type EffectivePolicy = { readonly "identity": Identity, readonly "keep_alive": KeepAlive, readonly "layout": Layout, readonly "max_displays": number, readonly "mode_conflict": ModeConflict, readonly "topology": Topology }
export const EffectivePolicy = Schema.Struct({ "identity": Identity, "keep_alive": KeepAlive, "layout": Layout, "max_displays": Schema.Number.annotate({ "format": "int32" }).check(Schema.isInt().annotate({ "expected": "an integer" })).check(Schema.isGreaterThanOrEqualTo(0).annotate({ "expected": "a value greater than or equal to 0" })), "mode_conflict": ModeConflict, "topology": Topology }).annotate({ "description": "The six resolved fields after preset expansion — what the lifecycle/registry and the Stage-0 call\nsites read, and what the mgmt API echoes as the \"currently in force\" policy. Pure output of\n[`DisplayPolicy::effective`].", "identifier": "EffectivePolicy" })
export type CustomPreset = { readonly "fields": EffectivePolicy, readonly "game_session"?: GameSession, readonly "id": string, readonly "name": string }
export const CustomPreset = Schema.Struct({ "fields": Schema.suspend((): Schema.Codec<EffectivePolicy> => EffectivePolicy).annotate({ "description": "The six display-behavior axes this preset applies (the same shape a built-in preset expands to)." }), "game_session": Schema.optionalKey(Schema.suspend((): Schema.Codec<GameSession> => GameSession).annotate({ "description": "The game-session routing this preset applies (orthogonal to the six axes; see [`GameSession`]).\nA custom preset captures the operator's *full* setup, so — unlike a built-in preset — applying\none does set this axis." })), "id": Schema.String.annotate({ "description": "Host-assigned, stable for the life of the entry (the `{id}` in the CRUD path)." }), "name": Schema.String.annotate({ "description": "User-facing name shown on the preset card; editable." }) }).annotate({ "description": "A user-defined named preset: a saved bundle of the six display-behavior axes (exactly what a\nbuilt-in [`Preset`] expands to) plus the orthogonal game-session axis, that the operator names\nand applies from the console.\n\nUnlike the built-in [`Preset`]s (a closed enum), custom presets are **data** — a catalog stored in\n`<config>/display-presets.json`. Applying one writes a `Custom` [`DisplayPolicy`] carrying these\nfields (the console reuses `PUT /display/settings`), so [`DisplayPolicy::effective`] stays pure and\nthe built-in set is never touched. The catalog is decoupled from the active `display-settings.json`:\nediting or deleting a preset never mutates the running policy (re-apply to adopt a change).", "identifier": "CustomPreset" })
export type CustomPresetInput = { readonly "fields": EffectivePolicy, readonly "game_session"?: GameSession, readonly "name": string }
export const CustomPresetInput = Schema.Struct({ "fields": EffectivePolicy, "game_session": Schema.optionalKey(GameSession), "name": Schema.String }).annotate({ "description": "Request body to create or replace a custom preset (no `id` — the host owns it).", "identifier": "CustomPresetInput" })
export type PresetInfo = { readonly "fields": EffectivePolicy, readonly "id": string, readonly "summary": string }
export const PresetInfo = Schema.Struct({ "fields": Schema.suspend((): Schema.Codec<EffectivePolicy> => EffectivePolicy).annotate({ "description": "The effective policy this preset expands to (the same fields a `custom` policy carries)." }), "id": Schema.String.annotate({ "description": "The preset id (`default` | `gaming-rig` | `shared-desktop` | `hotdesk` | `workstation`)." }), "summary": Schema.String.annotate({ "description": "One-line story shown next to the option." }) }).annotate({ "description": "One preset's human-facing description + the fields it expands to, so the console can render a\npreset picker with an accurate \"what this does\" preview without hardcoding the expansion.", "identifier": "PresetInfo" })
export type DisplaySettingsState = { readonly "configured": boolean, readonly "custom_presets": ReadonlyArray<CustomPreset>, readonly "effective": EffectivePolicy, readonly "enforced": ReadonlyArray<string>, readonly "presets": ReadonlyArray<PresetInfo>, readonly "settings": DisplayPolicy }
export const DisplaySettingsState = Schema.Struct({ "configured": Schema.Boolean.annotate({ "description": "True once a `display-settings.json` exists (the console has configured this host)." }), "custom_presets": Schema.Array(CustomPreset).annotate({ "description": "The operator's saved custom presets (`display-presets.json`) — named field-bundles rendered\nalongside the built-ins. Managed via `POST/PUT/DELETE /display/presets`; applied by writing a\n`Custom` policy carrying the preset's fields." }), "effective": Schema.suspend((): Schema.Codec<EffectivePolicy> => EffectivePolicy).annotate({ "description": "The effective (preset-expanded) policy currently in force." }), "enforced": Schema.Array(Schema.String).annotate({ "description": "Option names this build enforces right now. All five axes are now acted on (keep_alive +\ntopology since Stage 0-2, identity Stage 3, mode_conflict Stage 4, layout Stage 5) — the console\nreads this to know which controls are live vs. \"coming soon\" (per-backend nuance, e.g. layout\nposition apply being KWin-only, is reported per display in `/display/state`)." }), "presets": Schema.Array(PresetInfo).annotate({ "description": "Every named preset and what it expands to (for the picker's preview)." }), "settings": Schema.suspend((): Schema.Codec<DisplayPolicy> => DisplayPolicy).annotate({ "description": "The stored policy (preset + custom fields), or the built-in default when unconfigured." }) }).annotate({ "description": "Full display-management state for the console: the stored policy, every preset's expansion, the\nresolved effective policy, and which options this build actually enforces yet (Stage 0 wires\nkeep-alive linger + topology; the rest are stored but not yet acted on).", "identifier": "DisplaySettingsState" })
// schemas
export type ListCaptureMethods200 = ReadonlyArray<AvailableCaptureMethod>
export const ListCaptureMethods200 = Schema.Array(AvailableCaptureMethod)
export type ListCaptureMethods401 = ApiError
export const ListCaptureMethods401 = ApiError
export type ListPairedClients200 = ReadonlyArray<PairedClient>
export const ListPairedClients200 = Schema.Array(PairedClient)
export type ListPairedClients401 = ApiError
export const ListPairedClients401 = ApiError
export type UnpairClient400 = ApiError
export const UnpairClient400 = ApiError
export type UnpairClient401 = ApiError
export const UnpairClient401 = ApiError
export type UnpairClient404 = ApiError
export const UnpairClient404 = ApiError
export type ListCompositors200 = ReadonlyArray<AvailableCompositor>
export const ListCompositors200 = Schema.Array(AvailableCompositor)
export type ListCompositors401 = ApiError
export const ListCompositors401 = ApiError
export type ListHeadlessCompositors200 = ReadonlyArray<AvailableHeadlessCompositor>
export const ListHeadlessCompositors200 = Schema.Array(AvailableHeadlessCompositor)
export type ListHeadlessCompositors401 = ApiError
export const ListHeadlessCompositors401 = ApiError
export type GetDiagnosticsPreflight200 = PreflightReport
export const GetDiagnosticsPreflight200 = PreflightReport
export type GetDiagnosticsPreflight401 = ApiError
export const GetDiagnosticsPreflight401 = ApiError
export type SetDisplayLayoutRequestJson = DisplayLayoutRequest
export const SetDisplayLayoutRequestJson = DisplayLayoutRequest
export type SetDisplayLayout200 = DisplaySettingsState
export const SetDisplayLayout200 = DisplaySettingsState
export type SetDisplayLayout401 = ApiError
export const SetDisplayLayout401 = ApiError
export type SetDisplayLayout500 = ApiError
export const SetDisplayLayout500 = ApiError
export type GetDisplayMonitors200 = MonitorsResponse
export const GetDisplayMonitors200 = MonitorsResponse
export type GetDisplayMonitors401 = ApiError
export const GetDisplayMonitors401 = ApiError
export type ListCustomPresets200 = ReadonlyArray<CustomPreset>
export const ListCustomPresets200 = Schema.Array(CustomPreset)
export type ListCustomPresets401 = ApiError
export const ListCustomPresets401 = ApiError
export type CreateCustomPresetRequestJson = CustomPresetInput
export const CreateCustomPresetRequestJson = CustomPresetInput
export type CreateCustomPreset201 = CustomPreset
export const CreateCustomPreset201 = CustomPreset
export type CreateCustomPreset400 = ApiError
export const CreateCustomPreset400 = ApiError
export type CreateCustomPreset401 = ApiError
export const CreateCustomPreset401 = ApiError
export type CreateCustomPreset500 = ApiError
export const CreateCustomPreset500 = ApiError
export type UpdateCustomPresetRequestJson = CustomPresetInput
export const UpdateCustomPresetRequestJson = CustomPresetInput
export type UpdateCustomPreset200 = CustomPreset
export const UpdateCustomPreset200 = CustomPreset
export type UpdateCustomPreset400 = ApiError
export const UpdateCustomPreset400 = ApiError
export type UpdateCustomPreset401 = ApiError
export const UpdateCustomPreset401 = ApiError
export type UpdateCustomPreset404 = ApiError
export const UpdateCustomPreset404 = ApiError
export type UpdateCustomPreset500 = ApiError
export const UpdateCustomPreset500 = ApiError
export type DeleteCustomPreset401 = ApiError
export const DeleteCustomPreset401 = ApiError
export type DeleteCustomPreset404 = ApiError
export const DeleteCustomPreset404 = ApiError
export type DeleteCustomPreset500 = ApiError
export const DeleteCustomPreset500 = ApiError
export type ReleaseDisplayRequestJson = ReleaseDisplayRequest
export const ReleaseDisplayRequestJson = ReleaseDisplayRequest
export type ReleaseDisplay200 = ReleaseDisplayResult
export const ReleaseDisplay200 = ReleaseDisplayResult
export type ReleaseDisplay401 = ApiError
export const ReleaseDisplay401 = ApiError
export type GetDisplaySettings200 = DisplaySettingsState
export const GetDisplaySettings200 = DisplaySettingsState
export type GetDisplaySettings401 = ApiError
export const GetDisplaySettings401 = ApiError
export type SetDisplaySettingsRequestJson = DisplayPolicy
export const SetDisplaySettingsRequestJson = DisplayPolicy
export type SetDisplaySettings200 = DisplaySettingsState
export const SetDisplaySettings200 = DisplaySettingsState
export type SetDisplaySettings400 = ApiError
export const SetDisplaySettings400 = ApiError
export type SetDisplaySettings401 = ApiError
export const SetDisplaySettings401 = ApiError
export type SetDisplaySettings500 = ApiError
export const SetDisplaySettings500 = ApiError
export type GetDisplayState200 = DisplayStateResponse
export const GetDisplayState200 = DisplayStateResponse
export type GetDisplayState401 = ApiError
export const GetDisplayState401 = ApiError
export type StreamEventsParams = { readonly "since"?: number, readonly "kinds"?: string, readonly "Last-Event-ID"?: string | null }
export const StreamEventsParams = Schema.Struct({ "since": Schema.optionalKey(Schema.Number.annotate({ "format": "int64" }).check(Schema.isInt().annotate({ "expected": "an integer" })).check(Schema.isGreaterThanOrEqualTo(0).annotate({ "expected": "a value greater than or equal to 0" }))), "kinds": Schema.optionalKey(Schema.String), "Last-Event-ID": Schema.optionalKey(Schema.Union([Schema.String, Schema.Null])) })
export type StreamEvents200Sse = HostEvent
export const StreamEvents200Sse = HostEvent
export type StreamEvents401 = ApiError
export const StreamEvents401 = ApiError
export type StreamEvents503 = ApiError
export const StreamEvents503 = ApiError
export type EndGameRequestJson = EndGameRequest
export const EndGameRequestJson = EndGameRequest
export type EndGame200 = EndGameResult
export const EndGame200 = EndGameResult
export type EndGame401 = ApiError
export const EndGame401 = ApiError
export type EndGame409 = ApiError
export const EndGame409 = ApiError
export type ListGpus200 = GpuState
export const ListGpus200 = GpuState
export type ListGpus401 = ApiError
export const ListGpus401 = ApiError
export type SetGpuPreferenceRequestJson = SetGpuPreference
export const SetGpuPreferenceRequestJson = SetGpuPreference
export type SetGpuPreference200 = GpuState
export const SetGpuPreference200 = GpuState
export type SetGpuPreference400 = ApiError
export const SetGpuPreference400 = ApiError
export type SetGpuPreference401 = ApiError
export const SetGpuPreference401 = ApiError
export type SetGpuPreference500 = ApiError
export const SetGpuPreference500 = ApiError
export type GetHealth200 = Health
export const GetHealth200 = Health
export type GetHooks200 = HooksConfig
export const GetHooks200 = HooksConfig
export type GetHooks401 = ApiError
export const GetHooks401 = ApiError
export type SetHooksRequestJson = HooksConfig
export const SetHooksRequestJson = HooksConfig
export type SetHooks200 = HooksConfig
export const SetHooks200 = HooksConfig
export type SetHooks400 = ApiError
export const SetHooks400 = ApiError
export type SetHooks401 = ApiError
export const SetHooks401 = ApiError
export type SetHooks500 = ApiError
export const SetHooks500 = ApiError
export type GetHostInfo200 = HostInfo
export const GetHostInfo200 = HostInfo
export type GetHostInfo401 = ApiError
export const GetHostInfo401 = ApiError
export type GetHostConfig200 = HostConfigState
export const GetHostConfig200 = HostConfigState
export type GetHostConfig401 = ApiError
export const GetHostConfig401 = ApiError
export type SetHostConfigRequestJson = HostConfigFile
export const SetHostConfigRequestJson = HostConfigFile
export type SetHostConfig200 = HostConfigState
export const SetHostConfig200 = HostConfigState
export type SetHostConfig400 = ApiError
export const SetHostConfig400 = ApiError
export type SetHostConfig401 = ApiError
export const SetHostConfig401 = ApiError
export type SetHostConfig500 = ApiError
export const SetHostConfig500 = ApiError
export type SetMoonlightBroadcastRequestJson = MoonlightBroadcastRequest
export const SetMoonlightBroadcastRequestJson = MoonlightBroadcastRequest
export type SetMoonlightBroadcast200 = HostConfigState
export const SetMoonlightBroadcast200 = HostConfigState
export type SetMoonlightBroadcast401 = ApiError
export const SetMoonlightBroadcast401 = ApiError
export type SetMoonlightBroadcast500 = ApiError
export const SetMoonlightBroadcast500 = ApiError
export type RestartHost401 = ApiError
export const RestartHost401 = ApiError
export type RestartHost500 = ApiError
export const RestartHost500 = ApiError
export type ShutdownHost401 = ApiError
export const ShutdownHost401 = ApiError
export type GetLibraryParams = { readonly "provider"?: string, readonly "platform"?: string }
export const GetLibraryParams = Schema.Struct({ "provider": Schema.optionalKey(Schema.String), "platform": Schema.optionalKey(Schema.String) })
export type GetLibrary200 = ReadonlyArray<GameEntry>
export const GetLibrary200 = Schema.Array(GameEntry)
export type GetLibrary401 = ApiError
export const GetLibrary401 = ApiError
export type GetLibraryArt401 = ApiError
export const GetLibraryArt401 = ApiError
export type GetLibraryArt404 = ApiError
export const GetLibraryArt404 = ApiError
export type CreateCustomGameRequestJson = CustomInput
export const CreateCustomGameRequestJson = CustomInput
export type CreateCustomGame201 = CustomEntry
export const CreateCustomGame201 = CustomEntry
export type CreateCustomGame400 = ApiError
export const CreateCustomGame400 = ApiError
export type CreateCustomGame401 = ApiError
export const CreateCustomGame401 = ApiError
export type CreateCustomGame500 = ApiError
export const CreateCustomGame500 = ApiError
export type UpdateCustomGameRequestJson = CustomInput
export const UpdateCustomGameRequestJson = CustomInput
export type UpdateCustomGame200 = CustomEntry
export const UpdateCustomGame200 = CustomEntry
export type UpdateCustomGame400 = ApiError
export const UpdateCustomGame400 = ApiError
export type UpdateCustomGame401 = ApiError
export const UpdateCustomGame401 = ApiError
export type UpdateCustomGame404 = ApiError
export const UpdateCustomGame404 = ApiError
export type UpdateCustomGame500 = ApiError
export const UpdateCustomGame500 = ApiError
export type DeleteCustomGame401 = ApiError
export const DeleteCustomGame401 = ApiError
export type DeleteCustomGame404 = ApiError
export const DeleteCustomGame404 = ApiError
export type DeleteCustomGame500 = ApiError
export const DeleteCustomGame500 = ApiError
export type ReconcileProviderEntriesRequestJson = ReadonlyArray<ProviderEntryInput>
export const ReconcileProviderEntriesRequestJson = Schema.Array(ProviderEntryInput)
export type ReconcileProviderEntries200 = ReadonlyArray<CustomEntry>
export const ReconcileProviderEntries200 = Schema.Array(CustomEntry)
export type ReconcileProviderEntries400 = ApiError
export const ReconcileProviderEntries400 = ApiError
export type ReconcileProviderEntries401 = ApiError
export const ReconcileProviderEntries401 = ApiError
export type ReconcileProviderEntries500 = ApiError
export const ReconcileProviderEntries500 = ApiError
export type DeleteProviderEntries200 = ProviderRemoved
export const DeleteProviderEntries200 = ProviderRemoved
export type DeleteProviderEntries400 = ApiError
export const DeleteProviderEntries400 = ApiError
export type DeleteProviderEntries401 = ApiError
export const DeleteProviderEntries401 = ApiError
export type DeleteProviderEntries500 = ApiError
export const DeleteProviderEntries500 = ApiError
export type ListLibraryScanners200 = ReadonlyArray<ScannerInfo>
export const ListLibraryScanners200 = Schema.Array(ScannerInfo)
export type ListLibraryScanners401 = ApiError
export const ListLibraryScanners401 = ApiError
export type SetLibraryScannerRequestJson = ScannerToggle
export const SetLibraryScannerRequestJson = ScannerToggle
export type SetLibraryScanner200 = ReadonlyArray<ScannerInfo>
export const SetLibraryScanner200 = Schema.Array(ScannerInfo)
export type SetLibraryScanner401 = ApiError
export const SetLibraryScanner401 = ApiError
export type SetLibraryScanner404 = ApiError
export const SetLibraryScanner404 = ApiError
export type SetLibraryScanner500 = ApiError
export const SetLibraryScanner500 = ApiError
export type GetLocalSummary200 = LocalSummary
export const GetLocalSummary200 = LocalSummary
export type GetLocalSummary401 = ApiError
export const GetLocalSummary401 = ApiError
export type LogsGetParams = { readonly "after"?: number, readonly "limit"?: number }
export const LogsGetParams = Schema.Struct({ "after": Schema.optionalKey(Schema.Number.annotate({ "format": "int64" }).check(Schema.isInt().annotate({ "expected": "an integer" })).check(Schema.isGreaterThanOrEqualTo(0).annotate({ "expected": "a value greater than or equal to 0" }))), "limit": Schema.optionalKey(Schema.Number.annotate({ "format": "int32" }).check(Schema.isInt().annotate({ "expected": "an integer" })).check(Schema.isGreaterThanOrEqualTo(0).annotate({ "expected": "a value greater than or equal to 0" }))) })
export type LogsGet200 = LogPage
export const LogsGet200 = LogPage
export type LogsGet401 = ApiError
export const LogsGet401 = ApiError
export type ListNativeClients200 = ReadonlyArray<NativeClient>
export const ListNativeClients200 = Schema.Array(NativeClient)
export type ListNativeClients401 = ApiError
export const ListNativeClients401 = ApiError
export type UnpairNativeClient401 = ApiError
export const UnpairNativeClient401 = ApiError
export type UnpairNativeClient404 = ApiError
export const UnpairNativeClient404 = ApiError
export type UnpairNativeClient503 = ApiError
export const UnpairNativeClient503 = ApiError
export type GetNativePairing200 = NativePairStatus
export const GetNativePairing200 = NativePairStatus
export type GetNativePairing401 = ApiError
export const GetNativePairing401 = ApiError
export type DisarmNativePairing401 = ApiError
export const DisarmNativePairing401 = ApiError
export type DisarmNativePairing503 = ApiError
export const DisarmNativePairing503 = ApiError
export type ArmNativePairingRequestJson = ArmNativePairing
export const ArmNativePairingRequestJson = ArmNativePairing
export type ArmNativePairing200 = NativePairStatus
export const ArmNativePairing200 = NativePairStatus
export type ArmNativePairing401 = ApiError
export const ArmNativePairing401 = ApiError
export type ArmNativePairing503 = ApiError
export const ArmNativePairing503 = ApiError
export type ListPendingDevices200 = ReadonlyArray<PendingDevice>
export const ListPendingDevices200 = Schema.Array(PendingDevice)
export type ListPendingDevices401 = ApiError
export const ListPendingDevices401 = ApiError
export type ApprovePendingDeviceRequestJson = ApprovePending
export const ApprovePendingDeviceRequestJson = ApprovePending
export type ApprovePendingDevice200 = NativeClient
export const ApprovePendingDevice200 = NativeClient
export type ApprovePendingDevice401 = ApiError
export const ApprovePendingDevice401 = ApiError
export type ApprovePendingDevice404 = ApiError
export const ApprovePendingDevice404 = ApiError
export type ApprovePendingDevice500 = ApiError
export const ApprovePendingDevice500 = ApiError
export type ApprovePendingDevice503 = ApiError
export const ApprovePendingDevice503 = ApiError
export type DenyPendingDevice401 = ApiError
export const DenyPendingDevice401 = ApiError
export type DenyPendingDevice404 = ApiError
export const DenyPendingDevice404 = ApiError
export type DenyPendingDevice503 = ApiError
export const DenyPendingDevice503 = ApiError
export type GetPairingStatus200 = PairingStatus
export const GetPairingStatus200 = PairingStatus
export type GetPairingStatus401 = ApiError
export const GetPairingStatus401 = ApiError
export type SubmitPairingPinRequestJson = SubmitPin
export const SubmitPairingPinRequestJson = SubmitPin
export type SubmitPairingPin400 = ApiError
export const SubmitPairingPin400 = ApiError
export type SubmitPairingPin401 = ApiError
export const SubmitPairingPin401 = ApiError
export type SubmitPairingPin409 = ApiError
export const SubmitPairingPin409 = ApiError
export type SubmitPairingPin415 = ApiError
export const SubmitPairingPin415 = ApiError
export type SubmitPairingPin422 = ApiError
export const SubmitPairingPin422 = ApiError
export type ListPlugins200 = ReadonlyArray<PluginSummary>
export const ListPlugins200 = Schema.Array(PluginSummary)
export type ListPlugins401 = ApiError
export const ListPlugins401 = ApiError
export type RegisterPluginRequestJson = PluginRegistration
export const RegisterPluginRequestJson = PluginRegistration
export type RegisterPlugin400 = ApiError
export const RegisterPlugin400 = ApiError
export type RegisterPlugin401 = ApiError
export const RegisterPlugin401 = ApiError
export type DeregisterPlugin401 = ApiError
export const DeregisterPlugin401 = ApiError
export type GetPluginUiCredential200 = UiCredential
export const GetPluginUiCredential200 = UiCredential
export type GetPluginUiCredential401 = ApiError
export const GetPluginUiCredential401 = ApiError
export type GetPluginUiCredential404 = ApiError
export const GetPluginUiCredential404 = ApiError
export type StopSession401 = ApiError
export const StopSession401 = ApiError
export type RequestIdr401 = ApiError
export const RequestIdr401 = ApiError
export type RequestIdr409 = ApiError
export const RequestIdr409 = ApiError
export type GetSessionSettings200 = SessionSettingsState
export const GetSessionSettings200 = SessionSettingsState
export type GetSessionSettings401 = ApiError
export const GetSessionSettings401 = ApiError
export type SetSessionSettingsRequestJson = SessionSettings
export const SetSessionSettingsRequestJson = SessionSettings
export type SetSessionSettings200 = SessionSettingsState
export const SetSessionSettings200 = SessionSettingsState
export type SetSessionSettings400 = ApiError
export const SetSessionSettings400 = ApiError
export type SetSessionSettings401 = ApiError
export const SetSessionSettings401 = ApiError
export type SetSessionSettings500 = ApiError
export const SetSessionSettings500 = ApiError
export type StatsCaptureLive200 = Capture
export const StatsCaptureLive200 = Capture
export type StatsCaptureLive401 = ApiError
export const StatsCaptureLive401 = ApiError
export type StatsCaptureLive404 = ApiError
export const StatsCaptureLive404 = ApiError
export type StatsCaptureStart200 = StatsStatus
export const StatsCaptureStart200 = StatsStatus
export type StatsCaptureStart401 = ApiError
export const StatsCaptureStart401 = ApiError
export type StatsCaptureStatus200 = StatsStatus
export const StatsCaptureStatus200 = StatsStatus
export type StatsCaptureStatus401 = ApiError
export const StatsCaptureStatus401 = ApiError
export type StatsCaptureStop200 = CaptureMeta
export const StatsCaptureStop200 = CaptureMeta
export type StatsCaptureStop401 = ApiError
export const StatsCaptureStop401 = ApiError
export type StatsCaptureStop500 = ApiError
export const StatsCaptureStop500 = ApiError
export type StatsRecordingsList200 = ReadonlyArray<CaptureMeta>
export const StatsRecordingsList200 = Schema.Array(CaptureMeta)
export type StatsRecordingsList401 = ApiError
export const StatsRecordingsList401 = ApiError
export type StatsRecordingGet200 = Capture
export const StatsRecordingGet200 = Capture
export type StatsRecordingGet401 = ApiError
export const StatsRecordingGet401 = ApiError
export type StatsRecordingGet404 = ApiError
export const StatsRecordingGet404 = ApiError
export type StatsRecordingGet500 = ApiError
export const StatsRecordingGet500 = ApiError
export type StatsRecordingDelete401 = ApiError
export const StatsRecordingDelete401 = ApiError
export type StatsRecordingDelete404 = ApiError
export const StatsRecordingDelete404 = ApiError
export type StatsRecordingDelete500 = ApiError
export const StatsRecordingDelete500 = ApiError
export type GetStatus200 = RuntimeStatus
export const GetStatus200 = RuntimeStatus
export type GetStatus401 = ApiError
export const GetStatus401 = ApiError
export type GetPluginCatalog200 = CatalogResponse
export const GetPluginCatalog200 = CatalogResponse
export type GetPluginCatalog401 = ApiError
export const GetPluginCatalog401 = ApiError
export type GetPluginCatalog403 = ApiError
export const GetPluginCatalog403 = ApiError
export type InstallPluginRequestJson = InstallRequest
export const InstallPluginRequestJson = InstallRequest
export type InstallPlugin202 = JobRef
export const InstallPlugin202 = JobRef
export type InstallPlugin400 = ApiError
export const InstallPlugin400 = ApiError
export type InstallPlugin401 = ApiError
export const InstallPlugin401 = ApiError
export type InstallPlugin403 = ApiError
export const InstallPlugin403 = ApiError
export type InstallPlugin409 = ApiError
export const InstallPlugin409 = ApiError
export type ListInstalledPlugins200 = ReadonlyArray<InstalledView>
export const ListInstalledPlugins200 = Schema.Array(InstalledView)
export type ListInstalledPlugins401 = ApiError
export const ListInstalledPlugins401 = ApiError
export type ListInstalledPlugins403 = ApiError
export const ListInstalledPlugins403 = ApiError
export type ListPluginJobs200 = ReadonlyArray<Job>
export const ListPluginJobs200 = Schema.Array(Job)
export type ListPluginJobs401 = ApiError
export const ListPluginJobs401 = ApiError
export type ListPluginJobs403 = ApiError
export const ListPluginJobs403 = ApiError
export type GetPluginJob200 = Job
export const GetPluginJob200 = Job
export type GetPluginJob401 = ApiError
export const GetPluginJob401 = ApiError
export type GetPluginJob403 = ApiError
export const GetPluginJob403 = ApiError
export type GetPluginJob404 = ApiError
export const GetPluginJob404 = ApiError
export type RefreshPluginCatalog200 = CatalogResponse
export const RefreshPluginCatalog200 = CatalogResponse
export type RefreshPluginCatalog401 = ApiError
export const RefreshPluginCatalog401 = ApiError
export type RefreshPluginCatalog403 = ApiError
export const RefreshPluginCatalog403 = ApiError
export type GetPluginRuntime200 = RuntimeView
export const GetPluginRuntime200 = RuntimeView
export type GetPluginRuntime401 = ApiError
export const GetPluginRuntime401 = ApiError
export type GetPluginRuntime403 = ApiError
export const GetPluginRuntime403 = ApiError
export type SetPluginRuntimeRequestJson = RuntimeRequest
export const SetPluginRuntimeRequestJson = RuntimeRequest
export type SetPluginRuntime200 = RuntimeView
export const SetPluginRuntime200 = RuntimeView
export type SetPluginRuntime400 = ApiError
export const SetPluginRuntime400 = ApiError
export type SetPluginRuntime401 = ApiError
export const SetPluginRuntime401 = ApiError
export type SetPluginRuntime403 = ApiError
export const SetPluginRuntime403 = ApiError
export type ListPluginSources200 = ReadonlyArray<SourceView>
export const ListPluginSources200 = Schema.Array(SourceView)
export type ListPluginSources401 = ApiError
export const ListPluginSources401 = ApiError
export type ListPluginSources403 = ApiError
export const ListPluginSources403 = ApiError
export type PutPluginSourceRequestJson = SourceInput
export const PutPluginSourceRequestJson = SourceInput
export type PutPluginSource400 = ApiError
export const PutPluginSource400 = ApiError
export type PutPluginSource401 = ApiError
export const PutPluginSource401 = ApiError
export type PutPluginSource403 = ApiError
export const PutPluginSource403 = ApiError
export type DeletePluginSource401 = ApiError
export const DeletePluginSource401 = ApiError
export type DeletePluginSource403 = ApiError
export const DeletePluginSource403 = ApiError
export type UninstallPluginRequestJson = UninstallRequest
export const UninstallPluginRequestJson = UninstallRequest
export type UninstallPlugin202 = JobRef
export const UninstallPlugin202 = JobRef
export type UninstallPlugin400 = ApiError
export const UninstallPlugin400 = ApiError
export type UninstallPlugin401 = ApiError
export const UninstallPlugin401 = ApiError
export type UninstallPlugin403 = ApiError
export const UninstallPlugin403 = ApiError
export type UninstallPlugin409 = ApiError
export const UninstallPlugin409 = ApiError
export type SupportBundleCreate200 = SupportBundle
export const SupportBundleCreate200 = SupportBundle
export type SupportBundleCreate401 = ApiError
export const SupportBundleCreate401 = ApiError
export type SupportBundleCreate500 = ApiError
export const SupportBundleCreate500 = ApiError
export type SupportBundleGet200 = SupportBundle
export const SupportBundleGet200 = SupportBundle
export type SupportBundleGet401 = ApiError
export const SupportBundleGet401 = ApiError
export type SupportBundleGet404 = ApiError
export const SupportBundleGet404 = ApiError
export type SupportBundleGet500 = ApiError
export const SupportBundleGet500 = ApiError
export type SupportBundleDelete401 = ApiError
export const SupportBundleDelete401 = ApiError
export type SupportBundleDelete404 = ApiError
export const SupportBundleDelete404 = ApiError
export type ApplyUpdateRequestJson = ApplyRequest
export const ApplyUpdateRequestJson = ApplyRequest
export type ApplyUpdate202 = UpdateStatus
export const ApplyUpdate202 = UpdateStatus
export type ApplyUpdate401 = ApiError
export const ApplyUpdate401 = ApiError
export type ApplyUpdate409 = ApiError
export const ApplyUpdate409 = ApiError
export type ForceUpdateCheck200 = UpdateStatus
export const ForceUpdateCheck200 = UpdateStatus
export type ForceUpdateCheck401 = ApiError
export const ForceUpdateCheck401 = ApiError
export type ForceUpdateCheck409 = ApiError
export const ForceUpdateCheck409 = ApiError
export type ForceUpdateCheck429 = ApiError
export const ForceUpdateCheck429 = ApiError
export type GetUpdateStatus200 = UpdateStatus
export const GetUpdateStatus200 = UpdateStatus
export type GetUpdateStatus401 = ApiError
export const GetUpdateStatus401 = ApiError

export interface OperationConfig {
  /**
   * Whether or not the response should be included in the value returned from
   * an operation.
   *
   * If set to `true`, a tuple of `[A, HttpClientResponse]` will be returned,
   * where `A` is the success type of the operation.
   *
   * If set to `false`, only the success type of the operation will be returned.
   */
  readonly includeResponse?: boolean | undefined
}

/**
 * A utility type which optionally includes the response in the return result
 * of an operation based upon the value of the `includeResponse` configuration
 * option.
 */
export type WithOptionalResponse<A, Config extends OperationConfig> = Config extends {
  readonly includeResponse: true
} ? [A, HttpClientResponse.HttpClientResponse] : A

export const make = (
  httpClient: HttpClient.HttpClient,
  options: {
    readonly transformClient?: ((client: HttpClient.HttpClient) => Effect.Effect<HttpClient.HttpClient>) | undefined
  } = {}
): Slipstream => {
  const unexpectedStatus = (response: HttpClientResponse.HttpClientResponse) =>
    Effect.flatMap(
      Effect.orElseSucceed(response.json, () => "Unexpected status code"),
      (description) =>
        Effect.fail(
          new HttpClientError.HttpClientError({
            reason: new HttpClientError.StatusCodeError({
              request: response.request,
              response,
              description: typeof description === "string" ? description : JSON.stringify(description),
            }),
          }),
        ),
    )
  const withResponse = <Config extends OperationConfig>(config: Config | undefined) => (
    f: (response: HttpClientResponse.HttpClientResponse) => Effect.Effect<any, any>,
  ): (request: HttpClientRequest.HttpClientRequest) => Effect.Effect<any, any> => {
    const withOptionalResponse = (
      config?.includeResponse
        ? (response: HttpClientResponse.HttpClientResponse) => Effect.map(f(response), (a) => [a, response])
        : (response: HttpClientResponse.HttpClientResponse) => f(response)
    ) as any
    return options?.transformClient
      ? (request) =>
          Effect.flatMap(
            Effect.flatMap(options.transformClient!(httpClient), (client) => client.execute(request)),
            withOptionalResponse
          )
      : (request) => Effect.flatMap(httpClient.execute(request), withOptionalResponse)
  }
  const sseRequest = <
     Type,
     DecodingServices
    >(
      schema: Schema.ConstraintDecoder<Type, DecodingServices>
    ) =>
    (
      request: HttpClientRequest.HttpClientRequest
    ): Stream.Stream<
      { readonly event: string; readonly id: string | undefined; readonly data: Type },
      HttpClientError.HttpClientError | SchemaError | Sse.Retry,
      DecodingServices
    > =>
      HttpClient.filterStatusOk(httpClient).execute(request).pipe(
        Effect.map((response) => response.stream),
        Stream.unwrap,
        Stream.decodeText(),
        Stream.pipeThroughChannel(Sse.decodeDataSchema(schema))
      )
  const decodeSuccess =
    <Schema extends Schema.Constraint>(schema: Schema) =>
    (response: HttpClientResponse.HttpClientResponse) =>
      HttpClientResponse.schemaBodyJson(schema)(response)
  const decodeError =
    <const Tag extends string, Schema extends Schema.Constraint>(tag: Tag, schema: Schema) =>
    (response: HttpClientResponse.HttpClientResponse) =>
      Effect.flatMap(
        HttpClientResponse.schemaBodyJson(schema)(response),
        (cause) => Effect.fail(SlipstreamError(tag, cause, response)),
      )
  return {
    httpClient,
    "listCaptureMethods": (options) => HttpClientRequest.get(`/api/v1/capture/methods`).pipe(
    withResponse(options?.config)(HttpClientResponse.matchStatus({
      "2xx": decodeSuccess(ListCaptureMethods200),
      "401": decodeError("ListCaptureMethods401", ListCaptureMethods401),
      orElse: unexpectedStatus
    }))
  ),
    "listPairedClients": (options) => HttpClientRequest.get(`/api/v1/clients`).pipe(
    withResponse(options?.config)(HttpClientResponse.matchStatus({
      "2xx": decodeSuccess(ListPairedClients200),
      "401": decodeError("ListPairedClients401", ListPairedClients401),
      orElse: unexpectedStatus
    }))
  ),
    "unpairClient": (fingerprint, options) => HttpClientRequest.delete(`/api/v1/clients/${fingerprint}`).pipe(
    withResponse(options?.config)(HttpClientResponse.matchStatus({
      "400": decodeError("UnpairClient400", UnpairClient400),
      "401": decodeError("UnpairClient401", UnpairClient401),
      "404": decodeError("UnpairClient404", UnpairClient404),
      "204": () => Effect.void,
      orElse: unexpectedStatus
    }))
  ),
    "listCompositors": (options) => HttpClientRequest.get(`/api/v1/compositors`).pipe(
    withResponse(options?.config)(HttpClientResponse.matchStatus({
      "2xx": decodeSuccess(ListCompositors200),
      "401": decodeError("ListCompositors401", ListCompositors401),
      orElse: unexpectedStatus
    }))
  ),
    "listHeadlessCompositors": (options) => HttpClientRequest.get(`/api/v1/compositors/headless`).pipe(
    withResponse(options?.config)(HttpClientResponse.matchStatus({
      "2xx": decodeSuccess(ListHeadlessCompositors200),
      "401": decodeError("ListHeadlessCompositors401", ListHeadlessCompositors401),
      orElse: unexpectedStatus
    }))
  ),
    "getDiagnosticsPreflight": (options) => HttpClientRequest.get(`/api/v1/diagnostics/preflight`).pipe(
    withResponse(options?.config)(HttpClientResponse.matchStatus({
      "2xx": decodeSuccess(GetDiagnosticsPreflight200),
      "401": decodeError("GetDiagnosticsPreflight401", GetDiagnosticsPreflight401),
      orElse: unexpectedStatus
    }))
  ),
    "setDisplayLayout": (options) => HttpClientRequest.put(`/api/v1/display/layout`).pipe(
    HttpClientRequest.bodyJsonUnsafe(options.payload),
    withResponse(options.config)(HttpClientResponse.matchStatus({
      "2xx": decodeSuccess(SetDisplayLayout200),
      "401": decodeError("SetDisplayLayout401", SetDisplayLayout401),
      "500": decodeError("SetDisplayLayout500", SetDisplayLayout500),
      orElse: unexpectedStatus
    }))
  ),
    "getDisplayMonitors": (options) => HttpClientRequest.get(`/api/v1/display/monitors`).pipe(
    withResponse(options?.config)(HttpClientResponse.matchStatus({
      "2xx": decodeSuccess(GetDisplayMonitors200),
      "401": decodeError("GetDisplayMonitors401", GetDisplayMonitors401),
      orElse: unexpectedStatus
    }))
  ),
    "listCustomPresets": (options) => HttpClientRequest.get(`/api/v1/display/presets`).pipe(
    withResponse(options?.config)(HttpClientResponse.matchStatus({
      "2xx": decodeSuccess(ListCustomPresets200),
      "401": decodeError("ListCustomPresets401", ListCustomPresets401),
      orElse: unexpectedStatus
    }))
  ),
    "createCustomPreset": (options) => HttpClientRequest.post(`/api/v1/display/presets`).pipe(
    HttpClientRequest.bodyJsonUnsafe(options.payload),
    withResponse(options.config)(HttpClientResponse.matchStatus({
      "2xx": decodeSuccess(CreateCustomPreset201),
      "400": decodeError("CreateCustomPreset400", CreateCustomPreset400),
      "401": decodeError("CreateCustomPreset401", CreateCustomPreset401),
      "500": decodeError("CreateCustomPreset500", CreateCustomPreset500),
      orElse: unexpectedStatus
    }))
  ),
    "updateCustomPreset": (id, options) => HttpClientRequest.put(`/api/v1/display/presets/${id}`).pipe(
    HttpClientRequest.bodyJsonUnsafe(options.payload),
    withResponse(options.config)(HttpClientResponse.matchStatus({
      "2xx": decodeSuccess(UpdateCustomPreset200),
      "400": decodeError("UpdateCustomPreset400", UpdateCustomPreset400),
      "401": decodeError("UpdateCustomPreset401", UpdateCustomPreset401),
      "404": decodeError("UpdateCustomPreset404", UpdateCustomPreset404),
      "500": decodeError("UpdateCustomPreset500", UpdateCustomPreset500),
      orElse: unexpectedStatus
    }))
  ),
    "deleteCustomPreset": (id, options) => HttpClientRequest.delete(`/api/v1/display/presets/${id}`).pipe(
    withResponse(options?.config)(HttpClientResponse.matchStatus({
      "401": decodeError("DeleteCustomPreset401", DeleteCustomPreset401),
      "404": decodeError("DeleteCustomPreset404", DeleteCustomPreset404),
      "500": decodeError("DeleteCustomPreset500", DeleteCustomPreset500),
      "204": () => Effect.void,
      orElse: unexpectedStatus
    }))
  ),
    "releaseDisplay": (options) => HttpClientRequest.post(`/api/v1/display/release`).pipe(
    HttpClientRequest.bodyJsonUnsafe(options.payload),
    withResponse(options.config)(HttpClientResponse.matchStatus({
      "2xx": decodeSuccess(ReleaseDisplay200),
      "401": decodeError("ReleaseDisplay401", ReleaseDisplay401),
      orElse: unexpectedStatus
    }))
  ),
    "getDisplaySettings": (options) => HttpClientRequest.get(`/api/v1/display/settings`).pipe(
    withResponse(options?.config)(HttpClientResponse.matchStatus({
      "2xx": decodeSuccess(GetDisplaySettings200),
      "401": decodeError("GetDisplaySettings401", GetDisplaySettings401),
      orElse: unexpectedStatus
    }))
  ),
    "setDisplaySettings": (options) => HttpClientRequest.put(`/api/v1/display/settings`).pipe(
    HttpClientRequest.bodyJsonUnsafe(options.payload),
    withResponse(options.config)(HttpClientResponse.matchStatus({
      "2xx": decodeSuccess(SetDisplaySettings200),
      "400": decodeError("SetDisplaySettings400", SetDisplaySettings400),
      "401": decodeError("SetDisplaySettings401", SetDisplaySettings401),
      "500": decodeError("SetDisplaySettings500", SetDisplaySettings500),
      orElse: unexpectedStatus
    }))
  ),
    "getDisplayState": (options) => HttpClientRequest.get(`/api/v1/display/state`).pipe(
    withResponse(options?.config)(HttpClientResponse.matchStatus({
      "2xx": decodeSuccess(GetDisplayState200),
      "401": decodeError("GetDisplayState401", GetDisplayState401),
      orElse: unexpectedStatus
    }))
  ),
    "streamEvents": (options) => HttpClientRequest.get(`/api/v1/events`).pipe(
    HttpClientRequest.setUrlParams({ "since": options?.params?.["since"] as any, "kinds": options?.params?.["kinds"] as any }),
    HttpClientRequest.setHeaders({ "Last-Event-ID": options?.params?.["Last-Event-ID"] ?? undefined }),
    withResponse(options?.config)(HttpClientResponse.matchStatus({
      "401": decodeError("StreamEvents401", StreamEvents401),
      "503": decodeError("StreamEvents503", StreamEvents503),
      orElse: unexpectedStatus
    }))
  ),
    "streamEventsSse": (options) => HttpClientRequest.get(`/api/v1/events`).pipe(
      HttpClientRequest.setUrlParams({ "since": options?.params?.["since"] as any, "kinds": options?.params?.["kinds"] as any }),
      HttpClientRequest.setHeaders({ "Last-Event-ID": options?.params?.["Last-Event-ID"] ?? undefined }),
      sseRequest(StreamEvents200Sse)
    ),
    "endGame": (options) => HttpClientRequest.post(`/api/v1/game/end`).pipe(
    HttpClientRequest.bodyJsonUnsafe(options.payload),
    withResponse(options.config)(HttpClientResponse.matchStatus({
      "2xx": decodeSuccess(EndGame200),
      "401": decodeError("EndGame401", EndGame401),
      "409": decodeError("EndGame409", EndGame409),
      orElse: unexpectedStatus
    }))
  ),
    "listGpus": (options) => HttpClientRequest.get(`/api/v1/gpus`).pipe(
    withResponse(options?.config)(HttpClientResponse.matchStatus({
      "2xx": decodeSuccess(ListGpus200),
      "401": decodeError("ListGpus401", ListGpus401),
      orElse: unexpectedStatus
    }))
  ),
    "setGpuPreference": (options) => HttpClientRequest.put(`/api/v1/gpus/preference`).pipe(
    HttpClientRequest.bodyJsonUnsafe(options.payload),
    withResponse(options.config)(HttpClientResponse.matchStatus({
      "2xx": decodeSuccess(SetGpuPreference200),
      "400": decodeError("SetGpuPreference400", SetGpuPreference400),
      "401": decodeError("SetGpuPreference401", SetGpuPreference401),
      "500": decodeError("SetGpuPreference500", SetGpuPreference500),
      orElse: unexpectedStatus
    }))
  ),
    "getHealth": (options) => HttpClientRequest.get(`/api/v1/health`).pipe(
    withResponse(options?.config)(HttpClientResponse.matchStatus({
      "2xx": decodeSuccess(GetHealth200),
      orElse: unexpectedStatus
    }))
  ),
    "getHooks": (options) => HttpClientRequest.get(`/api/v1/hooks`).pipe(
    withResponse(options?.config)(HttpClientResponse.matchStatus({
      "2xx": decodeSuccess(GetHooks200),
      "401": decodeError("GetHooks401", GetHooks401),
      orElse: unexpectedStatus
    }))
  ),
    "setHooks": (options) => HttpClientRequest.put(`/api/v1/hooks`).pipe(
    HttpClientRequest.bodyJsonUnsafe(options.payload),
    withResponse(options.config)(HttpClientResponse.matchStatus({
      "2xx": decodeSuccess(SetHooks200),
      "400": decodeError("SetHooks400", SetHooks400),
      "401": decodeError("SetHooks401", SetHooks401),
      "500": decodeError("SetHooks500", SetHooks500),
      orElse: unexpectedStatus
    }))
  ),
    "getHostInfo": (options) => HttpClientRequest.get(`/api/v1/host`).pipe(
    withResponse(options?.config)(HttpClientResponse.matchStatus({
      "2xx": decodeSuccess(GetHostInfo200),
      "401": decodeError("GetHostInfo401", GetHostInfo401),
      orElse: unexpectedStatus
    }))
  ),
    "getHostConfig": (options) => HttpClientRequest.get(`/api/v1/host/config`).pipe(
    withResponse(options?.config)(HttpClientResponse.matchStatus({
      "2xx": decodeSuccess(GetHostConfig200),
      "401": decodeError("GetHostConfig401", GetHostConfig401),
      orElse: unexpectedStatus
    }))
  ),
    "setHostConfig": (options) => HttpClientRequest.put(`/api/v1/host/config`).pipe(
    HttpClientRequest.bodyJsonUnsafe(options.payload),
    withResponse(options.config)(HttpClientResponse.matchStatus({
      "2xx": decodeSuccess(SetHostConfig200),
      "400": decodeError("SetHostConfig400", SetHostConfig400),
      "401": decodeError("SetHostConfig401", SetHostConfig401),
      "500": decodeError("SetHostConfig500", SetHostConfig500),
      orElse: unexpectedStatus
    }))
  ),
    "setMoonlightBroadcast": (options) => HttpClientRequest.put(`/api/v1/host/moonlight`).pipe(
    HttpClientRequest.bodyJsonUnsafe(options.payload),
    withResponse(options.config)(HttpClientResponse.matchStatus({
      "2xx": decodeSuccess(SetMoonlightBroadcast200),
      "401": decodeError("SetMoonlightBroadcast401", SetMoonlightBroadcast401),
      "500": decodeError("SetMoonlightBroadcast500", SetMoonlightBroadcast500),
      orElse: unexpectedStatus
    }))
  ),
    "restartHost": (options) => HttpClientRequest.post(`/api/v1/host/restart`).pipe(
    withResponse(options?.config)(HttpClientResponse.matchStatus({
      "401": decodeError("RestartHost401", RestartHost401),
      "500": decodeError("RestartHost500", RestartHost500),
      "202": () => Effect.void,
      orElse: unexpectedStatus
    }))
  ),
    "shutdownHost": (options) => HttpClientRequest.post(`/api/v1/host/shutdown`).pipe(
    withResponse(options?.config)(HttpClientResponse.matchStatus({
      "401": decodeError("ShutdownHost401", ShutdownHost401),
      "202": () => Effect.void,
      orElse: unexpectedStatus
    }))
  ),
    "getLibrary": (options) => HttpClientRequest.get(`/api/v1/library`).pipe(
    HttpClientRequest.setUrlParams({ "provider": options?.params?.["provider"] as any, "platform": options?.params?.["platform"] as any }),
    withResponse(options?.config)(HttpClientResponse.matchStatus({
      "2xx": decodeSuccess(GetLibrary200),
      "401": decodeError("GetLibrary401", GetLibrary401),
      orElse: unexpectedStatus
    }))
  ),
    "getLibraryArt": (id, kind, options) => HttpClientRequest.get(`/api/v1/library/art/${id}/${kind}`).pipe(
    withResponse(options?.config)(HttpClientResponse.matchStatus({
      "401": decodeError("GetLibraryArt401", GetLibraryArt401),
      "404": decodeError("GetLibraryArt404", GetLibraryArt404),
      orElse: unexpectedStatus
    }))
  ),
    "createCustomGame": (options) => HttpClientRequest.post(`/api/v1/library/custom`).pipe(
    HttpClientRequest.bodyJsonUnsafe(options.payload),
    withResponse(options.config)(HttpClientResponse.matchStatus({
      "2xx": decodeSuccess(CreateCustomGame201),
      "400": decodeError("CreateCustomGame400", CreateCustomGame400),
      "401": decodeError("CreateCustomGame401", CreateCustomGame401),
      "500": decodeError("CreateCustomGame500", CreateCustomGame500),
      orElse: unexpectedStatus
    }))
  ),
    "updateCustomGame": (id, options) => HttpClientRequest.put(`/api/v1/library/custom/${id}`).pipe(
    HttpClientRequest.bodyJsonUnsafe(options.payload),
    withResponse(options.config)(HttpClientResponse.matchStatus({
      "2xx": decodeSuccess(UpdateCustomGame200),
      "400": decodeError("UpdateCustomGame400", UpdateCustomGame400),
      "401": decodeError("UpdateCustomGame401", UpdateCustomGame401),
      "404": decodeError("UpdateCustomGame404", UpdateCustomGame404),
      "500": decodeError("UpdateCustomGame500", UpdateCustomGame500),
      orElse: unexpectedStatus
    }))
  ),
    "deleteCustomGame": (id, options) => HttpClientRequest.delete(`/api/v1/library/custom/${id}`).pipe(
    withResponse(options?.config)(HttpClientResponse.matchStatus({
      "401": decodeError("DeleteCustomGame401", DeleteCustomGame401),
      "404": decodeError("DeleteCustomGame404", DeleteCustomGame404),
      "500": decodeError("DeleteCustomGame500", DeleteCustomGame500),
      "204": () => Effect.void,
      orElse: unexpectedStatus
    }))
  ),
    "reconcileProviderEntries": (provider, options) => HttpClientRequest.put(`/api/v1/library/provider/${provider}`).pipe(
    HttpClientRequest.bodyJsonUnsafe(options.payload),
    withResponse(options.config)(HttpClientResponse.matchStatus({
      "2xx": decodeSuccess(ReconcileProviderEntries200),
      "400": decodeError("ReconcileProviderEntries400", ReconcileProviderEntries400),
      "401": decodeError("ReconcileProviderEntries401", ReconcileProviderEntries401),
      "500": decodeError("ReconcileProviderEntries500", ReconcileProviderEntries500),
      orElse: unexpectedStatus
    }))
  ),
    "deleteProviderEntries": (provider, options) => HttpClientRequest.delete(`/api/v1/library/provider/${provider}`).pipe(
    withResponse(options?.config)(HttpClientResponse.matchStatus({
      "2xx": decodeSuccess(DeleteProviderEntries200),
      "400": decodeError("DeleteProviderEntries400", DeleteProviderEntries400),
      "401": decodeError("DeleteProviderEntries401", DeleteProviderEntries401),
      "500": decodeError("DeleteProviderEntries500", DeleteProviderEntries500),
      orElse: unexpectedStatus
    }))
  ),
    "listLibraryScanners": (options) => HttpClientRequest.get(`/api/v1/library/scanners`).pipe(
    withResponse(options?.config)(HttpClientResponse.matchStatus({
      "2xx": decodeSuccess(ListLibraryScanners200),
      "401": decodeError("ListLibraryScanners401", ListLibraryScanners401),
      orElse: unexpectedStatus
    }))
  ),
    "setLibraryScanner": (id, options) => HttpClientRequest.put(`/api/v1/library/scanners/${id}`).pipe(
    HttpClientRequest.bodyJsonUnsafe(options.payload),
    withResponse(options.config)(HttpClientResponse.matchStatus({
      "2xx": decodeSuccess(SetLibraryScanner200),
      "401": decodeError("SetLibraryScanner401", SetLibraryScanner401),
      "404": decodeError("SetLibraryScanner404", SetLibraryScanner404),
      "500": decodeError("SetLibraryScanner500", SetLibraryScanner500),
      orElse: unexpectedStatus
    }))
  ),
    "getLocalSummary": (options) => HttpClientRequest.get(`/api/v1/local/summary`).pipe(
    withResponse(options?.config)(HttpClientResponse.matchStatus({
      "2xx": decodeSuccess(GetLocalSummary200),
      "401": decodeError("GetLocalSummary401", GetLocalSummary401),
      orElse: unexpectedStatus
    }))
  ),
    "logsGet": (options) => HttpClientRequest.get(`/api/v1/logs`).pipe(
    HttpClientRequest.setUrlParams({ "after": options?.params?.["after"] as any, "limit": options?.params?.["limit"] as any }),
    withResponse(options?.config)(HttpClientResponse.matchStatus({
      "2xx": decodeSuccess(LogsGet200),
      "401": decodeError("LogsGet401", LogsGet401),
      orElse: unexpectedStatus
    }))
  ),
    "listNativeClients": (options) => HttpClientRequest.get(`/api/v1/native/clients`).pipe(
    withResponse(options?.config)(HttpClientResponse.matchStatus({
      "2xx": decodeSuccess(ListNativeClients200),
      "401": decodeError("ListNativeClients401", ListNativeClients401),
      orElse: unexpectedStatus
    }))
  ),
    "unpairNativeClient": (fingerprint, options) => HttpClientRequest.delete(`/api/v1/native/clients/${fingerprint}`).pipe(
    withResponse(options?.config)(HttpClientResponse.matchStatus({
      "401": decodeError("UnpairNativeClient401", UnpairNativeClient401),
      "404": decodeError("UnpairNativeClient404", UnpairNativeClient404),
      "503": decodeError("UnpairNativeClient503", UnpairNativeClient503),
      "204": () => Effect.void,
      orElse: unexpectedStatus
    }))
  ),
    "getNativePairing": (options) => HttpClientRequest.get(`/api/v1/native/pair`).pipe(
    withResponse(options?.config)(HttpClientResponse.matchStatus({
      "2xx": decodeSuccess(GetNativePairing200),
      "401": decodeError("GetNativePairing401", GetNativePairing401),
      orElse: unexpectedStatus
    }))
  ),
    "disarmNativePairing": (options) => HttpClientRequest.delete(`/api/v1/native/pair`).pipe(
    withResponse(options?.config)(HttpClientResponse.matchStatus({
      "401": decodeError("DisarmNativePairing401", DisarmNativePairing401),
      "503": decodeError("DisarmNativePairing503", DisarmNativePairing503),
      "204": () => Effect.void,
      orElse: unexpectedStatus
    }))
  ),
    "armNativePairing": (options) => HttpClientRequest.post(`/api/v1/native/pair/arm`).pipe(
    HttpClientRequest.bodyJsonUnsafe(options.payload),
    withResponse(options.config)(HttpClientResponse.matchStatus({
      "2xx": decodeSuccess(ArmNativePairing200),
      "401": decodeError("ArmNativePairing401", ArmNativePairing401),
      "503": decodeError("ArmNativePairing503", ArmNativePairing503),
      orElse: unexpectedStatus
    }))
  ),
    "listPendingDevices": (options) => HttpClientRequest.get(`/api/v1/native/pending`).pipe(
    withResponse(options?.config)(HttpClientResponse.matchStatus({
      "2xx": decodeSuccess(ListPendingDevices200),
      "401": decodeError("ListPendingDevices401", ListPendingDevices401),
      orElse: unexpectedStatus
    }))
  ),
    "approvePendingDevice": (id, options) => HttpClientRequest.post(`/api/v1/native/pending/${id}/approve`).pipe(
    HttpClientRequest.bodyJsonUnsafe(options.payload),
    withResponse(options.config)(HttpClientResponse.matchStatus({
      "2xx": decodeSuccess(ApprovePendingDevice200),
      "401": decodeError("ApprovePendingDevice401", ApprovePendingDevice401),
      "404": decodeError("ApprovePendingDevice404", ApprovePendingDevice404),
      "500": decodeError("ApprovePendingDevice500", ApprovePendingDevice500),
      "503": decodeError("ApprovePendingDevice503", ApprovePendingDevice503),
      orElse: unexpectedStatus
    }))
  ),
    "denyPendingDevice": (id, options) => HttpClientRequest.post(`/api/v1/native/pending/${id}/deny`).pipe(
    withResponse(options?.config)(HttpClientResponse.matchStatus({
      "401": decodeError("DenyPendingDevice401", DenyPendingDevice401),
      "404": decodeError("DenyPendingDevice404", DenyPendingDevice404),
      "503": decodeError("DenyPendingDevice503", DenyPendingDevice503),
      "204": () => Effect.void,
      orElse: unexpectedStatus
    }))
  ),
    "getPairingStatus": (options) => HttpClientRequest.get(`/api/v1/pair`).pipe(
    withResponse(options?.config)(HttpClientResponse.matchStatus({
      "2xx": decodeSuccess(GetPairingStatus200),
      "401": decodeError("GetPairingStatus401", GetPairingStatus401),
      orElse: unexpectedStatus
    }))
  ),
    "submitPairingPin": (options) => HttpClientRequest.post(`/api/v1/pair/pin`).pipe(
    HttpClientRequest.bodyJsonUnsafe(options.payload),
    withResponse(options.config)(HttpClientResponse.matchStatus({
      "400": decodeError("SubmitPairingPin400", SubmitPairingPin400),
      "401": decodeError("SubmitPairingPin401", SubmitPairingPin401),
      "409": decodeError("SubmitPairingPin409", SubmitPairingPin409),
      "415": decodeError("SubmitPairingPin415", SubmitPairingPin415),
      "422": decodeError("SubmitPairingPin422", SubmitPairingPin422),
      "204": () => Effect.void,
      orElse: unexpectedStatus
    }))
  ),
    "listPlugins": (options) => HttpClientRequest.get(`/api/v1/plugins`).pipe(
    withResponse(options?.config)(HttpClientResponse.matchStatus({
      "2xx": decodeSuccess(ListPlugins200),
      "401": decodeError("ListPlugins401", ListPlugins401),
      orElse: unexpectedStatus
    }))
  ),
    "registerPlugin": (id, options) => HttpClientRequest.put(`/api/v1/plugins/${id}`).pipe(
    HttpClientRequest.bodyJsonUnsafe(options.payload),
    withResponse(options.config)(HttpClientResponse.matchStatus({
      "400": decodeError("RegisterPlugin400", RegisterPlugin400),
      "401": decodeError("RegisterPlugin401", RegisterPlugin401),
      "204": () => Effect.void,
      orElse: unexpectedStatus
    }))
  ),
    "deregisterPlugin": (id, options) => HttpClientRequest.delete(`/api/v1/plugins/${id}`).pipe(
    withResponse(options?.config)(HttpClientResponse.matchStatus({
      "401": decodeError("DeregisterPlugin401", DeregisterPlugin401),
      "204": () => Effect.void,
      orElse: unexpectedStatus
    }))
  ),
    "getPluginUiCredential": (id, options) => HttpClientRequest.get(`/api/v1/plugins/${id}/ui-credential`).pipe(
    withResponse(options?.config)(HttpClientResponse.matchStatus({
      "2xx": decodeSuccess(GetPluginUiCredential200),
      "401": decodeError("GetPluginUiCredential401", GetPluginUiCredential401),
      "404": decodeError("GetPluginUiCredential404", GetPluginUiCredential404),
      orElse: unexpectedStatus
    }))
  ),
    "stopSession": (options) => HttpClientRequest.delete(`/api/v1/session`).pipe(
    withResponse(options?.config)(HttpClientResponse.matchStatus({
      "401": decodeError("StopSession401", StopSession401),
      "204": () => Effect.void,
      orElse: unexpectedStatus
    }))
  ),
    "requestIdr": (options) => HttpClientRequest.post(`/api/v1/session/idr`).pipe(
    withResponse(options?.config)(HttpClientResponse.matchStatus({
      "401": decodeError("RequestIdr401", RequestIdr401),
      "409": decodeError("RequestIdr409", RequestIdr409),
      "202": () => Effect.void,
      orElse: unexpectedStatus
    }))
  ),
    "getSessionSettings": (options) => HttpClientRequest.get(`/api/v1/session/settings`).pipe(
    withResponse(options?.config)(HttpClientResponse.matchStatus({
      "2xx": decodeSuccess(GetSessionSettings200),
      "401": decodeError("GetSessionSettings401", GetSessionSettings401),
      orElse: unexpectedStatus
    }))
  ),
    "setSessionSettings": (options) => HttpClientRequest.put(`/api/v1/session/settings`).pipe(
    HttpClientRequest.bodyJsonUnsafe(options.payload),
    withResponse(options.config)(HttpClientResponse.matchStatus({
      "2xx": decodeSuccess(SetSessionSettings200),
      "400": decodeError("SetSessionSettings400", SetSessionSettings400),
      "401": decodeError("SetSessionSettings401", SetSessionSettings401),
      "500": decodeError("SetSessionSettings500", SetSessionSettings500),
      orElse: unexpectedStatus
    }))
  ),
    "statsCaptureLive": (options) => HttpClientRequest.get(`/api/v1/stats/capture/live`).pipe(
    withResponse(options?.config)(HttpClientResponse.matchStatus({
      "2xx": decodeSuccess(StatsCaptureLive200),
      "401": decodeError("StatsCaptureLive401", StatsCaptureLive401),
      "404": decodeError("StatsCaptureLive404", StatsCaptureLive404),
      orElse: unexpectedStatus
    }))
  ),
    "statsCaptureStart": (options) => HttpClientRequest.post(`/api/v1/stats/capture/start`).pipe(
    withResponse(options?.config)(HttpClientResponse.matchStatus({
      "2xx": decodeSuccess(StatsCaptureStart200),
      "401": decodeError("StatsCaptureStart401", StatsCaptureStart401),
      orElse: unexpectedStatus
    }))
  ),
    "statsCaptureStatus": (options) => HttpClientRequest.get(`/api/v1/stats/capture/status`).pipe(
    withResponse(options?.config)(HttpClientResponse.matchStatus({
      "2xx": decodeSuccess(StatsCaptureStatus200),
      "401": decodeError("StatsCaptureStatus401", StatsCaptureStatus401),
      orElse: unexpectedStatus
    }))
  ),
    "statsCaptureStop": (options) => HttpClientRequest.post(`/api/v1/stats/capture/stop`).pipe(
    withResponse(options?.config)(HttpClientResponse.matchStatus({
      "2xx": decodeSuccess(StatsCaptureStop200),
      "401": decodeError("StatsCaptureStop401", StatsCaptureStop401),
      "500": decodeError("StatsCaptureStop500", StatsCaptureStop500),
      "204": () => Effect.void,
      orElse: unexpectedStatus
    }))
  ),
    "statsRecordingsList": (options) => HttpClientRequest.get(`/api/v1/stats/recordings`).pipe(
    withResponse(options?.config)(HttpClientResponse.matchStatus({
      "2xx": decodeSuccess(StatsRecordingsList200),
      "401": decodeError("StatsRecordingsList401", StatsRecordingsList401),
      orElse: unexpectedStatus
    }))
  ),
    "statsRecordingGet": (id, options) => HttpClientRequest.get(`/api/v1/stats/recordings/${id}`).pipe(
    withResponse(options?.config)(HttpClientResponse.matchStatus({
      "2xx": decodeSuccess(StatsRecordingGet200),
      "401": decodeError("StatsRecordingGet401", StatsRecordingGet401),
      "404": decodeError("StatsRecordingGet404", StatsRecordingGet404),
      "500": decodeError("StatsRecordingGet500", StatsRecordingGet500),
      orElse: unexpectedStatus
    }))
  ),
    "statsRecordingDelete": (id, options) => HttpClientRequest.delete(`/api/v1/stats/recordings/${id}`).pipe(
    withResponse(options?.config)(HttpClientResponse.matchStatus({
      "401": decodeError("StatsRecordingDelete401", StatsRecordingDelete401),
      "404": decodeError("StatsRecordingDelete404", StatsRecordingDelete404),
      "500": decodeError("StatsRecordingDelete500", StatsRecordingDelete500),
      "204": () => Effect.void,
      orElse: unexpectedStatus
    }))
  ),
    "getStatus": (options) => HttpClientRequest.get(`/api/v1/status`).pipe(
    withResponse(options?.config)(HttpClientResponse.matchStatus({
      "2xx": decodeSuccess(GetStatus200),
      "401": decodeError("GetStatus401", GetStatus401),
      orElse: unexpectedStatus
    }))
  ),
    "getPluginCatalog": (options) => HttpClientRequest.get(`/api/v1/store/catalog`).pipe(
    withResponse(options?.config)(HttpClientResponse.matchStatus({
      "2xx": decodeSuccess(GetPluginCatalog200),
      "401": decodeError("GetPluginCatalog401", GetPluginCatalog401),
      "403": decodeError("GetPluginCatalog403", GetPluginCatalog403),
      orElse: unexpectedStatus
    }))
  ),
    "installPlugin": (options) => HttpClientRequest.post(`/api/v1/store/install`).pipe(
    HttpClientRequest.bodyJsonUnsafe(options.payload),
    withResponse(options.config)(HttpClientResponse.matchStatus({
      "2xx": decodeSuccess(InstallPlugin202),
      "400": decodeError("InstallPlugin400", InstallPlugin400),
      "401": decodeError("InstallPlugin401", InstallPlugin401),
      "403": decodeError("InstallPlugin403", InstallPlugin403),
      "409": decodeError("InstallPlugin409", InstallPlugin409),
      orElse: unexpectedStatus
    }))
  ),
    "listInstalledPlugins": (options) => HttpClientRequest.get(`/api/v1/store/installed`).pipe(
    withResponse(options?.config)(HttpClientResponse.matchStatus({
      "2xx": decodeSuccess(ListInstalledPlugins200),
      "401": decodeError("ListInstalledPlugins401", ListInstalledPlugins401),
      "403": decodeError("ListInstalledPlugins403", ListInstalledPlugins403),
      orElse: unexpectedStatus
    }))
  ),
    "listPluginJobs": (options) => HttpClientRequest.get(`/api/v1/store/jobs`).pipe(
    withResponse(options?.config)(HttpClientResponse.matchStatus({
      "2xx": decodeSuccess(ListPluginJobs200),
      "401": decodeError("ListPluginJobs401", ListPluginJobs401),
      "403": decodeError("ListPluginJobs403", ListPluginJobs403),
      orElse: unexpectedStatus
    }))
  ),
    "getPluginJob": (id, options) => HttpClientRequest.get(`/api/v1/store/jobs/${id}`).pipe(
    withResponse(options?.config)(HttpClientResponse.matchStatus({
      "2xx": decodeSuccess(GetPluginJob200),
      "401": decodeError("GetPluginJob401", GetPluginJob401),
      "403": decodeError("GetPluginJob403", GetPluginJob403),
      "404": decodeError("GetPluginJob404", GetPluginJob404),
      orElse: unexpectedStatus
    }))
  ),
    "refreshPluginCatalog": (options) => HttpClientRequest.post(`/api/v1/store/refresh`).pipe(
    withResponse(options?.config)(HttpClientResponse.matchStatus({
      "2xx": decodeSuccess(RefreshPluginCatalog200),
      "401": decodeError("RefreshPluginCatalog401", RefreshPluginCatalog401),
      "403": decodeError("RefreshPluginCatalog403", RefreshPluginCatalog403),
      orElse: unexpectedStatus
    }))
  ),
    "getPluginRuntime": (options) => HttpClientRequest.get(`/api/v1/store/runtime`).pipe(
    withResponse(options?.config)(HttpClientResponse.matchStatus({
      "2xx": decodeSuccess(GetPluginRuntime200),
      "401": decodeError("GetPluginRuntime401", GetPluginRuntime401),
      "403": decodeError("GetPluginRuntime403", GetPluginRuntime403),
      orElse: unexpectedStatus
    }))
  ),
    "setPluginRuntime": (options) => HttpClientRequest.post(`/api/v1/store/runtime`).pipe(
    HttpClientRequest.bodyJsonUnsafe(options.payload),
    withResponse(options.config)(HttpClientResponse.matchStatus({
      "2xx": decodeSuccess(SetPluginRuntime200),
      "400": decodeError("SetPluginRuntime400", SetPluginRuntime400),
      "401": decodeError("SetPluginRuntime401", SetPluginRuntime401),
      "403": decodeError("SetPluginRuntime403", SetPluginRuntime403),
      orElse: unexpectedStatus
    }))
  ),
    "listPluginSources": (options) => HttpClientRequest.get(`/api/v1/store/sources`).pipe(
    withResponse(options?.config)(HttpClientResponse.matchStatus({
      "2xx": decodeSuccess(ListPluginSources200),
      "401": decodeError("ListPluginSources401", ListPluginSources401),
      "403": decodeError("ListPluginSources403", ListPluginSources403),
      orElse: unexpectedStatus
    }))
  ),
    "putPluginSource": (name, options) => HttpClientRequest.put(`/api/v1/store/sources/${name}`).pipe(
    HttpClientRequest.bodyJsonUnsafe(options.payload),
    withResponse(options.config)(HttpClientResponse.matchStatus({
      "400": decodeError("PutPluginSource400", PutPluginSource400),
      "401": decodeError("PutPluginSource401", PutPluginSource401),
      "403": decodeError("PutPluginSource403", PutPluginSource403),
      "204": () => Effect.void,
      orElse: unexpectedStatus
    }))
  ),
    "deletePluginSource": (name, options) => HttpClientRequest.delete(`/api/v1/store/sources/${name}`).pipe(
    withResponse(options?.config)(HttpClientResponse.matchStatus({
      "401": decodeError("DeletePluginSource401", DeletePluginSource401),
      "403": decodeError("DeletePluginSource403", DeletePluginSource403),
      "204": () => Effect.void,
      orElse: unexpectedStatus
    }))
  ),
    "uninstallPlugin": (options) => HttpClientRequest.post(`/api/v1/store/uninstall`).pipe(
    HttpClientRequest.bodyJsonUnsafe(options.payload),
    withResponse(options.config)(HttpClientResponse.matchStatus({
      "2xx": decodeSuccess(UninstallPlugin202),
      "400": decodeError("UninstallPlugin400", UninstallPlugin400),
      "401": decodeError("UninstallPlugin401", UninstallPlugin401),
      "403": decodeError("UninstallPlugin403", UninstallPlugin403),
      "409": decodeError("UninstallPlugin409", UninstallPlugin409),
      orElse: unexpectedStatus
    }))
  ),
    "supportBundleCreate": (options) => HttpClientRequest.post(`/api/v1/support-bundles`).pipe(
    withResponse(options?.config)(HttpClientResponse.matchStatus({
      "2xx": decodeSuccess(SupportBundleCreate200),
      "401": decodeError("SupportBundleCreate401", SupportBundleCreate401),
      "500": decodeError("SupportBundleCreate500", SupportBundleCreate500),
      orElse: unexpectedStatus
    }))
  ),
    "supportBundleGet": (id, options) => HttpClientRequest.get(`/api/v1/support-bundles/${id}`).pipe(
    withResponse(options?.config)(HttpClientResponse.matchStatus({
      "2xx": decodeSuccess(SupportBundleGet200),
      "401": decodeError("SupportBundleGet401", SupportBundleGet401),
      "404": decodeError("SupportBundleGet404", SupportBundleGet404),
      "500": decodeError("SupportBundleGet500", SupportBundleGet500),
      orElse: unexpectedStatus
    }))
  ),
    "supportBundleDelete": (id, options) => HttpClientRequest.delete(`/api/v1/support-bundles/${id}`).pipe(
    withResponse(options?.config)(HttpClientResponse.matchStatus({
      "401": decodeError("SupportBundleDelete401", SupportBundleDelete401),
      "404": decodeError("SupportBundleDelete404", SupportBundleDelete404),
      "204": () => Effect.void,
      orElse: unexpectedStatus
    }))
  ),
    "applyUpdate": (options) => HttpClientRequest.post(`/api/v1/update/apply`).pipe(
    HttpClientRequest.bodyJsonUnsafe(options.payload),
    withResponse(options.config)(HttpClientResponse.matchStatus({
      "2xx": decodeSuccess(ApplyUpdate202),
      "401": decodeError("ApplyUpdate401", ApplyUpdate401),
      "409": decodeError("ApplyUpdate409", ApplyUpdate409),
      orElse: unexpectedStatus
    }))
  ),
    "forceUpdateCheck": (options) => HttpClientRequest.post(`/api/v1/update/check`).pipe(
    withResponse(options?.config)(HttpClientResponse.matchStatus({
      "2xx": decodeSuccess(ForceUpdateCheck200),
      "401": decodeError("ForceUpdateCheck401", ForceUpdateCheck401),
      "409": decodeError("ForceUpdateCheck409", ForceUpdateCheck409),
      "429": decodeError("ForceUpdateCheck429", ForceUpdateCheck429),
      orElse: unexpectedStatus
    }))
  ),
    "getUpdateStatus": (options) => HttpClientRequest.get(`/api/v1/update/status`).pipe(
    withResponse(options?.config)(HttpClientResponse.matchStatus({
      "2xx": decodeSuccess(GetUpdateStatus200),
      "401": decodeError("GetUpdateStatus401", GetUpdateStatus401),
      orElse: unexpectedStatus
    }))
  )
  }
}

export interface Slipstream {
  readonly httpClient: HttpClient.HttpClient
  /**
* Lists desktop capture backends this host supports, with a best-effort availability probe.
* Pass an `id` as `SLIPSTREAM_CAPTURE_METHOD` / the host-config `capture_method` field.
* `auto` walks the preference order at session open.
*/
readonly "listCaptureMethods": <Config extends OperationConfig>(options: { readonly config?: Config | undefined } | undefined) => Effect.Effect<WithOptionalResponse<typeof ListCaptureMethods200.Type, Config>, HttpClientError.HttpClientError | SchemaError | SlipstreamError<"ListCaptureMethods401", typeof ListCaptureMethods401.Type>>
  /**
* List paired clients
*/
readonly "listPairedClients": <Config extends OperationConfig>(options: { readonly config?: Config | undefined } | undefined) => Effect.Effect<WithOptionalResponse<typeof ListPairedClients200.Type, Config>, HttpClientError.HttpClientError | SchemaError | SlipstreamError<"ListPairedClients401", typeof ListPairedClients401.Type>>
  /**
* Removes the client's certificate from the pairing store. Caveat: the nvhttp TLS layer
* does not yet reject unlisted certificates (`gamestream/tls.rs` accepts any well-formed
* client cert — a planned hardening step), so until that lands this removes the client
* from the listing without severing its ability to reconnect.
*/
readonly "unpairClient": <Config extends OperationConfig>(fingerprint: string, options: { readonly config?: Config | undefined } | undefined) => Effect.Effect<WithOptionalResponse<void, Config>, HttpClientError.HttpClientError | SchemaError | SlipstreamError<"UnpairClient400", typeof UnpairClient400.Type> | SlipstreamError<"UnpairClient401", typeof UnpairClient401.Type> | SlipstreamError<"UnpairClient404", typeof UnpairClient404.Type>>
  /**
* Lists every backend the host knows how to drive, flags which are usable right now, and marks
* the one an unspecified (`Auto`) client request resolves to. Clients pass an `id` to their
* `--compositor` flag (or `SLIPSTREAM_COMPOSITOR_*` over the C ABI) to request it.
*/
readonly "listCompositors": <Config extends OperationConfig>(options: { readonly config?: Config | undefined } | undefined) => Effect.Effect<WithOptionalResponse<typeof ListCompositors200.Type, Config>, HttpClientError.HttpClientError | SchemaError | SlipstreamError<"ListCompositors401", typeof ListCompositors401.Type>>
  /**
* Lists backends `SLIPSTREAM_HEADLESS_COMPOSITOR` / host-config `headless_compositor` can select.
* `available` is a PATH probe (`labwc`, `krfb-virtualmonitor`, `gamescope`); `auto` is available
* when any concrete backend is.
*/
readonly "listHeadlessCompositors": <Config extends OperationConfig>(options: { readonly config?: Config | undefined } | undefined) => Effect.Effect<WithOptionalResponse<typeof ListHeadlessCompositors200.Type, Config>, HttpClientError.HttpClientError | SchemaError | SlipstreamError<"ListHeadlessCompositors401", typeof ListHeadlessCompositors401.Type>>
  /**
* Evaluate host readiness without changing display or stream state.
*/
readonly "getDiagnosticsPreflight": <Config extends OperationConfig>(options: { readonly config?: Config | undefined } | undefined) => Effect.Effect<WithOptionalResponse<typeof GetDiagnosticsPreflight200.Type, Config>, HttpClientError.HttpClientError | SchemaError | SlipstreamError<"GetDiagnosticsPreflight401", typeof GetDiagnosticsPreflight401.Type>>
  /**
* Set the **manual** desktop arrangement — per-identity-slot `(x, y)` offsets so a multi-monitor
* group (§6A/§6B) comes back where the operator placed it. Persisted into the policy's layout block
* and switched to manual mode; applied from the next connect (a live group re-applies on its next
* acquire). Locks in the current effective behavior as explicit fields, so arranging displays never
* silently changes keep-alive/topology/conflict/identity. See `design/display-management.md` §6.2.
*/
readonly "setDisplayLayout": <Config extends OperationConfig>(options: { readonly payload: typeof SetDisplayLayoutRequestJson.Encoded; readonly config?: Config | undefined }) => Effect.Effect<WithOptionalResponse<typeof SetDisplayLayout200.Type, Config>, HttpClientError.HttpClientError | SchemaError | SlipstreamError<"SetDisplayLayout401", typeof SetDisplayLayout401.Type> | SlipstreamError<"SetDisplayLayout500", typeof SetDisplayLayout500.Type>>
  /**
* The heads this host actually has — for pinning capture at one (`SLIPSTREAM_CAPTURE_MONITOR`) and
* for rendering a picker. Read-only: this never creates, moves or disables anything. Note these
* are *not* the managed virtual displays — those are `/display/state`. See
* `design/per-monitor-portal-capture.md` §5.1.
*/
readonly "getDisplayMonitors": <Config extends OperationConfig>(options: { readonly config?: Config | undefined } | undefined) => Effect.Effect<WithOptionalResponse<typeof GetDisplayMonitors200.Type, Config>, HttpClientError.HttpClientError | SchemaError | SlipstreamError<"GetDisplayMonitors401", typeof GetDisplayMonitors401.Type>>
  /**
* The operator's named field-bundles (`display-presets.json`). These also ride the
* `GET /display/settings` response (`custom_presets`), so the console rarely needs this directly.
*/
readonly "listCustomPresets": <Config extends OperationConfig>(options: { readonly config?: Config | undefined } | undefined) => Effect.Effect<WithOptionalResponse<typeof ListCustomPresets200.Type, Config>, HttpClientError.HttpClientError | SchemaError | SlipstreamError<"ListCustomPresets401", typeof ListCustomPresets401.Type>>
  /**
* Stores a named bundle of the display-behavior axes (+ the game-session axis) the operator can
* apply later. The host assigns a stable id, returned in the body. Applying a preset is a
* `PUT /display/settings` with a `Custom` policy carrying its `fields` — no separate apply route.
*/
readonly "createCustomPreset": <Config extends OperationConfig>(options: { readonly payload: typeof CreateCustomPresetRequestJson.Encoded; readonly config?: Config | undefined }) => Effect.Effect<WithOptionalResponse<typeof CreateCustomPreset201.Type, Config>, HttpClientError.HttpClientError | SchemaError | SlipstreamError<"CreateCustomPreset400", typeof CreateCustomPreset400.Type> | SlipstreamError<"CreateCustomPreset401", typeof CreateCustomPreset401.Type> | SlipstreamError<"CreateCustomPreset500", typeof CreateCustomPreset500.Type>>
  /**
* Update a custom preset
*/
readonly "updateCustomPreset": <Config extends OperationConfig>(id: string, options: { readonly payload: typeof UpdateCustomPresetRequestJson.Encoded; readonly config?: Config | undefined }) => Effect.Effect<WithOptionalResponse<typeof UpdateCustomPreset200.Type, Config>, HttpClientError.HttpClientError | SchemaError | SlipstreamError<"UpdateCustomPreset400", typeof UpdateCustomPreset400.Type> | SlipstreamError<"UpdateCustomPreset401", typeof UpdateCustomPreset401.Type> | SlipstreamError<"UpdateCustomPreset404", typeof UpdateCustomPreset404.Type> | SlipstreamError<"UpdateCustomPreset500", typeof UpdateCustomPreset500.Type>>
  /**
* Removes it from the catalog. The active policy is untouched — if this preset was the one applied,
* the running behavior stays exactly as it was (the catalog and `display-settings.json` are decoupled).
*/
readonly "deleteCustomPreset": <Config extends OperationConfig>(id: string, options: { readonly config?: Config | undefined } | undefined) => Effect.Effect<WithOptionalResponse<void, Config>, HttpClientError.HttpClientError | SchemaError | SlipstreamError<"DeleteCustomPreset401", typeof DeleteCustomPreset401.Type> | SlipstreamError<"DeleteCustomPreset404", typeof DeleteCustomPreset404.Type> | SlipstreamError<"DeleteCustomPreset500", typeof DeleteCustomPreset500.Type>>
  /**
* Tear down lingering/pinned displays now — so a physical-screen user gets their screen back
* without waiting out the linger. `slot` releases one; omit it to release all kept displays.
* Active (streaming) displays are never torn down here (that is session control).
*/
readonly "releaseDisplay": <Config extends OperationConfig>(options: { readonly payload: typeof ReleaseDisplayRequestJson.Encoded; readonly config?: Config | undefined }) => Effect.Effect<WithOptionalResponse<typeof ReleaseDisplay200.Type, Config>, HttpClientError.HttpClientError | SchemaError | SlipstreamError<"ReleaseDisplay401", typeof ReleaseDisplay401.Type>>
  /**
* The stored virtual-display policy (lifecycle, topology, conflict handling, identity, layout),
* every preset's expansion, and which options this build enforces yet. See
* `design/display-management.md`.
*/
readonly "getDisplaySettings": <Config extends OperationConfig>(options: { readonly config?: Config | undefined } | undefined) => Effect.Effect<WithOptionalResponse<typeof GetDisplaySettings200.Type, Config>, HttpClientError.HttpClientError | SchemaError | SlipstreamError<"GetDisplaySettings401", typeof GetDisplaySettings401.Type>>
  /**
* Persists a new policy (validated + clamped) and applies it from the next connect/teardown — a
* running session keeps the display it opened on. `keep_alive: forever` (the gaming-rig preset) is
* honored (the display is Pinned; free it via `POST /display/release`).
*/
readonly "setDisplaySettings": <Config extends OperationConfig>(options: { readonly payload: typeof SetDisplaySettingsRequestJson.Encoded; readonly config?: Config | undefined }) => Effect.Effect<WithOptionalResponse<typeof SetDisplaySettings200.Type, Config>, HttpClientError.HttpClientError | SchemaError | SlipstreamError<"SetDisplaySettings400", typeof SetDisplaySettings400.Type> | SlipstreamError<"SetDisplaySettings401", typeof SetDisplaySettings401.Type> | SlipstreamError<"SetDisplaySettings500", typeof SetDisplaySettings500.Type>>
  /**
* The host's managed virtual displays right now — active (streaming), lingering (kept after
* disconnect, counting down to teardown), or pinned (kept indefinitely). See
* `design/display-management.md`.
*/
readonly "getDisplayState": <Config extends OperationConfig>(options: { readonly config?: Config | undefined } | undefined) => Effect.Effect<WithOptionalResponse<typeof GetDisplayState200.Type, Config>, HttpClientError.HttpClientError | SchemaError | SlipstreamError<"GetDisplayState401", typeof GetDisplayState401.Type>>
  /**
* Server-Sent Events stream of the host's lifecycle events: client connect/disconnect, session
* and stream start/end, pairing decisions, display create/release, library changes, host
* start/stop — both protocol planes. Frames carry `id:` = the event's monotonic `seq`,
* `event:` = its kind, and `data:` = the event JSON (schema-versioned, additive-only).
* 
* Resume: standard `Last-Event-ID` (or `?since=`) replays from the in-memory ring; a consumer
* that fell off the ring receives an `event: dropped` frame first and should resync via the
* REST snapshots. Keep-alive comments are sent every 15 s.
*/
readonly "streamEvents": <Config extends OperationConfig>(options: { readonly params?: typeof StreamEventsParams.Encoded | undefined; readonly config?: Config | undefined } | undefined) => Effect.Effect<WithOptionalResponse<void, Config>, HttpClientError.HttpClientError | SchemaError | SlipstreamError<"StreamEvents401", typeof StreamEvents401.Type> | SlipstreamError<"StreamEvents503", typeof StreamEvents503.Type>>
  /**
* Server-Sent Events stream of the host's lifecycle events: client connect/disconnect, session
* and stream start/end, pairing decisions, display create/release, library changes, host
* start/stop — both protocol planes. Frames carry `id:` = the event's monotonic `seq`,
* `event:` = its kind, and `data:` = the event JSON (schema-versioned, additive-only).
* 
* Resume: standard `Last-Event-ID` (or `?since=`) replays from the in-memory ring; a consumer
* that fell off the ring receives an `event: dropped` frame first and should resync via the
* REST snapshots. Keep-alive comments are sent every 15 s.
*/
readonly "streamEventsSse": (options: { readonly params?: typeof StreamEventsParams.Encoded | undefined } | undefined) => Stream.Stream<{ readonly event: string; readonly id: string | undefined; readonly data: typeof StreamEvents200Sse.Type }, HttpClientError.HttpClientError | SchemaError | Sse.Retry, typeof StreamEvents200Sse.DecodingServices>
  /**
* Ends a game whose session has already gone and which is waiting out its reconnect window — the
* console's "End now" for a game the host is about to close anyway. `app_id` picks one title; omit it
* to end every waiting game.
* 
* This does **not** touch a game whose session is still live: ending that is session management
* (`DELETE /session`), and how the game is treated then follows the operator's policy.
*/
readonly "endGame": <Config extends OperationConfig>(options: { readonly payload: typeof EndGameRequestJson.Encoded; readonly config?: Config | undefined }) => Effect.Effect<WithOptionalResponse<typeof EndGame200.Type, Config>, HttpClientError.HttpClientError | SchemaError | SlipstreamError<"EndGame401", typeof EndGame401.Type> | SlipstreamError<"EndGame409", typeof EndGame409.Type>>
  /**
* Lists the host's hardware GPUs, the persisted auto/manual preference, the GPU the next session
* will use (and why), and the GPU live sessions encode on right now.
*/
readonly "listGpus": <Config extends OperationConfig>(options: { readonly config?: Config | undefined } | undefined) => Effect.Effect<WithOptionalResponse<typeof ListGpus200.Type, Config>, HttpClientError.HttpClientError | SchemaError | SlipstreamError<"ListGpus401", typeof ListGpus401.Type>>
  /**
* `auto` restores automatic selection (`SLIPSTREAM_RENDER_ADAPTER` pin, else max dedicated VRAM);
* `manual` pins capture + encode to the given GPU. Persisted across restarts; applies to the
* **next** session (a running session keeps its GPU). If the preferred GPU is absent at session
* start the host falls back to automatic selection rather than failing.
*/
readonly "setGpuPreference": <Config extends OperationConfig>(options: { readonly payload: typeof SetGpuPreferenceRequestJson.Encoded; readonly config?: Config | undefined }) => Effect.Effect<WithOptionalResponse<typeof SetGpuPreference200.Type, Config>, HttpClientError.HttpClientError | SchemaError | SlipstreamError<"SetGpuPreference400", typeof SetGpuPreference400.Type> | SlipstreamError<"SetGpuPreference401", typeof SetGpuPreference401.Type> | SlipstreamError<"SetGpuPreference500", typeof SetGpuPreference500.Type>>
  /**
* Always available without authentication.
*/
readonly "getHealth": <Config extends OperationConfig>(options: { readonly config?: Config | undefined } | undefined) => Effect.Effect<WithOptionalResponse<typeof GetHealth200.Type, Config>, HttpClientError.HttpClientError | SchemaError>
  /**
* The operator's `hooks.json`: commands and webhooks fired on host lifecycle events. Empty
* when unconfigured.
*/
readonly "getHooks": <Config extends OperationConfig>(options: { readonly config?: Config | undefined } | undefined) => Effect.Effect<WithOptionalResponse<typeof GetHooks200.Type, Config>, HttpClientError.HttpClientError | SchemaError | SlipstreamError<"GetHooks401", typeof GetHooks401.Type>>
  /**
* Validates and persists a full `hooks.json` document (this is a whole-document PUT, not a
* patch). Applies from the next event, with no restart. Hook commands run as the host user, so
* treat this configuration as operator-privileged.
*/
readonly "setHooks": <Config extends OperationConfig>(options: { readonly payload: typeof SetHooksRequestJson.Encoded; readonly config?: Config | undefined }) => Effect.Effect<WithOptionalResponse<typeof SetHooks200.Type, Config>, HttpClientError.HttpClientError | SchemaError | SlipstreamError<"SetHooks400", typeof SetHooks400.Type> | SlipstreamError<"SetHooks401", typeof SetHooks401.Type> | SlipstreamError<"SetHooks500", typeof SetHooks500.Type>>
  /**
* Host identity and capabilities
*/
readonly "getHostInfo": <Config extends OperationConfig>(options: { readonly config?: Config | undefined } | undefined) => Effect.Effect<WithOptionalResponse<typeof GetHostInfo200.Type, Config>, HttpClientError.HttpClientError | SchemaError | SlipstreamError<"GetHostInfo401", typeof GetHostInfo401.Type>>
  /**
* Persisted operator knobs (name, encoder, AV, network, input). Written to `host-config.json`
* and dual-written to `host.env` for the next process start.
*/
readonly "getHostConfig": <Config extends OperationConfig>(options: { readonly config?: Config | undefined } | undefined) => Effect.Effect<WithOptionalResponse<typeof GetHostConfig200.Type, Config>, HttpClientError.HttpClientError | SchemaError | SlipstreamError<"GetHostConfig401", typeof GetHostConfig401.Type>>
  /**
* Saves the console form to disk. Most fields require a host restart before the running
* process picks them up (`requires_restart` stays true).
*/
readonly "setHostConfig": <Config extends OperationConfig>(options: { readonly payload: typeof SetHostConfigRequestJson.Encoded; readonly config?: Config | undefined }) => Effect.Effect<WithOptionalResponse<typeof SetHostConfig200.Type, Config>, HttpClientError.HttpClientError | SchemaError | SlipstreamError<"SetHostConfig400", typeof SetHostConfig400.Type> | SlipstreamError<"SetHostConfig401", typeof SetHostConfig401.Type> | SlipstreamError<"SetHostConfig500", typeof SetHostConfig500.Type>>
  /**
* This is the only write path for the GameStream/Moonlight compatibility plane. The general
* configuration form preserves the current value when it saves a draft.
*/
readonly "setMoonlightBroadcast": <Config extends OperationConfig>(options: { readonly payload: typeof SetMoonlightBroadcastRequestJson.Encoded; readonly config?: Config | undefined }) => Effect.Effect<WithOptionalResponse<typeof SetMoonlightBroadcast200.Type, Config>, HttpClientError.HttpClientError | SchemaError | SlipstreamError<"SetMoonlightBroadcast401", typeof SetMoonlightBroadcast401.Type> | SlipstreamError<"SetMoonlightBroadcast500", typeof SetMoonlightBroadcast500.Type>>
  /**
* Schedules a bounce of `slipstream-host` (service manager when available, otherwise re-exec).
* Returns immediately with `202`; the process exits shortly after so the response can flush.
* Any live stream drops. Does not reboot the machine.
*/
readonly "restartHost": <Config extends OperationConfig>(options: { readonly config?: Config | undefined } | undefined) => Effect.Effect<WithOptionalResponse<void, Config>, HttpClientError.HttpClientError | SchemaError | SlipstreamError<"RestartHost401", typeof RestartHost401.Type> | SlipstreamError<"RestartHost500", typeof RestartHost500.Type>>
  /**
* Schedules a clean stop of `slipstream-host` (session takeover restore, then exit). Returns
* immediately with `202`. The process does not start again until an operator or supervisor
* starts it. Does not power off the machine.
*/
readonly "shutdownHost": <Config extends OperationConfig>(options: { readonly config?: Config | undefined } | undefined) => Effect.Effect<WithOptionalResponse<void, Config>, HttpClientError.HttpClientError | SchemaError | SlipstreamError<"ShutdownHost401", typeof ShutdownHost401.Type>>
  /**
* Every installed-store title (Steam, read from the host's local files — no Steam API key)
* merged with the user's custom entries, sorted by title. Artwork fields are URLs the client
* fetches directly (the public Steam CDN for Steam titles). `?provider=` narrows to the
* entries a given external provider owns; `?platform=` to one platform (case-insensitive —
* installed-store titles are `PC`, custom/provider entries carry whatever was authored).
*/
readonly "getLibrary": <Config extends OperationConfig>(options: { readonly params?: typeof GetLibraryParams.Encoded | undefined; readonly config?: Config | undefined } | undefined) => Effect.Effect<WithOptionalResponse<typeof GetLibrary200.Type, Config>, HttpClientError.HttpClientError | SchemaError | SlipstreamError<"GetLibrary401", typeof GetLibrary401.Type>>
  /**
* Resolves `kind` (`portrait` | `hero` | `logo` | `header`) for the given library id and streams
* the image bytes. For a Steam title, the host's own local Steam cache is tried first (exact —
* it's what the user's Steam client already shows for it), the public Steam CDN's flat URL
* convention as a fallback (newer titles' CDN assets can live at a per-asset-hash path the host
* can't predict, in which case this 404s and the client falls through to its next art candidate).
* Only Steam ids are backed today; any other store 404s.
*/
readonly "getLibraryArt": <Config extends OperationConfig>(id: string, kind: string, options: { readonly config?: Config | undefined } | undefined) => Effect.Effect<WithOptionalResponse<void, Config>, HttpClientError.HttpClientError | SchemaError | SlipstreamError<"GetLibraryArt401", typeof GetLibraryArt401.Type> | SlipstreamError<"GetLibraryArt404", typeof GetLibraryArt404.Type>>
  /**
* Creates a user-curated title (e.g. a non-Steam game, an emulator, a ROM) with caller-supplied
* artwork URLs. The host assigns a stable id, returned in the body.
*/
readonly "createCustomGame": <Config extends OperationConfig>(options: { readonly payload: typeof CreateCustomGameRequestJson.Encoded; readonly config?: Config | undefined }) => Effect.Effect<WithOptionalResponse<typeof CreateCustomGame201.Type, Config>, HttpClientError.HttpClientError | SchemaError | SlipstreamError<"CreateCustomGame400", typeof CreateCustomGame400.Type> | SlipstreamError<"CreateCustomGame401", typeof CreateCustomGame401.Type> | SlipstreamError<"CreateCustomGame500", typeof CreateCustomGame500.Type>>
  /**
* Update a custom library entry
*/
readonly "updateCustomGame": <Config extends OperationConfig>(id: string, options: { readonly payload: typeof UpdateCustomGameRequestJson.Encoded; readonly config?: Config | undefined }) => Effect.Effect<WithOptionalResponse<typeof UpdateCustomGame200.Type, Config>, HttpClientError.HttpClientError | SchemaError | SlipstreamError<"UpdateCustomGame400", typeof UpdateCustomGame400.Type> | SlipstreamError<"UpdateCustomGame401", typeof UpdateCustomGame401.Type> | SlipstreamError<"UpdateCustomGame404", typeof UpdateCustomGame404.Type> | SlipstreamError<"UpdateCustomGame500", typeof UpdateCustomGame500.Type>>
  /**
* Delete a custom library entry
*/
readonly "deleteCustomGame": <Config extends OperationConfig>(id: string, options: { readonly config?: Config | undefined } | undefined) => Effect.Effect<WithOptionalResponse<void, Config>, HttpClientError.HttpClientError | SchemaError | SlipstreamError<"DeleteCustomGame401", typeof DeleteCustomGame401.Type> | SlipstreamError<"DeleteCustomGame404", typeof DeleteCustomGame404.Type> | SlipstreamError<"DeleteCustomGame500", typeof DeleteCustomGame500.Type>>
  /**
* Atomically replaces the full entry set owned by `{provider}` (RFC §8): the payload is the
* provider's desired list, keyed by its own stable `external_id` — the host diffs, keeps each
* surviving title's host id stable across reconciles, drops orphans, and never touches manual
* entries or other providers'. An empty array removes everything the provider owns. Emits
* `library.changed` with the provider as `source`.
*/
readonly "reconcileProviderEntries": <Config extends OperationConfig>(provider: string, options: { readonly payload: typeof ReconcileProviderEntriesRequestJson.Encoded; readonly config?: Config | undefined }) => Effect.Effect<WithOptionalResponse<typeof ReconcileProviderEntries200.Type, Config>, HttpClientError.HttpClientError | SchemaError | SlipstreamError<"ReconcileProviderEntries400", typeof ReconcileProviderEntries400.Type> | SlipstreamError<"ReconcileProviderEntries401", typeof ReconcileProviderEntries401.Type> | SlipstreamError<"ReconcileProviderEntries500", typeof ReconcileProviderEntries500.Type>>
  /**
* Deletes every entry owned by `{provider}` — the clean-uninstall path for a provider plugin
* (RFC §8). Emits `library.changed` when anything was removed.
*/
readonly "deleteProviderEntries": <Config extends OperationConfig>(provider: string, options: { readonly config?: Config | undefined } | undefined) => Effect.Effect<WithOptionalResponse<typeof DeleteProviderEntries200.Type, Config>, HttpClientError.HttpClientError | SchemaError | SlipstreamError<"DeleteProviderEntries400", typeof DeleteProviderEntries400.Type> | SlipstreamError<"DeleteProviderEntries401", typeof DeleteProviderEntries401.Type> | SlipstreamError<"DeleteProviderEntries500", typeof DeleteProviderEntries500.Type>>
  /**
* The installed-store scanners this host supports are discovered from local Linux paths, so the
* console renders a toggle only for scanners that can do anything here. Scanners default to enabled;
* disabling one hides its titles from every library surface from the next read. The user-curated
* custom store is not a scanner and is always on.
*/
readonly "listLibraryScanners": <Config extends OperationConfig>(options: { readonly config?: Config | undefined } | undefined) => Effect.Effect<WithOptionalResponse<typeof ListLibraryScanners200.Type, Config>, HttpClientError.HttpClientError | SchemaError | SlipstreamError<"ListLibraryScanners401", typeof ListLibraryScanners401.Type>>
  /**
* Persists the toggle and applies it from the next library read (no restart). Disabling a scanner
* hides its titles everywhere — the console grid, native clients, and the GameStream app list —
* and re-enabling brings them straight back (nothing is deleted; the scan just runs again). Emits
* `library.changed` with the scanner id as `source` when the state changed.
*/
readonly "setLibraryScanner": <Config extends OperationConfig>(id: string, options: { readonly payload: typeof SetLibraryScannerRequestJson.Encoded; readonly config?: Config | undefined }) => Effect.Effect<WithOptionalResponse<typeof SetLibraryScanner200.Type, Config>, HttpClientError.HttpClientError | SchemaError | SlipstreamError<"SetLibraryScanner401", typeof SetLibraryScanner401.Type> | SlipstreamError<"SetLibraryScanner404", typeof SetLibraryScanner404.Type> | SlipstreamError<"SetLibraryScanner500", typeof SetLibraryScanner500.Type>>
  /**
* Non-sensitive status (counts, booleans, and the streaming client's display name — no PIN
* values, no fingerprints). Unauthenticated, but served to loopback peers only.
*/
readonly "getLocalSummary": <Config extends OperationConfig>(options: { readonly config?: Config | undefined } | undefined) => Effect.Effect<WithOptionalResponse<typeof GetLocalSummary200.Type, Config>, HttpClientError.HttpClientError | SchemaError | SlipstreamError<"GetLocalSummary401", typeof GetLocalSummary401.Type>>
  /**
* The host's recent log entries — an in-memory ring of the newest few thousand, captured at
* DEBUG and above regardless of `RUST_LOG`. Follow live by polling with `after` set to the last
* response's `next` cursor; a `dropped: true` means entries were evicted between polls (the ring
* wrapped). Bearer-only: logs can reference client identities and host paths, so this is part of
* the loopback-only admin surface, never the LAN-readable mTLS one.
*/
readonly "logsGet": <Config extends OperationConfig>(options: { readonly params?: typeof LogsGetParams.Encoded | undefined; readonly config?: Config | undefined } | undefined) => Effect.Effect<WithOptionalResponse<typeof LogsGet200.Type, Config>, HttpClientError.HttpClientError | SchemaError | SlipstreamError<"LogsGet401", typeof LogsGet401.Type>>
  /**
* List native paired clients
*/
readonly "listNativeClients": <Config extends OperationConfig>(options: { readonly config?: Config | undefined } | undefined) => Effect.Effect<WithOptionalResponse<typeof ListNativeClients200.Type, Config>, HttpClientError.HttpClientError | SchemaError | SlipstreamError<"ListNativeClients401", typeof ListNativeClients401.Type>>
  /**
* Removes a slipstream/1 client from the native trust store by fingerprint.
*/
readonly "unpairNativeClient": <Config extends OperationConfig>(fingerprint: string, options: { readonly config?: Config | undefined } | undefined) => Effect.Effect<WithOptionalResponse<void, Config>, HttpClientError.HttpClientError | SchemaError | SlipstreamError<"UnpairNativeClient401", typeof UnpairNativeClient401.Type> | SlipstreamError<"UnpairNativeClient404", typeof UnpairNativeClient404.Type> | SlipstreamError<"UnpairNativeClient503", typeof UnpairNativeClient503.Type>>
  /**
* The native (slipstream/1) pairing window. Poll while armed to show the PIN + countdown.
* `enabled: false` means this host runs GameStream only (no `--native`).
*/
readonly "getNativePairing": <Config extends OperationConfig>(options: { readonly config?: Config | undefined } | undefined) => Effect.Effect<WithOptionalResponse<typeof GetNativePairing200.Type, Config>, HttpClientError.HttpClientError | SchemaError | SlipstreamError<"GetNativePairing401", typeof GetNativePairing401.Type>>
  /**
* Closes the pairing window immediately (no new ceremonies accepted).
*/
readonly "disarmNativePairing": <Config extends OperationConfig>(options: { readonly config?: Config | undefined } | undefined) => Effect.Effect<WithOptionalResponse<void, Config>, HttpClientError.HttpClientError | SchemaError | SlipstreamError<"DisarmNativePairing401", typeof DisarmNativePairing401.Type> | SlipstreamError<"DisarmNativePairing503", typeof DisarmNativePairing503.Type>>
  /**
* Opens a pairing window and mints a fresh PIN to display. The user enters it on their device
* within `ttl_secs`; the device then appears in the native client list.
*/
readonly "armNativePairing": <Config extends OperationConfig>(options: { readonly payload: typeof ArmNativePairingRequestJson.Encoded; readonly config?: Config | undefined }) => Effect.Effect<WithOptionalResponse<typeof ArmNativePairing200.Type, Config>, HttpClientError.HttpClientError | SchemaError | SlipstreamError<"ArmNativePairing401", typeof ArmNativePairing401.Type> | SlipstreamError<"ArmNativePairing503", typeof ArmNativePairing503.Type>>
  /**
* Unpaired devices that tried to connect while the host requires pairing. Approve one to pair
* it without a PIN (delegated approval); entries expire after ~10 minutes.
*/
readonly "listPendingDevices": <Config extends OperationConfig>(options: { readonly config?: Config | undefined } | undefined) => Effect.Effect<WithOptionalResponse<typeof ListPendingDevices200.Type, Config>, HttpClientError.HttpClientError | SchemaError | SlipstreamError<"ListPendingDevices401", typeof ListPendingDevices401.Type>>
  /**
* Pairs the device's certificate fingerprint — it can connect immediately (no PIN). Optionally
* relabel it via the body; send `{}` to keep the name it knocked with.
*/
readonly "approvePendingDevice": <Config extends OperationConfig>(id: string, options: { readonly payload: typeof ApprovePendingDeviceRequestJson.Encoded; readonly config?: Config | undefined }) => Effect.Effect<WithOptionalResponse<typeof ApprovePendingDevice200.Type, Config>, HttpClientError.HttpClientError | SchemaError | SlipstreamError<"ApprovePendingDevice401", typeof ApprovePendingDevice401.Type> | SlipstreamError<"ApprovePendingDevice404", typeof ApprovePendingDevice404.Type> | SlipstreamError<"ApprovePendingDevice500", typeof ApprovePendingDevice500.Type> | SlipstreamError<"ApprovePendingDevice503", typeof ApprovePendingDevice503.Type>>
  /**
* Drops the request. Not a blocklist — the device's next attempt knocks again.
*/
readonly "denyPendingDevice": <Config extends OperationConfig>(id: string, options: { readonly config?: Config | undefined } | undefined) => Effect.Effect<WithOptionalResponse<void, Config>, HttpClientError.HttpClientError | SchemaError | SlipstreamError<"DenyPendingDevice401", typeof DenyPendingDevice401.Type> | SlipstreamError<"DenyPendingDevice404", typeof DenyPendingDevice404.Type> | SlipstreamError<"DenyPendingDevice503", typeof DenyPendingDevice503.Type>>
  /**
* Poll this to know when to prompt the user for the PIN Moonlight displays.
*/
readonly "getPairingStatus": <Config extends OperationConfig>(options: { readonly config?: Config | undefined } | undefined) => Effect.Effect<WithOptionalResponse<typeof GetPairingStatus200.Type, Config>, HttpClientError.HttpClientError | SchemaError | SlipstreamError<"GetPairingStatus401", typeof GetPairingStatus401.Type>>
  /**
* Delivers the PIN the Moonlight client is displaying, completing the out-of-band half
* of the pairing handshake.
*/
readonly "submitPairingPin": <Config extends OperationConfig>(options: { readonly payload: typeof SubmitPairingPinRequestJson.Encoded; readonly config?: Config | undefined }) => Effect.Effect<WithOptionalResponse<void, Config>, HttpClientError.HttpClientError | SchemaError | SlipstreamError<"SubmitPairingPin400", typeof SubmitPairingPin400.Type> | SlipstreamError<"SubmitPairingPin401", typeof SubmitPairingPin401.Type> | SlipstreamError<"SubmitPairingPin409", typeof SubmitPairingPin409.Type> | SlipstreamError<"SubmitPairingPin415", typeof SubmitPairingPin415.Type> | SlipstreamError<"SubmitPairingPin422", typeof SubmitPairingPin422.Type>>
  /**
* The live plugin directory (lease not expired), sorted by title. **Secret-free**: each entry
* reports its id, title, optional version, and — for plugins that serve one — a UI descriptor
* (loopback port + icon). The console renders these as nav entries and proxies to the port; it
* fetches the secret separately, server-side.
*/
readonly "listPlugins": <Config extends OperationConfig>(options: { readonly config?: Config | undefined } | undefined) => Effect.Effect<WithOptionalResponse<typeof ListPlugins200.Type, Config>, HttpClientError.HttpClientError | SchemaError | SlipstreamError<"ListPlugins401", typeof ListPlugins401.Type>>
  /**
* Upserts the plugin's directory entry and renews its lease (TTL 90 s). Idempotent: a plugin PUTs
* this every ~30 s while it runs. The optional `ui` block declares a loopback UI surface the console
* will proxy and add to its nav. Emits `plugins.changed` when an operator-visible field changed
* (first registration, restart, or re-scan) — a pure renewal is silent.
*/
readonly "registerPlugin": <Config extends OperationConfig>(id: string, options: { readonly payload: typeof RegisterPluginRequestJson.Encoded; readonly config?: Config | undefined }) => Effect.Effect<WithOptionalResponse<void, Config>, HttpClientError.HttpClientError | SchemaError | SlipstreamError<"RegisterPlugin400", typeof RegisterPlugin400.Type> | SlipstreamError<"RegisterPlugin401", typeof RegisterPlugin401.Type>>
  /**
* The clean-shutdown path: removes the plugin's directory entry immediately (the SDK helper calls
* this from its scope finalizer on `SIGTERM`). Emits `plugins.changed` when a live entry was
* removed. Idempotent — deleting an unknown/expired id is a no-op `204`.
*/
readonly "deregisterPlugin": <Config extends OperationConfig>(id: string, options: { readonly config?: Config | undefined } | undefined) => Effect.Effect<WithOptionalResponse<void, Config>, HttpClientError.HttpClientError | SchemaError | SlipstreamError<"DeregisterPlugin401", typeof DeregisterPlugin401.Type>>
  /**
* Returns `{port, secret}` for a live plugin's loopback UI — the console proxy's server-side lookup.
* Bearer + loopback only (like every mutation), and additionally excluded from the console's browser
* passthrough: the secret never reaches a browser.
*/
readonly "getPluginUiCredential": <Config extends OperationConfig>(id: string, options: { readonly config?: Config | undefined } | undefined) => Effect.Effect<WithOptionalResponse<typeof GetPluginUiCredential200.Type, Config>, HttpClientError.HttpClientError | SchemaError | SlipstreamError<"GetPluginUiCredential401", typeof GetPluginUiCredential401.Type> | SlipstreamError<"GetPluginUiCredential404", typeof GetPluginUiCredential404.Type>>
  /**
* Kicks the connected client: stops the video/audio stream threads and clears the launch
* state. Idempotent — succeeds even when nothing is streaming.
* 
* Counts as a **deliberate** stop, exactly like a client pressing Stop: the display skips its
* keep-alive linger, and the end-game-on-session-end policy (if the operator enabled one) applies.
*/
readonly "stopSession": <Config extends OperationConfig>(options: { readonly config?: Config | undefined } | undefined) => Effect.Effect<WithOptionalResponse<void, Config>, HttpClientError.HttpClientError | SchemaError | SlipstreamError<"StopSession401", typeof StopSession401.Type>>
  /**
* Asks the encoder for an IDR frame on the active video stream (what a client requests
* after unrecoverable loss — exposed for debugging).
*/
readonly "requestIdr": <Config extends OperationConfig>(options: { readonly config?: Config | undefined } | undefined) => Effect.Effect<WithOptionalResponse<void, Config>, HttpClientError.HttpClientError | SchemaError | SlipstreamError<"RequestIdr401", typeof RequestIdr401.Type> | SlipstreamError<"RequestIdr409", typeof RequestIdr409.Type>>
  /**
* Whether a launched game's exit ends the streaming session, and whether a session ending ends the
* game (with the reconnect window that protects a dropped client's unsaved progress). See
* `design/session-game-lifetime.md`.
*/
readonly "getSessionSettings": <Config extends OperationConfig>(options: { readonly config?: Config | undefined } | undefined) => Effect.Effect<WithOptionalResponse<typeof GetSessionSettings200.Type, Config>, HttpClientError.HttpClientError | SchemaError | SlipstreamError<"GetSessionSettings401", typeof GetSessionSettings401.Type>>
  /**
* Persists the settings (clamped) and applies them from the next decision — including to a session
* that is already streaming, since the policy is read when a session ends rather than when it starts.
*/
readonly "setSessionSettings": <Config extends OperationConfig>(options: { readonly payload: typeof SetSessionSettingsRequestJson.Encoded; readonly config?: Config | undefined }) => Effect.Effect<WithOptionalResponse<typeof SetSessionSettings200.Type, Config>, HttpClientError.HttpClientError | SchemaError | SlipstreamError<"SetSessionSettings400", typeof SetSessionSettings400.Type> | SlipstreamError<"SetSessionSettings401", typeof SetSessionSettings401.Type> | SlipstreamError<"SetSessionSettings500", typeof SetSessionSettings500.Type>>
  /**
* The full sample time-series of the capture currently recording, for live graphing. `404` when
* nothing is armed.
*/
readonly "statsCaptureLive": <Config extends OperationConfig>(options: { readonly config?: Config | undefined } | undefined) => Effect.Effect<WithOptionalResponse<typeof StatsCaptureLive200.Type, Config>, HttpClientError.HttpClientError | SchemaError | SlipstreamError<"StatsCaptureLive401", typeof StatsCaptureLive401.Type> | SlipstreamError<"StatsCaptureLive404", typeof StatsCaptureLive404.Type>>
  /**
* Arms a new performance-stats capture. Idempotent: if a capture is already running this returns
* the current status unchanged. While armed, the streaming loops emit aggregated samples (~ every
* 1–2 s) into the in-progress capture, readable live via `GET /stats/capture/live`.
*/
readonly "statsCaptureStart": <Config extends OperationConfig>(options: { readonly config?: Config | undefined } | undefined) => Effect.Effect<WithOptionalResponse<typeof StatsCaptureStart200.Type, Config>, HttpClientError.HttpClientError | SchemaError | SlipstreamError<"StatsCaptureStart401", typeof StatsCaptureStart401.Type>>
  /**
* Whether a capture is armed, its sample count, and start time. Poll this (e.g. every 2 s) to
* drive the capture-control UI.
*/
readonly "statsCaptureStatus": <Config extends OperationConfig>(options: { readonly config?: Config | undefined } | undefined) => Effect.Effect<WithOptionalResponse<typeof StatsCaptureStatus200.Type, Config>, HttpClientError.HttpClientError | SchemaError | SlipstreamError<"StatsCaptureStatus401", typeof StatsCaptureStatus401.Type>>
  /**
* Disarms the in-progress capture and writes it to disk atomically, returning its summary. If
* nothing was recording, returns `204 No Content`.
*/
readonly "statsCaptureStop": <Config extends OperationConfig>(options: { readonly config?: Config | undefined } | undefined) => Effect.Effect<WithOptionalResponse<typeof StatsCaptureStop200.Type, Config>, HttpClientError.HttpClientError | SchemaError | SlipstreamError<"StatsCaptureStop401", typeof StatsCaptureStop401.Type> | SlipstreamError<"StatsCaptureStop500", typeof StatsCaptureStop500.Type>>
  /**
* Every saved capture's summary (the `meta` head only — not the sample body), newest first.
*/
readonly "statsRecordingsList": <Config extends OperationConfig>(options: { readonly config?: Config | undefined } | undefined) => Effect.Effect<WithOptionalResponse<typeof StatsRecordingsList200.Type, Config>, HttpClientError.HttpClientError | SchemaError | SlipstreamError<"StatsRecordingsList401", typeof StatsRecordingsList401.Type>>
  /**
* The full capture (meta + samples) for `id`, for graphing or download.
*/
readonly "statsRecordingGet": <Config extends OperationConfig>(id: string, options: { readonly config?: Config | undefined } | undefined) => Effect.Effect<WithOptionalResponse<typeof StatsRecordingGet200.Type, Config>, HttpClientError.HttpClientError | SchemaError | SlipstreamError<"StatsRecordingGet401", typeof StatsRecordingGet401.Type> | SlipstreamError<"StatsRecordingGet404", typeof StatsRecordingGet404.Type> | SlipstreamError<"StatsRecordingGet500", typeof StatsRecordingGet500.Type>>
  /**
* Removes the recording `id` from disk. `404` if there is no such recording.
*/
readonly "statsRecordingDelete": <Config extends OperationConfig>(id: string, options: { readonly config?: Config | undefined } | undefined) => Effect.Effect<WithOptionalResponse<void, Config>, HttpClientError.HttpClientError | SchemaError | SlipstreamError<"StatsRecordingDelete401", typeof StatsRecordingDelete401.Type> | SlipstreamError<"StatsRecordingDelete404", typeof StatsRecordingDelete404.Type> | SlipstreamError<"StatsRecordingDelete500", typeof StatsRecordingDelete500.Type>>
  /**
* Live host status
*/
readonly "getStatus": <Config extends OperationConfig>(options: { readonly config?: Config | undefined } | undefined) => Effect.Effect<WithOptionalResponse<typeof GetStatus200.Type, Config>, HttpClientError.HttpClientError | SchemaError | SlipstreamError<"GetStatus401", typeof GetStatus401.Type>>
  /**
* The merged shelf across every configured source, annotated with what this host already has and
* what it can run. Sources past their freshness window are refreshed first; a source that can't be
* reached keeps serving its last good copy, marked `stale` (a LAN-only host still has a working
* store — an entry's pin travelled with the entry).
*/
readonly "getPluginCatalog": <Config extends OperationConfig>(options: { readonly config?: Config | undefined } | undefined) => Effect.Effect<WithOptionalResponse<typeof GetPluginCatalog200.Type, Config>, HttpClientError.HttpClientError | SchemaError | SlipstreamError<"GetPluginCatalog401", typeof GetPluginCatalog401.Type> | SlipstreamError<"GetPluginCatalog403", typeof GetPluginCatalog403.Type>>
  /**
* Either `{source, id}` — a catalogued entry, installed at its pinned version after its integrity
* is re-checked against the registry — or `{spec, accept_unverified: true}`, which installs an
* unreviewed package the operator takes responsibility for. Returns `202` with a job id; watch it
* at `GET /store/jobs/{id}`.
* 
* One package operation runs at a time (`409` otherwise): `bun` operations share a lockfile and a
* `node_modules` tree.
*/
readonly "installPlugin": <Config extends OperationConfig>(options: { readonly payload: typeof InstallPluginRequestJson.Encoded; readonly config?: Config | undefined }) => Effect.Effect<WithOptionalResponse<typeof InstallPlugin202.Type, Config>, HttpClientError.HttpClientError | SchemaError | SlipstreamError<"InstallPlugin400", typeof InstallPlugin400.Type> | SlipstreamError<"InstallPlugin401", typeof InstallPlugin401.Type> | SlipstreamError<"InstallPlugin403", typeof InstallPlugin403.Type> | SlipstreamError<"InstallPlugin409", typeof InstallPlugin409.Type>>
  /**
* What's actually in the plugins directory, joined with how it got there (the provenance manifest)
* and whether it is registered right now. A package with no provenance record was installed with
* the CLI and reports `tier: "cli"` — absence is the answer, not a gap.
*/
readonly "listInstalledPlugins": <Config extends OperationConfig>(options: { readonly config?: Config | undefined } | undefined) => Effect.Effect<WithOptionalResponse<typeof ListInstalledPlugins200.Type, Config>, HttpClientError.HttpClientError | SchemaError | SlipstreamError<"ListInstalledPlugins401", typeof ListInstalledPlugins401.Type> | SlipstreamError<"ListInstalledPlugins403", typeof ListInstalledPlugins403.Type>>
  /**
* List recent package jobs
*/
readonly "listPluginJobs": <Config extends OperationConfig>(options: { readonly config?: Config | undefined } | undefined) => Effect.Effect<WithOptionalResponse<typeof ListPluginJobs200.Type, Config>, HttpClientError.HttpClientError | SchemaError | SlipstreamError<"ListPluginJobs401", typeof ListPluginJobs401.Type> | SlipstreamError<"ListPluginJobs403", typeof ListPluginJobs403.Type>>
  /**
* Poll this while `state` is `running`; `log` carries the tail of the package manager's output.
*/
readonly "getPluginJob": <Config extends OperationConfig>(id: string, options: { readonly config?: Config | undefined } | undefined) => Effect.Effect<WithOptionalResponse<typeof GetPluginJob200.Type, Config>, HttpClientError.HttpClientError | SchemaError | SlipstreamError<"GetPluginJob401", typeof GetPluginJob401.Type> | SlipstreamError<"GetPluginJob403", typeof GetPluginJob403.Type> | SlipstreamError<"GetPluginJob404", typeof GetPluginJob404.Type>>
  /**
* Bypasses the freshness window and re-fetches all sources, then returns the merged catalog.
*/
readonly "refreshPluginCatalog": <Config extends OperationConfig>(options: { readonly config?: Config | undefined } | undefined) => Effect.Effect<WithOptionalResponse<typeof RefreshPluginCatalog200.Type, Config>, HttpClientError.HttpClientError | SchemaError | SlipstreamError<"RefreshPluginCatalog401", typeof RefreshPluginCatalog401.Type> | SlipstreamError<"RefreshPluginCatalog403", typeof RefreshPluginCatalog403.Type>>
  /**
* Installed plugins only run while the runner is on, and the runner discovers units at startup —
* so this is both the "is anything running" answer and the explanation for a freshly installed
* plugin that hasn't appeared yet.
*/
readonly "getPluginRuntime": <Config extends OperationConfig>(options: { readonly config?: Config | undefined } | undefined) => Effect.Effect<WithOptionalResponse<typeof GetPluginRuntime200.Type, Config>, HttpClientError.HttpClientError | SchemaError | SlipstreamError<"GetPluginRuntime401", typeof GetPluginRuntime401.Type> | SlipstreamError<"GetPluginRuntime403", typeof GetPluginRuntime403.Type>>
  /**
* Turn the plugin runner on or off
*/
readonly "setPluginRuntime": <Config extends OperationConfig>(options: { readonly payload: typeof SetPluginRuntimeRequestJson.Encoded; readonly config?: Config | undefined }) => Effect.Effect<WithOptionalResponse<typeof SetPluginRuntime200.Type, Config>, HttpClientError.HttpClientError | SchemaError | SlipstreamError<"SetPluginRuntime400", typeof SetPluginRuntime400.Type> | SlipstreamError<"SetPluginRuntime401", typeof SetPluginRuntime401.Type> | SlipstreamError<"SetPluginRuntime403", typeof SetPluginRuntime403.Type>>
  /**
* List catalog sources
*/
readonly "listPluginSources": <Config extends OperationConfig>(options: { readonly config?: Config | undefined } | undefined) => Effect.Effect<WithOptionalResponse<typeof ListPluginSources200.Type, Config>, HttpClientError.HttpClientError | SchemaError | SlipstreamError<"ListPluginSources401", typeof ListPluginSources401.Type> | SlipstreamError<"ListPluginSources403", typeof ListPluginSources403.Type>>
  /**
* Adding a source is a trust decision: its entries become installable on this host. They are
* attributed to it in the console and never carry the "verified" badge, which belongs to the
* built-in source alone.
*/
readonly "putPluginSource": <Config extends OperationConfig>(name: string, options: { readonly payload: typeof PutPluginSourceRequestJson.Encoded; readonly config?: Config | undefined }) => Effect.Effect<WithOptionalResponse<void, Config>, HttpClientError.HttpClientError | SchemaError | SlipstreamError<"PutPluginSource400", typeof PutPluginSource400.Type> | SlipstreamError<"PutPluginSource401", typeof PutPluginSource401.Type> | SlipstreamError<"PutPluginSource403", typeof PutPluginSource403.Type>>
  /**
* Remove a catalog source
*/
readonly "deletePluginSource": <Config extends OperationConfig>(name: string, options: { readonly config?: Config | undefined } | undefined) => Effect.Effect<WithOptionalResponse<void, Config>, HttpClientError.HttpClientError | SchemaError | SlipstreamError<"DeletePluginSource401", typeof DeletePluginSource401.Type> | SlipstreamError<"DeletePluginSource403", typeof DeletePluginSource403.Type>>
  /**
* Removes the package and forgets its provenance, then restarts the runner. Only names the runner
* would actually supervise are accepted, so this can't be used to rip a shared dependency out of
* the tree.
*/
readonly "uninstallPlugin": <Config extends OperationConfig>(options: { readonly payload: typeof UninstallPluginRequestJson.Encoded; readonly config?: Config | undefined }) => Effect.Effect<WithOptionalResponse<typeof UninstallPlugin202.Type, Config>, HttpClientError.HttpClientError | SchemaError | SlipstreamError<"UninstallPlugin400", typeof UninstallPlugin400.Type> | SlipstreamError<"UninstallPlugin401", typeof UninstallPlugin401.Type> | SlipstreamError<"UninstallPlugin403", typeof UninstallPlugin403.Type> | SlipstreamError<"UninstallPlugin409", typeof UninstallPlugin409.Type>>
  /**
* Create and persist a redacted support bundle.
*/
readonly "supportBundleCreate": <Config extends OperationConfig>(options: { readonly config?: Config | undefined } | undefined) => Effect.Effect<WithOptionalResponse<typeof SupportBundleCreate200.Type, Config>, HttpClientError.HttpClientError | SchemaError | SlipstreamError<"SupportBundleCreate401", typeof SupportBundleCreate401.Type> | SlipstreamError<"SupportBundleCreate500", typeof SupportBundleCreate500.Type>>
  /**
* Read a previously generated support bundle.
*/
readonly "supportBundleGet": <Config extends OperationConfig>(id: string, options: { readonly config?: Config | undefined } | undefined) => Effect.Effect<WithOptionalResponse<typeof SupportBundleGet200.Type, Config>, HttpClientError.HttpClientError | SchemaError | SlipstreamError<"SupportBundleGet401", typeof SupportBundleGet401.Type> | SlipstreamError<"SupportBundleGet404", typeof SupportBundleGet404.Type> | SlipstreamError<"SupportBundleGet500", typeof SupportBundleGet500.Type>>
  /**
* Delete a locally stored support bundle.
*/
readonly "supportBundleDelete": <Config extends OperationConfig>(id: string, options: { readonly config?: Config | undefined } | undefined) => Effect.Effect<WithOptionalResponse<void, Config>, HttpClientError.HttpClientError | SchemaError | SlipstreamError<"SupportBundleDelete401", typeof SupportBundleDelete401.Type> | SlipstreamError<"SupportBundleDelete404", typeof SupportBundleDelete404.Type>>
  /**
* Starts the one-click apply for Linux install kinds that support it. The request carries no
* version or URL, and the host installs exactly what its verified manifest
* announced. Progress is polled via `GET /update/status` (`job`); the host restarts as part
* of the apply, and the outcome lands in `last_result` after it comes back.
*/
readonly "applyUpdate": <Config extends OperationConfig>(options: { readonly payload: typeof ApplyUpdateRequestJson.Encoded; readonly config?: Config | undefined }) => Effect.Effect<WithOptionalResponse<typeof ApplyUpdate202.Type, Config>, HttpClientError.HttpClientError | SchemaError | SlipstreamError<"ApplyUpdate401", typeof ApplyUpdate401.Type> | SlipstreamError<"ApplyUpdate409", typeof ApplyUpdate409.Type>>
  /**
* Forces a manifest fetch + verification and returns the refreshed state. Rate-limited to
* one forced check per 30 s.
*/
readonly "forceUpdateCheck": <Config extends OperationConfig>(options: { readonly config?: Config | undefined } | undefined) => Effect.Effect<WithOptionalResponse<typeof ForceUpdateCheck200.Type, Config>, HttpClientError.HttpClientError | SchemaError | SlipstreamError<"ForceUpdateCheck401", typeof ForceUpdateCheck401.Type> | SlipstreamError<"ForceUpdateCheck409", typeof ForceUpdateCheck409.Type> | SlipstreamError<"ForceUpdateCheck429", typeof ForceUpdateCheck429.Type>>
  /**
* How this host was installed, which channel it follows, whether a newer release is known,
* and how to update. Reading this may kick a background refresh when the cached check is
* older than 6 h; the response never blocks on the network.
*/
readonly "getUpdateStatus": <Config extends OperationConfig>(options: { readonly config?: Config | undefined } | undefined) => Effect.Effect<WithOptionalResponse<typeof GetUpdateStatus200.Type, Config>, HttpClientError.HttpClientError | SchemaError | SlipstreamError<"GetUpdateStatus401", typeof GetUpdateStatus401.Type>>
}

export interface SlipstreamError<Tag extends string, E> {
  readonly _tag: Tag
  readonly request: HttpClientRequest.HttpClientRequest
  readonly response: HttpClientResponse.HttpClientResponse
  readonly cause: E
}

class SlipstreamErrorImpl extends Data.Error<{
  _tag: string
  cause: any
  request: HttpClientRequest.HttpClientRequest
  response: HttpClientResponse.HttpClientResponse
}> {}

export const SlipstreamError = <Tag extends string, E>(
  tag: Tag,
  cause: E,
  response: HttpClientResponse.HttpClientResponse,
): SlipstreamError<Tag, E> =>
  new SlipstreamErrorImpl({
    _tag: tag,
    cause,
    response,
    request: response.request,
  }) as any

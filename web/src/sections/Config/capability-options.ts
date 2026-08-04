/** Build select options from host capability probes (or safe fallbacks). */

export type CapabilityOption = {
	value: string;
	label: string;
	available: boolean;
	/** Detected/default choice for this host. */
	marked?: boolean;
	title?: string;
};

/** Localized titles/labels for capability option builders. */
export type CapabilityCopy = {
	savedUnavailable: string;
	detectedDefault: string;
	unavailable: string;
	autoDetect: string;
	autoDetectHelp: string;
	headlessOff: string;
	headlessOffHelp: string;
};

type CapabilityRow = {
	id: string;
	label: string;
	available: boolean;
	default?: boolean;
};

const CAPTURE_FALLBACK: CapabilityRow[] = [
	{ id: "auto", label: "Auto", available: true },
	{ id: "portal", label: "XDG Portal", available: false },
	{ id: "kwin", label: "KWin Screencast", available: false },
	{ id: "wlr", label: "wlroots screencopy", available: false },
	{ id: "kms", label: "DRM/KMS primary plane", available: false },
	{ id: "x11", label: "X11", available: false },
	{ id: "nvfbc", label: "NVIDIA NvFBC", available: false },
];

const COMPOSITOR_FALLBACK: CapabilityRow[] = [
	{ id: "kwin", label: "KWin", available: false },
	{ id: "mutter", label: "Mutter", available: false },
	{ id: "wlroots", label: "wlroots / Sway", available: false },
	{ id: "hyprland", label: "Hyprland", available: false },
	{ id: "gamescope", label: "Gamescope", available: false },
];

const HEADLESS_FALLBACK: CapabilityRow[] = [
	{ id: "auto", label: "Auto", available: false },
	{ id: "labwc", label: "labwc (wlroots)", available: false },
	{ id: "krfb", label: "krfb-virtualmonitor", available: false },
	{ id: "gamescope", label: "Gamescope", available: false },
];

function ensureSelected(
	options: CapabilityOption[],
	selected: string,
	fallbackLabel: string,
	copy: CapabilityCopy,
): CapabilityOption[] {
	if (options.some((o) => o.value === selected)) return options;
	if (selected === "") return options;
	return [
		...options,
		{
			value: selected,
			label: fallbackLabel || selected,
			available: false,
			title: copy.savedUnavailable,
		},
	];
}

function fromRows(
	rows: CapabilityRow[],
	selected: string,
	copy: CapabilityCopy,
	opts?: { markDefault?: boolean },
): CapabilityOption[] {
	const options: CapabilityOption[] = rows.map((row) => ({
		value: row.id,
		label: row.label,
		available: row.available,
		marked: opts?.markDefault ? Boolean(row.default) : false,
		title: row.available
			? row.default
				? copy.detectedDefault
				: undefined
			: copy.unavailable,
	}));
	return ensureSelected(options, selected, selected, copy);
}

/** Capture-method options. Query failure uses safe Auto + unavailable pins. */
export function buildCaptureMethodOptions(
	rows: CapabilityRow[] | null | undefined,
	selected: string,
	copy: CapabilityCopy,
): CapabilityOption[] {
	const source =
		rows && rows.length > 0
			? rows
			: CAPTURE_FALLBACK.map((r) => ({
					...r,
					available: r.id === "auto",
				}));
	return fromRows(source, selected || "auto", copy);
}

/**
 * Virtual compositor options. Always includes Auto-detect (empty value).
 * Query failure marks concrete backends unavailable.
 */
export function buildCompositorOptions(
	rows: CapabilityRow[] | null | undefined,
	selected: string,
	copy: CapabilityCopy,
): CapabilityOption[] {
	const auto: CapabilityOption = {
		value: "",
		label: copy.autoDetect,
		available: true,
		marked: !rows?.some((r) => r.default),
		title: copy.autoDetectHelp,
	};
	const source =
		rows && rows.length > 0
			? rows
			: COMPOSITOR_FALLBACK.map((r) => ({ ...r, available: false }));
	const rest = fromRows(source, selected, copy, { markDefault: true }).filter(
		(o) => o.value !== "",
	);
	const withAuto = [auto, ...rest];
	if (selected && !withAuto.some((o) => o.value === selected)) {
		return ensureSelected(withAuto, selected, selected, copy);
	}
	return withAuto;
}

/**
 * Headless compositor options. Always includes Off (null mapped to "off").
 * Query failure keeps Off available and marks spawn backends unavailable.
 */
export function buildHeadlessCompositorOptions(
	rows: CapabilityRow[] | null | undefined,
	selected: string,
	copy: CapabilityCopy,
): CapabilityOption[] {
	const off: CapabilityOption = {
		value: "off",
		label: copy.headlessOff,
		available: true,
		marked: selected === "" || selected === "off",
		title: copy.headlessOffHelp,
	};
	const source =
		rows && rows.length > 0
			? rows
			: HEADLESS_FALLBACK.map((r) => ({ ...r, available: false }));
	const rest = fromRows(source, selected === "off" ? "" : selected, copy).filter(
		(o) => o.value !== "off" && o.value !== "",
	);
	const withOff = [off, ...rest];
	if (selected && selected !== "off" && !withOff.some((o) => o.value === selected)) {
		return ensureSelected(withOff, selected, selected, copy);
	}
	return withOff;
}

export function formatCapabilityOptionLabel(
	option: CapabilityOption,
	marks: { detected: string; unavailable: string },
): string {
	const bits = [option.label];
	if (option.marked) bits.push(`(${marks.detected})`);
	if (!option.available) bits.push(`(${marks.unavailable})`);
	return bits.join(" ");
}

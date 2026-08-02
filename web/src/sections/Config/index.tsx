import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import Section from "@unom/ui/section";
import { toast } from "@unom/ui/toast";
import {
	CheckCircle2,
	CircleAlert,
	Info,
	Save as SaveIcon,
} from "lucide-react";
import {
	type FC,
	type ReactNode,
	type SelectHTMLAttributes,
	useEffect,
	useId,
	useMemo,
	useState,
} from "react";
import {
	getHostConfig,
	type HostConfigFile,
	setHostConfig,
} from "@/api/host-config";
import {
	HelpOption,
	HelpTip,
	RecommendedMark,
} from "@/components/option-help";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
	Card,
	CardContent,
	CardDescription,
	CardHeader,
	CardTitle,
} from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Switch } from "@/components/ui/switch";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { useLocale } from "@/lib/i18n";
import { cn } from "@/lib/utils";
import { m } from "@/paraglide/messages";

const QK = ["host-config"] as const;

const fieldControlClass =
	"h-9 w-full rounded-md border border-input bg-background px-3 text-sm text-foreground shadow-sm transition-colors placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 focus-visible:ring-offset-background disabled:cursor-not-allowed disabled:opacity-50 sm:w-56";

function FieldSelect({
	className,
	children,
	...props
}: SelectHTMLAttributes<HTMLSelectElement>) {
	return (
		<select className={cn(fieldControlClass, className)} {...props}>
			{children}
		</select>
	);
}

function FieldGroup({
	title,
	children,
}: {
	title?: ReactNode;
	children: ReactNode;
}) {
	return (
		<fieldset className="m-0 min-w-0 rounded-xl border border-border/70 bg-muted/15 p-0">
			{title ? (
				<legend className="ml-3 max-w-full px-1 text-[0.68rem] font-semibold uppercase tracking-[0.12em] text-muted-foreground">
					{title}
				</legend>
			) : null}
			<div className={cn("divide-y divide-border/80", title && "-mt-1")}>
				{children}
			</div>
		</fieldset>
	);
}

function Row({
	label,
	hint,
	help,
	recommended,
	htmlFor,
	controlWidth = "md",
	children,
}: {
	label: string;
	hint?: string;
	help?: string;
	recommended?: string;
	htmlFor?: string;
	controlWidth?: "sm" | "md";
	children: ReactNode;
}) {
	const labelId = useId();
	const hintId = useId();
	const recommendedId = useId();
	const describedBy = [hint ? hintId : null, recommended ? recommendedId : null]
		.filter(Boolean)
		.join(" ");

	return (
		<fieldset
			aria-describedby={describedBy || undefined}
			className="m-0 grid gap-2 border-0 px-3 py-3 sm:grid-cols-[minmax(0,1fr)_auto] sm:items-center sm:gap-6 sm:px-4 sm:py-3.5"
		>
			<legend className="sr-only">{label}</legend>
			<div className="min-w-0 space-y-1">
				<div className="flex items-center gap-1.5">
					<Label
						id={labelId}
						htmlFor={htmlFor}
						className="text-sm font-medium leading-snug text-foreground"
					>
						{label}
					</Label>
					{help ? <HelpTip label={label} text={help} /> : null}
				</div>
				{hint ? (
					<p
						id={hintId}
						className="text-xs leading-relaxed text-muted-foreground"
					>
						{hint}
					</p>
				) : null}
				{recommended ? (
					<div id={recommendedId}>
						<RecommendedMark value={recommended} />
					</div>
				) : null}
			</div>
			<div
				className={cn(
					"flex w-full justify-start sm:justify-end",
					controlWidth === "sm" ? "sm:min-w-28" : "sm:min-w-56",
				)}
			>
				{children}
			</div>
		</fieldset>
	);
}

function ConfigCard({
	title,
	description,
	advanced = false,
	children,
}: {
	title: ReactNode;
	description: ReactNode;
	advanced?: boolean;
	children: ReactNode;
}) {
	return (
		<Card className="overflow-hidden">
			<CardHeader className="border-b border-border/50 bg-muted/10">
				<div className="flex items-start justify-between gap-4">
					<div className="min-w-0 space-y-1">
						<CardTitle className="tracking-tight">{title}</CardTitle>
						<CardDescription>{description}</CardDescription>
					</div>
					<Badge
						variant={advanced ? "outline" : "secondary"}
						className="shrink-0"
					>
						{advanced ? "Advanced" : "Basic"}
					</Badge>
				</div>
			</CardHeader>
			<CardContent className="space-y-4">{children}</CardContent>
		</Card>
	);
}

function ConfigTabTrigger({
	value,
	label,
	description,
}: {
	value: string;
	label: ReactNode;
	description: string;
}) {
	return (
		<TabsTrigger
			value={value}
			className="!h-auto min-h-10 flex-1 shrink-0 items-start justify-start whitespace-normal rounded-lg px-3 py-2.5 text-left text-sm leading-snug lg:w-full lg:flex-none"
		>
			<span className="flex min-w-0 flex-col items-start gap-0.5">
				<span className="font-medium leading-tight">{label}</span>
				<span className="hidden text-xs font-normal leading-snug text-muted-foreground lg:block">
					{description}
				</span>
			</span>
		</TabsTrigger>
	);
}

function ToggleRow(props: {
	label: string;
	hint?: string;
	help?: string;
	recommended?: string;
	checked: boolean;
	onChange: (v: boolean) => void;
}) {
	const id = useId();
	return (
		<Row
			label={props.label}
			hint={props.hint}
			help={props.help}
			recommended={props.recommended}
			htmlFor={id}
			controlWidth="sm"
		>
			<Switch
				id={id}
				checked={props.checked}
				onCheckedChange={props.onChange}
				className="focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 focus-visible:ring-offset-background"
			/>
		</Row>
	);
}

export const SectionConfig: FC = () => {
	useLocale();
	const qc = useQueryClient();
	const q = useQuery({
		queryKey: QK,
		queryFn: getHostConfig,
		staleTime: 5_000,
	});
	const [draft, setDraft] = useState<HostConfigFile | null>(null);

	useEffect(() => {
		if (q.data) setDraft(structuredClone(q.data.settings));
	}, [q.data]);

	const dirty = useMemo(() => {
		if (!q.data || !draft) return false;
		return JSON.stringify(draft) !== JSON.stringify(q.data.settings);
	}, [draft, q.data]);

	const save = useMutation({
		mutationFn: setHostConfig,
		onSuccess: (state) => {
			qc.setQueryData(QK, state);
			setDraft(structuredClone(state.settings));
			toast.success("Configuration saved. Restart the host to apply.");
		},
		onError: (e: Error) => {
			toast.error(e.message || "Could not save configuration.");
		},
	});

	if (q.isError) {
		return (
			<Section maxWidth={false}>
				<div className="flex flex-col gap-4 rounded-xl border border-destructive/40 bg-destructive/5 p-4 sm:p-5">
					<div className="flex items-start gap-3" role="alert">
						<CircleAlert
							className="mt-0.5 size-5 shrink-0 text-destructive"
							aria-hidden="true"
						/>
						<div className="min-w-0 space-y-1">
							<h1 className="text-lg font-semibold tracking-tight">
								{m.display_config_title()}
							</h1>
							<p className="text-sm font-medium text-destructive">
								{m.common_error()}
							</p>
							<p className="text-sm leading-relaxed text-muted-foreground">
								Could not load host configuration. Is the host management API
								up? This page needs slipstream-host with GET /api/v1/host/config
								(0.23+).
							</p>
						</div>
					</div>
					<div>
						<Button variant="outline" onClick={() => q.refetch()}>
							{m.common_retry()}
						</Button>
					</div>
				</div>
			</Section>
		);
	}

	if (q.isLoading || !draft) {
		return (
			<Section maxWidth={false}>
				<div
					role="status"
					aria-live="polite"
					className="flex items-center gap-2 text-sm text-muted-foreground"
				>
					<span
						className="size-2 animate-pulse rounded-full bg-primary"
						aria-hidden="true"
					/>
					{m.common_loading()}
				</div>
			</Section>
		);
	}

	const patch = (fn: (d: HostConfigFile) => void) => {
		setDraft((prev) => {
			if (!prev) return prev;
			const next = structuredClone(prev);
			fn(next);
			return next;
		});
	};

	const onSave = () => save.mutate(draft);

	return (
		<Section maxWidth={false}>
			<div className={cn("flex flex-col gap-card", dirty && "pb-20 sm:pb-0")}>
					<header className="flex flex-col gap-5 rounded-xl border border-border/70 bg-card/80 p-4 shadow-sm sm:p-5 lg:flex-row lg:items-start lg:justify-between lg:gap-8">
						<div className="min-w-0 space-y-2">
							<p className="text-xs font-semibold uppercase tracking-[0.14em] text-muted-foreground">
								{m.nav_host()}
							</p>
							<h1 className="text-3xl font-semibold tracking-tight">
								{m.display_config_title()}
							</h1>
							<p className="max-w-2xl text-sm leading-relaxed text-muted-foreground">
								Manage the host's capture, input, network, and encoder defaults
								from one workspace. Hover the help icons for what each option
								does.
							</p>
						</div>
						<div className="flex w-full flex-col gap-2 sm:w-auto sm:items-end">
							<Button
								type="button"
								disabled={!dirty || save.isPending}
								onClick={onSave}
								aria-busy={save.isPending || undefined}
								aria-describedby="config-save-status"
								className="w-full min-w-28 sm:w-auto"
							>
								<SaveIcon className="size-4" aria-hidden="true" />
								{save.isPending ? m.common_loading() : m.display_save()}
							</Button>
							<p
								id="config-save-status"
								role="status"
								aria-live="polite"
								className="text-right text-xs text-muted-foreground"
							>
								{save.isPending
									? m.common_loading()
									: dirty
										? m.display_unsaved_hint()
										: m.display_all_saved()}
							</p>
						</div>
					</header>

					<div
						role="status"
						aria-live="polite"
						aria-busy={save.isPending || undefined}
						className={cn(
							"flex flex-col gap-2 rounded-xl border px-4 py-3 sm:flex-row sm:items-center sm:justify-between",
							dirty
								? "border-warning/40 bg-warning/10"
								: "border-border/70 bg-muted/20",
						)}
					>
						<div className="flex min-w-0 items-start gap-3">
							{dirty ? (
								<CircleAlert
									className="mt-0.5 size-4 shrink-0 text-[var(--warning)]"
									aria-hidden="true"
								/>
							) : (
								<CheckCircle2
									className="mt-0.5 size-4 shrink-0 text-[var(--success)]"
									aria-hidden="true"
								/>
							)}
							<div className="min-w-0 space-y-0.5">
								<p className="text-sm font-medium">
									{dirty ? m.display_unsaved() : m.display_all_saved()}
								</p>
								<p className="min-w-0 break-words text-xs leading-relaxed text-muted-foreground">
									{save.isPending
										? m.common_loading()
										: dirty
											? m.display_unsaved_hint()
											: m.display_all_saved()}
								</p>
							</div>
						</div>
						{save.isPending ? (
							<Badge variant="secondary" className="self-start sm:self-auto">
								{m.common_loading()}
							</Badge>
						) : null}
					</div>

					{save.isError ? (
						<div
							role="alert"
							className="flex items-start gap-3 rounded-lg border border-destructive/40 bg-destructive/5 px-3.5 py-3 text-sm"
						>
							<CircleAlert
								className="mt-0.5 size-4 shrink-0 text-destructive"
								aria-hidden="true"
							/>
							<p className="min-w-0 text-destructive">
								{save.error instanceof Error && save.error.message
									? save.error.message
									: m.common_error()}
							</p>
						</div>
					) : null}

					{q.data?.requires_restart ? (
						<div
							role="status"
							aria-live="polite"
							className="flex items-start gap-3 rounded-lg border border-warning/40 bg-warning/10 px-3.5 py-3 text-sm leading-relaxed text-foreground"
						>
							<Info
								className="mt-0.5 size-4 shrink-0 text-[var(--warning)]"
								aria-hidden="true"
							/>
							<p className="min-w-0 break-words">
								Saved values write to{" "}
								<code className="break-all rounded bg-muted px-1 py-0.5 font-mono text-xs">
									{q.data.env_path}
								</code>
								. Restart{" "}
								<code className="rounded bg-muted px-1 py-0.5 font-mono text-xs">
									slipstream-host
								</code>{" "}
								for them to take effect in the running process.
							</p>
						</div>
					) : null}

					<Tabs defaultValue="general" className="flex flex-col gap-5">
						<div className="grid gap-5 lg:grid-cols-[15rem_minmax(0,1fr)] lg:items-start">
							<aside className="space-y-3">
								<div className="space-y-1 px-1">
									<p className="text-xs font-semibold uppercase tracking-[0.14em] text-muted-foreground">
										Configuration areas
									</p>
									<p className="text-sm leading-relaxed text-muted-foreground">
										Start with the core controls, then tune the advanced capture
										and encoder paths.
									</p>
								</div>
								<div className="overflow-x-auto rounded-xl lg:overflow-visible">
									<TabsList
										aria-label={m.display_config_title()}
										className="inline-flex !h-auto min-h-0 min-w-full w-max items-stretch justify-start gap-1 overflow-visible rounded-xl border border-border/70 bg-muted/70 p-1 lg:flex lg:w-full lg:min-w-0 lg:flex-col"
									>
										<ConfigTabTrigger
											value="general"
											label="General"
											description="Host basics"
										/>
										<ConfigTabTrigger
											value="input"
											label="Input"
											description="Input routing"
										/>
										<ConfigTabTrigger
											value="av"
											label={`${m.status_audio()} / ${m.status_video()}`}
											description="Capture paths"
										/>
										<ConfigTabTrigger
											value="network"
											label="Network"
											description="Discovery and FEC"
										/>
										<ConfigTabTrigger
											value="encoders"
											label="Encoders / GPU"
											description="Encode and GPU"
										/>
									</TabsList>
								</div>
							</aside>

							<div className="min-w-0">
								<TabsContent value="general" className="mt-0 outline-none">
									<ConfigCard
										title="General"
										description="Set the host identity and diagnostic behavior."
									>
										<FieldGroup title={m.host_identity()}>
											<Row
												label={m.host_hostname()}
												hint="Shown in Moonlight and on the LAN."
												help="Friendly name clients use when discovering this host. Leave blank to use this PC's system hostname."
												recommended="Blank (use the system hostname)"
												htmlFor="cfg-host-name"
											>
												<Input
													id="cfg-host-name"
													className={fieldControlClass}
													value={draft.general.host_name ?? ""}
													placeholder="This PC"
													title="Leave blank to use the system hostname."
													onChange={(e) =>
														patch((d) => {
															d.general.host_name = e.target.value || null;
														})
													}
												/>
											</Row>
											<ToggleRow
												label="Performance logging"
												hint="Extra host diagnostics in the log."
												help="Writes per-stage timing into the host log. Useful when chasing stutter or encode delays. Leave off for normal use; it adds noise and a little overhead."
												recommended="Off"
												checked={draft.general.perf}
												onChange={(v) =>
													patch((d) => {
														d.general.perf = v;
													})
												}
											/>
										</FieldGroup>
									</ConfigCard>
								</TabsContent>

								<TabsContent value="input" className="mt-0 outline-none">
									<ConfigCard
										title="Input"
										description="Choose how local input is exposed to streaming sessions."
									>
										<FieldGroup title="Input routing">
											<Row
												label="Gamepad backend"
												hint="Leave blank for auto."
												help="Which virtual gamepad stack the host uses for client controllers. Blank lets Slipstream pick the best backend for this OS."
												recommended="Blank / auto"
												htmlFor="cfg-gamepad"
											>
												<Input
													id="cfg-gamepad"
													className={fieldControlClass}
													value={draft.input.gamepad ?? ""}
													placeholder="auto"
													title="Leave blank for automatic gamepad backend selection."
													onChange={(e) =>
														patch((d) => {
															d.input.gamepad = e.target.value || null;
														})
													}
												/>
											</Row>
											<ToggleRow
												label="Gamescope grab cursor"
												hint="Force relative mouse capture in bare gamescope launches."
												help="Adds --force-grab-cursor so FPS mouselook works over the injected pointer. Can break absolute-pointer menus and some desktop apps, so leave it off unless a title needs it."
												recommended="Off"
												checked={draft.input.gamescope_grab_cursor}
												onChange={(v) =>
													patch((d) => {
														d.input.gamescope_grab_cursor = v;
													})
												}
											/>
										</FieldGroup>
									</ConfigCard>
								</TabsContent>

								<TabsContent value="av" className="mt-0 outline-none">
									<ConfigCard
										title={`${m.status_audio()} / ${m.status_video()}`}
										description="Choose the display pipeline, then tune stream output. Linux resolves the compositor and capture backend together."
										advanced
									>
										<FieldGroup title="Capture and display">
											<Row
												label="Video source"
												hint="Where the stream picture comes from."
												help="Virtual display creates a per-client output at the client's resolution. Portal / PipeWire mirrors an existing desktop instead."
												recommended="Virtual display"
												htmlFor="cfg-video-source"
											>
												<FieldSelect
													id="cfg-video-source"
													value={draft.audio_video.video_source ?? ""}
													onChange={(e) =>
														patch((d) => {
															d.audio_video.video_source =
																e.target.value || null;
														})
													}
												>
													<HelpOption
														value=""
														title="Use the host default (virtual display)."
													>
														Default
													</HelpOption>
													<HelpOption
														value="virtual"
														recommended
														title="Create a dedicated virtual display at the client's mode. Best for streaming and multi-client use."
													>
														Virtual display
													</HelpOption>
													<HelpOption
														value="portal"
														title="Capture an existing monitor through XDG Portal / PipeWire. Good for mirroring your current desktop."
													>
														Portal / PipeWire
													</HelpOption>
												</FieldSelect>
											</Row>
											<Row
												label="Capture method"
												hint="How an existing desktop is grabbed."
												help="Only matters on the portal/mirror path. Auto picks the best backend for this desktop. Hermes-KMS is not offered."
												recommended="Auto"
												htmlFor="cfg-capture-method"
											>
												<FieldSelect
													id="cfg-capture-method"
													value={draft.audio_video.capture_method ?? "auto"}
													onChange={(e) =>
														patch((d) => {
															d.audio_video.capture_method =
																e.target.value || null;
														})
													}
												>
													<HelpOption
														value="auto"
														recommended
														title="Let Slipstream choose the capture backend for this desktop."
													>
														Auto
													</HelpOption>
													<HelpOption
														value="portal"
														title="XDG Desktop Portal screencast. Works across many Wayland desktops, may prompt for permission."
													>
														XDG Portal
													</HelpOption>
													<HelpOption
														value="kwin"
														title="KWin Screencast API. Prefer this on Plasma when auto is wrong."
													>
														KWin Screencast
													</HelpOption>
													<HelpOption
														value="wlr"
														title="wlroots screencopy. Prefer this on Sway, Hyprland, and similar."
													>
														wlroots screencopy
													</HelpOption>
													<HelpOption
														value="kms"
														title="DRM/KMS primary-plane dma-buf capture. Best when the display is already scanning out on a usable DRM card."
													>
														DRM/KMS primary plane
													</HelpOption>
													<HelpOption
														value="x11"
														title="X11 capture path for X sessions."
													>
														X11
													</HelpOption>
													<HelpOption
														value="nvfbc"
														title="NVIDIA NvFBC shared-CUDA capture. Requires an X11 display, NVIDIA capture support, and CUDA."
													>
														NVIDIA NvFBC
													</HelpOption>
												</FieldSelect>
											</Row>
											<Row
												label="Virtual compositor"
												hint="Backend for virtual displays (live session)."
												help="Pin which compositor owns virtual outputs. Leave auto-detect unless you are forcing a specific backend for testing."
												recommended="Auto-detect"
												htmlFor="cfg-compositor"
											>
												<FieldSelect
													id="cfg-compositor"
													value={draft.audio_video.compositor ?? ""}
													onChange={(e) =>
														patch((d) => {
															d.audio_video.compositor =
																e.target.value || null;
														})
													}
												>
													<HelpOption
														value=""
														recommended
														title="Detect the running compositor automatically."
													>
														Auto-detect
													</HelpOption>
													<HelpOption
														value="kwin"
														title="Force KWin virtual-output support (Plasma)."
													>
														KWin
													</HelpOption>
													<HelpOption
														value="mutter"
														title="Force Mutter virtual-output support (GNOME)."
													>
														Mutter
													</HelpOption>
													<HelpOption
														value="wlroots"
														title="Force wlroots / Sway virtual-output support."
													>
														wlroots / Sway
													</HelpOption>
													<HelpOption
														value="hyprland"
														title="Force Hyprland virtual-output support."
													>
														Hyprland
													</HelpOption>
													<HelpOption
														value="gamescope"
														title="Use gamescope as the virtual display / nested session backend."
													>
														Gamescope
													</HelpOption>
												</FieldSelect>
											</Row>
											<Row
												label="Headless compositor"
												hint="Spawn a private Wayland session when none is live."
												help="For boxes with no logged-in desktop. Off is right for a normal interactive PC. Auto/labwc/gamescope can spawn a private session on headless hosts."
												recommended="Off (interactive desktop)"
												htmlFor="cfg-headless-compositor"
											>
												<FieldSelect
													id="cfg-headless-compositor"
													value={
														draft.audio_video.headless_compositor ?? "off"
													}
													onChange={(e) =>
														patch((d) => {
															d.audio_video.headless_compositor =
																e.target.value === "off"
																	? null
																	: e.target.value || null;
														})
													}
												>
													<HelpOption
														value="off"
														recommended
														title="Do not spawn a private compositor. Use this when a desktop session is already running."
													>
														Off
													</HelpOption>
													<HelpOption
														value="auto"
														title="Pick a headless compositor automatically when no session is live."
													>
														Auto
													</HelpOption>
													<HelpOption
														value="labwc"
														title="Spawn labwc (wlroots) as a private Wayland session."
													>
														labwc (wlroots)
													</HelpOption>
													<HelpOption
														value="krfb"
														title="Spawn krfb-virtualmonitor for a private virtual head."
													>
														krfb-virtualmonitor
													</HelpOption>
													<HelpOption
														value="gamescope"
														title="Spawn a private gamescope nested session."
													>
														Gamescope
													</HelpOption>
												</FieldSelect>
											</Row>
										</FieldGroup>

										<FieldGroup title="Stream preferences">
											<Row
												label="Max FPS"
												hint="Blank = no host-side game cap."
												help="Caps how fast the game may render through the compositor. It does not reduce the client's negotiated stream rate. Leave blank unless you want to free GPU time."
												recommended="Blank (no cap)"
												htmlFor="cfg-max-fps"
												controlWidth="sm"
											>
												<Input
													id="cfg-max-fps"
													className={cn(fieldControlClass, "sm:w-28")}
													type="number"
													min={15}
													max={480}
													value={draft.audio_video.max_fps ?? ""}
													title="Leave blank for no host-side FPS cap."
													onChange={(e) =>
														patch((d) => {
															const n = Number(e.target.value);
															d.audio_video.max_fps = e.target.value
																? n
																: null;
														})
													}
												/>
											</Row>
											<Row
												label="PipeWire latency hint"
												hint="1 to 40 ms, blank = 8 ms."
												help="Requests a small scheduling quantum from the Linux PipeWire capture node. The compositor or driver may choose a larger value."
												recommended="8 ms"
												htmlFor="cfg-pipewire-latency"
												controlWidth="sm"
											>
												<Input
													id="cfg-pipewire-latency"
													className={cn(fieldControlClass, "sm:w-28")}
													type="number"
													min={1}
													max={40}
													value={draft.audio_video.pipewire_latency_ms ?? ""}
													onChange={(e) =>
														patch((d) => {
															const n = Number(e.target.value);
															d.audio_video.pipewire_latency_ms = e.target.value
																? n
																: null;
														})
													}
												/>
											</Row>
											<Row
												label="Capture age warning"
												hint="1 to 500 ms, blank = 50 ms."
												help="Marks stats samples when the newest source frame is older than this threshold. It is diagnostic only and does not drop frames."
												recommended="50 ms"
												htmlFor="cfg-capture-max-age"
												controlWidth="sm"
											>
												<Input
													id="cfg-capture-max-age"
													className={cn(fieldControlClass, "sm:w-28")}
													type="number"
													min={1}
													max={500}
													value={draft.audio_video.capture_max_age_ms ?? ""}
													onChange={(e) =>
														patch((d) => {
															const n = Number(e.target.value);
															d.audio_video.capture_max_age_ms = e.target.value
																? n
																: null;
														})
													}
												/>
											</Row>
											<ToggleRow
												label="Prefer 10-bit"
												hint="Allow HDR / Main10 when the client asks."
												help="Host policy gate for 10-bit encode. A session still needs client support and a capable GPU. Leave on; turn off only if a client or GPU path misbehaves."
												recommended="On"
												checked={draft.audio_video.ten_bit}
												onChange={(v) =>
													patch((d) => {
														d.audio_video.ten_bit = v;
													})
												}
											/>
											<ToggleRow
												label="Prefer 4:4:4"
												hint="Allow full-chroma when the client asks."
												help="Host policy gate for HEVC 4:4:4. Useful for sharp text/UI. Client must opt in; leave on unless encode probes fail."
												recommended="On"
												checked={draft.audio_video.four_four_four}
												onChange={(v) =>
													patch((d) => {
														d.audio_video.four_four_four = v;
													})
												}
											/>
											<ToggleRow
												label="Gamescope HDR"
												hint="Allow HDR sessions on the gamescope backend."
												help="Attempts HDR only when slipstream-gamescope and client caps support it. Stock gamescope stays SDR either way. Leave on; set off as an escape hatch."
												recommended="On"
												checked={draft.audio_video.gamescope_hdr}
												onChange={(v) =>
													patch((d) => {
														d.audio_video.gamescope_hdr = v;
													})
												}
											/>
										</FieldGroup>
									</ConfigCard>
								</TabsContent>

								<TabsContent value="network" className="mt-0 outline-none">
									<ConfigCard
										title="Network"
										description="Tune discovery and packet recovery for this host."
										advanced
									>
										<FieldGroup title="Connectivity">
											<ToggleRow
												label="mDNS discovery"
												hint="Advertise this host on the LAN."
												help="Publishes the host so Moonlight and Slipstream clients can find it without typing an IP. Turn off only on multicast-dead networks."
												recommended="On"
												checked={draft.network.mdns}
												onChange={(v) =>
													patch((d) => {
														d.network.mdns = v;
													})
												}
											/>
											<ToggleRow
												label="Prefer ChaCha20"
												hint="Better on soft-AES clients (some TVs)."
												help="Allows ChaCha20-Poly1305 when a soft-AES client asks for it. AES-GCM stays the default for everyone else. Leave on."
												recommended="On"
												checked={draft.network.chacha20}
												onChange={(v) =>
													patch((d) => {
														d.network.chacha20 = v;
													})
												}
											/>
											<Row
												label="FEC %"
												hint="Native plane packet recovery."
												help="Forward error correction for the native Slipstream plane. Higher values survive lossy Wi-Fi better but cost bitrate. Leave blank for the host default."
												recommended="Blank (host default)"
												htmlFor="cfg-fec"
												controlWidth="sm"
											>
												<Input
													id="cfg-fec"
													className={cn(fieldControlClass, "sm:w-28")}
													type="number"
													min={0}
													max={90}
													value={draft.network.fec_pct ?? ""}
													title="Leave blank to use the host default FEC percentage."
													onChange={(e) =>
														patch((d) => {
															d.network.fec_pct = e.target.value
																? Number(e.target.value)
																: null;
														})
													}
												/>
											</Row>
										</FieldGroup>
									</ConfigCard>
								</TabsContent>

								<TabsContent value="encoders" className="mt-0 outline-none">
									<ConfigCard
										title="Encoders / GPU"
										description="Select the encoding backend and rendering adapter."
										advanced
									>
										<FieldGroup title="Encoder path">
											<Row
												label="Encoder"
												hint="Which encode backend to prefer."
												help="Auto picks from the installed GPU stack. Pin a vendor only when auto lands on the wrong path."
												recommended="Auto"
												htmlFor="cfg-encoder"
											>
												<FieldSelect
													id="cfg-encoder"
													value={draft.encoders.encoder}
													onChange={(e) =>
														patch((d) => {
															d.encoders.encoder = e.target.value;
														})
													}
												>
													<HelpOption
														value="auto"
														recommended
														title="Detect the best encoder for this GPU."
													>
														Auto
													</HelpOption>
													<HelpOption
														value="nvenc"
														title="NVIDIA NVENC hardware encoder."
													>
														NVENC
													</HelpOption>
													<HelpOption
														value="amf"
														title="AMD AMF hardware encoder."
													>
														AMF
													</HelpOption>
													<HelpOption
														value="qsv"
														title="Intel Quick Sync Video encoder."
													>
														QSV
													</HelpOption>
													<HelpOption
														value="vaapi"
														title="Linux VAAPI encoder path."
													>
														VAAPI
													</HelpOption>
													<HelpOption
														value="software"
														title="CPU encoder. Slow fallback for debugging only."
													>
														Software
													</HelpOption>
												</FieldSelect>
											</Row>
											<Row
												label="Render adapter"
												hint="Substring match. Blank = auto."
												help="Pins the render GPU by matching part of its name, for example NVIDIA or AMD. Leave blank unless this PC has multiple GPUs and auto picks the wrong one."
												recommended="Blank (auto)"
												htmlFor="cfg-render-adapter"
											>
												<Input
													id="cfg-render-adapter"
													className={fieldControlClass}
													value={draft.encoders.render_adapter ?? ""}
													title="Leave blank to auto-select the render adapter."
													placeholder="e.g. NVIDIA"
													onChange={(e) =>
														patch((d) => {
															d.encoders.render_adapter =
																e.target.value || null;
														})
													}
												/>
											</Row>
											<Row
												label="Zero-copy"
												hint="Unset uses the vendor default."
												help="Keeps frames on the GPU from capture into encode when possible. Vendor default is safest; force On/Off only when debugging a specific stack."
												recommended="Vendor default"
												htmlFor="cfg-zerocopy"
											>
												<FieldSelect
													id="cfg-zerocopy"
													value={
														draft.encoders.zerocopy === null ||
														draft.encoders.zerocopy === undefined
															? ""
															: draft.encoders.zerocopy
																? "1"
																: "0"
													}
													onChange={(e) =>
														patch((d) => {
															d.encoders.zerocopy =
																e.target.value === ""
																	? null
																	: e.target.value === "1";
														})
													}
												>
													<HelpOption
														value=""
														recommended
														title="Use the per-vendor default (safest)."
													>
														Vendor default
													</HelpOption>
													<HelpOption
														value="1"
														title="Force zero-copy on for this host."
													>
														On
													</HelpOption>
													<HelpOption
														value="0"
														title="Force zero-copy off and fall back to a CPU path."
													>
														Off
													</HelpOption>
												</FieldSelect>
											</Row>
										</FieldGroup>
									</ConfigCard>
								</TabsContent>
							</div>
						</div>
					</Tabs>
				</div>

				{dirty ? (
					<div className="fixed inset-x-0 bottom-0 z-40 border-t border-border bg-card/95 p-3 backdrop-blur sm:hidden">
						<div className="flex items-center justify-between gap-3">
							<Badge variant="warning" className="shrink-0">
								{m.display_unsaved()}
							</Badge>
							<Button
								type="button"
								disabled={save.isPending}
								onClick={onSave}
								className="min-w-20"
							>
								<SaveIcon className="size-4" aria-hidden="true" />
								{save.isPending ? m.common_loading() : m.display_save()}
							</Button>
						</div>
					</div>
				) : null}
		</Section>
	);
};

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
	htmlFor,
	controlWidth = "md",
	children,
}: {
	label: string;
	hint?: string;
	htmlFor?: string;
	controlWidth?: "sm" | "md";
	children: ReactNode;
}) {
	const labelId = useId();
	const hintId = useId();

	return (
		<fieldset
			aria-describedby={hint ? hintId : undefined}
			className="m-0 grid gap-2 border-0 px-3 py-3 sm:grid-cols-[minmax(0,1fr)_auto] sm:items-center sm:gap-6 sm:px-4 sm:py-3.5"
		>
			<legend className="sr-only">{label}</legend>
			<div className="min-w-0 space-y-0.5">
				<Label
					id={labelId}
					htmlFor={htmlFor}
					className="text-sm font-medium leading-snug text-foreground"
				>
					{label}
				</Label>
				{hint ? (
					<p
						id={hintId}
						className="text-xs leading-relaxed text-muted-foreground"
					>
						{hint}
					</p>
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
			className="min-h-10 flex-1 justify-start rounded-lg px-3 py-2 text-left text-sm lg:w-full lg:flex-none"
		>
			<span className="flex min-w-0 flex-col items-start gap-0.5">
				<span className="font-medium">{label}</span>
				<span className="hidden text-xs font-normal text-muted-foreground lg:block">
					{description}
				</span>
			</span>
		</TabsTrigger>
	);
}

function ToggleRow(props: {
	label: string;
	hint?: string;
	checked: boolean;
	onChange: (v: boolean) => void;
}) {
	const id = useId();
	return (
		<Row label={props.label} hint={props.hint} htmlFor={id} controlWidth="sm">
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
							from one workspace.
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
							<p className="text-xs leading-relaxed text-muted-foreground">
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
						<p>
							Saved values write to{" "}
							<code className="rounded bg-muted px-1 py-0.5 font-mono text-xs">
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
							<div className="overflow-x-auto rounded-xl">
								<TabsList
									aria-label={m.display_config_title()}
									className="inline-flex h-auto min-w-full w-max items-stretch justify-start gap-1 rounded-xl border border-border/70 bg-muted/70 p-1 lg:flex lg:w-full lg:min-w-0 lg:flex-col"
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
											htmlFor="cfg-host-name"
										>
											<Input
												id="cfg-host-name"
												className={fieldControlClass}
												value={draft.general.host_name ?? ""}
												placeholder="This PC"
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
											htmlFor="cfg-gamepad"
										>
											<Input
												id="cfg-gamepad"
												className={fieldControlClass}
												value={draft.input.gamepad ?? ""}
												placeholder="auto"
												onChange={(e) =>
													patch((d) => {
														d.input.gamepad = e.target.value || null;
													})
												}
											/>
										</Row>
										<ToggleRow
											label="Gamescope grab cursor"
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
									description="Choose capture and compositor paths, then tune stream output."
									advanced
								>
									<FieldGroup title="Capture and display">
										<Row
											label="Video source"
											hint="virtual or portal"
											htmlFor="cfg-video-source"
										>
											<FieldSelect
												id="cfg-video-source"
												value={draft.audio_video.video_source ?? ""}
												onChange={(e) =>
													patch((d) => {
														d.audio_video.video_source = e.target.value || null;
													})
												}
											>
												<option value="">Default</option>
												<option value="virtual">Virtual display</option>
												<option value="portal">Portal / PipeWire</option>
											</FieldSelect>
										</Row>
										<Row
											label="Capture method"
											hint="How an existing desktop is grabbed (SolarFlare-shaped). Hermes-KMS is not offered."
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
												<option value="auto">Auto</option>
												<option value="portal">XDG Portal</option>
												<option value="kwin">KWin Screencast</option>
												<option value="wlr">wlroots screencopy</option>
												<option value="kms">KMS</option>
												<option value="x11">X11</option>
												<option value="nvfbc">NvFBC</option>
											</FieldSelect>
										</Row>
										<Row
											label="Virtual compositor"
											hint="Backend for virtual displays (live session)."
											htmlFor="cfg-compositor"
										>
											<FieldSelect
												id="cfg-compositor"
												value={draft.audio_video.compositor ?? ""}
												onChange={(e) =>
													patch((d) => {
														d.audio_video.compositor = e.target.value || null;
													})
												}
											>
												<option value="">Auto-detect</option>
												<option value="kwin">KWin</option>
												<option value="mutter">Mutter</option>
												<option value="wlroots">wlroots / Sway</option>
												<option value="hyprland">Hyprland</option>
												<option value="gamescope">Gamescope</option>
											</FieldSelect>
										</Row>
										<Row
											label="Headless compositor"
											hint="Spawn a private Wayland session when none is live."
											htmlFor="cfg-headless-compositor"
										>
											<FieldSelect
												id="cfg-headless-compositor"
												value={draft.audio_video.headless_compositor ?? "off"}
												onChange={(e) =>
													patch((d) => {
														d.audio_video.headless_compositor =
															e.target.value === "off"
																? null
																: e.target.value || null;
													})
												}
											>
												<option value="off">Off</option>
												<option value="auto">Auto</option>
												<option value="labwc">labwc (wlroots)</option>
												<option value="krfb">krfb-virtualmonitor</option>
												<option value="gamescope">Gamescope</option>
											</FieldSelect>
										</Row>
									</FieldGroup>

									<FieldGroup title="Stream preferences">
										<Row
											label="Max FPS"
											hint="Blank = no host-side cap."
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
												onChange={(e) =>
													patch((d) => {
														const n = Number(e.target.value);
														d.audio_video.max_fps = e.target.value ? n : null;
													})
												}
											/>
										</Row>
										<ToggleRow
											label="Prefer 10-bit"
											checked={draft.audio_video.ten_bit}
											onChange={(v) =>
												patch((d) => {
													d.audio_video.ten_bit = v;
												})
											}
										/>
										<ToggleRow
											label="Prefer 4:4:4"
											checked={draft.audio_video.four_four_four}
											onChange={(v) =>
												patch((d) => {
													d.audio_video.four_four_four = v;
												})
											}
										/>
										<ToggleRow
											label="Gamescope HDR"
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
											checked={draft.network.chacha20}
											onChange={(v) =>
												patch((d) => {
													d.network.chacha20 = v;
												})
											}
										/>
										<Row
											label="FEC %"
											hint="Native plane. Blank = default."
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
										<Row label="Encoder" htmlFor="cfg-encoder">
											<FieldSelect
												id="cfg-encoder"
												value={draft.encoders.encoder}
												onChange={(e) =>
													patch((d) => {
														d.encoders.encoder = e.target.value;
													})
												}
											>
												<option value="auto">Auto</option>
												<option value="nvenc">NVENC</option>
												<option value="amf">AMF</option>
												<option value="qsv">QSV</option>
												<option value="vaapi">VAAPI</option>
												<option value="software">Software</option>
											</FieldSelect>
										</Row>
										<Row
											label="Render adapter"
											hint="Substring match. Blank = auto."
											htmlFor="cfg-render-adapter"
										>
											<Input
												id="cfg-render-adapter"
												className={fieldControlClass}
												value={draft.encoders.render_adapter ?? ""}
												onChange={(e) =>
													patch((d) => {
														d.encoders.render_adapter = e.target.value || null;
													})
												}
											/>
										</Row>
										<Row
											label="Zero-copy"
											hint="Unset uses the vendor default."
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
												<option value="">Vendor default</option>
												<option value="1">On</option>
												<option value="0">Off</option>
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

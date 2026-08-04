import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Link } from "@tanstack/react-router";
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
	captureMethodsQueryKey,
	compositorsQueryKey,
	getCaptureMethods,
	getCompositors,
	getHeadlessCompositors,
	getHostConfig,
	headlessCompositorsQueryKey,
	hostConfigQueryKey,
	restartHost,
	setHostConfig,
	type HostConfigFile,
} from "@/api/host-config";
import {
	HelpOption,
	HelpTip,
	RecommendedMark,
} from "@/components/option-help";
import { Badge } from "@/components/ui/badge";
import { Button, buttonVariants } from "@/components/ui/button";
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
import {
	buildCaptureMethodOptions,
	buildCompositorOptions,
	buildHeadlessCompositorOptions,
	formatCapabilityOptionLabel,
	type CapabilityOption,
} from "./capability-options";
import { ConfigModeToggle, type ConfigMode } from "./ConfigModeToggle";
import { DirtySaveBar } from "./DirtySaveBar";
import {
	clearConfigDraft,
	readConfigDraft,
	restoreConfigDraft,
	writeConfigDraft,
} from "./draft-session";
import { RestartOffer } from "./RestartOffer";

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

function CapabilitySelect({
	id,
	value,
	options,
	loading = false,
	loadingLabel,
	onChange,
	className,
}: {
	id: string;
	value: string;
	options: CapabilityOption[];
	loading?: boolean;
	loadingLabel: string;
	onChange: (value: string) => void;
	className?: string;
}) {
	const marks = {
		detected: m.config_option_detected(),
		unavailable: m.config_option_unavailable(),
	};
	return (
		<FieldSelect
			id={id}
			className={className}
			value={value}
			onChange={(e) => onChange(e.target.value)}
		>
			{loading ? (
				<option value={value}>{loadingLabel}</option>
			) : (
				options.map((option) => (
					<option
						key={option.value || "__auto__"}
						value={option.value}
						disabled={!option.available && option.value !== value}
						title={option.title}
					>
						{formatCapabilityOptionLabel(option, marks)}
					</option>
				))
			)}
		</FieldSelect>
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
						{advanced ? m.config_advanced_badge() : m.config_basic_badge()}
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
	const locale = useLocale();
	const qc = useQueryClient();
	const capabilityCopy = useMemo(
		() => ({
			savedUnavailable: m.config_capability_saved_unavailable(),
			detectedDefault: m.config_capability_detected_default(),
			unavailable: m.config_capability_unavailable(),
			autoDetect: m.config_compositor_auto_detect(),
			autoDetectHelp: m.config_compositor_auto_detect_help(),
			headlessOff: m.config_headless_off(),
			headlessOffHelp: m.config_headless_off_help(),
		}),
		[locale],
	);
	const q = useQuery({
		queryKey: hostConfigQueryKey,
		queryFn: getHostConfig,
		staleTime: 5_000,
	});
	const compositorsQ = useQuery({
		queryKey: compositorsQueryKey,
		queryFn: getCompositors,
		staleTime: 30_000,
		retry: false,
	});
	const captureQ = useQuery({
		queryKey: captureMethodsQueryKey,
		queryFn: getCaptureMethods,
		staleTime: 30_000,
		retry: false,
	});
	const headlessQ = useQuery({
		queryKey: headlessCompositorsQueryKey,
		queryFn: getHeadlessCompositors,
		staleTime: 30_000,
		retry: false,
	});

	const [draft, setDraft] = useState<HostConfigFile | null>(null);
	const [mode, setMode] = useState<ConfigMode>("recommended");
	const [showRestartOffer, setShowRestartOffer] = useState(false);
	const [restartConfirmOpen, setRestartConfirmOpen] = useState(false);
	const [seeded, setSeeded] = useState(false);

	useEffect(() => {
		if (!q.data || seeded) return;
		const saved = readConfigDraft<HostConfigFile>();
		setDraft(restoreConfigDraft(q.data.settings, saved));
		setSeeded(true);
	}, [q.data, seeded]);

	const dirty = useMemo(() => {
		if (!q.data || !draft) return false;
		return JSON.stringify(draft) !== JSON.stringify(q.data.settings);
	}, [draft, q.data]);

	useEffect(() => {
		if (!seeded || !draft || !q.data) return;
		if (dirty) writeConfigDraft(draft);
		else clearConfigDraft();
	}, [draft, dirty, q.data, seeded]);

	// When the server snapshot updates and there is no session draft, stay in sync.
	useEffect(() => {
		if (!q.data || !seeded || dirty) return;
		if (readConfigDraft<HostConfigFile>()) return;
		setDraft(structuredClone(q.data.settings));
	}, [q.data, seeded, dirty]);

	const save = useMutation({
		mutationFn: setHostConfig,
		onSuccess: (state) => {
			qc.setQueryData(hostConfigQueryKey, state);
			setDraft(structuredClone(state.settings));
			clearConfigDraft();
			setShowRestartOffer(true);
			toast.success(m.config_saved());
		},
		onError: (e: Error) => {
			toast.error(e.message || m.config_save_failed());
		},
	});

	const restart = useMutation({
		mutationFn: restartHost,
		onSuccess: () => {
			setRestartConfirmOpen(false);
			setShowRestartOffer(false);
			toast.success(m.config_restart_pending());
		},
	});

	const captureOptions = useMemo(
		() =>
			buildCaptureMethodOptions(
				captureQ.isError ? null : captureQ.data,
				draft?.audio_video.capture_method ?? "auto",
				capabilityCopy,
			),
		[
			captureQ.data,
			captureQ.isError,
			capabilityCopy,
			draft?.audio_video.capture_method,
		],
	);

	const compositorOptions = useMemo(
		() =>
			buildCompositorOptions(
				compositorsQ.isError ? null : compositorsQ.data,
				draft?.audio_video.compositor ?? "",
				capabilityCopy,
			),
		[
			compositorsQ.data,
			compositorsQ.isError,
			capabilityCopy,
			draft?.audio_video.compositor,
		],
	);

	const headlessOptions = useMemo(
		() =>
			buildHeadlessCompositorOptions(
				headlessQ.isError ? null : headlessQ.data,
				draft?.audio_video.headless_compositor ?? "off",
				capabilityCopy,
			),
		[
			headlessQ.data,
			headlessQ.isError,
			capabilityCopy,
			draft?.audio_video.headless_compositor,
		],
	);

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
								{m.config_load_failed()}
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

	const onSave = () => {
		if (!q.data) return;
		const payload = structuredClone(draft);
		// Moonlight broadcast is owned by Host. Keep a current server value in this
		// payload so a stale Configuration draft cannot undo a Host-page change.
		payload.network.gamestream = q.data.settings.network.gamestream;
		save.mutate(payload);
	};

	const onDiscard = () => {
		if (!confirm(m.config_discard_confirm())) return;
		if (!q.data) return;
		clearConfigDraft();
		setDraft(structuredClone(q.data.settings));
		setShowRestartOffer(false);
	};

	const moonlightOn = draft.network.gamestream ?? false;
	const gamescopePathRelevant =
		draft.audio_video.gamescope_hdr &&
		(!draft.audio_video.compositor ||
			draft.audio_video.compositor === "gamescope" ||
			draft.audio_video.headless_compositor === "gamescope");

	return (
		<Section maxWidth={false}>
			<div
				className={cn(
					"flex flex-col gap-card",
					dirty && "pb-[calc(12rem+env(safe-area-inset-bottom,0px))] sm:pb-0",
				)}
			>
				<header className="flex flex-col gap-5 rounded-xl border border-border/70 bg-card/80 p-4 shadow-sm sm:p-5 lg:flex-row lg:items-start lg:justify-between lg:gap-8">
					<div className="min-w-0 space-y-2">
						<p className="text-xs font-semibold uppercase tracking-[0.14em] text-muted-foreground">
							{m.nav_host()}
						</p>
						<h1 className="text-3xl font-semibold tracking-tight">
							{m.display_config_title()}
						</h1>
						<p className="max-w-2xl text-sm leading-relaxed text-muted-foreground">
							{m.config_intro()}
						</p>
					</div>
					<div className="flex w-full flex-col gap-2 sm:w-auto sm:items-end">
						<div className="flex w-full flex-col gap-2 sm:w-auto sm:flex-row">
							<Button
								type="button"
								variant="outline"
								disabled={!dirty || save.isPending}
								onClick={onDiscard}
								className="hidden w-full min-w-28 sm:inline-flex sm:w-auto"
							>
								{m.config_discard()}
							</Button>
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
						</div>
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

				<ConfigModeToggle
					mode={mode}
					label={m.config_mode_label()}
					recommendedLabel={m.config_mode_recommended()}
					allLabel={m.config_mode_all()}
					onChange={setMode}
				/>

				{mode === "all" ? (
					<div
						role="note"
						data-testid="config-all-warning"
						className="flex items-start gap-3 rounded-xl border border-warning/40 bg-warning/10 px-4 py-3 text-sm leading-relaxed"
					>
						<CircleAlert
							className="mt-0.5 size-4 shrink-0 text-[var(--warning)]"
							aria-hidden="true"
						/>
						<p className="min-w-0">{m.config_all_warning()}</p>
					</div>
				) : null}

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
							{m.config_restart_banner({ path: q.data.env_path })}
						</p>
					</div>
				) : null}

				<RestartOffer
					open={showRestartOffer}
					confirmOpen={restartConfirmOpen}
					pending={restart.isPending}
					error={
						restart.isError
							? restart.error instanceof Error
								? restart.error.message
								: m.config_restart_failed()
							: null
					}
					title={m.config_restart_offer_title()}
					body={m.config_restart_offer_body()}
					restartLabel={m.config_restart_now()}
					laterLabel={m.config_restart_later()}
					confirmTitle={m.config_restart_confirm_title()}
					confirmBody={m.config_restart_confirm_body()}
					confirmLabel={m.config_restart_confirm()}
					cancelLabel={m.config_restart_cancel()}
					pendingLabel={m.config_restart_pending()}
					onLater={() => {
						setShowRestartOffer(false);
						setRestartConfirmOpen(false);
						restart.reset();
					}}
					onConfirmOpenChange={(open) => {
						setRestartConfirmOpen(open);
						if (!open) restart.reset();
					}}
					onRestart={() => restart.mutate()}
				/>

				{mode === "recommended" ? (
					<div className="space-y-5" data-testid="config-recommended">
						<ConfigCard
							title={m.config_mode_recommended()}
							description={m.config_intro()}
						>
							<FieldGroup title={m.host_identity()}>
								<Row
									label={m.host_hostname()}
									hint={m.config_host_name_hint()}
									help={m.config_host_name_help()}
									recommended={m.config_host_name_recommended()}
									htmlFor="cfg-rec-host-name"
								>
									<Input
										id="cfg-rec-host-name"
										className={fieldControlClass}
										value={draft.general.host_name ?? ""}
										placeholder={m.config_host_name_placeholder()}
										onChange={(e) =>
											patch((d) => {
												d.general.host_name = e.target.value || null;
											})
										}
									/>
								</Row>
								<Row
									label={m.config_video_source()}
									hint={m.config_video_source_hint()}
									help={m.config_video_source_help()}
									recommended={m.config_video_source_virtual()}
									htmlFor="cfg-rec-video-source"
								>
									<FieldSelect
										id="cfg-rec-video-source"
										value={draft.audio_video.video_source ?? "virtual"}
										onChange={(e) =>
											patch((d) => {
												d.audio_video.video_source = e.target.value || null;
											})
										}
									>
										<HelpOption value="virtual" recommended>
											{m.config_video_source_virtual()}
										</HelpOption>
										<HelpOption value="portal">
											{m.config_video_source_portal()}
										</HelpOption>
									</FieldSelect>
								</Row>
							</FieldGroup>

							<FieldGroup title={m.config_network()}>
								<ToggleRow
									label={m.config_mdns()}
									hint={m.config_mdns_hint()}
									help={m.config_mdns_help()}
									recommended={m.config_on()}
									checked={draft.network.mdns}
									onChange={(v) =>
										patch((d) => {
											d.network.mdns = v;
										})
									}
								/>
								<Row
									label={m.config_moonlight()}
									hint={m.config_moonlight_hint()}
									help={m.config_moonlight_help_readonly()}
								>
									<div className="flex w-full flex-col items-stretch gap-2 sm:w-56 sm:items-end">
										<div className="flex items-center gap-2 text-sm">
											<span
												aria-hidden
												className={cn(
													"size-2 rounded-full",
													moonlightOn
														? "bg-emerald-500"
														: "bg-muted-foreground/40",
												)}
											/>
											<span className="text-muted-foreground">
												{moonlightOn
													? m.config_moonlight_on()
													: m.config_moonlight_off()}
											</span>
										</div>
										<Link
											to="/host"
											className={cn(
												buttonVariants({ variant: "outline" }),
												"w-full justify-center",
											)}
										>
											{m.config_moonlight_open_host()}
										</Link>
									</div>
								</Row>
								<Row
									label={m.config_clipboard()}
									hint={m.config_clipboard_hint()}
									help={m.config_clipboard_hint()}
									recommended={m.config_clipboard_off()}
									htmlFor="cfg-rec-clipboard"
								>
									<FieldSelect
										id="cfg-rec-clipboard"
										value={draft.clipboard}
										onChange={(e) =>
											patch((d) => {
												d.clipboard =
													e.target.value as HostConfigFile["clipboard"];
											})
										}
									>
										<HelpOption value="off" recommended>
											{m.config_clipboard_off()}
										</HelpOption>
										<HelpOption value="text-only">
											{m.config_clipboard_text_only()}
										</HelpOption>
										<HelpOption value="on">
											{m.config_clipboard_on()}
										</HelpOption>
									</FieldSelect>
								</Row>
							</FieldGroup>

							<FieldGroup title={m.config_input()}>
								<ToggleRow
									label={m.config_pen()}
									hint={m.config_pen_hint()}
									help={m.config_pen_help()}
									recommended={m.config_on()}
									checked={draft.input.pen}
									onChange={(v) =>
										patch((d) => {
											d.input.pen = v;
										})
									}
								/>
							</FieldGroup>

							<FieldGroup title={m.config_hdr_audio()}>
								<ToggleRow
									label={m.config_ten_bit()}
									hint={m.config_ten_bit_hint()}
									help={m.config_ten_bit_help()}
									recommended={m.config_on()}
									checked={draft.audio_video.ten_bit}
									onChange={(v) =>
										patch((d) => {
											d.audio_video.ten_bit = v;
										})
									}
								/>
								<ToggleRow
									label={m.config_four_four_four()}
									hint={m.config_four_four_four_hint()}
									help={m.config_four_four_four_help()}
									recommended={m.config_on()}
									checked={draft.audio_video.four_four_four}
									onChange={(v) =>
										patch((d) => {
											d.audio_video.four_four_four = v;
										})
									}
								/>
								<ToggleRow
									label={m.config_gamescope_hdr()}
									hint={m.config_gamescope_hdr_hint()}
									help={m.config_gamescope_hdr_help()}
									recommended={m.config_on()}
									checked={draft.audio_video.gamescope_hdr}
									onChange={(v) =>
										patch((d) => {
											d.audio_video.gamescope_hdr = v;
										})
									}
								/>
								<ToggleRow
									label={m.config_audio_fec()}
									hint={m.config_audio_fec_hint()}
									help={m.config_audio_fec_help()}
									recommended={m.config_on()}
									checked={draft.audio_video.audio_fec ?? true}
									onChange={(v) =>
										patch((d) => {
											d.audio_video.audio_fec = v;
										})
									}
								/>
							</FieldGroup>

							<FieldGroup title={m.config_encoders()}>
								<Row
									label={m.config_encoder()}
									hint={m.config_encoder_hint()}
									help={m.config_encoder_help()}
									recommended={m.config_encoder_auto()}
									htmlFor="cfg-rec-encoder"
								>
									<FieldSelect
										id="cfg-rec-encoder"
										value={draft.encoders.encoder || "auto"}
										onChange={(e) =>
											patch((d) => {
												d.encoders.encoder = e.target.value;
											})
										}
									>
										<HelpOption
											value="auto"
											recommended
											title={m.config_encoder_auto_title()}
										>
											{m.config_encoder_auto()}
										</HelpOption>
										<HelpOption
											value="nvenc"
											title={m.config_encoder_nvenc_title()}
										>
											NVENC
										</HelpOption>
										<HelpOption
											value="amf"
											title={m.config_encoder_amf_title()}
										>
											AMF
										</HelpOption>
										<HelpOption
											value="qsv"
											title={m.config_encoder_qsv_title()}
										>
											QSV
										</HelpOption>
										<HelpOption
											value="vaapi"
											title={m.config_encoder_vaapi_title()}
										>
											VAAPI
										</HelpOption>
										<HelpOption
											value="software"
											title={m.config_encoder_software_title()}
										>
											{m.config_encoder_software()}
										</HelpOption>
									</FieldSelect>
								</Row>
							</FieldGroup>
						</ConfigCard>
						<ConfigCard
							title={m.config_stream_profiles()}
							description={m.config_stream_profiles_hint()}
						>
							<FieldGroup title={m.config_stream_profiles()}>
								<Row
									label={m.config_performance_profile()}
									hint={m.config_performance_profile_hint()}
									help={m.config_profile_low_latency_help()}
									recommended={m.config_profile_balanced()}
									htmlFor="cfg-rec-performance-profile"
								>
									<FieldSelect
										id="cfg-rec-performance-profile"
										value={draft.performance_profile}
										onChange={(e) =>
											patch((d) => {
												d.performance_profile =
													e.target.value as HostConfigFile["performance_profile"];
											})
										}
									>
										<HelpOption
											value="balanced"
											recommended
											title={m.config_profile_balanced_help()}
										>
											{m.config_profile_balanced()}
										</HelpOption>
										<HelpOption
											value="low_latency"
											title={m.config_profile_low_latency_help()}
										>
											{m.config_profile_low_latency()}
										</HelpOption>
									</FieldSelect>
								</Row>
								<Row
									label={m.config_latency_profile()}
									hint={m.config_latency_profile_hint()}
									help={m.config_profile_low_latency_help()}
									recommended={m.config_profile_balanced()}
									htmlFor="cfg-rec-latency-profile"
								>
									<FieldSelect
										id="cfg-rec-latency-profile"
										value={draft.latency_profile}
										onChange={(e) =>
											patch((d) => {
												d.latency_profile =
													e.target.value as HostConfigFile["latency_profile"];
											})
										}
									>
										<HelpOption
											value="balanced"
											recommended
											title={m.config_profile_balanced_help()}
										>
											{m.config_profile_balanced()}
										</HelpOption>
										<HelpOption
											value="low_latency"
											title={m.config_profile_low_latency_help()}
										>
											{m.config_profile_low_latency()}
										</HelpOption>
									</FieldSelect>
								</Row>
								<Row
									label={m.config_network_policy()}
									hint={m.config_network_policy_hint()}
									help={m.config_profile_auto_help()}
									recommended={m.config_profile_auto()}
									htmlFor="cfg-rec-network-policy"
								>
									<FieldSelect
										id="cfg-rec-network-policy"
										value={draft.network_policy}
										onChange={(e) =>
											patch((d) => {
												d.network_policy =
													e.target.value as HostConfigFile["network_policy"];
											})
										}
									>
										<HelpOption
											value="auto"
											recommended
											title={m.config_profile_auto_help()}
										>
											{m.config_profile_auto()}
										</HelpOption>
										<HelpOption
											value="lan"
											title={m.config_profile_lan_help()}
										>
											{m.config_profile_lan()}
										</HelpOption>
										<HelpOption
											value="wan"
											title={m.config_profile_wan_help()}
										>
											{m.config_profile_wan()}
										</HelpOption>
									</FieldSelect>
								</Row>
							</FieldGroup>
						</ConfigCard>
					</div>
				) : (
					<Tabs
						defaultValue="general"
						className="flex flex-col gap-5"
						data-testid="config-all-settings"
					>
						<div className="grid gap-5 lg:grid-cols-[15rem_minmax(0,1fr)] lg:items-start">
							<aside className="space-y-3">
								<div className="space-y-1 px-1">
									<p className="text-xs font-semibold uppercase tracking-[0.14em] text-muted-foreground">
										{m.config_areas()}
									</p>
									<p className="text-sm leading-relaxed text-muted-foreground">
										{m.config_areas_hint()}
									</p>
								</div>
								<div className="overflow-x-auto rounded-xl lg:overflow-visible">
									<TabsList
										aria-label={m.display_config_title()}
										className="inline-flex !h-auto min-h-0 min-w-full w-max items-stretch justify-start gap-1 overflow-visible rounded-xl border border-border/70 bg-muted/70 p-1 lg:flex lg:w-full lg:min-w-0 lg:flex-col"
									>
										<ConfigTabTrigger
											value="general"
											label={m.config_general()}
											description={m.config_general_desc()}
										/>
										<ConfigTabTrigger
											value="input"
											label={m.config_input()}
											description={m.config_input_desc()}
										/>
										<ConfigTabTrigger
											value="av"
											label={m.config_av_title()}
											description={m.config_av_tab_desc()}
										/>
										<ConfigTabTrigger
											value="network"
											label={m.config_network()}
											description={m.config_network_desc()}
										/>
										<ConfigTabTrigger
											value="encoders"
											label={m.config_encoders()}
											description={m.config_encoders_desc()}
										/>
									</TabsList>
								</div>
							</aside>

							<div className="min-w-0">
								<TabsContent value="general" className="mt-0 outline-none">
									<ConfigCard
										title={m.config_general()}
										description={m.config_general_card_desc()}
									>
										<FieldGroup title={m.host_identity()}>
											<Row
												label={m.host_hostname()}
												hint={m.config_host_name_hint()}
												help={m.config_host_name_help()}
												recommended={m.config_host_name_recommended()}
												htmlFor="cfg-host-name"
											>
												<Input
													id="cfg-host-name"
													className={fieldControlClass}
													value={draft.general.host_name ?? ""}
													placeholder={m.config_host_name_placeholder()}
													title={m.config_host_name_title()}
													onChange={(e) =>
														patch((d) => {
															d.general.host_name = e.target.value || null;
														})
													}
												/>
											</Row>
											<ToggleRow
												label={m.config_perf()}
												hint={m.config_perf_hint()}
												help={m.config_perf_help()}
												recommended={m.config_off()}
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
										title={m.config_input()}
										description={m.config_input_card_desc()}
									>
										<FieldGroup title={m.config_input_routing_group()}>
											<Row
												label={m.config_gamepad()}
												hint={m.config_gamepad_hint()}
												help={m.config_gamepad_help()}
												recommended={m.config_gamepad_recommended()}
												htmlFor="cfg-gamepad"
											>
												<Input
													id="cfg-gamepad"
													className={fieldControlClass}
													value={draft.input.gamepad ?? ""}
													placeholder={m.config_gamepad_placeholder()}
													title={m.config_gamepad_title()}
													onChange={(e) =>
														patch((d) => {
															d.input.gamepad = e.target.value || null;
														})
													}
												/>
											</Row>
												<ToggleRow
													label={m.config_pen()}
													hint={m.config_pen_hint()}
													help={m.config_pen_help()}
													recommended={m.config_on()}
													checked={draft.input.pen}
													onChange={(v) =>
														patch((d) => {
															d.input.pen = v;
														})
													}
												/>
											<ToggleRow
												label={m.config_gamescope_grab()}
												hint={m.config_gamescope_grab_hint()}
												help={m.config_gamescope_grab_help()}
												recommended={m.config_off()}
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
										title={m.config_av_title()}
										description={m.config_av_card_desc()}
										advanced
									>
										<FieldGroup title={m.config_capture_display_group()}>
											<Row
												label={m.config_video_source()}
												hint={m.config_video_source_hint()}
												help={m.config_video_source_help()}
												recommended={m.config_video_source_virtual()}
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
														title={m.config_video_source_default_title()}
													>
														{m.config_video_source_default()}
													</HelpOption>
													<HelpOption
														value="virtual"
														recommended
														title={m.config_video_source_virtual_title()}
													>
														{m.config_video_source_virtual()}
													</HelpOption>
													<HelpOption
														value="portal"
														title={m.config_video_source_portal_title()}
													>
														{m.config_video_source_portal()}
													</HelpOption>
												</FieldSelect>
											</Row>
											<Row
												label={m.config_capture_method()}
												hint={m.config_capture_method_hint()}
												help={m.config_capture_method_help()}
												recommended={m.config_profile_auto()}
												htmlFor="cfg-capture-method"
											>
												<CapabilitySelect
													id="cfg-capture-method"
													value={draft.audio_video.capture_method ?? "auto"}
													options={captureOptions}
													loading={captureQ.isPending && !captureQ.data}
													loadingLabel={m.common_loading()}
													onChange={(value) =>
														patch((d) => {
															d.audio_video.capture_method = value || null;
														})
													}
												/>
											</Row>
											<Row
												label={m.config_virtual_compositor()}
												hint={m.config_virtual_compositor_hint()}
												help={m.config_virtual_compositor_help()}
												recommended={m.config_compositor_auto_detect()}
												htmlFor="cfg-compositor"
											>
												<CapabilitySelect
													id="cfg-compositor"
													value={draft.audio_video.compositor ?? ""}
													options={compositorOptions}
													loading={
														compositorsQ.isPending && !compositorsQ.data
													}
													loadingLabel={m.common_loading()}
													onChange={(value) =>
														patch((d) => {
															d.audio_video.compositor = value || null;
														})
													}
												/>
											</Row>
											<Row
												label={m.config_headless()}
												hint={m.config_headless_hint()}
												help={m.config_headless_help()}
												recommended={m.config_headless_recommended()}
												htmlFor="cfg-headless-compositor"
											>
												<CapabilitySelect
													id="cfg-headless-compositor"
													value={
														draft.audio_video.headless_compositor ?? "off"
													}
													options={headlessOptions}
													loading={headlessQ.isPending && !headlessQ.data}
													loadingLabel={m.common_loading()}
													onChange={(value) =>
														patch((d) => {
															d.audio_video.headless_compositor =
																value === "off" ? null : value || null;
														})
													}
												/>
											</Row>
										</FieldGroup>

										<FieldGroup title={m.config_stream_prefs_group()}>
											<Row
												label={m.config_max_fps()}
												hint={m.config_max_fps_hint()}
												help={m.config_max_fps_help()}
												recommended={m.config_max_fps_recommended()}
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
													title={m.config_max_fps_title()}
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
												label={m.config_pipewire_latency()}
												hint={m.config_pipewire_latency_hint()}
												help={m.config_pipewire_latency_help()}
												recommended={m.config_pipewire_latency_recommended()}
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
															d.audio_video.pipewire_latency_ms = e.target
																.value
																? n
																: null;
														})
													}
												/>
											</Row>
											<Row
												label={m.config_capture_age()}
												hint={m.config_capture_age_hint()}
												help={m.config_capture_age_help()}
												recommended={m.config_capture_age_recommended()}
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
															d.audio_video.capture_max_age_ms = e.target
																.value
																? n
																: null;
														})
													}
												/>
											</Row>
										</FieldGroup>

										<FieldGroup title={m.config_audio_group()}>
											<ToggleRow
												label={m.config_audio_fec()}
												hint={m.config_audio_fec_hint()}
												help={m.config_audio_fec_help_advanced()}
												recommended={m.config_on()}
												checked={draft.audio_video.audio_fec ?? true}
												onChange={(v) =>
													patch((d) => {
														d.audio_video.audio_fec = v;
													})
												}
											/>
											<Row
												label={m.config_audio_gain()}
												hint={m.config_audio_gain_hint()}
												help={m.config_audio_gain_help()}
												recommended={m.config_audio_gain_recommended()}
												htmlFor="cfg-audio-gain"
												controlWidth="sm"
											>
												<Input
													id="cfg-audio-gain"
													className={cn(fieldControlClass, "sm:w-28")}
													type="number"
													min={0}
													max={4}
													step={0.1}
													value={draft.audio_video.audio_gain ?? ""}
													onChange={(e) =>
														patch((d) => {
															const n = Number(e.target.value);
															d.audio_video.audio_gain = e.target.value
																? n
																: null;
														})
													}
												/>
											</Row>
											<Row
												label={m.config_audio_capture()}
												hint={m.config_audio_capture_hint()}
												help={m.config_audio_capture_help()}
												recommended={m.config_audio_capture_stream_sink()}
												htmlFor="cfg-audio-capture"
												controlWidth="sm"
											>
												<FieldSelect
													id="cfg-audio-capture"
													className={cn(fieldControlClass, "sm:w-40")}
													value={
														draft.audio_video.audio_capture ?? "stream-sink"
													}
													onChange={(e) =>
														patch((d) => {
															d.audio_video.audio_capture =
																e.target.value || null;
														})
													}
												>
													<HelpOption
														value="stream-sink"
														recommended
														title={m.config_audio_capture_stream_sink_help()}
													>
														{m.config_audio_capture_stream_sink()}
													</HelpOption>
													<HelpOption
														value="monitor"
														title={m.config_audio_capture_monitor_help()}
													>
														{m.config_audio_capture_monitor()}
													</HelpOption>
												</FieldSelect>
											</Row>
										</FieldGroup>

										<FieldGroup title={m.config_hdr_audio()}>
											<ToggleRow
												label={m.config_ten_bit()}
												hint={m.config_ten_bit_hint()}
												help={m.config_ten_bit_help_advanced()}
												recommended={m.config_on()}
												checked={draft.audio_video.ten_bit}
												onChange={(v) =>
													patch((d) => {
														d.audio_video.ten_bit = v;
													})
												}
											/>
											<ToggleRow
												label={m.config_four_four_four()}
												hint={m.config_four_four_four_hint()}
												help={m.config_four_four_four_help_advanced()}
												recommended={m.config_on()}
												checked={draft.audio_video.four_four_four}
												onChange={(v) =>
													patch((d) => {
														d.audio_video.four_four_four = v;
													})
												}
											/>
											<ToggleRow
												label={m.config_gamescope_hdr()}
												hint={m.config_gamescope_hdr_hint()}
												help={m.config_gamescope_hdr_help_advanced()}
												recommended={m.config_on()}
												checked={draft.audio_video.gamescope_hdr}
												onChange={(v) =>
													patch((d) => {
														d.audio_video.gamescope_hdr = v;
													})
												}
											/>
										</FieldGroup>
										<FieldGroup title={m.config_gamescope_hdr()}>
											<ToggleRow
												label={m.config_gamescope_splash()}
												hint={m.config_gamescope_splash_hint()}
												help={m.config_gamescope_splash_help()}
												recommended={m.config_on()}
												checked={draft.audio_video.gamescope_splash}
												onChange={(v) =>
													patch((d) => {
														d.audio_video.gamescope_splash = v;
													})
												}
											/>
											<Row
												label={m.config_vdisplay_multiplier()}
												hint={m.config_vdisplay_multiplier_hint()}
												help={m.config_vdisplay_multiplier_help()}
												recommended={m.config_vdisplay_multiplier_1()}
												htmlFor="cfg-vdisplay-multiplier"
											>
												<FieldSelect
													id="cfg-vdisplay-multiplier"
													value={String(draft.audio_video.vdisplay_hz_mult)}
													onChange={(e) =>
														patch((d) => {
															d.audio_video.vdisplay_hz_mult = Number(
																e.target.value,
															);
														})
													}
												>
													<HelpOption value="1" recommended>
														{m.config_vdisplay_multiplier_1()}
													</HelpOption>
													<HelpOption value="2">
														{m.config_vdisplay_multiplier_2()}
													</HelpOption>
													<HelpOption value="3">
														{m.config_vdisplay_multiplier_3()}
													</HelpOption>
													<HelpOption value="4">
														{m.config_vdisplay_multiplier_4()}
													</HelpOption>
												</FieldSelect>
											</Row>
											{gamescopePathRelevant ? (
												<Row
													label={m.config_gamescope_sdr_nits()}
													hint={m.config_gamescope_sdr_nits_hint()}
													help={m.config_gamescope_sdr_nits_help()}
													recommended={m.config_blank_gamescope_default()}
													htmlFor="cfg-gamescope-sdr-nits"
													controlWidth="sm"
												>
													<Input
														id="cfg-gamescope-sdr-nits"
														className={cn(fieldControlClass, "sm:w-32")}
														type="number"
														min={1}
														max={10000}
														step={1}
														value={draft.audio_video.gamescope_sdr_nits ?? ""}
														onChange={(e) =>
															patch((d) => {
																d.audio_video.gamescope_sdr_nits = e.target
																	.value
																	? Number(e.target.value)
																	: null;
															})
														}
													/>
												</Row>
											) : null}
										</FieldGroup>
									</ConfigCard>
								</TabsContent>

								<TabsContent value="network" className="mt-0 outline-none">
									<ConfigCard
										title={m.config_network()}
										description={m.config_network_card_desc()}
										advanced
									>
										<FieldGroup title={m.config_connectivity_group()}>
											<ToggleRow
												label={m.config_mdns()}
												hint={m.config_mdns_hint()}
												help={m.config_mdns_help()}
												recommended={m.config_on()}
												checked={draft.network.mdns}
												onChange={(v) =>
													patch((d) => {
														d.network.mdns = v;
													})
												}
											/>
											<Row
												label={m.config_moonlight()}
												hint={m.config_moonlight_hint()}
												help={m.config_moonlight_help_readonly_network()}
											>
												<div className="flex w-full flex-col items-stretch gap-2 sm:w-56 sm:items-end">
													<div className="flex items-center gap-2 text-sm">
														<span
															aria-hidden
															className={cn(
																"size-2 rounded-full",
																moonlightOn
																	? "bg-emerald-500"
																	: "bg-muted-foreground/40",
															)}
														/>
														<span className="text-muted-foreground">
															{moonlightOn
																? m.config_moonlight_on()
																: m.config_moonlight_off()}
														</span>
													</div>
													<Link
														to="/host"
														className={cn(
															buttonVariants({ variant: "outline" }),
															"w-full justify-center",
														)}
													>
														{m.config_moonlight_open_host()}
													</Link>
												</div>
											</Row>
											<Row
												label={m.config_clipboard()}
												hint={m.config_clipboard_hint()}
												help={m.config_clipboard_hint()}
												recommended={m.config_clipboard_off()}
												htmlFor="cfg-clipboard"
											>
												<FieldSelect
													id="cfg-clipboard"
													value={draft.clipboard}
													onChange={(e) =>
														patch((d) => {
															d.clipboard =
																e.target.value as HostConfigFile["clipboard"];
														})
													}
												>
													<HelpOption value="off" recommended>
														{m.config_clipboard_off()}
													</HelpOption>
													<HelpOption value="text-only">
														{m.config_clipboard_text_only()}
													</HelpOption>
													<HelpOption value="on">
														{m.config_clipboard_on()}
													</HelpOption>
												</FieldSelect>
											</Row>
											<ToggleRow
												label={m.config_chacha20()}
												hint={m.config_chacha20_hint()}
												help={m.config_chacha20_help()}
												recommended={m.config_on()}
												checked={draft.network.chacha20}
												onChange={(v) =>
													patch((d) => {
														d.network.chacha20 = v;
													})
												}
											/>
											<Row
												label={m.config_fec()}
												hint={m.config_fec_hint()}
												help={m.config_fec_help()}
												recommended={m.config_fec_recommended()}
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
													title={m.config_fec_title()}
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
										<FieldGroup title={m.config_stream_profiles()}>
											<p className="text-sm leading-relaxed text-muted-foreground">
												{m.config_stream_profiles_hint()}
											</p>
											<Row
												label={m.config_performance_profile()}
												hint={m.config_performance_profile_hint()}
												help={m.config_profile_low_latency_help()}
												recommended={m.config_profile_balanced()}
												htmlFor="cfg-performance-profile"
											>
												<FieldSelect
													id="cfg-performance-profile"
													value={draft.performance_profile}
													onChange={(e) =>
														patch((d) => {
															d.performance_profile =
																e.target.value as HostConfigFile["performance_profile"];
														})
													}
												>
													<HelpOption
														value="balanced"
														recommended
														title={m.config_profile_balanced_help()}
													>
														{m.config_profile_balanced()}
													</HelpOption>
													<HelpOption
														value="low_latency"
														title={m.config_profile_low_latency_help()}
													>
														{m.config_profile_low_latency()}
													</HelpOption>
												</FieldSelect>
											</Row>
											<Row
												label={m.config_latency_profile()}
												hint={m.config_latency_profile_hint()}
												help={m.config_profile_low_latency_help()}
												recommended={m.config_profile_balanced()}
												htmlFor="cfg-latency-profile"
											>
												<FieldSelect
													id="cfg-latency-profile"
													value={draft.latency_profile}
													onChange={(e) =>
														patch((d) => {
															d.latency_profile =
																e.target.value as HostConfigFile["latency_profile"];
														})
													}
												>
													<HelpOption
														value="balanced"
														recommended
														title={m.config_profile_balanced_help()}
													>
														{m.config_profile_balanced()}
													</HelpOption>
													<HelpOption
														value="low_latency"
														title={m.config_profile_low_latency_help()}
													>
														{m.config_profile_low_latency()}
													</HelpOption>
												</FieldSelect>
											</Row>
											<Row
												label={m.config_network_policy()}
												hint={m.config_network_policy_hint()}
												help={m.config_profile_auto_help()}
												recommended={m.config_profile_auto()}
												htmlFor="cfg-network-policy"
											>
												<FieldSelect
													id="cfg-network-policy"
													value={draft.network_policy}
													onChange={(e) =>
														patch((d) => {
															d.network_policy =
																e.target.value as HostConfigFile["network_policy"];
														})
													}
												>
													<HelpOption
														value="auto"
														recommended
														title={m.config_profile_auto_help()}
													>
														{m.config_profile_auto()}
													</HelpOption>
													<HelpOption
														value="lan"
														title={m.config_profile_lan_help()}
													>
														{m.config_profile_lan()}
													</HelpOption>
													<HelpOption
														value="wan"
														title={m.config_profile_wan_help()}
													>
														{m.config_profile_wan()}
													</HelpOption>
												</FieldSelect>
											</Row>
										</FieldGroup>
									</ConfigCard>
								</TabsContent>

								<TabsContent value="encoders" className="mt-0 outline-none">
									<ConfigCard
										title={m.config_encoders()}
										description={m.config_encoders_card_desc()}
										advanced
									>
										<FieldGroup title={m.config_encoder_path_group()}>
											<Row
												label={m.config_encoder()}
												hint={m.config_encoder_hint()}
												help={m.config_encoder_help()}
												recommended={m.config_encoder_auto()}
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
														title={m.config_encoder_auto_title()}
													>
														{m.config_encoder_auto()}
													</HelpOption>
													<HelpOption
														value="nvenc"
														title={m.config_encoder_nvenc_title()}
													>
														NVENC
													</HelpOption>
													<HelpOption
														value="amf"
														title={m.config_encoder_amf_title()}
													>
														AMF
													</HelpOption>
													<HelpOption
														value="qsv"
														title={m.config_encoder_qsv_title()}
													>
														QSV
													</HelpOption>
													<HelpOption
														value="vaapi"
														title={m.config_encoder_vaapi_title()}
													>
														VAAPI
													</HelpOption>
													<HelpOption
														value="software"
														title={m.config_encoder_software_title()}
													>
														{m.config_encoder_software()}
													</HelpOption>
												</FieldSelect>
											</Row>
											<Row
												label={m.config_render_adapter()}
												hint={m.config_render_adapter_hint()}
												help={m.config_render_adapter_help()}
												recommended={m.config_render_adapter_recommended()}
												htmlFor="cfg-render-adapter"
											>
												<Input
													id="cfg-render-adapter"
													className={fieldControlClass}
													value={draft.encoders.render_adapter ?? ""}
													title={m.config_render_adapter_title()}
													placeholder={m.config_render_adapter_placeholder()}
													onChange={(e) =>
														patch((d) => {
															d.encoders.render_adapter =
																e.target.value || null;
														})
													}
												/>
											</Row>
											<Row
												label={m.config_zerocopy()}
												hint={m.config_zerocopy_hint()}
												help={m.config_zerocopy_help()}
												recommended={m.config_zerocopy_vendor_default()}
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
														title={m.config_zerocopy_vendor_default_title()}
													>
														{m.config_zerocopy_vendor_default()}
													</HelpOption>
													<HelpOption
														value="1"
														title={m.config_zerocopy_on_title()}
													>
														{m.config_on()}
													</HelpOption>
													<HelpOption
														value="0"
														title={m.config_zerocopy_off_title()}
													>
														{m.config_off()}
													</HelpOption>
												</FieldSelect>
											</Row>
										</FieldGroup>
									</ConfigCard>
								</TabsContent>
							</div>
						</div>
					</Tabs>
				)}
			</div>

			{dirty ? (
				<DirtySaveBar
					unsavedLabel={m.display_unsaved()}
					saveLabel={m.display_save()}
					loadingLabel={m.common_loading()}
					discardLabel={m.config_discard()}
					pending={save.isPending}
					onSave={onSave}
					onDiscard={onDiscard}
				/>
			) : null}
		</Section>
	);
};

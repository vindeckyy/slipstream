import type { Meta, StoryObj } from "@storybook/react-vite";
import { useState } from "react";
import { m } from "@/paraglide/messages";
import { ConfigModeToggle, type ConfigMode } from "@/sections/Config/ConfigModeToggle";
import { DirtySaveBar } from "@/sections/Config/DirtySaveBar";
import { RestartOffer } from "@/sections/Config/RestartOffer";
import {
	buildCaptureMethodOptions,
	buildCompositorOptions,
	formatCapabilityOptionLabel,
} from "@/sections/Config/capability-options";

function ModeDemo() {
	const [mode, setMode] = useState<ConfigMode>("recommended");
	return (
		<div className="space-y-4">
			<ConfigModeToggle
				mode={mode}
				label={m.config_mode_label()}
				recommendedLabel={m.config_mode_recommended()}
				allLabel={m.config_mode_all()}
				onChange={setMode}
			/>
			{mode === "all" ? (
				<p
					data-testid="config-all-warning"
					className="rounded-xl border border-warning/40 bg-warning/10 px-4 py-3 text-sm"
				>
					{m.config_all_warning()}
				</p>
			) : (
				<p data-testid="config-recommended" className="text-sm text-muted-foreground">
					{m.config_intro()}
				</p>
			)}
		</div>
	);
}

function CapabilityDemo() {
	const copy = {
		savedUnavailable: m.config_capability_saved_unavailable(),
		detectedDefault: m.config_capability_detected_default(),
		unavailable: m.config_capability_unavailable(),
		autoDetect: m.config_compositor_auto_detect(),
		autoDetectHelp: m.config_compositor_auto_detect_help(),
		headlessOff: m.config_headless_off(),
		headlessOffHelp: m.config_headless_off_help(),
	};
	const capture = buildCaptureMethodOptions(
		[
			{ id: "auto", label: "Auto", available: true },
			{ id: "kwin", label: "KWin Screencast", available: false },
			{ id: "wlr", label: "wlroots screencopy", available: true },
		],
		"kwin",
		copy,
	);
	const compositor = buildCompositorOptions(
		[
			{ id: "kwin", label: "KWin", available: true, default: true },
			{ id: "mutter", label: "Mutter", available: false },
		],
		"mutter",
		copy,
	);
	const marks = {
		detected: m.config_option_detected(),
		unavailable: m.config_option_unavailable(),
	};
	return (
		<div className="space-y-4">
			<label className="block space-y-1 text-sm">
				<span className="font-medium">{m.config_capture_method()}</span>
				<select
					className="h-9 w-full max-w-sm rounded-md border border-input bg-background px-3"
					defaultValue="kwin"
					data-testid="capture-select"
				>
					{capture.map((o) => (
						<option
							key={o.value}
							value={o.value}
							disabled={!o.available && o.value !== "kwin"}
						>
							{formatCapabilityOptionLabel(o, marks)}
						</option>
					))}
				</select>
			</label>
			<label className="block space-y-1 text-sm">
				<span className="font-medium">{m.config_virtual_compositor()}</span>
				<select
					className="h-9 w-full max-w-sm rounded-md border border-input bg-background px-3"
					defaultValue="mutter"
					data-testid="compositor-select"
				>
					{compositor.map((o) => (
						<option
							key={o.value || "__auto__"}
							value={o.value}
							disabled={!o.available && o.value !== "mutter"}
						>
							{formatCapabilityOptionLabel(o, marks)}
						</option>
					))}
				</select>
			</label>
		</div>
	);
}

function RestartDemo({ error = null }: { error?: string | null }) {
	const [open, setOpen] = useState(true);
	const [confirm, setConfirm] = useState(false);
	const [pending, setPending] = useState(false);
	return (
		<RestartOffer
			open={open}
			confirmOpen={confirm}
			pending={pending}
			error={error}
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
				setOpen(false);
				setConfirm(false);
			}}
			onConfirmOpenChange={setConfirm}
			onRestart={() => {
				setPending(true);
				setTimeout(() => {
					setPending(false);
					setConfirm(false);
					setOpen(false);
				}, 800);
			}}
		/>
	);
}

function DirtyBarDemo() {
	return (
		<div className="relative min-h-48">
			<p className="text-sm text-muted-foreground">
				{m.config_mobile_dirty_hint()}
			</p>
			<div
				className="fixed inset-x-0 bottom-0 z-50 border-t border-border bg-card/95 py-3 text-center text-xs text-muted-foreground sm:hidden"
				style={{ paddingBottom: "env(safe-area-inset-bottom)" }}
			>
				Bottom nav stand-in
			</div>
			<DirtySaveBar
				unsavedLabel={m.display_unsaved()}
				saveLabel={m.config_save()}
				loadingLabel={m.common_loading()}
				discardLabel={m.config_discard()}
				onSave={() => undefined}
				onDiscard={() => undefined}
			/>
		</div>
	);
}

const meta = {
	title: "Pages/Config",
} satisfies Meta;

export default meta;
type Story = StoryObj;

export const RecommendedVsAll: Story = {
	render: () => <ModeDemo />,
};

export const CapabilityAvailability: Story = {
	render: () => <CapabilityDemo />,
};

export const RestartPrompt: Story = {
	render: () => <RestartDemo />,
};

export const RestartPromptError: Story = {
	render: () => <RestartDemo error={m.config_restart_failed()} />,
};

export const MobileDirtySave: Story = {
	parameters: {
		viewport: { defaultViewport: "mobile1" },
	},
	render: () => <DirtyBarDemo />,
};

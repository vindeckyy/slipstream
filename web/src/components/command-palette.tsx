import { useNavigate } from "@tanstack/react-router";
import {
	AppWindow,
	Command as CommandIcon,
	GaugeCircle,
	KeyRound,
	MonitorPlay,
	Server,
	Settings,
	Workflow,
} from "lucide-react";
import type { LucideIcon } from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import {
	Dialog,
	DialogContent,
	DialogDescription,
	DialogHeader,
	DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { useLocale } from "@/lib/i18n";
import { m } from "@/paraglide/messages";

type CommandTarget =
	| "/"
	| "/sessions"
	| "/host"
	| "/displays"
	| "/pin"
	| "/apps"
	| "/automation"
	| "/stats"
	| "/settings";

type CommandItem = {
	to: CommandTarget;
	icon: LucideIcon;
	label: () => string;
	keywords: readonly string[];
};

const COMMANDS: readonly CommandItem[] = [
	{
		to: "/",
		icon: GaugeCircle,
		label: () => m.status_title(),
		keywords: ["dashboard", "overview", "status", "live"],
	},
	{
		to: "/sessions",
		icon: MonitorPlay,
		label: () => m.status_session(),
		keywords: ["session", "stream", "play"],
	},
	{
		to: "/host",
		icon: Server,
		label: () => m.nav_host(),
		keywords: ["host", "gpu", "update", "system"],
	},
	{
		to: "/displays",
		icon: MonitorPlay,
		label: () => m.nav_displays(),
		keywords: ["display", "screen", "virtual"],
	},
	{
		to: "/pin",
		icon: KeyRound,
		label: () => m.nav_pairing(),
		keywords: ["pair", "pin", "client"],
	},
	{
		to: "/apps",
		icon: AppWindow,
		label: () => m.nav_library(),
		keywords: ["library", "game", "app"],
	},
	{
		to: "/automation",
		icon: Workflow,
		label: () => m.nav_automation(),
		keywords: ["automation", "hook", "webhook"],
	},
	{
		to: "/stats",
		icon: GaugeCircle,
		label: () => m.nav_stats(),
		keywords: ["stats", "capture", "recording", "monitor"],
	},
	{
		to: "/settings",
		icon: Settings,
		label: () => m.nav_settings(),
		keywords: ["settings", "preferences", "language"],
	},
];

function matchesCommand(command: CommandItem, query: string): boolean {
	if (!query.trim()) return true;
	const haystack = `${command.label()} ${command.keywords.join(" ")}`.toLocaleLowerCase();
	return haystack.includes(query.trim().toLocaleLowerCase());
}

export function CommandPalette() {
	useLocale();
	const navigate = useNavigate();
	const [open, setOpen] = useState(false);
	const [query, setQuery] = useState("");
	const [selectedIndex, setSelectedIndex] = useState(0);
	const filteredCommands = useMemo(
		() => COMMANDS.filter((command) => matchesCommand(command, query)),
		[query],
	);

	useEffect(() => {
		const onKeyDown = (event: KeyboardEvent) => {
			if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === "k") {
				event.preventDefault();
				setOpen((current) => !current);
			}
		};
		window.addEventListener("keydown", onKeyDown);
		return () => window.removeEventListener("keydown", onKeyDown);
	}, []);

	useEffect(() => {
		setSelectedIndex(0);
	}, [query, open]);

	const selectCommand = (command: CommandItem | undefined) => {
		if (!command) return;
		setOpen(false);
		setQuery("");
		void navigate({ to: command.to });
	};

	return (
		<>
			<button
				type="button"
				onClick={() => setOpen(true)}
				aria-label={m.nav_command_palette()}
				aria-keyshortcuts="Control+K Meta+K"
				className="inline-flex min-h-7 items-center gap-2 rounded-full border border-border/70 bg-card/70 px-2.5 py-1 text-xs text-muted-foreground outline-none transition-colors hover:border-primary/35 hover:bg-primary/5 hover:text-foreground focus-visible:ring-2 focus-visible:ring-ring/50"
			>
				<CommandIcon className="size-3.5" aria-hidden />
				<span className="hidden sm:inline">{m.nav_command_hint()}</span>
				<kbd className="rounded border border-border/70 bg-muted/50 px-1.5 py-0.5 font-mono text-[10px] leading-none text-foreground/75">
					{m.nav_command_shortcut()}
				</kbd>
			</button>

			<Dialog
				open={open}
				onOpenChange={(nextOpen) => {
					setOpen(nextOpen);
					if (!nextOpen) setQuery("");
				}}
			>
				<DialogContent className="gap-0 overflow-hidden p-0">
					<DialogHeader className="border-b border-border/70 px-4 py-4 sm:px-5">
						<DialogTitle>{m.nav_command_palette()}</DialogTitle>
						<DialogDescription>{m.nav_command_hint()}</DialogDescription>
					</DialogHeader>
					<div className="border-b border-border/70 p-3">
						<Input
							autoFocus
							value={query}
							onChange={(event) => setQuery(event.target.value)}
							onKeyDown={(event) => {
								if (event.key === "ArrowDown") {
									event.preventDefault();
									setSelectedIndex((index) =>
										filteredCommands.length === 0
											? 0
											: (index + 1) % filteredCommands.length,
									);
								} else if (event.key === "ArrowUp") {
									event.preventDefault();
									setSelectedIndex((index) =>
										filteredCommands.length === 0
											? 0
											: (index - 1 + filteredCommands.length) %
												filteredCommands.length,
									);
								} else if (event.key === "Enter") {
									event.preventDefault();
									selectCommand(filteredCommands[selectedIndex]);
								}
							}}
							placeholder={m.nav_command_hint()}
							aria-label={m.nav_command_palette()}
						/>
					</div>
					<div
						className="max-h-[min(52vh,24rem)] overflow-y-auto p-2"
						role="listbox"
						aria-label={m.nav_command_palette()}
					>
						{filteredCommands.length > 0 ? (
							filteredCommands.map((command, index) => {
								const Icon = command.icon;
								const selected = index === selectedIndex;
								return (
									<button
										key={command.to}
										type="button"
										role="option"
										aria-selected={selected}
										onMouseEnter={() => setSelectedIndex(index)}
										onClick={() => selectCommand(command)}
										className={`flex w-full items-center gap-3 rounded-lg px-3 py-3 text-left outline-none transition-colors ${
											selected
												? "bg-primary/10 text-foreground"
												: "text-muted-foreground hover:bg-muted/60 hover:text-foreground"
										}`}
									>
										<span className="flex size-8 shrink-0 items-center justify-center rounded-md bg-muted/60">
											<Icon className="size-4" aria-hidden />
										</span>
										<span className="min-w-0 flex-1 truncate text-sm font-medium">
											{command.label()}
										</span>
										<span aria-hidden className="text-xs text-muted-foreground">
											{command.to}
										</span>
									</button>
								);
							})
						) : (
							<p className="px-3 py-8 text-center text-sm text-muted-foreground">
								{m.nav_command_empty()}
							</p>
						)}
					</div>
				</DialogContent>
			</Dialog>
		</>
	);
}

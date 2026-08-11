import { useQueryClient } from "@tanstack/react-query";
import { X } from "lucide-react";
import {
	type FC,
	type FormEvent,
	type ReactNode,
	useEffect,
	useState,
} from "react";
import {
	getGetLibraryQueryKey,
	useCreateCustomGame,
	useGetCustomGame,
	useUpdateCustomGame,
} from "@/api/gen/library/library";
import type {
	CustomEntry,
	CustomInput,
	DetectHint,
	GameEntry,
	PrepCmd,
} from "@/api/gen/model";
import { OptionLabel } from "@/components/option-help";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { apiErrorMessage } from "@/lib/errors";
import { m } from "@/paraglide/messages";
import { customId } from "./helpers";

interface FormState {
	title: string;
	portrait: string;
	hero: string;
	header: string;
	logo: string;
	command: string;
	// Details — the flattened GameMeta fields; numbers and lists are kept as the raw
	// text the user typed and only parsed on submit.
	platform: string;
	description: string;
	developer: string;
	publisher: string;
	releaseYear: string;
	genres: string;
	tags: string;
	region: string;
	players: string;
	// Process recognition (detect) — the fields the catalog never serializes, round-tripped
	// via the admin-only detail route so an edit no longer drops them.
	installDir: string;
	exe: string;
	processName: string;
	// Per-title prep/undo steps, preserved in order.
	prep: { do: string; undo: string }[];
}

const emptyForm: FormState = {
	title: "",
	portrait: "",
	hero: "",
	header: "",
	logo: "",
	command: "",
	platform: "",
	description: "",
	developer: "",
	publisher: "",
	releaseYear: "",
	genres: "",
	tags: "",
	region: "",
	players: "",
	installDir: "",
	exe: "",
	processName: "",
	prep: [],
};

function formFrom(entry: GameEntry): FormState {
	return {
		title: entry.title,
		portrait: entry.art.portrait ?? "",
		hero: entry.art.hero ?? "",
		header: entry.art.header ?? "",
		logo: entry.art.logo ?? "",
		command: entry.launch?.kind === "command" ? entry.launch.value : "",
		platform: entry.platform ?? "",
		description: entry.description ?? "",
		developer: entry.developer ?? "",
		publisher: entry.publisher ?? "",
		releaseYear: entry.release_year?.toString() ?? "",
		genres: entry.genres?.join(", ") ?? "",
		tags: entry.tags?.join(", ") ?? "",
		region: entry.region ?? "",
		players: entry.players?.toString() ?? "",
		installDir: "",
		exe: "",
		processName: "",
		prep: [],
	};
}

function formFromCustom(entry: CustomEntry): FormState {
	return {
		title: entry.title,
		portrait: entry.art?.portrait ?? "",
		hero: entry.art?.hero ?? "",
		header: entry.art?.header ?? "",
		logo: entry.art?.logo ?? "",
		command: entry.launch?.kind === "command" ? entry.launch.value : "",
		platform: entry.platform ?? "",
		description: entry.description ?? "",
		developer: entry.developer ?? "",
		publisher: entry.publisher ?? "",
		releaseYear: entry.release_year?.toString() ?? "",
		genres: entry.genres?.join(", ") ?? "",
		tags: entry.tags?.join(", ") ?? "",
		region: entry.region ?? "",
		players: entry.players?.toString() ?? "",
		installDir: entry.detect?.install_dir ?? "",
		exe: entry.detect?.exe ?? "",
		processName: entry.detect?.process_name ?? "",
		prep: (entry.prep ?? []).map((p) => ({ do: p.do, undo: p.undo ?? "" })),
	};
}

/** Map the form to the API body — only attach `launch` when a command was given. `update_custom`
 * REPLACES the whole entry (art AND the metadata fields), so every field the form knows must
 * round-trip (else editing a game with a `logo` or a `platform` would silently drop it). */
function toInput(f: FormState): CustomInput {
	const trim = (s: string) => {
		const t = s.trim();
		return t ? t : undefined;
	};
	// "RPG, Platformer" → ["RPG", "Platformer"]; empty input → omitted entirely.
	const list = (s: string) => {
		const items = s
			.split(",")
			.map((x) => x.trim())
			.filter(Boolean);
		return items.length ? items : undefined;
	};
	const int = (s: string) => {
		const n = Number.parseInt(s.trim(), 10);
		return Number.isFinite(n) ? n : undefined;
	};
	const command = f.command.trim();
	const detect: DetectHint = {
		install_dir: trim(f.installDir),
		exe: trim(f.exe),
		process_name: trim(f.processName),
	};
	const prep: PrepCmd[] = f.prep
		.map((p) => ({
			do: p.do.trim(),
			undo: p.undo.trim() ? p.undo.trim() : null,
		}))
		.filter((p) => p.do.length > 0);
	return {
		title: f.title.trim(),
		art: {
			portrait: trim(f.portrait),
			hero: trim(f.hero),
			header: trim(f.header),
			logo: trim(f.logo),
		},
		launch: command ? { kind: "command", value: command } : null,
		detect,
		prep,
		platform: trim(f.platform),
		description: trim(f.description),
		developer: trim(f.developer),
		publisher: trim(f.publisher),
		release_year: int(f.releaseYear),
		genres: list(f.genres),
		tags: list(f.tags),
		region: trim(f.region),
		players: int(f.players),
	};
}

/** What the form targets: an existing custom entry to edit, or "new" for a fresh add. */
export type FormTarget = GameEntry | "new";

/**
 * Container: the add/edit form — owns the create + update mutations and derives the
 * initial field state from the target. Kept entirely separate from the overview grid
 * (own file, own queries) so the two concerns don't share a component.
 */
export const GameFormSection: FC<{
	target: FormTarget;
	onClose: () => void;
}> = ({ target, onClose }) => {
	const qc = useQueryClient();
	const create = useCreateCustomGame();
	const update = useUpdateCustomGame();
	const invalidate = () =>
		qc.invalidateQueries({ queryKey: getGetLibraryQueryKey() });

	// Edit mode fetches the full stored entry so `detect` and `prep` round-trip.
	const detail = useGetCustomGame(
		target === "new" ? "" : customId(target),
		{ query: { enabled: target !== "new" } },
	);
	const detailError = detail.error
		? apiErrorMessage(detail.error) ?? "Could not load this entry."
		: undefined;
	const initial: FormState =
		target === "new"
			? emptyForm
			: detail.data
				? formFromCustom(detail.data)
				: formFrom(target);

	const [form, setForm] = useState<FormState>(initial);
	useEffect(() => {
		if (target === "new") return;
		if (detail.data) setForm(formFromCustom(detail.data));
	}, [detail.data, target]);

	const set = (key: keyof FormState) => (value: string) =>
		setForm((f) => ({ ...f, [key]: value }));

	const setPrep = (index: number, patch: Partial<{ do: string; undo: string }>) =>
		setForm((f) => ({
			...f,
			prep: f.prep.map((row, i) => (i === index ? { ...row, ...patch } : row)),
		}));
	const addPrep = () =>
		setForm((f) => ({ ...f, prep: [...f.prep, { do: "", undo: "" }] }));
	const removePrep = (index: number) =>
		setForm((f) => ({
			...f,
			prep: f.prep.filter((_, i) => i !== index),
		}));

	// URL-ish art fields accept web URLs, data URLs, proxy paths, and local host paths —
	// the host treats them as free-form sources, so no `type="url"` constraint.
	const releaseYearNum = Number.parseInt(form.releaseYear, 10);
	const releaseYearValid =
		form.releaseYear.trim() === "" ||
		(Number.isInteger(releaseYearNum) && releaseYearNum >= 0 && releaseYearNum <= 65535);
	const playersNum = Number.parseInt(form.players, 10);
	const playersValid =
		form.players.trim() === "" ||
		(Number.isInteger(playersNum) && playersNum >= 0 && playersNum <= 255);
	const ready = form.title.trim().length > 0 && releaseYearValid && playersValid;

	const handleSubmit = (e: FormEvent) => {
		e.preventDefault();
		if (!form.title.trim()) return;
		onSubmit(toInput(form));
	};

	const onSubmit = async (data: CustomInput) => {
		try {
			if (target === "new") await create.mutateAsync({ data });
			else await update.mutateAsync({ id: customId(target), data });
		} catch {
			return; // the message is rendered from the mutation's own error state below
		}
		invalidate();
		onClose();
	};

	const error = detailError ?? apiErrorMessage(create.error ?? update.error);
	const mode = target === "new" ? "add" : "edit";
	const isSaving = create.isPending || update.isPending;
	const onCancel = onClose;

	return (
		<Card className="max-w-xl ring-accent/50">
			<CardHeader className="flex-row items-center justify-between space-y-0 pb-3">
				<CardTitle className="text-base tracking-tight">
					{mode === "edit" ? m.library_edit_title() : m.library_add_title()}
				</CardTitle>
				<Button
					variant="ghost"
					size="icon"
					aria-label={m.library_cancel()}
					onClick={onCancel}
				>
					<X className="size-4" />
				</Button>
			</CardHeader>
			<CardContent>
				<form onSubmit={handleSubmit} className="space-y-4">
					<Field
						id="title"
						label={m.library_field_title()}
						value={form.title}
						onChange={set("title")}
						help={m.library_field_title_help()}
						required
					/>
					<Field
						id="portrait"
						label={m.library_field_portrait()}
						value={form.portrait}
						onChange={set("portrait")}
						help={m.library_field_portrait_help()}
						recommended={m.library_field_portrait_recommended()}
					/>
					<Field
						id="hero"
						label={m.library_field_hero()}
						value={form.hero}
						onChange={set("hero")}
						help={m.library_field_hero_help()}
						recommended={m.library_field_hero_recommended()}
					/>
					<Field
						id="header"
						label={m.library_field_header()}
						value={form.header}
						onChange={set("header")}
						help={m.library_field_header_help()}
						recommended={m.library_field_header_recommended()}
					/>
					<Field
						id="logo"
						label={m.library_field_logo()}
						value={form.logo}
						onChange={set("logo")}
						help={m.library_field_logo_help()}
						recommended={m.library_field_logo_recommended()}
					/>
					<Field
						id="command"
						label={m.library_field_command()}
						value={form.command}
						onChange={set("command")}
						help={m.library_field_command_help()}
						recommended={m.library_field_command_recommended()}
					/>
					<fieldset className="space-y-4 rounded-lg border border-border/70 bg-muted/20 p-3 sm:p-4">
						<legend className="sr-only">{m.library_details_legend()}</legend>
						<p
							aria-hidden
							className="text-sm font-medium tracking-tight text-foreground"
						>
							{m.library_details_legend()}
						</p>
						<Field
							id="platform"
							label={m.library_field_platform()}
							value={form.platform}
							onChange={set("platform")}
							help={m.library_field_platform_help()}
							recommended={m.library_field_platform_recommended()}
						/>
						<Field
							id="description"
							label={m.library_field_description()}
							value={form.description}
							onChange={set("description")}
							help={m.library_field_description_help()}
							recommended={m.library_field_description_recommended()}
						/>
						<div className="grid grid-cols-1 gap-4 sm:grid-cols-2">
							<Field
								id="developer"
								label={m.library_field_developer()}
								value={form.developer}
								onChange={set("developer")}
								help={m.library_field_developer_help()}
							/>
							<Field
								id="publisher"
								label={m.library_field_publisher()}
								value={form.publisher}
								onChange={set("publisher")}
								help={m.library_field_publisher_help()}
							/>
						</div>
						<div className="grid grid-cols-1 gap-4 sm:grid-cols-2">
							<Field
								id="releaseYear"
								label={m.library_field_release_year()}
								value={form.releaseYear}
								onChange={set("releaseYear")}
								help={m.library_field_release_year_help()}
								recommended={m.library_field_release_year_recommended()}
								error={
									releaseYearValid
										? undefined
										: m.library_field_release_year_error()
								}
							/>
							<Field
								id="players"
								label={m.library_field_players()}
								value={form.players}
								onChange={set("players")}
								help={m.library_field_players_help()}
								recommended={m.library_field_players_recommended()}
								error={
									playersValid ? undefined : m.library_field_players_error()
								}
							/>
						</div>
						<Field
							id="region"
							label={m.library_field_region()}
							value={form.region}
							onChange={set("region")}
							help={m.library_field_region_help()}
							recommended={m.library_field_region_recommended()}
						/>
						<Field
							id="genres"
							label={m.library_field_genres()}
							value={form.genres}
							onChange={set("genres")}
							help={m.library_field_genres_help()}
							recommended={m.library_field_genres_recommended()}
						/>
						<Field
							id="tags"
							label={m.library_field_tags()}
							value={form.tags}
							onChange={set("tags")}
							help={m.library_field_tags_help()}
							recommended={m.library_field_tags_recommended()}
						/>
					</fieldset>

					<fieldset className="space-y-4 rounded-lg border border-border/70 bg-muted/20 p-3 sm:p-4">
						<legend className="sr-only">{m.library_advanced_legend()}</legend>
						<p
							aria-hidden
							className="text-sm font-medium tracking-tight text-foreground"
						>
							{m.library_advanced_legend()}
						</p>
						<Field
							id="installDir"
							label={m.library_field_install_dir()}
							value={form.installDir}
							onChange={set("installDir")}
							help={m.library_field_install_dir_help()}
						/>
						<Field
							id="exe"
							label={m.library_field_exe()}
							value={form.exe}
							onChange={set("exe")}
							help={m.library_field_exe_help()}
						/>
						<Field
							id="processName"
							label={m.library_field_process_name()}
							value={form.processName}
							onChange={set("processName")}
							help={m.library_field_process_name_help()}
						/>
						<div className="space-y-2">
							<OptionLabel
								label={m.library_field_prep()}
								help={m.library_field_prep_help()}
							/>
							{form.prep.map((row, index) => (
								<div
									key={index}
									className="grid gap-2 rounded-lg border border-border/70 bg-muted/10 p-2 sm:grid-cols-[1fr_1fr_auto] sm:items-center"
								>
									<Input
										value={row.do}
										placeholder={m.library_field_prep_do()}
										onChange={(e) => setPrep(index, { do: e.target.value })}
									/>
									<Input
										value={row.undo}
										placeholder={m.library_field_prep_undo()}
										onChange={(e) =>
											setPrep(index, { undo: e.target.value })
										}
									/>
									<Button
										type="button"
										variant="ghost"
										size="icon"
										aria-label={m.library_field_prep_remove()}
										onClick={() => removePrep(index)}
									>
										<X className="size-4" />
									</Button>
								</div>
							))}
							<Button type="button" variant="outline" size="sm" onClick={addPrep}>
								{m.library_field_prep_add()}
							</Button>
						</div>
					</fieldset>

					{error && (
						<p
							role="alert"
							className="rounded-lg border border-destructive/40 bg-destructive/10 px-3 py-2.5 text-sm text-destructive"
						>
							{error}
						</p>
					)}
					<div className="flex flex-wrap gap-2">
						<Button type="submit" disabled={!ready || isSaving}>
							{mode === "edit" ? m.library_save() : m.library_create()}
						</Button>
						<Button type="button" variant="outline" onClick={onCancel}>
							{m.library_cancel()}
						</Button>
					</div>
				</form>
			</CardContent>
		</Card>
	);
};

/** One labeled text input bound to a FormState key — the form is a stack of these. */
const Field: FC<{
	id: keyof FormState;
	label: string;
	value: string;
	onChange: (value: string) => void;
	help?: string;
	recommended?: ReactNode;
	error?: string;
	required?: boolean;
}> = ({ id, label, value, onChange, help, recommended, error, required }) => (
	<div className="space-y-2">
		<OptionLabel
			label={label}
			help={help}
			recommended={recommended}
			htmlFor={`lib-${id}`}
		/>
		<Input
			id={`lib-${id}`}
			required={required}
			aria-invalid={error ? true : undefined}
			aria-describedby={error ? `lib-${id}-error` : undefined}
			value={value}
			onChange={(e) => onChange(e.target.value)}
		/>
		{error ? (
			<p id={`lib-${id}-error`} role="alert" className="text-xs text-destructive">
				{error}
			</p>
		) : null}
	</div>
);

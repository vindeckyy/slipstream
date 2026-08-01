import { useQueryClient } from "@tanstack/react-query";
import { X } from "lucide-react";
import {
	type FC,
	type FormEvent,
	type ReactNode,
	useState,
} from "react";
import {
	getGetLibraryQueryKey,
	useCreateCustomGame,
	useUpdateCustomGame,
} from "@/api/gen/library/library";
import type { CustomInput } from "@/api/gen/model/customInput";
import type { GameEntry } from "@/api/gen/model/gameEntry";
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
	return {
		title: f.title.trim(),
		art: {
			portrait: trim(f.portrait),
			hero: trim(f.hero),
			header: trim(f.header),
			logo: trim(f.logo),
		},
		launch: command ? { kind: "command", value: command } : null,
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

	// A rejected save must not close the form and must not look like a success. It used to do both:
	// nothing read `create.error`/`update.error`, and the un-caught `mutateAsync` rejection meant
	// the entry silently didn't save while the dialog disappeared — taking the operator's typing
	// with it.
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

	return (
		<GameForm
			initial={target === "new" ? emptyForm : formFrom(target)}
			mode={target === "new" ? "add" : "edit"}
			onSubmit={onSubmit}
			onCancel={onClose}
			isSaving={create.isPending || update.isPending}
			error={apiErrorMessage(create.error ?? update.error)}
		/>
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
	type?: string;
	required?: boolean;
}> = ({ id, label, value, onChange, help, recommended, type, required }) => (
	<div className="space-y-2">
		<OptionLabel
			label={label}
			help={help}
			recommended={recommended}
			htmlFor={`lib-${id}`}
		/>
		<Input
			id={`lib-${id}`}
			type={type}
			inputMode={
				type === "url" ? "url" : type === "number" ? "numeric" : undefined
			}
			required={required}
			value={value}
			onChange={(e) => onChange(e.target.value)}
		/>
	</div>
);

/**
 * The add/edit form card. Owns only its own field state (re-seeded per mount — the
 * parent keys it by target); reports a ready-to-send `CustomInput` on submit.
 */
export const GameForm: FC<{
	initial: FormState;
	mode: "add" | "edit";
	onSubmit: (data: CustomInput) => void;
	onCancel: () => void;
	isSaving: boolean;
	/** The host's refusal, if the last save failed — shown next to the button that caused it. */
	error?: string;
}> = ({ initial, mode, onSubmit, onCancel, isSaving, error }) => {
	const [form, setForm] = useState<FormState>(initial);
	const set = (key: keyof FormState) => (value: string) =>
		setForm((f) => ({ ...f, [key]: value }));

	const handleSubmit = (e: FormEvent) => {
		e.preventDefault();
		const data = toInput(form);
		if (!data.title) return;
		onSubmit(data);
	};

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
						type="url"
						help={m.library_field_portrait_help()}
						recommended={m.library_field_portrait_recommended()}
					/>
					<Field
						id="hero"
						label={m.library_field_hero()}
						value={form.hero}
						onChange={set("hero")}
						type="url"
						help={m.library_field_hero_help()}
						recommended={m.library_field_hero_recommended()}
					/>
					<Field
						id="header"
						label={m.library_field_header()}
						value={form.header}
						onChange={set("header")}
						type="url"
						help={m.library_field_header_help()}
						recommended={m.library_field_header_recommended()}
					/>
					<Field
						id="logo"
						label={m.library_field_logo()}
						value={form.logo}
						onChange={set("logo")}
						type="url"
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
								type="number"
								help={m.library_field_release_year_help()}
								recommended={m.library_field_release_year_recommended()}
							/>
							<Field
								id="players"
								label={m.library_field_players()}
								value={form.players}
								onChange={set("players")}
								type="number"
								help={m.library_field_players_help()}
								recommended={m.library_field_players_recommended()}
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
					{/* Data-loss warning, not a nicety.
					`PUT /library/custom/{id}` REPLACES the entry (host: library/custom.rs
					`update_custom` assigns `slot.prep = input.prep; slot.detect = input.detect`),
					but `GET /library` returns a `GameEntry`, which carries neither field. So the
					console cannot round-trip them — anything configured outside this form is dropped
					by a save it did not intend to touch. The real fix is host-side (expose `detect`
					and `prep` on the read model); until then, say so before the operator finds out. */}
					{mode === "edit" && (
						<p className="rounded-lg border border-amber-500/40 bg-amber-500/10 px-3 py-2.5 text-sm text-amber-700 dark:text-amber-400">
							{m.library_edit_overwrites()}
						</p>
					)}
					{error && (
						<p
							role="alert"
							className="rounded-lg border border-destructive/40 bg-destructive/10 px-3 py-2.5 text-sm text-destructive"
						>
							{error}
						</p>
					)}
					<div className="flex flex-wrap gap-2">
						<Button type="submit" disabled={isSaving || !form.title.trim()}>
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

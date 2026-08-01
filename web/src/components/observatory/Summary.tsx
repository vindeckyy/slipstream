import type { ComponentProps, ReactNode } from "react";
import {
	Card,
	CardContent,
	CardDescription,
	CardHeader,
	CardTitle,
} from "@/components/ui/card";
import {
	Table,
	TableBody,
	TableCell,
	TableHead,
	TableHeader,
	TableRow,
} from "@/components/ui/table";
import { cn } from "@/lib/utils";
import { StateNotice } from "./state";
import type {
	ObservatoryState,
	StateCopy,
	SummaryColumn,
	SummaryPoint,
} from "./types";

const ALIGN_CLASS = {
	left: "text-left",
	center: "text-center",
	right: "text-right",
} as const;

function cellValue<Row>(
	column: SummaryColumn<Row>,
	row: Row,
	index: number,
): ReactNode {
	if (column.render) return column.render(row, index);
	const value = (row as Record<string, ReactNode>)[column.key];
	return value ?? "N/A";
}

export interface SummaryTableProps<Row>
	extends Omit<ComponentProps<typeof Card>, "title" | "children"> {
	rows?: readonly Row[];
	columns: readonly SummaryColumn<Row>[];
	title?: ReactNode;
	description?: ReactNode;
	caption?: ReactNode;
	ariaLabel?: string;
	state?: ObservatoryState;
	stateCopy?: StateCopy;
	emptyMessage?: ReactNode;
	onRetry?: () => void;
	retryLabel?: ReactNode;
}

/** A generic typed table surface for small operator summaries. */
export function SummaryTable<Row>({
	rows = [],
	columns,
	title,
	description,
	caption,
	ariaLabel,
	state = "ready",
	stateCopy,
	emptyMessage,
	onRetry,
	retryLabel,
	className,
	...props
}: SummaryTableProps<Row>) {
	const effectiveState: ObservatoryState =
		rows.length === 0 && state === "ready" ? "empty" : state;
	const nonReadyState = effectiveState === "ready" ? null : effectiveState;
	const effectiveCopy =
		emptyMessage === undefined
			? stateCopy
			: { ...stateCopy, empty: emptyMessage };

	return (
		<Card
			className={cn(
				"min-w-0",
				effectiveState === "stale" && "ring-warning/30",
				effectiveState === "error" && "ring-destructive/30",
				className,
			)}
			aria-busy={effectiveState === "loading" || undefined}
			data-state={effectiveState}
			{...props}
		>
			{title || description ? (
				<CardHeader className="space-y-1">
					{title ? <CardTitle className="text-base">{title}</CardTitle> : null}
					{description ? (
						<CardDescription>{description}</CardDescription>
					) : null}
				</CardHeader>
			) : null}
			{nonReadyState && rows.length === 0 ? (
				<CardContent>
					<StateNotice
						state={nonReadyState}
						stateCopy={effectiveCopy}
						onRetry={onRetry}
						retryLabel={retryLabel}
					/>
				</CardContent>
			) : (
				<CardContent className="space-y-3" flush>
					{nonReadyState ? (
						<div className="px-4 pt-4 sm:px-6 sm:pt-6">
							<StateNotice
								state={nonReadyState}
								stateCopy={effectiveCopy}
								compact
								onRetry={onRetry}
								retryLabel={retryLabel}
							/>
						</div>
					) : null}
					<Table aria-label={ariaLabel}>
						{caption ? (
							<caption className="px-4 py-3 text-left text-xs text-muted-foreground sm:px-6">
								{caption}
							</caption>
						) : null}
						<TableHeader>
							<TableRow className="hover:bg-transparent">
								{columns.map((column) => (
									<TableHead
										key={column.key}
										className={cn(
											ALIGN_CLASS[column.align ?? "left"],
											column.headerClassName,
										)}
									>
										{column.header}
									</TableHead>
								))}
							</TableRow>
						</TableHeader>
						<TableBody>
							{rows.map((row, index) => (
								<TableRow key={index}>
									{columns.map((column) => (
										<TableCell
											key={column.key}
											className={cn(
												ALIGN_CLASS[column.align ?? "left"],
												column.className,
											)}
										>
											{cellValue(column, row, index)}
										</TableCell>
									))}
								</TableRow>
							))}
						</TableBody>
					</Table>
				</CardContent>
			)}
		</Card>
	);
}

export interface SummaryChartProps
	extends Omit<ComponentProps<typeof Card>, "title" | "children"> {
	data?: readonly SummaryPoint[];
	title?: ReactNode;
	description?: ReactNode;
	valueFormatter?: (value: number, point: SummaryPoint) => ReactNode;
	maxValue?: number;
	ariaLabel?: string;
	state?: ObservatoryState;
	stateCopy?: StateCopy;
	emptyMessage?: ReactNode;
	onRetry?: () => void;
	retryLabel?: ReactNode;
}

/** A CSS bar summary that stays deterministic in SSR and Storybook. */
export function SummaryChart({
	data = [],
	title,
	description,
	valueFormatter = (value) => String(value),
	maxValue,
	ariaLabel,
	state = "ready",
	stateCopy,
	emptyMessage,
	onRetry,
	retryLabel,
	className,
	...props
}: SummaryChartProps) {
	const effectiveState: ObservatoryState =
		data.length === 0 && state === "ready" ? "empty" : state;
	const nonReadyState = effectiveState === "ready" ? null : effectiveState;
	const effectiveCopy =
		emptyMessage === undefined
			? stateCopy
			: { ...stateCopy, empty: emptyMessage };
	const finiteValues = data.map((point) =>
		Number.isFinite(point.value) ? Math.max(0, point.value) : 0,
	);
	const domainMax = Math.max(
		1,
		maxValue && Number.isFinite(maxValue) ? maxValue : 0,
		...finiteValues,
	);

	return (
		<Card
			className={cn(
				"min-w-0",
				effectiveState === "stale" && "ring-warning/30",
				effectiveState === "error" && "ring-destructive/30",
				className,
			)}
			aria-busy={effectiveState === "loading" || undefined}
			data-state={effectiveState}
			{...props}
		>
			{title || description ? (
				<CardHeader className="space-y-1">
					{title ? <CardTitle className="text-base">{title}</CardTitle> : null}
					{description ? (
						<CardDescription>{description}</CardDescription>
					) : null}
				</CardHeader>
			) : null}
			<CardContent className="space-y-3">
				{nonReadyState && data.length === 0 ? (
					<StateNotice
						state={nonReadyState}
						stateCopy={effectiveCopy}
						onRetry={onRetry}
						retryLabel={retryLabel}
					/>
				) : null}
				{nonReadyState && data.length > 0 ? (
					<StateNotice
						state={nonReadyState}
						stateCopy={effectiveCopy}
						compact
						onRetry={onRetry}
						retryLabel={retryLabel}
					/>
				) : null}
				{data.length > 0 ? (
					<figure aria-label={ariaLabel}>
						<div className="space-y-3">
							{data.map((point, index) => {
								const value = finiteValues[index] ?? 0;
								const width = `${(value / domainMax) * 100}%`;
								return (
									<div
										key={point.id}
										className="grid grid-cols-[minmax(0,1fr)_auto] items-center gap-x-3 gap-y-1"
									>
										<div className="min-w-0">
											<div className="flex items-baseline justify-between gap-2 text-xs">
												<span className="truncate font-medium">
													{point.label}
												</span>
												<span className="shrink-0 tabular-nums text-muted-foreground">
													{valueFormatter(value, point)}
												</span>
											</div>
											<div
												className="mt-1.5 h-2 overflow-hidden rounded-full bg-muted/60"
												aria-hidden="true"
											>
												<div
													className="h-full rounded-full bg-primary motion-reduce:transition-none"
													style={{
														width,
														...(point.color
															? { backgroundColor: point.color }
															: {}),
													}}
												/>
											</div>
										</div>
										{point.detail ? (
											<span className="text-xs text-muted-foreground">
												{point.detail}
											</span>
										) : null}
									</div>
								);
							})}
						</div>
					</figure>
				) : null}
			</CardContent>
		</Card>
	);
}

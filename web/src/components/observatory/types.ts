import type { Key, ReactNode } from "react";

export const OBSERVATORY_STATES = [
	"ready",
	"loading",
	"stale",
	"error",
	"empty",
] as const;

export type ObservatoryState = (typeof OBSERVATORY_STATES)[number];
export type NonReadyObservatoryState = Exclude<ObservatoryState, "ready">;

export type StateCopy = Partial<Record<NonReadyObservatoryState, ReactNode>>;

export type StatusIndicatorStatus =
	| "healthy"
	| "degraded"
	| "offline"
	| "unknown";

export type AlertVariant = "info" | "success" | "warning" | "error";

export type TimelineTone =
	| "neutral"
	| "info"
	| "success"
	| "warning"
	| "danger";

export interface TimelineEvent {
	id: Key;
	title: ReactNode;
	description?: ReactNode;
	timestamp?: ReactNode;
	tone?: TimelineTone;
	icon?: ReactNode;
}

export interface SummaryColumn<Row> {
	key: string;
	header: ReactNode;
	render?: (row: Row, index: number) => ReactNode;
	align?: "left" | "center" | "right";
	className?: string;
	headerClassName?: string;
}

export interface SummaryPoint {
	id: Key;
	label: ReactNode;
	value: number;
	detail?: ReactNode;
	color?: string;
}

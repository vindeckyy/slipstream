import { createLazyFileRoute } from "@tanstack/react-router";
import { SectionStats } from "@/sections/Stats";

export const Route = createLazyFileRoute("/stats")({
	component: SectionStats,
});

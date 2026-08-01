import { createLazyFileRoute } from "@tanstack/react-router";
import { SectionLogs } from "@/sections/Logs";

export const Route = createLazyFileRoute("/logs")({
	component: SectionLogs,
});

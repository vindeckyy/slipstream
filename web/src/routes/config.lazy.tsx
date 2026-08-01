import { createLazyFileRoute } from "@tanstack/react-router";
import { SectionConfig } from "@/sections/Config";

export const Route = createLazyFileRoute("/config")({
	component: SectionConfig,
});

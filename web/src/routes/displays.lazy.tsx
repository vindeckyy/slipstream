import { createLazyFileRoute } from "@tanstack/react-router";
import { SectionDisplays } from "@/sections/Displays";

export const Route = createLazyFileRoute("/displays")({
	component: SectionDisplays,
});

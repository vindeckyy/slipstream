import { createLazyFileRoute } from "@tanstack/react-router";
import { SectionPlugin } from "@/sections/Plugins";

export const Route = createLazyFileRoute("/plugins/$pluginId/$")({
	component: SectionPlugin,
});

import { createFileRoute } from "@tanstack/react-router";
import { SectionSetup } from "@/sections/Setup";

export const Route = createFileRoute("/setup")({
	validateSearch: (search: Record<string, unknown>): { next?: string } => ({
		next: typeof search.next === "string" ? search.next : undefined,
	}),
	component: RouteComponent,
});

function RouteComponent() {
	const { next } = Route.useSearch();
	return <SectionSetup next={next} />;
}

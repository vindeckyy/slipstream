import { createFileRoute } from "@tanstack/react-router";
import { SectionSessions } from "@/sections/Sessions";

export const Route = createFileRoute("/sessions")({ component: SectionSessions });

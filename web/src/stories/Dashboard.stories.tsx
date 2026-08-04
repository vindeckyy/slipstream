import type { Meta, StoryObj } from "@storybook/react-vite";
import {
	createMemoryHistory,
	createRootRoute,
	createRoute,
	createRouter,
	Outlet,
	RouterProvider,
} from "@tanstack/react-router";
import type { ComponentProps } from "react";
import { DashboardView } from "@/sections/Dashboard/view";
import { statusActive, statusGrace, statusIdle } from "./lib/fixtures";

const statusUnpaired = {
	...statusIdle,
	paired_clients: 0,
	native_paired_clients: 0,
	pin_pending: false,
};

function DashboardStory(args: ComponentProps<typeof DashboardView>) {
	const rootRoute = createRootRoute({ component: () => <Outlet /> });
	const dashboardRoute = createRoute({
		getParentRoute: () => rootRoute,
		path: "/",
		component: () => <DashboardView {...args} />,
	});
	const sessionsRoute = createRoute({
		getParentRoute: () => rootRoute,
		path: "/sessions",
		component: () => null,
	});
	const hostRoute = createRoute({
		getParentRoute: () => rootRoute,
		path: "/host",
		component: () => null,
	});
	const pairingRoute = createRoute({
		getParentRoute: () => rootRoute,
		path: "/pairing",
		component: () => null,
	});
	const pinRoute = createRoute({
		getParentRoute: () => rootRoute,
		path: "/pin",
		component: () => null,
	});
	const libraryRoute = createRoute({
		getParentRoute: () => rootRoute,
		path: "/library",
		component: () => null,
	});
	const appsRoute = createRoute({
		getParentRoute: () => rootRoute,
		path: "/apps",
		component: () => null,
	});
	const displaysRoute = createRoute({
		getParentRoute: () => rootRoute,
		path: "/displays",
		component: () => null,
	});
	const router = createRouter({
		routeTree: rootRoute.addChildren([
			dashboardRoute,
			sessionsRoute,
			hostRoute,
			pairingRoute,
			pinRoute,
			libraryRoute,
			appsRoute,
			displaysRoute,
		]),
		history: createMemoryHistory({ initialEntries: ["/"] }),
	});
	return <RouterProvider router={router} />;
}

const meta = {
	title: "Pages/Dashboard",
	component: DashboardView,
	render: (args) => <DashboardStory {...args} />,
	args: {
		onStopSession: () => {},
		onRequestIdr: () => {},
		onEndGame: () => {},
		isStopping: false,
		isRequestingIdr: false,
		isEndingGame: false,
	},
} satisfies Meta<typeof DashboardView>;

export default meta;
type Story = StoryObj<typeof meta>;

export const ActiveSession: Story = {
	args: { status: { data: statusActive, isLoading: false, error: null } },
};

export const Overview: Story = {
	args: {
		status: {
			data: {
				...statusIdle,
				paired_clients: 3,
				native_paired_clients: 1,
				pin_pending: false,
			},
			isLoading: false,
			error: null,
		},
	},
};

export const Session: Story = {
	args: {
		status: {
			data: {
				...statusActive,
				active_sessions: 2,
				games: [
					...statusActive.games,
					{
						session_id: 2,
						client: "studio-deck",
						app_id: "custom:retroarch",
						title: "RetroArch",
						store: "custom",
						plane: "gamestream",
						state: "launching",
					},
				],
			},
			isLoading: false,
			error: null,
		},
	},
};

export const Idle: Story = {
	args: { status: { data: statusIdle, isLoading: false, error: null } },
};

/** A game whose client vanished: the host closes it when the countdown runs out. */
export const GameWaitingForItsClient: Story = {
	args: { status: { data: statusGrace, isLoading: false, error: null } },
};

/** No paired clients yet: the skippable first-run checklist. */
export const FirstRun: Story = {
	args: {
		status: { data: statusUnpaired, isLoading: false, error: null },
		gettingStarted: {
			pinPending: false,
			preflightReady: true,
			onDismiss: () => {},
		},
	},
};

/** First-run with a Moonlight PIN waiting to be entered. */
export const FirstRunPendingPin: Story = {
	args: {
		status: {
			data: { ...statusUnpaired, pin_pending: true },
			isLoading: false,
			error: null,
		},
		gettingStarted: {
			pinPending: true,
			preflightReady: true,
			onDismiss: () => {},
		},
	},
};

/** First-run when host preflight has a blocked check. */
export const FirstRunPreflightBlocked: Story = {
	args: {
		status: { data: statusUnpaired, isLoading: false, error: null },
		gettingStarted: {
			pinPending: false,
			preflightReady: false,
			onDismiss: () => {},
		},
	},
};

/** First-run when preflight has not loaded or failed: no ready/blocked badge. */
export const FirstRunPreflightUnknown: Story = {
	args: {
		status: { data: statusUnpaired, isLoading: false, error: null },
		gettingStarted: {
			pinPending: false,
			preflightReady: null,
			onDismiss: () => {},
		},
	},
};

import type { Meta, StoryObj } from "@storybook/react-vite";
import {
	createMemoryHistory,
	createRootRoute,
	createRoute,
	createRouter,
	RouterProvider,
} from "@tanstack/react-router";
import type { ReactNode } from "react";
import { AppShell } from "@/components/app-shell";

// AppShell is built from TanStack Router <Link>s, so it needs a router context.
// We stand up a throwaway in-memory router whose routes mirror the nav targets
// (so links resolve + the active highlight works) and render the shell from the
// root route. No loaders/data — purely for designing the chrome offline.
function ShellHarness({
	initialPath,
	children = <OverviewFixture />,
}: {
	initialPath: string;
	children?: ReactNode;
}) {
	const rootRoute = createRootRoute({
		component: () => <AppShell>{children}</AppShell>,
	});

	const navPaths = [
		"/",
		"/pin",
		"/apps",
		"/config",
		"/troubleshoot",
		"/host",
		"/displays",
		"/stats",
		"/automation",
		"/plugins",
		"/settings",
	];
	const navRoutes = navPaths.map((path) =>
		createRoute({
			getParentRoute: () => rootRoute,
			path,
			component: () => null,
		}),
	);
	// Splat so any other <Link> target still resolves without throwing.
	const splat = createRoute({
		getParentRoute: () => rootRoute,
		path: "$",
		component: () => null,
	});

	const router = createRouter({
		routeTree: rootRoute.addChildren([...navRoutes, splat]),
		history: createMemoryHistory({ initialEntries: [initialPath] }),
	});

	return <RouterProvider router={router} />;
}

function OverviewFixture() {
	return (
		<div className="space-y-5">
			<header className="space-y-1">
				<p className="text-xs font-medium uppercase tracking-[0.08em] text-primary">
					Overview
				</p>
				<h1 className="text-2xl font-semibold tracking-tight">
					Your host at a glance
				</h1>
				<p className="max-w-prose text-sm text-muted-foreground">
					A quiet host with the controls you need close by.
				</p>
			</header>
			<div className="grid gap-3 sm:grid-cols-3">
				<FixtureMetric
					label="Stream"
					value="Ready"
					detail="Video and audio idle"
				/>
				<FixtureMetric
					label="Clients"
					value="3 paired"
					detail="2 native, 1 GameStream"
				/>
				<FixtureMetric
					label="PIN"
					value="Available"
					detail="Ready for a new device"
				/>
			</div>
		</div>
	);
}

function SessionFixture() {
	return (
		<div className="space-y-5">
			<header className="space-y-1">
				<p className="text-xs font-medium uppercase tracking-[0.08em] text-primary">
					Session
				</p>
				<h1 className="text-2xl font-semibold tracking-tight">
					Living room stream
				</h1>
				<p className="max-w-prose text-sm text-muted-foreground">
					The host is sending a high-refresh session to the living room TV.
				</p>
			</header>
			<section className="rounded-xl border border-border/70 bg-card p-4 shadow-sm sm:p-5">
				<div className="flex flex-wrap items-start justify-between gap-3">
					<div>
						<p className="text-sm font-semibold">Hades</p>
						<p className="mt-1 text-sm text-muted-foreground">
							Living room TV, Native
						</p>
					</div>
					<span className="rounded-full bg-success/15 px-2.5 py-1 text-xs font-medium text-success">
						Streaming
					</span>
				</div>
				<dl className="mt-5 grid grid-cols-2 gap-4 sm:grid-cols-4">
					<FixtureMetric label="Resolution" value="5120 x 1440" />
					<FixtureMetric label="Refresh" value="240 fps" />
					<FixtureMetric label="Codec" value="HEVC" />
					<FixtureMetric label="Bitrate" value="150 Mbps" />
				</dl>
			</section>
		</div>
	);
}

function MonitorFixture() {
	return (
		<div className="space-y-5">
			<header className="space-y-1">
				<p className="text-xs font-medium uppercase tracking-[0.08em] text-primary">
					Monitor
				</p>
				<h1 className="text-2xl font-semibold tracking-tight">Stream health</h1>
				<p className="max-w-prose text-sm text-muted-foreground">
					Live measurements from the current session, ready for a quick check.
				</p>
			</header>
			<div className="grid gap-3 sm:grid-cols-2 lg:grid-cols-4">
				<FixtureMetric
					label="Frame pacing"
					value="Stable"
					detail="0 dropped frames"
				/>
				<FixtureMetric
					label="Latency p50"
					value="1.3 ms"
					detail="Capture to send"
				/>
				<FixtureMetric
					label="Packet loss"
					value="0.02%"
					detail="2 recovered by FEC"
				/>
				<FixtureMetric label="Encoder" value="NVENC" detail="RTX 4090" />
			</div>
		</div>
	);
}

function FixtureMetric({
	label,
	value,
	detail,
}: {
	label: string;
	value: string;
	detail?: string;
}) {
	return (
		<div className="rounded-xl border border-border/70 bg-card p-4 shadow-sm">
			<dt className="text-xs font-medium text-muted-foreground">{label}</dt>
			<dd className="mt-2 text-lg font-semibold tracking-tight">{value}</dd>
			{detail && <p className="mt-1 text-xs text-muted-foreground">{detail}</p>}
		</div>
	);
}

const meta = {
	title: "Shell/AppShell",
	component: AppShell,
	parameters: { layout: "fullscreen" },
	// AppShell requires `children`; the harness supplies the real content, so this
	// placeholder just satisfies the arg type.
	args: { children: null },
} satisfies Meta<typeof AppShell>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Dashboard: Story = {
	render: () => <ShellHarness initialPath="/" />,
};

export const HostActive: Story = {
	render: () => (
		<ShellHarness initialPath="/host">
			<SessionFixture />
		</ShellHarness>
	),
};

export const Overview: Story = {
	render: () => (
		<ShellHarness initialPath="/">
			<OverviewFixture />
		</ShellHarness>
	),
};

export const Session: Story = {
	render: () => (
		<ShellHarness initialPath="/host">
			<SessionFixture />
		</ShellHarness>
	),
};

export const Monitor: Story = {
	render: () => (
		<ShellHarness initialPath="/stats">
			<MonitorFixture />
		</ShellHarness>
	),
};

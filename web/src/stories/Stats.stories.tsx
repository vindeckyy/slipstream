import type { Meta, StoryObj } from "@storybook/react-vite";
import type { StatsStatus } from "@/api/gen/model/statsStatus";
import { CaptureControlCard } from "@/sections/Stats/CaptureControl";
import { DetailCard } from "@/sections/Stats/Detail";
import { LiveCard } from "@/sections/Stats/LiveCard";
import { RecordingsCard } from "@/sections/Stats/Recordings";
import { StatsView } from "@/sections/Stats/view";
import { captureDetail, captureMetas, statsStatusIdle } from "./lib/fixtures";

const noop = () => {};
const idle = { isLoading: false, error: null, refetch: noop };
const statsStatusLive: StatsStatus = {
	...statsStatusIdle,
	armed: true,
	sample_count: captureDetail.samples.length,
	started_unix_ms: captureMetas[0]?.started_unix_ms ?? 0,
	elapsed_ms: captureMetas[0]?.duration_ms ?? 0,
};

// Renders the REAL page layout (StatsView) — the same component index.tsx uses — with the pure
// cards + mock state in its slots, so there's no duplicated composition to drift.
const meta = {
	title: "Pages/Stats",
	component: StatsView,
	parameters: { layout: "padded" },
} satisfies Meta<typeof StatsView>;

export default meta;
type Story = StoryObj<typeof meta>;

// A finished run open in the detail view: recordings table populated and the full graph set
// (latency stack · throughput · loss/FEC) rendered from a deterministic fixture series — no live
// host or capture needed.
export const Recording: Story = {
	args: {
		control: (
			<CaptureControlCard
				status={{ data: statsStatusIdle, ...idle }}
				onStart={noop}
				onStop={noop}
				isStarting={false}
				isStopping={false}
			/>
		),
		live: null,
		recordings: (
			<RecordingsCard
				recordings={{ data: captureMetas, ...idle }}
				selectedId={captureMetas[0]?.id ?? null}
				onSelect={noop}
				onDownload={noop}
				onDelete={noop}
				isDeleting={false}
			/>
		),
		detail: (
			<DetailCard detail={{ data: captureDetail, ...idle }} onClose={noop} />
		),
	},
};

const liveStoryArgs = {
	control: (
		<CaptureControlCard
			status={{ data: statsStatusLive, ...idle }}
			onStart={noop}
			onStop={noop}
			isStarting={false}
			isStopping={false}
		/>
	),
	live: <LiveCard live={{ data: captureDetail, ...idle }} />,
	recordings: (
		<RecordingsCard
			recordings={{ data: captureMetas, ...idle }}
			selectedId={null}
			onSelect={noop}
			onDownload={noop}
			onDelete={noop}
			isDeleting={false}
		/>
	),
	detail: null,
};

export const Monitor: Story = {
	args: liveStoryArgs,
};

export const LiveCapture: Story = {
	args: liveStoryArgs,
};

export const Empty: Story = {
	args: {
		control: (
			<CaptureControlCard
				status={{ data: statsStatusIdle, ...idle }}
				onStart={noop}
				onStop={noop}
				isStarting={false}
				isStopping={false}
			/>
		),
		live: null,
		recordings: (
			<RecordingsCard
				recordings={{ data: [], ...idle }}
				selectedId={null}
				onSelect={noop}
				onDownload={noop}
				onDelete={noop}
				isDeleting={false}
			/>
		),
		detail: null,
	},
};

import type { FC } from "react";
import { useGetHostInfo } from "@/api/gen/host/host";
import { useLocale } from "@/lib/i18n";
import { ConflictsCard } from "./ConflictsCard";
import { GpuSection } from "./GpuCard";
import { UpdateSection } from "./UpdateCard";
import { HostView } from "./view";

export const SectionHost: FC = () => {
	useLocale();
	const host = useGetHostInfo();

	return (
		<HostView
			host={host}
			conflicts={<ConflictsCard />}
			gpu={<GpuSection />}
			update={<UpdateSection />}
		/>
	);
};

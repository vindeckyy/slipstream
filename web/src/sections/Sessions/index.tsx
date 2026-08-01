import { useQueryClient } from "@tanstack/react-query";
import { toast } from "@unom/ui/toast";
import type { FC } from "react";
import { getGetStatusQueryKey, useGetStatus } from "@/api/gen/host/host";
import { useGetLibrary } from "@/api/gen/library/library";
import type { ActiveGame } from "@/api/gen/model/activeGame";
import {
	useEndGame,
	useRequestIdr,
	useStopSession,
} from "@/api/gen/session/session";
import { apiErrorMessage } from "@/lib/errors";
import { useLocale } from "@/lib/i18n";
import { m } from "@/paraglide/messages";
import { SessionsView } from "./view";

export const SectionSessions: FC = () => {
	useLocale();
	const qc = useQueryClient();
	const status = useGetStatus({
		query: {
			refetchInterval: (q) =>
				q.state.data?.video_streaming || (q.state.data?.games?.length ?? 0) > 0
					? 2_000
					: 15_000,
		},
	});
	const library = useGetLibrary(undefined, {
		query: { staleTime: 5 * 60_000 },
	});
	const stop = useStopSession();
	const idr = useRequestIdr();
	const endGame = useEndGame();

	const invalidate = () =>
		qc.invalidateQueries({ queryKey: getGetStatusQueryKey() });

	const failed = (fallback: string) => (e: unknown) =>
		toast.error(apiErrorMessage(e) ?? fallback);

	const activeSessions = status.data?.active_sessions ?? 0;
	const confirmStopAll = (): boolean => {
		if (activeSessions <= 1) return true;
		return confirm(m.action_stop_session_all_confirm({ count: activeSessions }));
	};

	const onEndGame = (game: ActiveGame) => {
		const games = status.data?.games ?? [];
		if (game.state === "grace") {
			const waiting = games.filter((g) => g.state === "grace").length;
			if (
				!game.app_id &&
				waiting > 1 &&
				!confirm(m.games_end_all_waiting_confirm({ count: waiting }))
			)
				return;
			endGame.mutate(
				{ data: { app_id: game.app_id ?? null } },
				{ onSuccess: invalidate, onError: failed(m.games_end_failed()) },
			);
			return;
		}
		if (!confirmStopAll()) return;
		stop.mutate(undefined, {
			onSuccess: invalidate,
			onError: failed(m.action_stop_failed()),
		});
	};

	return (
		<SessionsView
			status={status}
			library={library.data}
			onStopSession={() => {
				if (!confirmStopAll()) return;
				stop.mutate(undefined, {
					onSuccess: invalidate,
					onError: failed(m.action_stop_failed()),
				});
			}}
			onRequestIdr={() =>
				idr.mutate(undefined, { onError: failed(m.action_idr_failed()) })
			}
			onEndGame={onEndGame}
			isStopping={stop.isPending}
			isRequestingIdr={idr.isPending}
			isEndingGame={endGame.isPending || stop.isPending}
		/>
	);
};

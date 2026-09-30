export const BOARD_LIVE_WINDOW_MS = 10_000;

export type BoardConnectionState = "connecting" | "live" | "offline" | "stale" | "error";

export function boardConnectionState({
  hasData,
  isError,
  isPaused,
  dataUpdatedAt,
  now,
}: {
  hasData: boolean;
  isError: boolean;
  isPaused: boolean;
  dataUpdatedAt: number;
  now: number;
}): BoardConnectionState {
  if (isPaused) return "offline";
  if (isError) return "error";
  if (!hasData) return "connecting";
  if (dataUpdatedAt <= 0 || now - dataUpdatedAt > BOARD_LIVE_WINDOW_MS) return "stale";
  return "live";
}

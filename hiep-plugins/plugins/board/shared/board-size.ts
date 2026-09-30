import { defineSettings } from "@getpaseo/plugin";
import { z } from "zod";

export const BOARD_SIZE_DEFAULT = 100;
export const BOARD_SIZE_MIN = 10;
export const BOARD_SIZE_MAX = 200;
export const BOARD_SIZE_STEP = 10;
// 100% renders what the original 70% did; other sizes scale from that base.
const BASE_SCALE = 0.7;

export const boardSize = defineSettings({
  id: "board-size",
  scope: "host",
  version: 1,
  schema: z.object({
    size: z.number().int().min(BOARD_SIZE_MIN).max(BOARD_SIZE_MAX).default(BOARD_SIZE_DEFAULT),
  }),
});

export function clampBoardSize(size: number): number {
  return Math.max(BOARD_SIZE_MIN, Math.min(BOARD_SIZE_MAX, size));
}

export function boardScale(size: number): number {
  return (size / 100) * BASE_SCALE;
}

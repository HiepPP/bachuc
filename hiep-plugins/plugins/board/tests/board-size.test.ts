import assert from "node:assert/strict";
import test from "node:test";
import {
  BOARD_SIZE_DEFAULT,
  BOARD_SIZE_MAX,
  BOARD_SIZE_MIN,
  boardScale,
  boardSize,
  clampBoardSize,
} from "../shared/board-size";

test("100% renders the former 70% and other sizes scale from it", () => {
  assert.equal(BOARD_SIZE_DEFAULT, 100);
  assert.equal(boardScale(100), 0.7);
  assert.equal(boardScale(50), 0.35);
  assert.equal(boardScale(200), 1.4);
});

test("stored size defaults to 100%, survives a JSON round trip, and stays in range", () => {
  assert.deepEqual(boardSize.schema.parse({}), { size: 100 });
  const restored = boardSize.schema.parse(JSON.parse(JSON.stringify({ size: 70 })));
  assert.deepEqual(restored, { size: 70 });
  assert.throws(() => boardSize.schema.parse({ size: BOARD_SIZE_MAX + 10 }));
  assert.throws(() => boardSize.schema.parse({ size: BOARD_SIZE_MIN - 10 }));
  assert.equal(clampBoardSize(BOARD_SIZE_MAX + 10), BOARD_SIZE_MAX);
  assert.equal(clampBoardSize(0), BOARD_SIZE_MIN);
});

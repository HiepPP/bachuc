export const CHANGE_BAR_SQUARES = 7;

export type ChangeBarSquare = "add" | "remove";

/** Splits the squares by the added share; a side that changed keeps at least one square. */
export function changeBar(added: number, removed: number): ChangeBarSquare[] {
  const total = added + removed;
  if (total === 0) return [];
  let addSquares = Math.round((added / total) * CHANGE_BAR_SQUARES);
  if (added > 0 && removed > 0) {
    addSquares = Math.min(CHANGE_BAR_SQUARES - 1, Math.max(1, addSquares));
  }
  return Array.from({ length: CHANGE_BAR_SQUARES }, (_, index) =>
    index < addSquares ? "add" : "remove",
  );
}

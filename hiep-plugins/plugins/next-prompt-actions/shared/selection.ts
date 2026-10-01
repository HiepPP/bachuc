// Pure selection rules for structured suggestion blocks.
export type Group =
  | { kind: "choice"; index: number; ids: string[] }
  | { kind: "together"; set: number; ids: string[] }
  | { kind: "follow-up" | "alone"; ids: string[] };

// Exclusive groups become radio groups. Other suggestions linked by allowed combinations share a
// group: "follow-up" when a linked combination includes an exclusive choice, else "together".
// Suggestions in no combination are grouped as "alone".
export function layout(order: string[], groups: string[][], combos: string[][]): Group[] {
  const exclusive = new Set(groups.flat());
  const result: Group[] = groups.map((ids, index) => ({ kind: "choice", index, ids }));
  const seen = new Set<string>();
  const together: { set: number }[] = [];
  for (const id of order) {
    if (exclusive.has(id) || seen.has(id) || !combos.some((c) => c.includes(id))) continue;
    const ids = [id];
    seen.add(id);
    for (let i = 0; i < ids.length; i++)
      for (const combo of combos)
        if (combo.includes(ids[i]))
          for (const other of combo)
            if (!exclusive.has(other) && !seen.has(other)) {
              seen.add(other);
              ids.push(other);
            }
    const follow = combos.some(
      (c) => c.some((x) => ids.includes(x)) && c.some((x) => exclusive.has(x)),
    );
    if (follow) result.push({ kind: "follow-up", ids });
    else {
      const group = { kind: "together" as const, set: 0, ids };
      together.push(group);
      result.push(group);
    }
  }
  if (together.length > 1) together.forEach((group, index) => (group.set = index + 1));
  const alone = order.filter((id) => !exclusive.has(id) && !combos.some((c) => c.includes(id)));
  if (alone.length) result.push({ kind: "alone", ids: alone });
  return result;
}

// A radio replaces its exclusive group's selection; the result must fit one declared combination.
export function reachable(
  selected: string[],
  id: string,
  groups: string[][],
  combos: string[][],
): boolean {
  const group = groups.find((g) => g.includes(id)) ?? [];
  const next = [...selected.filter((x) => !group.includes(x)), id];
  return next.length === 1 || combos.some((combo) => next.every((x) => combo.includes(x)));
}

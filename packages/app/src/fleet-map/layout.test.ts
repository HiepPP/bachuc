import { describe, expect, it } from "vitest";
import { computeFleetLayout, type FleetLayoutNode } from "./layout";

function node(
  id: string,
  workspaceId: string,
  createdAt: number,
  parentId: string | null = null,
): FleetLayoutNode {
  return { id, workspaceId, parentId, createdAt };
}

function pos(layout: ReturnType<typeof computeFleetLayout>, id: string) {
  const position = layout.positions.get(id);
  if (!position) throw new Error(`missing position for ${id}`);
  return position;
}

describe("computeFleetLayout", () => {
  it("returns empty output for empty input", () => {
    const layout = computeFleetLayout([]);
    expect(layout.positions.size).toBe(0);
    expect(layout.edges).toEqual([]);
    expect(layout.clusters).toEqual([]);
    expect(layout.boundingRadius).toBe(0);
  });

  it("gives deep-equal output for the same input, whatever the input order", () => {
    const nodes = [
      node("a", "w1", 1),
      node("b", "w1", 2, "a"),
      node("c", "w2", 3),
      node("d", "w2", 3),
      node("e", "w3", 0),
    ];
    const first = computeFleetLayout(nodes);
    expect(computeFleetLayout(nodes)).toEqual(first);
    expect(computeFleetLayout(nodes.toReversed())).toEqual(first);
  });

  it("accepts Date and number createdAt values alike", () => {
    const withNumbers = [node("a", "w", 5), node("b", "w", 9)];
    const withDates = withNumbers.map((n) => ({ ...n, createdAt: new Date(n.createdAt) }));
    expect(computeFleetLayout(withDates)).toEqual(computeFleetLayout(withNumbers));
  });

  it("keeps existing positions in a workspace when a node is added", () => {
    const before = [
      node("a", "w1", 1),
      node("b", "w1", 2),
      node("c", "w1", 3, "a"),
      node("x", "w2", 5),
    ];
    const layoutBefore = computeFleetLayout(before);
    for (const added of [node("d", "w1", 10), node("e", "w1", 11, "a"), node("f", "w1", 12, "d")]) {
      const layoutAfter = computeFleetLayout([...before, added]);
      for (const n of before.filter((entry) => entry.workspaceId === "w1")) {
        expect(pos(layoutAfter, n.id)).toEqual(pos(layoutBefore, n.id));
      }
    }
  });

  it("places a subagent above its parent and links each parent", () => {
    const layout = computeFleetLayout([
      node("root", "w", 1),
      node("child", "w", 2, "root"),
      node("grandchild", "w", 3, "child"),
      node("other", "w", 4),
    ]);
    expect(pos(layout, "child").y).toBeGreaterThan(pos(layout, "root").y);
    expect(pos(layout, "grandchild").y).toBeGreaterThan(pos(layout, "child").y);
    expect(pos(layout, "other").y).toBe(pos(layout, "root").y);
    expect(layout.edges).toEqual(
      expect.arrayContaining([
        ["root", "child"],
        ["child", "grandchild"],
      ]),
    );
    expect(layout.edges).toHaveLength(2);
  });

  it("orbits a subagent close to its parent on the plane", () => {
    const layout = computeFleetLayout([node("p", "w", 1), node("c", "w", 2, "p")]);
    const parent = pos(layout, "p");
    const child = pos(layout, "c");
    const planar = Math.hypot(child.x - parent.x, child.z - parent.z);
    expect(planar).toBeGreaterThan(0);
    expect(planar).toBeLessThan(5);
  });

  it("separates workspace clusters by more than the sum of their radii", () => {
    const nodes: FleetLayoutNode[] = [];
    for (let w = 0; w < 6; w += 1) {
      for (let i = 0; i < 3 + w * 4; i += 1) {
        nodes.push(node(`w${w}-n${i}`, `w${w}`, w * 100 + i));
      }
    }
    const { clusters } = computeFleetLayout(nodes);
    expect(clusters).toHaveLength(6);
    for (let i = 0; i < clusters.length; i += 1) {
      for (let j = i + 1; j < clusters.length; j += 1) {
        const a = clusters[i];
        const b = clusters[j];
        expect(Math.hypot(a.x - b.x, a.z - b.z)).toBeGreaterThan(a.radius + b.radius);
      }
    }
  });

  it("keeps every node inside its cluster ring and inside the bounding radius", () => {
    const nodes = [
      node("a", "w1", 1),
      node("b", "w1", 2, "a"),
      node("c", "w1", 3),
      node("d", "w2", 4),
      node("e", "w2", 5, "d"),
    ];
    const layout = computeFleetLayout(nodes);
    for (const n of nodes) {
      const p = pos(layout, n.id);
      const cluster = layout.clusters.find((c) => c.workspaceId === n.workspaceId);
      if (!cluster) throw new Error("missing cluster");
      expect(Math.hypot(p.x - cluster.x, p.z - cluster.z)).toBeLessThanOrEqual(cluster.radius);
      expect(Math.hypot(p.x, p.y, p.z)).toBeLessThanOrEqual(layout.boundingRadius);
    }
  });

  it("orders clusters by earliest createdAt, then workspace id", () => {
    const layout = computeFleetLayout([
      node("late", "w-late", 50),
      node("tie-b", "w-b", 10),
      node("tie-a", "w-a", 10),
      node("early", "w-early", 1),
    ]);
    expect(layout.clusters.map((c) => c.workspaceId)).toEqual(["w-early", "w-a", "w-b", "w-late"]);
  });

  it("treats a missing parent as a root", () => {
    const layout = computeFleetLayout([node("orphan", "w", 1, "ghost"), node("r", "w", 2)]);
    expect(pos(layout, "orphan").y).toBe(0);
    expect(layout.edges).toEqual([]);
  });

  it("does not throw on parent cycles and turns cycle nodes into roots", () => {
    const nodes = [
      node("self", "w", 1, "self"),
      node("a", "w", 2, "b"),
      node("b", "w", 3, "a"),
      node("tail", "w", 4, "a"),
    ];
    const layout = computeFleetLayout(nodes);
    expect(layout.positions.size).toBe(4);
    expect(pos(layout, "self").y).toBe(0);
    expect(pos(layout, "a").y).toBe(0);
    expect(pos(layout, "b").y).toBe(0);
    expect(pos(layout, "tail").y).toBeGreaterThan(0);
    expect(layout.edges).toEqual([["a", "tail"]]);
  });

  it("lays out 500 nodes in 20 workspaces in under 25 ms", () => {
    const nodes: FleetLayoutNode[] = [];
    for (let i = 0; i < 500; i += 1) {
      const workspaceId = `w${i % 20}`;
      const parent = i >= 40 && i % 3 === 0 ? `n${i - 40}` : null;
      nodes.push(node(`n${i}`, workspaceId, i, parent));
    }
    computeFleetLayout(nodes);
    const start = performance.now();
    const layout = computeFleetLayout(nodes);
    const elapsed = performance.now() - start;
    expect(layout.positions.size).toBe(500);
    expect(elapsed).toBeLessThan(25);
  });
});

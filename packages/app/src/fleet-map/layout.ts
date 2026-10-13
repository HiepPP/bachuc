export interface FleetLayoutNode {
  id: string;
  workspaceId: string;
  parentId: string | null;
  createdAt: Date | number;
}

export interface FleetPosition {
  x: number;
  y: number;
  z: number;
}

export interface FleetCluster {
  workspaceId: string;
  x: number;
  z: number;
  radius: number;
}

export interface FleetLayout {
  positions: Map<string, FleetPosition>;
  edges: Array<[string, string]>;
  clusters: FleetCluster[];
  boundingRadius: number;
}

const GOLDEN_ANGLE = Math.PI * (3 - Math.sqrt(5));
const ROOT_SPACING = 4;
const ORBIT_RADIUS = 2.5;
const HEIGHT_STEP = 6;
const NODE_PADDING = 1.5;
const MIN_CLUSTER_RADIUS = 3;
const CLUSTER_GAP = 4;
const CLUSTER_SPIRAL_SPACING = 3;

interface SortedNode {
  node: FleetLayoutNode;
  time: number;
}

function toTime(value: Date | number): number {
  const time = Number(value);
  return Number.isFinite(time) ? time : 0;
}

function compareNodes(a: SortedNode, b: SortedNode): number {
  if (a.time !== b.time) return a.time - b.time;
  if (a.node.id === b.node.id) return 0;
  return a.node.id < b.node.id ? -1 : 1;
}

// A parent link only shapes the layout when the parent sits in the same workspace.
function findCycleMembers(parentOf: Map<string, string>): Set<string> {
  const members = new Set<string>();
  const settled = new Set<string>();
  for (const start of parentOf.keys()) {
    if (settled.has(start)) continue;
    const path: string[] = [];
    const onPath = new Set<string>();
    let current: string | undefined = start;
    while (current !== undefined && !settled.has(current) && !onPath.has(current)) {
      onPath.add(current);
      path.push(current);
      current = parentOf.get(current);
    }
    if (current !== undefined && onPath.has(current)) {
      for (let i = path.indexOf(current); i < path.length; i += 1) members.add(path[i]);
    }
    for (const id of path) settled.add(id);
  }
  return members;
}

interface LocalCluster {
  workspaceId: string;
  earliest: number;
  local: Array<{ id: string; x: number; y: number; z: number }>;
  radius: number;
}

function layoutWorkspace(workspaceId: string, sorted: SortedNode[], parentOf: Map<string, string>) {
  const childrenOf = new Map<string, SortedNode[]>();
  const roots: SortedNode[] = [];
  for (const entry of sorted) {
    const parent = parentOf.get(entry.node.id);
    if (parent === undefined) {
      roots.push(entry);
      continue;
    }
    const siblings = childrenOf.get(parent);
    if (siblings) siblings.push(entry);
    else childrenOf.set(parent, [entry]);
  }

  const local: LocalCluster["local"] = [];
  let maxReach = 0;
  const place = (id: string, x: number, y: number, z: number) => {
    local.push({ id, x, y, z });
    maxReach = Math.max(maxReach, Math.hypot(x, z));
  };

  roots.forEach((root, index) => {
    const distance = ROOT_SPACING * Math.sqrt(index);
    const angle = index * GOLDEN_ANGLE;
    const stack = [
      { id: root.node.id, x: distance * Math.cos(angle), y: 0, z: distance * Math.sin(angle) },
    ];
    while (stack.length > 0) {
      const current = stack.pop();
      if (!current) break;
      place(current.id, current.x, current.y, current.z);
      const children = childrenOf.get(current.id);
      if (!children) continue;
      children.forEach((child, childIndex) => {
        const childAngle = childIndex * GOLDEN_ANGLE;
        stack.push({
          id: child.node.id,
          x: current.x + ORBIT_RADIUS * Math.cos(childAngle),
          y: current.y + HEIGHT_STEP,
          z: current.z + ORBIT_RADIUS * Math.sin(childAngle),
        });
      });
    }
  });

  return {
    workspaceId,
    earliest: sorted[0].time,
    local,
    radius: Math.max(MIN_CLUSTER_RADIUS, maxReach + NODE_PADDING),
  } satisfies LocalCluster;
}

function placeClusters(clusters: LocalCluster[]): FleetCluster[] {
  const placed: FleetCluster[] = [];
  for (const cluster of clusters) {
    let step = 0;
    for (;;) {
      const distance = CLUSTER_SPIRAL_SPACING * Math.sqrt(step);
      const angle = step * GOLDEN_ANGLE;
      const x = distance * Math.cos(angle);
      const z = distance * Math.sin(angle);
      const clear = placed.every(
        (other) =>
          Math.hypot(other.x - x, other.z - z) > other.radius + cluster.radius + CLUSTER_GAP,
      );
      if (clear) {
        placed.push({ workspaceId: cluster.workspaceId, x, z, radius: cluster.radius });
        break;
      }
      step += 1;
    }
  }
  return placed;
}

export function computeFleetLayout(nodes: readonly FleetLayoutNode[]): FleetLayout {
  const byId = new Map<string, SortedNode>();
  for (const node of nodes) {
    const entry = { node, time: toTime(node.createdAt) };
    const existing = byId.get(node.id);
    if (!existing || compareNodes(entry, existing) < 0) byId.set(node.id, entry);
  }

  const sortedAll = [...byId.values()].sort(compareNodes);

  const layoutParent = new Map<string, string>();
  for (const { node } of sortedAll) {
    if (node.parentId === null || node.parentId === node.id) continue;
    const parent = byId.get(node.parentId);
    if (parent && parent.node.workspaceId === node.workspaceId) {
      layoutParent.set(node.id, node.parentId);
    }
  }
  const cycleMembers = findCycleMembers(layoutParent);
  for (const id of cycleMembers) layoutParent.delete(id);

  const byWorkspace = new Map<string, SortedNode[]>();
  for (const entry of sortedAll) {
    const group = byWorkspace.get(entry.node.workspaceId);
    if (group) group.push(entry);
    else byWorkspace.set(entry.node.workspaceId, [entry]);
  }

  const localClusters = [...byWorkspace.entries()]
    .map(([workspaceId, group]) => layoutWorkspace(workspaceId, group, layoutParent))
    .sort((a, b) => {
      if (a.earliest !== b.earliest) return a.earliest - b.earliest;
      if (a.workspaceId === b.workspaceId) return 0;
      return a.workspaceId < b.workspaceId ? -1 : 1;
    });
  const clusters = placeClusters(localClusters);

  const positions = new Map<string, FleetPosition>();
  let boundingRadius = 0;
  localClusters.forEach((local, index) => {
    const cluster = clusters[index];
    boundingRadius = Math.max(boundingRadius, Math.hypot(cluster.x, cluster.z) + cluster.radius);
    for (const entry of local.local) {
      const position = { x: cluster.x + entry.x, y: entry.y, z: cluster.z + entry.z };
      positions.set(entry.id, position);
      boundingRadius = Math.max(boundingRadius, Math.hypot(position.x, position.y, position.z));
    }
  });

  // An edge follows every parent link to a listed parent, including links across workspaces.
  const edges: Array<[string, string]> = [];
  for (const { node } of sortedAll) {
    if (node.parentId === null || node.parentId === node.id) continue;
    if (!byId.has(node.parentId) || cycleMembers.has(node.id)) continue;
    edges.push([node.parentId, node.id]);
  }

  return { positions, edges, clusters, boundingRadius };
}

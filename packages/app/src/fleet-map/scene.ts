import type * as ThreeNamespace from "three";
import type { FleetPosition } from "./layout";

type Three = typeof ThreeNamespace;

export const MAX_PIXEL_RATIO = 1.5;

const NODE_RADIUS = 0.5;
const HOVER_SCALE = 1.4;
const PULSE_AMPLITUDE = 0.2;
const MIN_CAPACITY = 16;
const CAMERA_FOV_DEGREES = 40;
// Fixed oblique view: from the front, raised about 50 degrees above the layout plane.
const CAMERA_DIRECTION = { x: 0, y: 1.2, z: 1 };
const FIT_MARGIN = 1.15;

export interface FleetSceneNode {
  id: string;
  position: FleetPosition;
  // Plain hex, for example 0x3b82f6. The scene never reads the theme.
  color: number;
  pulsing: boolean;
}

export interface FleetSceneGraph {
  nodes: readonly FleetSceneNode[];
  edges: ReadonlyArray<readonly [string, string]>;
  boundingRadius: number;
  edgeColor: number;
}

export type FleetSceneRenderer = Pick<
  ThreeNamespace.WebGLRenderer,
  "render" | "getPixelRatio" | "setPixelRatio" | "getSize" | "setSize"
>;

export interface FleetScene {
  setGraph: (graph: FleetSceneGraph) => void;
  setHovered: (id: string | null) => void;
  // 0..1 through one pulse cycle.
  setPulsePhase: (phase: number) => void;
  // CSS pixels of the view's canvas and the device pixel ratio, capped at 1.5.
  resize: (width: number, height: number, devicePixelRatio: number) => void;
  // CSS pixels relative to the view's canvas.
  pick: (x: number, y: number) => string | null;
  renderWith: (renderer: FleetSceneRenderer) => void;
  dispose: () => void;
}

export function createFleetScene({ THREE }: { THREE: Three }): FleetScene {
  const scene = new THREE.Scene();
  const camera = new THREE.PerspectiveCamera(CAMERA_FOV_DEGREES, 1, 0.1, 1000);
  const nodeGeometry = new THREE.SphereGeometry(NODE_RADIUS, 20, 14);
  const nodeMaterial = new THREE.MeshBasicMaterial({ color: 0xffffff });
  const edgeGeometry = new THREE.BufferGeometry();
  const edgeMaterial = new THREE.LineBasicMaterial();
  const edges = new THREE.LineSegments(edgeGeometry, edgeMaterial);
  scene.add(edges);

  let mesh = createMesh(MIN_CAPACITY);
  let nodes: readonly FleetSceneNode[] = [];
  let hoveredIndex = -1;
  let pulsePhase = 0;
  let boundingRadius = 1;
  let width = 1;
  let height = 1;
  let pixelRatio = 1;

  const raycaster = new THREE.Raycaster();
  const pointer = new THREE.Vector2();
  const matrix = new THREE.Matrix4();
  const color = new THREE.Color();
  const sizeProbe = new THREE.Vector2();

  function createMesh(capacity: number): ThreeNamespace.InstancedMesh {
    const next = new THREE.InstancedMesh(nodeGeometry, nodeMaterial, capacity);
    next.count = 0;
    scene.add(next);
    return next;
  }

  function writeInstance(index: number) {
    const node = nodes[index];
    if (!node) {
      return;
    }
    let scale = 1;
    if (index === hoveredIndex) {
      scale *= HOVER_SCALE;
    }
    if (node.pulsing) {
      scale *= 1 + PULSE_AMPLITUDE * Math.sin(pulsePhase * Math.PI * 2);
    }
    const { x, y, z } = node.position;
    matrix.makeScale(scale, scale, scale).setPosition(x, y, z);
    mesh.setMatrixAt(index, matrix);
    mesh.instanceMatrix.needsUpdate = true;
  }

  function fitCamera() {
    const radius = Math.max(boundingRadius, 1) * FIT_MARGIN;
    const aspect = width / height;
    const verticalHalf = THREE.MathUtils.degToRad(CAMERA_FOV_DEGREES) / 2;
    const horizontalHalf = Math.atan(Math.tan(verticalHalf) * aspect);
    const distance = radius / Math.sin(Math.min(verticalHalf, horizontalHalf));
    camera.aspect = aspect;
    camera.near = Math.max(0.1, distance - radius * 2);
    camera.far = distance + radius * 2;
    camera.position
      .set(CAMERA_DIRECTION.x, CAMERA_DIRECTION.y, CAMERA_DIRECTION.z)
      .normalize()
      .multiplyScalar(distance);
    camera.lookAt(0, 0, 0);
    camera.updateProjectionMatrix();
    camera.updateMatrixWorld();
  }

  function setEdges(graph: FleetSceneGraph) {
    const indexById = new Map(graph.nodes.map((node, index) => [node.id, index]));
    const points: number[] = [];
    for (const [from, to] of graph.edges) {
      const start = graph.nodes[indexById.get(from) ?? -1]?.position;
      const end = graph.nodes[indexById.get(to) ?? -1]?.position;
      if (start && end) {
        points.push(start.x, start.y, start.z, end.x, end.y, end.z);
      }
    }
    edgeGeometry.setAttribute("position", new THREE.Float32BufferAttribute(points, 3));
    edgeGeometry.computeBoundingSphere();
    edgeMaterial.color.setHex(graph.edgeColor);
  }

  fitCamera();

  return {
    setGraph: (graph) => {
      const hoveredId = nodes[hoveredIndex]?.id ?? null;
      nodes = graph.nodes;
      if (nodes.length > mesh.instanceMatrix.count) {
        scene.remove(mesh);
        mesh.dispose();
        mesh = createMesh(Math.max(nodes.length, mesh.instanceMatrix.count * 2));
      }
      mesh.count = nodes.length;
      hoveredIndex = hoveredId === null ? -1 : nodes.findIndex((node) => node.id === hoveredId);
      nodes.forEach((node, index) => {
        writeInstance(index);
        mesh.setColorAt(index, color.setHex(node.color));
      });
      if (mesh.instanceColor) {
        mesh.instanceColor.needsUpdate = true;
      }
      mesh.computeBoundingSphere();
      setEdges(graph);
      boundingRadius = graph.boundingRadius;
      fitCamera();
    },
    setHovered: (id) => {
      const previous = hoveredIndex;
      hoveredIndex = id === null ? -1 : nodes.findIndex((node) => node.id === id);
      if (previous !== hoveredIndex) {
        writeInstance(previous);
        writeInstance(hoveredIndex);
      }
    },
    setPulsePhase: (phase) => {
      pulsePhase = phase;
      nodes.forEach((node, index) => {
        if (node.pulsing) {
          writeInstance(index);
        }
      });
    },
    resize: (nextWidth, nextHeight, devicePixelRatio) => {
      width = Math.max(nextWidth, 1);
      height = Math.max(nextHeight, 1);
      pixelRatio = Math.min(devicePixelRatio, MAX_PIXEL_RATIO);
      fitCamera();
    },
    pick: (x, y) => {
      if (mesh.count === 0) {
        return null;
      }
      pointer.set((x / width) * 2 - 1, -(y / height) * 2 + 1);
      raycaster.setFromCamera(pointer, camera);
      const instanceId = raycaster.intersectObject(mesh, false)[0]?.instanceId;
      return instanceId === undefined ? null : (nodes[instanceId]?.id ?? null);
    },
    renderWith: (renderer) => {
      // Another view may have resized the shared renderer since this view's last frame.
      renderer.getSize(sizeProbe);
      if (renderer.getPixelRatio() !== pixelRatio) {
        renderer.setPixelRatio(pixelRatio);
      }
      if (sizeProbe.x !== width || sizeProbe.y !== height) {
        renderer.setSize(width, height, false);
      }
      renderer.render(scene, camera);
    },
    dispose: () => {
      scene.clear();
      mesh.dispose();
      nodeGeometry.dispose();
      nodeMaterial.dispose();
      edgeGeometry.dispose();
      edgeMaterial.dispose();
    },
  };
}

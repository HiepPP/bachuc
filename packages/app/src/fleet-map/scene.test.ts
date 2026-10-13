import { describe, expect, it, vi } from "vitest";
import { createFleetScene, type FleetSceneGraph, type FleetSceneRenderer } from "./scene";

// Only scene.ts and shared-renderer.ts may import three statically.
const THREE = await import("three");
type InstancedMesh = InstanceType<typeof THREE.InstancedMesh>;
type LineSegments = InstanceType<typeof THREE.LineSegments>;

const WIDTH = 400;
const HEIGHT = 300;

function createFakeRenderer() {
  const size = { width: 0, height: 0 };
  let pixelRatio = 1;
  const render = vi.fn<FleetSceneRenderer["render"]>();
  const renderer: FleetSceneRenderer = {
    render,
    getPixelRatio: () => pixelRatio,
    setPixelRatio: (value) => {
      pixelRatio = value;
    },
    getSize: (target) => target.set(size.width, size.height),
    setSize: (width, height) => {
      size.width = width;
      size.height = height;
    },
  };
  return { renderer, render, size, getPixelRatio: () => pixelRatio };
}

function renderedObjects(graph: FleetSceneGraph) {
  const scene = createFleetScene({ THREE });
  scene.resize(WIDTH, HEIGHT, 1);
  scene.setGraph(graph);
  const fake = createFakeRenderer();
  scene.renderWith(fake.renderer);
  const root = fake.render.mock.calls[0]?.[0];
  const meshes: InstancedMesh[] = [];
  const lines: LineSegments[] = [];
  root?.traverse((object) => {
    if (object instanceof THREE.InstancedMesh) {
      meshes.push(object);
    } else if (object instanceof THREE.LineSegments) {
      lines.push(object);
    }
  });
  return { scene, meshes, lines };
}

function node(id: string, x: number, z: number, color = 0x3b82f6, pulsing = false) {
  return { id, position: { x, y: 0, z }, color, pulsing };
}

function instanceScale(mesh: InstancedMesh, index: number): number {
  const matrix = new THREE.Matrix4();
  mesh.getMatrixAt(index, matrix);
  const scale = new THREE.Vector3();
  matrix.decompose(new THREE.Vector3(), new THREE.Quaternion(), scale);
  return scale.x;
}

const graph: FleetSceneGraph = {
  nodes: [node("a", 0, 0, 0xff0000), node("b", 6, 0, 0x00ff00), node("c", -6, 0, 0x0000ff, true)],
  edges: [
    ["a", "b"],
    ["a", "c"],
  ],
  boundingRadius: 7,
  edgeColor: 0x888888,
};

describe("createFleetScene", () => {
  it("builds one instanced mesh with N instances and one line set with M segments", () => {
    const { meshes, lines } = renderedObjects(graph);

    expect(meshes).toHaveLength(1);
    expect(meshes[0]?.count).toBe(3);
    expect(lines).toHaveLength(1);
    expect(lines[0]?.geometry.getAttribute("position").count).toBe(2 * 2);
  });

  it("gives each instance the color the caller passed", () => {
    const { meshes } = renderedObjects(graph);
    const color = new THREE.Color();

    meshes[0]?.getColorAt(1, color);
    expect(color.getHex()).toBe(0x00ff00);
  });

  it("keeps one mesh when a later graph has more nodes", () => {
    const scene = createFleetScene({ THREE });
    scene.resize(WIDTH, HEIGHT, 1);
    scene.setGraph(graph);
    const nodes = Array.from({ length: 40 }, (_, index) => node(`n${index}`, index, 0));
    scene.setGraph({ nodes, edges: [], boundingRadius: 40, edgeColor: 0x888888 });
    const fake = createFakeRenderer();
    scene.renderWith(fake.renderer);

    const meshes: InstancedMesh[] = [];
    fake.render.mock.calls[0]?.[0].traverse((object) => {
      if (object instanceof THREE.InstancedMesh) {
        meshes.push(object);
      }
    });
    expect(meshes).toHaveLength(1);
    expect(meshes[0]?.count).toBe(40);
  });

  it("picks the node under the pointer and null over empty space", () => {
    const { scene } = renderedObjects(graph);

    expect(scene.pick(WIDTH / 2, HEIGHT / 2)).toBe("a");
    expect(scene.pick(1, 1)).toBeNull();
  });

  it("picks null on an empty graph", () => {
    const { scene } = renderedObjects({ nodes: [], edges: [], boundingRadius: 0, edgeColor: 0 });

    expect(scene.pick(WIDTH / 2, HEIGHT / 2)).toBeNull();
  });

  it("enlarges the hovered node and restores it after hover ends", () => {
    const { scene, meshes } = renderedObjects(graph);
    const mesh = meshes[0];
    if (!mesh) {
      throw new Error("expected an instanced mesh");
    }
    const base = instanceScale(mesh, 1);

    scene.setHovered("b");
    expect(instanceScale(mesh, 1)).toBeGreaterThan(base);
    scene.setHovered(null);
    expect(instanceScale(mesh, 1)).toBeCloseTo(base);
  });

  it("scales only pulsing nodes with the pulse phase", () => {
    const { scene, meshes } = renderedObjects(graph);
    const mesh = meshes[0];
    if (!mesh) {
      throw new Error("expected an instanced mesh");
    }
    const steady = instanceScale(mesh, 0);
    const pulsingBefore = instanceScale(mesh, 2);

    scene.setPulsePhase(0.25);
    expect(instanceScale(mesh, 0)).toBeCloseTo(steady);
    expect(instanceScale(mesh, 2)).not.toBeCloseTo(pulsingBefore);
  });

  it("caps the pixel ratio at 1.5 and sizes the shared renderer to the view", () => {
    const scene = createFleetScene({ THREE });
    scene.resize(WIDTH, HEIGHT, 3);
    scene.setGraph(graph);
    const fake = createFakeRenderer();
    scene.renderWith(fake.renderer);

    expect(fake.getPixelRatio()).toBe(1.5);
    expect(fake.size).toEqual({ width: WIDTH, height: HEIGHT });
  });
});

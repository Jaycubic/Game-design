// Low-level helpers for crafting faceted, vertex-coloured meshes in code.

import { Color3, Mesh, MeshBuilder, Scene, Vector3, VertexBuffer } from "@babylonjs/core";

export type Rnd = () => number;

export function seeded(seed: number): Rnd {
  let s = seed >>> 0 || 1;
  return () => {
    s ^= s << 13;
    s ^= s >>> 17;
    s ^= s << 5;
    return ((s >>> 0) % 1_000_000) / 1_000_000;
  };
}

// Small value-noise / fBm for terrain and displacement (deterministic, no deps).
function hash2(x: number, y: number): number {
  const h = Math.sin(x * 127.1 + y * 311.7) * 43758.5453123;
  return h - Math.floor(h);
}
function smooth(t: number) {
  return t * t * (3 - 2 * t);
}
export function noise2(x: number, y: number): number {
  const xi = Math.floor(x),
    yi = Math.floor(y);
  const xf = x - xi,
    yf = y - yi;
  const a = hash2(xi, yi),
    b = hash2(xi + 1, yi),
    c = hash2(xi, yi + 1),
    d = hash2(xi + 1, yi + 1);
  const u = smooth(xf),
    v = smooth(yf);
  return a + (b - a) * u + (c - a) * v + (a - b - c + d) * u * v;
}
export function fbm(x: number, y: number, oct = 4): number {
  let sum = 0,
    amp = 0.5,
    f = 1,
    norm = 0;
  for (let i = 0; i < oct; i++) {
    sum += noise2(x * f, y * f) * amp;
    norm += amp;
    amp *= 0.5;
    f *= 2.03;
  }
  return sum / norm;
}

/** Push vertices of a roughly round mesh in/out along their direction from `centre`. */
export function lumpy(mesh: Mesh, amount: number, freq: number, seed: number, centre = Vector3.Zero()) {
  const pos = mesh.getVerticesData(VertexBuffer.PositionKind)!;
  for (let i = 0; i < pos.length; i += 3) {
    const x = pos[i] - centre.x,
      y = pos[i + 1] - centre.y,
      z = pos[i + 2] - centre.z;
    const len = Math.hypot(x, y, z) || 1;
    const n = fbm((x / len) * freq + seed, (y / len) * freq + (z / len) * freq * 0.7 + seed * 0.37, 3);
    const k = 1 + (n - 0.5) * amount * 2;
    pos[i] = centre.x + x * k;
    pos[i + 1] = centre.y + y * k;
    pos[i + 2] = centre.z + z * k;
  }
  mesh.setVerticesData(VertexBuffer.PositionKind, pos);
}

/** Flat-shade a mesh and give every face a slightly varied colour (hand-painted look). */
export function paint(mesh: Mesh, color: Color3, jitter: number, rnd: Rnd, flat = true, gradient?: { top: Color3; y0: number; y1: number }) {
  if (flat) mesh.convertToFlatShadedMesh();
  const pos = mesh.getVerticesData(VertexBuffer.PositionKind)!;
  const n = pos.length / 3;
  const cols = new Float32Array(n * 4);
  for (let f = 0; f < n; f += 3) {
    const k = 1 + (rnd() - 0.5) * jitter * 2;
    for (let j = 0; j < 3 && f + j < n; j++) {
      const i = f + j;
      let c = color;
      if (gradient) {
        const t = Math.min(1, Math.max(0, (pos[i * 3 + 1] - gradient.y0) / (gradient.y1 - gradient.y0)));
        c = Color3.Lerp(color, gradient.top, t);
      }
      cols[i * 4] = c.r * k;
      cols[i * 4 + 1] = c.g * k;
      cols[i * 4 + 2] = c.b * k;
      cols[i * 4 + 3] = 1;
    }
  }
  mesh.setVerticesData(VertexBuffer.ColorKind, cols);
  return mesh;
}

export function place(m: Mesh, p: [number, number, number], s?: [number, number, number] | number, r?: [number, number, number]) {
  m.position.set(p[0], p[1], p[2]);
  if (s !== undefined) typeof s === "number" ? m.scaling.setAll(s) : m.scaling.set(s[0], s[1], s[2]);
  if (r) m.rotation.set(r[0], r[1], r[2]);
  return m;
}

/** Merge parts into one mesh (all parts must carry vertex colours). */
export function merge(name: string, parts: Mesh[]): Mesh {
  const m = Mesh.MergeMeshes(parts, true, true, undefined, false, false)!;
  m.name = name;
  return m;
}

export function blob(scene: Scene, r: number, seed: number, subdiv = 2) {
  const m = MeshBuilder.CreateIcoSphere("b", { radius: r, subdivisions: subdiv, flat: false }, scene);
  lumpy(m, 0.22, 1.6, seed);
  return m;
}

export function cyl(scene: Scene, h: number, top: number, bottom: number, tess = 6) {
  const m = MeshBuilder.CreateCylinder("c", { height: h, diameterTop: top, diameterBottom: bottom, tessellation: tess }, scene);
  m.position.y = h / 2;
  m.bakeCurrentTransformIntoVertices();
  return m;
}

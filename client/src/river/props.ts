// Procedural meshes for the river run: boats, litter, weed mats, logs, debris, pipes, bank buildings.

import { Color3, Mesh, MeshBuilder, Scene } from "@babylonjs/core";
import { blob, cyl, merge, paint, place, seeded } from "../render/shapes";

const hex = (h: string) => Color3.FromHexString(h);

/** A small river-cleanup boat: hull, cabin, net boom, flag in the player's colour. */
export function boatMesh(scene: Scene, colour: string, seed: number): Mesh {
  const r = seeded(seed);
  const c = hex(colour);
  const hull = MeshBuilder.CreateBox("hull", { width: 1.1, height: 0.38, depth: 2.0 }, scene);
  place(hull, [0, 0.12, 0]);
  hull.bakeCurrentTransformIntoVertices();
  const bow = MeshBuilder.CreateCylinder("bow", { height: 1.1, diameter: 1.1, tessellation: 3 }, scene);
  place(bow, [0, 0.12, 1.15], [1, 1, 0.55], [0, 0, Math.PI / 2]);
  bow.bakeCurrentTransformIntoVertices();
  const rim = MeshBuilder.CreateBox("rim", { width: 1.18, height: 0.08, depth: 2.08 }, scene);
  place(rim, [0, 0.33, 0]);
  rim.bakeCurrentTransformIntoVertices();
  const cabin = MeshBuilder.CreateBox("cabin", { width: 0.7, height: 0.45, depth: 0.6 }, scene);
  place(cabin, [0, 0.55, -0.45]);
  cabin.bakeCurrentTransformIntoVertices();
  const roof = MeshBuilder.CreateBox("roof", { width: 0.82, height: 0.07, depth: 0.74 }, scene);
  place(roof, [0, 0.8, -0.45]);
  roof.bakeCurrentTransformIntoVertices();
  const boom = MeshBuilder.CreateCylinder("boom", { height: 1.0, diameter: 0.05, tessellation: 5 }, scene);
  place(boom, [0, 0.55, 1.05], 1, [Math.PI / 2.6, 0, 0]);
  boom.bakeCurrentTransformIntoVertices();
  const net = MeshBuilder.CreateBox("net", { width: 0.9, height: 0.3, depth: 0.08 }, scene);
  place(net, [0, 0.25, 1.45], 1, [0.35, 0, 0]);
  net.bakeCurrentTransformIntoVertices();
  const mast = cyl(scene, 0.55, 0.025, 0.025, 4);
  place(mast, [0.25, 0.82, -0.6]);
  mast.bakeCurrentTransformIntoVertices();
  const flag = MeshBuilder.CreateBox("flag", { width: 0.02, height: 0.18, depth: 0.3 }, scene);
  place(flag, [0.25, 1.28, -0.47]);
  flag.bakeCurrentTransformIntoVertices();
  return merge("boat", [
    paint(hull, c, 0.06, r),
    paint(bow, c, 0.06, r),
    paint(rim, hex("#f4efe2"), 0.03, r),
    paint(cabin, hex("#f4efe2"), 0.04, r),
    paint(roof, c.scale(0.7), 0.05, r),
    paint(boom, hex("#5a5550"), 0.05, r),
    paint(net, hex("#2f6f5a"), 0.1, r),
    paint(mast, hex("#5a5550"), 0.05, r),
    paint(flag, c, 0.02, r),
  ]);
}

export function bottleMesh(scene: Scene, colour: string, seed: number): Mesh {
  const r = seeded(seed);
  const body = cyl(scene, 0.42, 0.13, 0.15, 7);
  const neck = cyl(scene, 0.12, 0.06, 0.12, 7);
  place(neck, [0, 0.42, 0]);
  neck.bakeCurrentTransformIntoVertices();
  const cap = cyl(scene, 0.06, 0.07, 0.07, 7);
  place(cap, [0, 0.54, 0]);
  cap.bakeCurrentTransformIntoVertices();
  const m = merge("bottle", [paint(body, hex(colour), 0.08, r), paint(neck, hex(colour), 0.08, r), paint(cap, hex("#2a6fd6"), 0.05, r)]);
  m.rotation.set(Math.PI / 2.3, 0, 0.3);
  m.bakeCurrentTransformIntoVertices();
  return m;
}

export function bagMesh(scene: Scene): Mesh {
  const r = seeded(77);
  const b = blob(scene, 0.28, 12, 1);
  b.scaling.set(1.1, 0.35, 0.9);
  b.bakeCurrentTransformIntoVertices();
  return paint(b, hex("#e9e6dc"), 0.12, r);
}

/** Floating weed mat: round glossy leaves with lilac flower spikes. */
export function weedMesh(scene: Scene): Mesh {
  const r = seeded(91);
  const parts: Mesh[] = [];
  for (let i = 0; i < 9; i++) {
    const a = r() * Math.PI * 2;
    const d = r() * 0.8;
    const leaf = MeshBuilder.CreateIcoSphere("leaf", { radius: 0.32, subdivisions: 1 }, scene);
    place(leaf, [Math.cos(a) * d, 0.05, Math.sin(a) * d], [1, 0.3, 1]);
    leaf.bakeCurrentTransformIntoVertices();
    parts.push(paint(leaf, i % 3 ? hex("#3f8a2e") : hex("#5aa83a"), 0.12, r));
  }
  for (let i = 0; i < 4; i++) {
    const a = r() * Math.PI * 2;
    const spike = cyl(scene, 0.35, 0.03, 0.08, 5);
    place(spike, [Math.cos(a) * 0.4, 0.1, Math.sin(a) * 0.4]);
    spike.bakeCurrentTransformIntoVertices();
    parts.push(paint(spike, hex("#b48ae0"), 0.1, r));
  }
  return merge("weed", parts);
}

export function logMesh(scene: Scene): Mesh {
  const r = seeded(33);
  const log = MeshBuilder.CreateCylinder("log", { height: 2.0, diameter: 0.45, tessellation: 8 }, scene);
  place(log, [0, 0.1, 0], 1, [0, 0, Math.PI / 2]);
  log.bakeCurrentTransformIntoVertices();
  const branch = MeshBuilder.CreateCylinder("br", { height: 0.7, diameter: 0.12, tessellation: 5 }, scene);
  place(branch, [0.4, 0.35, 0], 1, [0, 0, -0.6]);
  branch.bakeCurrentTransformIntoVertices();
  return merge("log", [paint(log, hex("#6b4a2e"), 0.12, r), paint(branch, hex("#5a3d25"), 0.1, r)]);
}

/** One lane-wide piece of a debris wave: tangled branches, junk and a foam crest. */
export function debrisMesh(scene: Scene): Mesh {
  const r = seeded(55);
  const parts: Mesh[] = [];
  const crest = MeshBuilder.CreateBox("crest", { width: 2.3, height: 0.35, depth: 0.5 }, scene);
  place(crest, [0, 0.1, 0.35]);
  crest.bakeCurrentTransformIntoVertices();
  parts.push(paint(crest, hex("#e8f2f0"), 0.08, r));
  for (let i = 0; i < 5; i++) {
    const b = MeshBuilder.CreateCylinder("d", { height: 0.9 + r() * 0.6, diameter: 0.15, tessellation: 5 }, scene);
    place(b, [(r() - 0.5) * 1.8, 0.35, (r() - 0.5) * 0.4], 1, [r() * 0.6, r() * 3, Math.PI / 2 + (r() - 0.5)]);
    b.bakeCurrentTransformIntoVertices();
    parts.push(paint(b, hex("#5a4632"), 0.12, r));
  }
  for (let i = 0; i < 3; i++) {
    const junk = MeshBuilder.CreateBox("j", { size: 0.22 }, scene);
    place(junk, [(r() - 0.5) * 1.6, 0.45, (r() - 0.5) * 0.3], 1, [r(), r(), r()]);
    junk.bakeCurrentTransformIntoVertices();
    parts.push(paint(junk, [hex("#d94f3d"), hex("#3a7bd5"), hex("#e0c341")][i], 0.05, r));
  }
  return merge("debris", parts);
}

/** Concrete outfall pipe set into the bank (points toward the river along -x). */
export function pipeMesh(scene: Scene): Mesh {
  const r = seeded(21);
  const pipe = MeshBuilder.CreateCylinder("pipe", { height: 1.6, diameter: 0.7, tessellation: 10 }, scene);
  place(pipe, [0, 0.35, 0], 1, [0, 0, Math.PI / 2]);
  pipe.bakeCurrentTransformIntoVertices();
  const mouth = MeshBuilder.CreateCylinder("mouth", { height: 0.05, diameter: 0.55, tessellation: 10 }, scene);
  place(mouth, [-0.81, 0.35, 0], 1, [0, 0, Math.PI / 2]);
  mouth.bakeCurrentTransformIntoVertices();
  const wall = MeshBuilder.CreateBox("wall", { width: 0.4, height: 1.3, depth: 2.2 }, scene);
  place(wall, [0.6, 0.3, 0]);
  wall.bakeCurrentTransformIntoVertices();
  return merge("pipe", [paint(pipe, hex("#8e8a82"), 0.1, r), paint(mouth, hex("#1e1a14"), 0, r), paint(wall, hex("#a39e94"), 0.08, r)]);
}

export function plugMesh(scene: Scene): Mesh {
  const r = seeded(22);
  const cap = MeshBuilder.CreateCylinder("cap", { height: 0.18, diameter: 0.66, tessellation: 10 }, scene);
  place(cap, [-0.85, 0.35, 0], 1, [0, 0, Math.PI / 2]);
  cap.bakeCurrentTransformIntoVertices();
  const leaf = MeshBuilder.CreateDisc("lf", { radius: 0.18, tessellation: 6 }, scene);
  place(leaf, [-0.95, 0.35, 0], [1, 1.5, 1], [0, Math.PI / 2, 0]);
  leaf.bakeCurrentTransformIntoVertices();
  return merge("plug", [paint(cap, hex("#3f9a52"), 0.05, r), paint(leaf, hex("#bff0a8"), 0.05, r)]);
}

export function houseMesh(scene: Scene, wall: string, roof: string, seed: number): Mesh {
  const r = seeded(seed);
  const h = 1.6 + r() * 1.4;
  const body = MeshBuilder.CreateBox("house", { width: 2.4, height: h, depth: 2.2 }, scene);
  place(body, [0, h / 2, 0]);
  body.bakeCurrentTransformIntoVertices();
  const top = MeshBuilder.CreateCylinder("roof", { height: 0.9, diameterTop: 0, diameterBottom: 3.4, tessellation: 4 }, scene);
  place(top, [0, h + 0.45, 0], 1, [0, Math.PI / 4, 0]);
  top.bakeCurrentTransformIntoVertices();
  const parts = [paint(body, hex(wall), 0.05, r), paint(top, hex(roof), 0.08, r)];
  for (let i = 0; i < 2; i++) {
    const win = MeshBuilder.CreateBox("win", { width: 0.05, height: 0.4, depth: 0.35 }, scene);
    place(win, [-1.22, h * 0.6, -0.5 + i]);
    win.bakeCurrentTransformIntoVertices();
    parts.push(paint(win, hex("#2a3440"), 0.05, r));
  }
  return merge("house", parts);
}

/** Stone steps down to the water (riverside stairs). */
export function stepsMesh(scene: Scene): Mesh {
  const r = seeded(65);
  const parts: Mesh[] = [];
  for (let i = 0; i < 4; i++) {
    const s = MeshBuilder.CreateBox("step", { width: 0.45, height: 0.22, depth: 3.2 }, scene);
    place(s, [i * 0.45, -0.25 + i * 0.22, 0]);
    s.bakeCurrentTransformIntoVertices();
    parts.push(paint(s, hex("#b6a58a"), 0.06, r));
  }
  return merge("steps", parts);
}

export function lampMesh(scene: Scene): Mesh {
  const r = seeded(66);
  const post = paint(cyl(scene, 1.8, 0.05, 0.08, 6), hex("#3a3a3a"), 0.04, r);
  const lamp = MeshBuilder.CreateIcoSphere("lamp", { radius: 0.13, subdivisions: 1 }, scene);
  place(lamp, [0, 1.85, 0]);
  lamp.bakeCurrentTransformIntoVertices();
  return merge("lamp", [post, paint(lamp, hex("#ffe7a3"), 0.02, r)]);
}

export function reedMesh(scene: Scene): Mesh {
  const r = seeded(67);
  const parts: Mesh[] = [];
  for (let i = 0; i < 7; i++) {
    const blade = MeshBuilder.CreateCylinder("rd", { height: 0.6 + r() * 0.5, diameterTop: 0, diameterBottom: 0.04, tessellation: 3 }, scene);
    place(blade, [(r() - 0.5) * 0.5, 0.3, (r() - 0.5) * 0.5], 1, [(r() - 0.5) * 0.4, 0, (r() - 0.5) * 0.4]);
    blade.bakeCurrentTransformIntoVertices();
    parts.push(paint(blade, hex("#6f9a3a"), 0.15, r, true, { top: hex("#c8c26a"), y0: 0, y1: 0.9 }));
  }
  return merge("reed", parts);
}

/** A footbridge spanning the whole river overhead. */
export function bridgeMesh(scene: Scene, span: number): Mesh {
  const r = seeded(68);
  const deck = MeshBuilder.CreateBox("deck", { width: span, height: 0.35, depth: 2.2 }, scene);
  place(deck, [0, 3.6, 0]);
  deck.bakeCurrentTransformIntoVertices();
  const parts = [paint(deck, hex("#b9ab92"), 0.05, r)];
  for (const s of [-1, 1]) {
    const rail = MeshBuilder.CreateBox("rail", { width: span, height: 0.5, depth: 0.12 }, scene);
    place(rail, [0, 4.0, s * 1.05]);
    rail.bakeCurrentTransformIntoVertices();
    parts.push(paint(rail, hex("#d8cdb6"), 0.04, r));
    const pier = MeshBuilder.CreateBox("pier", { width: 0.8, height: 4.6, depth: 1.6 }, scene);
    place(pier, [s * (span / 2 - 0.4), 1.3, 0]);
    pier.bakeCurrentTransformIntoVertices();
    parts.push(paint(pier, hex("#a39a86"), 0.05, r));
  }
  return merge("footbridge", parts);
}

/** Ghost net (Mangrove Maze): a sagging fishing net across two lanes, orange
 *  floats on top and plastic caught in the mesh. `width` is two lanes. */
export function netMesh(scene: Scene, width: number): Mesh {
  const r = seeded(61);
  const parts: Mesh[] = [];
  // cyl() grows up from its base, so after turning it on its side it runs from x to x - width.
  for (const y of [0.28, -0.12]) {
    const line = cyl(scene, width, 0.07, 0.07, 5);
    place(line, [width / 2, y, 0], 1, [0, 0, Math.PI / 2]);
    line.bakeCurrentTransformIntoVertices();
    parts.push(paint(line, hex("#e6e0cc"), 0.05, r));
  }
  for (let i = 0; i <= 6; i++) {
    const f = blob(scene, 0.13, 70 + i, 1);
    place(f, [-width / 2 + (i * width) / 6, 0.3, 0], [1, 0.8, 1]);
    f.bakeCurrentTransformIntoVertices();
    parts.push(paint(f, hex("#e8743b"), 0.06, r));
  }
  for (let i = 0; i <= 18; i++) {
    const strand = MeshBuilder.CreateBox("strand", { width: 0.06, height: 0.5, depth: 0.06 }, scene);
    place(strand, [-width / 2 + (i * width) / 18, 0.08, 0], 1, [0, 0, i % 2 ? 0.5 : -0.5]);
    strand.bakeCurrentTransformIntoVertices();
    parts.push(paint(strand, hex("#c9d6c4"), 0.08, r));
  }
  for (let i = 0; i < 4; i++) {
    const junk = cyl(scene, 0.32, 0.12, 0.13, 6);
    place(junk, [(r() - 0.5) * width * 0.8, 0.05, (r() - 0.5) * 0.2], 1, [Math.PI / 2, r() * 3, 0]);
    junk.bakeCurrentTransformIntoVertices();
    parts.push(paint(junk, [hex("#9fd3e8"), hex("#7fbf7a"), hex("#e9e6dc"), hex("#e0c341")][i], 0.05, r));
  }
  return merge("net", parts);
}

/** Current (Rapids Gorge): three flat chevrons pointing +x, laid on the water. */
export function currentMesh(scene: Scene): Mesh {
  const r = seeded(63);
  const parts: Mesh[] = [];
  for (let i = 0; i < 3; i++) {
    for (const s of [-1, 1]) {
      const bar = MeshBuilder.CreateBox("chev", { width: 0.62, height: 0.02, depth: 0.14 }, scene);
      place(bar, [-0.55 + i * 0.55, 0, s * 0.2], 1, [0, s * 0.75, 0]);
      bar.bakeCurrentTransformIntoVertices();
      parts.push(paint(bar, hex("#e8fbff"), 0.02, r));
    }
  }
  return merge("current", parts);
}

// Procedural Western Ghats vegetation and props. Each builder returns one merged,
// vertex-coloured mesh used as a thin-instance prototype (one draw call each).

import { Color3, Mesh, MeshBuilder, Scene } from "@babylonjs/core";
import { C } from "./palette";
import { blob, cyl, merge, paint, place, seeded } from "./shapes";

export type ProtoKey =
  | "rain0"
  | "rain1"
  | "rain2"
  | "young"
  | "sapling"
  | "emergent"
  | "fig"
  | "fern"
  | "stump"
  | "snag"
  | "lantana"
  | "grass"
  | "grassGold"
  | "rock"
  | "pebble"
  | "marking"
  | "stake"
  | "bridge";

/** Tropical evergreen: tapered trunk with buttress flare and layered crowns. */
function rainTree(scene: Scene, seed: number): Mesh {
  const r = seeded(seed);
  const h = 0.5 + r() * 0.15;
  const trunk = paint(cyl(scene, h, 0.035, 0.075, 6), C.bark, 0.12, r);
  const flare = paint(place(cyl(scene, 0.07, 0.07, 0.15, 6), [0, 0, 0]), C.barkLight, 0.1, r);
  const parts = [trunk, flare];
  const layers = 2 + Math.floor(r() * 2);
  for (let i = 0; i < layers; i++) {
    const rad = 0.2 - i * 0.035 + r() * 0.03;
    const b = blob(scene, rad, seed * 7 + i, 1);
    place(b, [(r() - 0.5) * 0.12, h - 0.02 + i * 0.13, (r() - 0.5) * 0.12], [1, 0.72, 1]);
    b.bakeCurrentTransformIntoVertices();
    parts.push(paint(b, i === layers - 1 ? C.leafLight : i % 2 ? C.leaf : C.leafDeep, 0.14, r, true, { top: C.leafLight, y0: h - 0.1, y1: h + 0.4 }));
  }
  return merge("rain" + seed, parts);
}

/** Restored young forest: slimmer, brighter. */
function youngTree(scene: Scene): Mesh {
  const r = seeded(91);
  const trunk = paint(cyl(scene, 0.32, 0.025, 0.045, 5), C.barkLight, 0.1, r);
  const b1 = place(blob(scene, 0.14, 4, 1), [0, 0.36, 0], [1, 0.85, 1]);
  b1.bakeCurrentTransformIntoVertices();
  const b2 = place(blob(scene, 0.1, 9, 1), [0.03, 0.5, -0.02]);
  b2.bakeCurrentTransformIntoVertices();
  return merge("young", [trunk, paint(b1, C.leafLight, 0.12, r), paint(b2, C.leafYoung, 0.12, r)]);
}

function sapling(scene: Scene): Mesh {
  const r = seeded(5);
  const stem = paint(cyl(scene, 0.16, 0.012, 0.02, 4), C.barkLight, 0.1, r);
  const b = place(blob(scene, 0.06, 2, 1), [0, 0.17, 0]);
  b.bakeCurrentTransformIntoVertices();
  const guard = paint(place(cyl(scene, 0.12, 0.09, 0.09, 6), [0, 0, 0]), C.stake, 0.05, r);
  guard.scaling.set(1, 1, 1);
  return merge("sapling", [stem, paint(b, C.leafYoung, 0.15, r), guard]);
}

/** Emergent dipterocarp-style nest tree: very tall, flat crown, nest cavity. */
function emergent(scene: Scene): Mesh {
  const r = seeded(13);
  const trunk = paint(cyl(scene, 0.95, 0.05, 0.11, 7), C.bark, 0.1, r);
  const parts = [trunk];
  for (let i = 0; i < 4; i++) {
    const a = (i / 4) * Math.PI * 2 + 0.4;
    const fin = MeshBuilder.CreateBox("fin", { width: 0.02, height: 0.16, depth: 0.12 }, scene);
    place(fin, [Math.cos(a) * 0.06, 0.08, Math.sin(a) * 0.06], 1, [0, -a, 0]);
    fin.bakeCurrentTransformIntoVertices();
    parts.push(paint(fin, C.barkLight, 0.1, r));
  }
  const crown = place(blob(scene, 0.32, 21, 2), [0, 1.0, 0], [1, 0.42, 1]);
  crown.bakeCurrentTransformIntoVertices();
  parts.push(paint(crown, C.leaf, 0.14, r, true, { top: C.leafLight, y0: 0.9, y1: 1.15 }));
  const crown2 = place(blob(scene, 0.2, 23, 1), [0.12, 1.1, 0.05], [1, 0.5, 1]);
  crown2.bakeCurrentTransformIntoVertices();
  parts.push(paint(crown2, C.leafLight, 0.14, r));
  const hole = MeshBuilder.CreateBox("hole", { width: 0.045, height: 0.07, depth: 0.03 }, scene);
  place(hole, [0, 0.62, 0.07]);
  hole.bakeCurrentTransformIntoVertices();
  parts.push(paint(hole, new Color3(0.06, 0.04, 0.03), 0, r));
  return merge("emergent", parts);
}

/** Strangler fig: thick trunk, aerial roots, wide crown studded with fruit. */
function figTree(scene: Scene): Mesh {
  const r = seeded(31);
  const parts: Mesh[] = [paint(cyl(scene, 0.45, 0.08, 0.13, 7), C.barkLight, 0.12, r)];
  for (let i = 0; i < 6; i++) {
    const a = (i / 6) * Math.PI * 2;
    const root = cyl(scene, 0.42, 0.012, 0.018, 4);
    place(root, [Math.cos(a) * 0.08, 0, Math.sin(a) * 0.08], 1, [Math.sin(a) * 0.12, 0, -Math.cos(a) * 0.12]);
    root.bakeCurrentTransformIntoVertices();
    parts.push(paint(root, C.bark, 0.1, r));
  }
  const crown = place(blob(scene, 0.34, 33, 2), [0, 0.55, 0], [1, 0.5, 1]);
  crown.bakeCurrentTransformIntoVertices();
  parts.push(paint(crown, C.figLeaf, 0.13, r, true, { top: C.leafLight, y0: 0.45, y1: 0.75 }));
  for (let i = 0; i < 16; i++) {
    const a = r() * Math.PI * 2;
    const rr = 0.18 + r() * 0.15;
    const f = MeshBuilder.CreateIcoSphere("fig", { radius: 0.022, subdivisions: 1 }, scene);
    place(f, [Math.cos(a) * rr, 0.5 + r() * 0.16, Math.sin(a) * rr]);
    f.bakeCurrentTransformIntoVertices();
    parts.push(paint(f, r() > 0.4 ? C.fig : C.flowerOrange, 0.1, r));
  }
  return merge("fig", parts);
}

function fern(scene: Scene): Mesh {
  const r = seeded(41);
  const parts: Mesh[] = [];
  for (let i = 0; i < 6; i++) {
    const a = (i / 6) * Math.PI * 2;
    const frond = MeshBuilder.CreateDisc("f", { radius: 0.06, tessellation: 5 }, scene);
    place(frond, [Math.cos(a) * 0.05, 0.04, Math.sin(a) * 0.05], [0.6, 1.4, 1], [Math.PI / 2 - 0.6, -a + Math.PI / 2, 0]);
    frond.bakeCurrentTransformIntoVertices();
    parts.push(paint(frond, C.leafLight, 0.18, r));
  }
  return merge("fern", parts);
}

function stump(scene: Scene): Mesh {
  const r = seeded(51);
  const s = paint(cyl(scene, 0.09, 0.08, 0.11, 7), C.stump, 0.12, r);
  const top = MeshBuilder.CreateDisc("t", { radius: 0.04, tessellation: 7 }, scene);
  place(top, [0, 0.091, 0], 1, [Math.PI / 2, 0, 0]);
  top.bakeCurrentTransformIntoVertices();
  return merge("stump", [s, paint(top, C.deadwood, 0.05, r)]);
}

function snag(scene: Scene): Mesh {
  const r = seeded(53);
  const t = paint(cyl(scene, 0.42, 0.015, 0.05, 5), C.deadwood, 0.15, r);
  const br = cyl(scene, 0.16, 0.008, 0.018, 4);
  place(br, [0.02, 0.26, 0], 1, [0, 0, -0.9]);
  br.bakeCurrentTransformIntoVertices();
  const log = cyl(scene, 0.3, 0.04, 0.05, 6);
  place(log, [0.1, 0.03, 0.08], 1, [0, 0.6, Math.PI / 2]);
  log.bakeCurrentTransformIntoVertices();
  return merge("snag", [t, paint(br, C.deadwood, 0.1, r), paint(log, C.stump, 0.12, r)]);
}

/** Lantana camara thicket: tangled olive mound with pink and orange flower heads. */
function lantana(scene: Scene): Mesh {
  const r = seeded(61);
  const parts: Mesh[] = [];
  for (let i = 0; i < 3; i++) {
    const b = place(blob(scene, 0.09 + r() * 0.04, 60 + i, 1), [(r() - 0.5) * 0.14, 0.06, (r() - 0.5) * 0.14], [1, 0.7, 1]);
    b.bakeCurrentTransformIntoVertices();
    parts.push(paint(b, C.lantana, 0.18, r));
  }
  for (let i = 0; i < 14; i++) {
    const f = MeshBuilder.CreateIcoSphere("fl", { radius: 0.018, subdivisions: 0 }, scene);
    const a = r() * Math.PI * 2;
    place(f, [Math.cos(a) * (0.05 + r() * 0.08), 0.1 + r() * 0.05, Math.sin(a) * (0.05 + r() * 0.08)]);
    f.bakeCurrentTransformIntoVertices();
    parts.push(paint(f, r() > 0.5 ? C.flowerPink : C.flowerOrange, 0.1, r));
  }
  return merge("lantana", parts);
}

function grass(scene: Scene, color: Color3, seed: number): Mesh {
  const r = seeded(seed);
  const parts: Mesh[] = [];
  for (let i = 0; i < 6; i++) {
    const blade = MeshBuilder.CreateCylinder("g", { height: 0.09 + r() * 0.06, diameterTop: 0, diameterBottom: 0.014, tessellation: 3 }, scene);
    const a = r() * Math.PI * 2;
    place(blade, [Math.cos(a) * 0.02, 0.05, Math.sin(a) * 0.02], 1, [(r() - 0.5) * 0.6, a, (r() - 0.5) * 0.6]);
    blade.bakeCurrentTransformIntoVertices();
    parts.push(paint(blade, color, 0.2, r, true, { top: Color3.Lerp(color, C.grassGold, 0.5), y0: 0, y1: 0.12 }));
  }
  return merge("grass", parts);
}

function rock(scene: Scene, r0: number, seed: number): Mesh {
  const r = seeded(seed);
  const b = MeshBuilder.CreateIcoSphere("r", { radius: r0, subdivisions: 1 }, scene);
  b.scaling.set(1.2, 0.7, 1);
  b.bakeCurrentTransformIntoVertices();
  return paint(b, C.rock, 0.16, r, true, { top: Color3.Lerp(C.rock, new Color3(0.75, 0.74, 0.68), 0.5), y0: -r0, y1: r0 });
}

function marking(scene: Scene): Mesh {
  const m = MeshBuilder.CreateBox("mk", { width: 0.22, height: 0.004, depth: 0.03 }, scene);
  return paint(m, C.marking, 0.02, seeded(3), false);
}

/** Restoration-site peg with a ribbon flag. */
function stake(scene: Scene): Mesh {
  const r = seeded(7);
  const post = paint(cyl(scene, 0.22, 0.012, 0.016, 5), C.stake, 0.05, r);
  const flag = MeshBuilder.CreateBox("fl", { width: 0.07, height: 0.04, depth: 0.004 }, scene);
  place(flag, [0.037, 0.19, 0]);
  flag.bakeCurrentTransformIntoVertices();
  return merge("stake", [post, paint(flag, C.flag, 0.05, r, false)]);
}

/** Rope canopy bridge spanning one cell along X (rotated per road). */
function bridge(scene: Scene): Mesh {
  const r = seeded(17);
  const parts: Mesh[] = [];
  const sag = (t: number) => 0.62 - Math.sin(t * Math.PI) * 0.06;
  for (let i = 0; i < 9; i++) {
    const t = i / 8;
    const p = MeshBuilder.CreateBox("pl", { width: 0.035, height: 0.01, depth: 0.16 }, scene);
    place(p, [-0.55 + t * 1.1, sag(t), 0]);
    p.bakeCurrentTransformIntoVertices();
    parts.push(paint(p, C.plank, 0.15, r, false));
  }
  for (const side of [-0.09, 0.09]) {
    // Handrail rope: short segments following the same sag as the deck.
    const segs = 11;
    for (let i = 0; i < segs; i++) {
      const t0 = i / segs,
        t1 = (i + 1) / segs;
      const x0 = -0.55 + t0 * 1.1,
        x1 = -0.55 + t1 * 1.1;
      const y0 = sag(t0) + 0.11,
        y1 = sag(t1) + 0.11;
      const len = Math.hypot(x1 - x0, y1 - y0);
      const seg = MeshBuilder.CreateCylinder("rope", { height: len, diameter: 0.012, tessellation: 4 }, scene);
      place(seg, [(x0 + x1) / 2, (y0 + y1) / 2, side], 1, [0, 0, -Math.PI / 2 + Math.atan2(y1 - y0, x1 - x0)]);
      seg.bakeCurrentTransformIntoVertices();
      parts.push(paint(seg, C.rope, 0.05, r, false));
      if (i % 2 === 0) {
        const hanger = MeshBuilder.CreateCylinder("h", { height: 0.11, diameter: 0.006, tessellation: 3 }, scene);
        place(hanger, [x0, sag(t0) + 0.055, side]);
        hanger.bakeCurrentTransformIntoVertices();
        parts.push(paint(hanger, C.rope, 0.05, r, false));
      }
    }
    // posts at both ends
    for (const x of [-0.55, 0.55]) {
      const post = cyl(scene, 0.78, 0.02, 0.025, 5);
      place(post, [x, 0, side]);
      post.bakeCurrentTransformIntoVertices();
      parts.push(paint(post, C.bark, 0.1, r));
    }
  }
  return merge("bridge", parts);
}

export function buildPrototypes(scene: Scene): Record<ProtoKey, Mesh> {
  return {
    rain0: rainTree(scene, 1),
    rain1: rainTree(scene, 2),
    rain2: rainTree(scene, 3),
    young: youngTree(scene),
    sapling: sapling(scene),
    emergent: emergent(scene),
    fig: figTree(scene),
    fern: fern(scene),
    stump: stump(scene),
    snag: snag(scene),
    lantana: lantana(scene),
    grass: grass(scene, C.grass, 71),
    grassGold: grass(scene, C.grassGold, 72),
    rock: rock(scene, 0.14, 81),
    pebble: rock(scene, 0.05, 82),
    marking: marking(scene),
    stake: stake(scene),
    bridge: bridge(scene),
  };
}

// The river run's 3D scene: chase camera, endless banks, water that shows
// River Health, instanced river objects, boats and splash effects.

import {
  AbstractEngine,
  Color3,
  Color4,
  DirectionalLight,
  DynamicTexture,
  HemisphericLight,
  ImageProcessingConfiguration,
  TransformNode,
  Matrix,
  Mesh,
  MeshBuilder,
  ParticleSystem,
  Quaternion,
  Scene,
  ScenePerformancePriority,
  StandardMaterial,
  Texture,
  UniversalCamera,
  Vector3,
} from "@babylonjs/core";
import { buildPrototypes } from "../render/flora";
import { buildBoat, DEFAULT_BOAT, type BuiltBoat } from "./fleet";
import { fbm, seeded } from "../render/shapes";
import {
  bagMesh,
  boatMesh,
  bottleMesh,
  bridgeMesh,
  debrisMesh,
  houseMesh,
  lampMesh,
  logMesh,
  pipeMesh,
  plugMesh,
  reedMesh,
  stepsMesh,
  weedMesh,
  netMesh,
  currentMesh,
  murkMesh,
} from "./props";
import { LANE_W, SEAT_COLOURS, type River, type RunObj } from "./types";
import { perf } from "../render/engine";

/** Per-river look: water colour when clean, bank colours and heights, what lines the banks. */
interface Theme {
  water: Color3;
  /** [mud edge, path, land] */
  bank: [Color3, Color3, Color3];
  /** Bank profile heights from the water outwards. */
  ys: number[];
  houses: boolean;
  trees: number;
  rocks: number;
}
const THEMES: Record<River, Theme> = {
  canal: {
    water: new Color3(0.1, 0.42, 0.46),
    bank: [new Color3(0.55, 0.5, 0.42), new Color3(0.62, 0.56, 0.44), new Color3(0.38, 0.5, 0.26)],
    ys: [-0.5, 0.45, 0.62, 0.66, 0.8, 1.6],
    houses: true,
    trees: 0.45,
    rocks: 0.3,
  },
  mangrove: {
    water: new Color3(0.07, 0.24, 0.2),
    bank: [new Color3(0.32, 0.28, 0.2), new Color3(0.3, 0.38, 0.2), new Color3(0.2, 0.42, 0.18)],
    ys: [-0.5, 0.25, 0.4, 0.5, 0.7, 1.4],
    houses: false,
    trees: 1,
    rocks: 0.1,
  },
  gorge: {
    water: new Color3(0.13, 0.42, 0.55),
    bank: [new Color3(0.45, 0.43, 0.4), new Color3(0.55, 0.52, 0.48), new Color3(0.5, 0.47, 0.42)],
    ys: [-0.5, 1.4, 2.6, 3.6, 5.5, 9],
    houses: false,
    trees: 0.25,
    rocks: 1,
  },
};

/** Water edge (x) for a river of `lanes` lanes. */
const halfFor = (lanes: number) => (LANE_W * lanes) / 2 + 1.0;
const HALF3 = halfFor(3);
const HALF_MAX = halfFor(6);
const CHUNK = 40;
const AHEAD = 9;

// Scratch objects shared by every batch (no allocation per instance per frame).
const T_SCALE = new Vector3();
const T_POS = new Vector3();
const T_ROT = new Quaternion();
const T_MAT = new Matrix();

/**
 * Thin-instance batch rebuilt every frame. Matrices are written into one growing
 * Float32Array that is reused, and the GPU buffer is updated in place unless it
 * has to grow, so a frame allocates nothing here.
 */
class Batch {
  private buf = new Float32Array(16 * 16);
  private count = 0;
  private bound: Float32Array | null = null;
  constructor(readonly mesh: Mesh) {
    mesh.isVisible = false;
    mesh.isPickable = false;
    mesh.alwaysSelectAsActiveMesh = true;
  }
  begin() {
    this.count = 0;
  }
  add(x: number, y: number, z: number, yaw = 0, s = 1, pitch = 0, roll = 0, sx = s, sz = s) {
    if ((this.count + 1) * 16 > this.buf.length) {
      const bigger = new Float32Array(this.buf.length * 2);
      bigger.set(this.buf);
      this.buf = bigger;
    }
    T_SCALE.set(sx, s, sz);
    T_POS.set(x, y, z);
    Quaternion.RotationYawPitchRollToRef(yaw, pitch, roll, T_ROT);
    Matrix.ComposeToRef(T_SCALE, T_ROT, T_POS, T_MAT);
    T_MAT.copyToArray(this.buf, this.count * 16);
    this.count++;
  }
  commit() {
    if (!this.count) {
      this.mesh.thinInstanceCount = 0;
      this.mesh.isVisible = false;
      return;
    }
    if (this.bound !== this.buf) {
      this.mesh.thinInstanceSetBuffer("matrix", this.buf, 16, false);
      this.bound = this.buf;
    } else {
      this.mesh.thinInstanceBufferUpdated("matrix");
    }
    this.mesh.thinInstanceCount = this.count;
    this.mesh.isVisible = true;
  }
}

export interface BoatPose {
  seat: number;
  x: number;
  y: number;
  z: number;
  roll: number;
  yaw: number;
  boosting: boolean;
  /** Protected after a capsize: the boat blinks. */
  ghost?: boolean;
  /** Fixing a just-plugged pipe: the crew member hammers away. */
  fixing?: boolean;
}

/** The local boat, for danger hints on the water. */
export interface Viewer {
  lane: number;
  z: number;
  speed: number;
}

/** Logs closer than this (seconds at current speed) get a warning shadow. */
const WARN_SECONDS = 1.5;

export class RiverScene {
  readonly scene: Scene;
  readonly camera: UniversalCamera;
  private water: Mesh;
  private waterMat: StandardMaterial;
  private waterTex: Texture;
  private vc: StandardMaterial;
  private objBatches: Record<string, Batch>;
  private bankBatches: Record<string, Batch>;
  private chunkMeshes = new Map<number, Mesh>();
  private chunkKey = "";
  /** seat → boat root; holds a quick code-built stand-in until the model loads. */
  private boats = new Map<number, TransformNode>();
  private built = new Map<number, BuiltBoat>();
  private splash: ParticleSystem;
  private wake: ParticleSystem;
  private shake = 0;
  private fovBoost = 0;
  private time = 0;
  private plumeMat: StandardMaterial;
  private bankMat: StandardMaterial;

  constructor(engine: AbstractEngine) {
    const scene = new Scene(engine);
    this.scene = scene;
    scene.performancePriority = ScenePerformancePriority.Intermediate;
    scene.skipPointerMovePicking = true;
    const sky = new Color3(0.78, 0.86, 0.9);
    scene.clearColor = new Color4(sky.r, sky.g, sky.b, 1);
    scene.fogMode = Scene.FOGMODE_LINEAR;
    scene.fogStart = 120;
    scene.fogEnd = perf.low ? 210 : 300;
    scene.fogColor = new Color3(0.8, 0.84, 0.82);
    const ip = scene.imageProcessingConfiguration;
    ip.toneMappingEnabled = true;
    ip.toneMappingType = ImageProcessingConfiguration.TONEMAPPING_ACES;
    ip.exposure = 1.3;
    ip.contrast = 1.15;
    ip.vignetteEnabled = true;
    ip.vignetteWeight = 1.2;

    const hemi = new HemisphericLight("sky", new Vector3(0.1, 1, -0.2), scene);
    hemi.intensity = 0.75;
    hemi.groundColor = new Color3(0.35, 0.3, 0.22);
    const sun = new DirectionalLight("sun", new Vector3(-0.4, -1, 0.6), scene);
    sun.intensity = 1.8;
    sun.diffuse = new Color3(1, 0.92, 0.78);

    this.skyDome();

    this.camera = new UniversalCamera("chase", new Vector3(0, 4, -8), scene);
    this.camera.minZ = 0.2;
    // Low-end mode: a shorter view (fog hides the cut), so far fewer objects are drawn.
    this.camera.maxZ = perf.low ? 230 : 400;
    this.camera.fov = 0.95;

    this.vc = new StandardMaterial("vc", scene);
    this.vc.specularColor = new Color3(0.08, 0.08, 0.08);
    this.bankMat = new StandardMaterial("bankMat", scene);
    this.bankMat.specularColor = Color3.Black();
    this.bankMat.backFaceCulling = false;

    // Dry Season: a mud flat covers the closed lanes, so the narrower river is obvious.
    this.mud = MeshBuilder.CreateGround("mud", { width: 1, height: 420, subdivisions: 1 }, scene);
    const mudMat = new StandardMaterial("mudMat", scene);
    mudMat.diffuseColor = new Color3(0.42, 0.33, 0.22);
    mudMat.specularColor = Color3.Black();
    this.mud.material = mudMat;
    this.mud.isPickable = false;
    this.mud.setEnabled(false);

    // Water: one plane that follows the camera; colour tracks River Health.
    this.water = MeshBuilder.CreateGround("water", { width: HALF_MAX * 2 + 2, height: 420, subdivisions: 1 }, scene);
    this.waterMat = new StandardMaterial("waterMat", scene);
    this.waterMat.specularColor = new Color3(0.9, 0.95, 1);
    this.waterMat.specularPower = 80;
    this.waterMat.alpha = 0.92;
    this.waterTex = waterNormals(scene);
    this.waterTex.uScale = (HALF_MAX * 2 + 2) / 6;
    this.waterTex.vScale = 420 / 6;
    this.waterMat.bumpTexture = this.waterTex;
    this.water.material = this.waterMat;
    this.water.isPickable = false;

    this.plumeMat = new StandardMaterial("plume", scene);
    this.plumeMat.diffuseColor = new Color3(0.16, 0.12, 0.07);
    this.plumeMat.specularColor = Color3.Black();
    this.plumeMat.alpha = 0.78;

    const flora = buildPrototypes(scene);
    for (const m of Object.values(flora)) m.isVisible = false;
    const withMat = (m: Mesh, mat: StandardMaterial = this.vc) => {
      m.material = mat;
      return m;
    };
    const plume = MeshBuilder.CreateDisc("plume", { radius: 1, tessellation: 20 }, scene);
    plume.rotation.x = Math.PI / 2;
    plume.bakeCurrentTransformIntoVertices();
    const disc = (name: string, colour: Color3, alpha: number) => {
      const d = MeshBuilder.CreateDisc(name, { radius: 1, tessellation: 24 }, scene);
      d.rotation.x = Math.PI / 2;
      d.bakeCurrentTransformIntoVertices();
      const m = new StandardMaterial(name + "-mat", scene);
      m.diffuseColor = colour;
      m.emissiveColor = colour.scale(0.6);
      m.specularColor = Color3.Black();
      m.alpha = alpha;
      m.disableDepthWrite = true;
      d.material = m;
      return d;
    };
    this.objBatches = {
      danger: new Batch(disc("danger", new Color3(0.35, 0.05, 0.04), 0.5)),
      net: new Batch(withMat(netMesh(scene, LANE_W * 2))),
      murk: new Batch(withMat(murkMesh(scene))),
      current: new Batch(withMat(currentMesh(scene), this.currentMat())),
      boulder: new Batch(withMat(flora.rock.clone("boulder"))),
      gap: new Batch(disc("gap", new Color3(0.45, 1, 0.6), 0.35)),
      bottle: new Batch(withMat(bottleMesh(scene, "#9fd3e8", 1))),
      bottle2: new Batch(withMat(bottleMesh(scene, "#7fbf7a", 2))),
      bag: new Batch(withMat(bagMesh(scene))),
      weed: new Batch(withMat(weedMesh(scene))),
      log: new Batch(withMat(logMesh(scene))),
      debris: new Batch(withMat(debrisMesh(scene))),
      pipe: new Batch(withMat(pipeMesh(scene))),
      plug: new Batch(withMat(plugMesh(scene))),
      plume: new Batch(withMat(plume, this.plumeMat)),
    };
    this.bankBatches = {
      houseA: new Batch(withMat(houseMesh(scene, "#e8d5a8", "#a8543a", 3))),
      houseB: new Batch(withMat(houseMesh(scene, "#a8c8d8", "#6a5a4a", 4))),
      houseC: new Batch(withMat(houseMesh(scene, "#e6a6a0", "#7a4a3a", 5))),
      steps: new Batch(withMat(stepsMesh(scene))),
      lamp: new Batch(withMat(lampMesh(scene))),
      reed: new Batch(withMat(reedMesh(scene))),
      tree0: new Batch(withMat(flora.rain0.clone("t0"))),
      tree1: new Batch(withMat(flora.rain1.clone("t1"))),
      fig: new Batch(withMat(flora.fig.clone("fig"))),
      rock: new Batch(withMat(flora.rock.clone("rk"))),
      grass: new Batch(withMat(flora.grass.clone("gr"))),
      footbridge: new Batch(withMat(bridgeMesh(scene, HALF3 * 2 + 4))),
    };
    for (const m of Object.values(flora)) m.dispose();

    const dot = new DynamicTexture("dot", { width: 32, height: 32 }, scene, false);
    const ctx = dot.getContext() as CanvasRenderingContext2D;
    const g = ctx.createRadialGradient(16, 16, 2, 16, 16, 15);
    g.addColorStop(0, "rgba(255,255,255,1)");
    g.addColorStop(1, "rgba(255,255,255,0)");
    ctx.fillStyle = g;
    ctx.fillRect(0, 0, 32, 32);
    dot.hasAlpha = true;
    dot.update();
    this.splash = particles(scene, dot, 400, new Color4(0.9, 0.97, 1, 0.9), 3.5);
    this.splash.manualEmitCount = 0;
    this.splash.start();
    this.wake = particles(scene, dot, 300, new Color4(0.95, 1, 1, 0.55), 1.4);
    this.wake.emitRate = 60;
    this.wake.start();
  }

  private skyDome() {
    const dome = MeshBuilder.CreateSphere("skydome", { diameter: 700, segments: 20, sideOrientation: 1 }, this.scene);
    const pos = dome.getVerticesData("position")!;
    const cols: number[] = [];
    const zen = new Color3(0.35, 0.58, 0.82),
      hor = new Color3(0.95, 0.86, 0.72);
    for (let i = 0; i < pos.length; i += 3) {
      const t = Math.max(0, pos[i + 1] / 350);
      const c = Color3.Lerp(hor, zen, Math.pow(t, 0.5));
      cols.push(c.r, c.g, c.b, 1);
    }
    dome.setVerticesData("color", cols);
    const m = new StandardMaterial("skym", this.scene);
    m.disableLighting = true;
    m.emissiveColor = Color3.White();
    m.fogEnabled = false;
    m.backFaceCulling = false;
    dome.material = m;
    dome.infiniteDistance = true;
    dome.isPickable = false;
  }

  private mud: Mesh;
  private laneCount = 3;
  private half = HALF3;
  private theme: Theme = THEMES.canal;
  private riverKey: River = "canal";
  private healthNow = 70;

  private currentMat() {
    const m = new StandardMaterial("current-mat", this.scene);
    m.diffuseColor = new Color3(0.85, 0.97, 1);
    m.emissiveColor = new Color3(0.5, 0.75, 0.8);
    m.specularColor = Color3.Black();
    m.alpha = 0.75;
    return m;
  }

  /** Switch the river's look (water, banks, what lines them). */
  setRiver(river: River) {
    if (river === this.riverKey || !THEMES[river]) return;
    this.riverKey = river;
    this.theme = THEMES[river];
    this.setHealth(this.healthNow);
    for (const m of this.chunkMeshes.values()) m.dispose();
    this.chunkMeshes.clear();
    this.chunkKey = "";
  }

  /** World x of a pipe on bank `side` (-1 left, +1 right). */
  pipeX(side: number) {
    return side * (this.half + 1.2);
  }

  /** World x of a lane's centre (lanes are numbered from 0 on the left). */
  laneX(lane: number) {
    return (lane - (this.laneCount - 1) / 2) * LANE_W;
  }

  /** Set the river width (3, 4 or 6 lanes): water, banks and bridges follow. */
  setLanes(count: number) {
    if (count === this.laneCount || count < 1) return;
    this.laneCount = count;
    this.half = halfFor(count);
    const w = this.half * 2 + 2;
    this.water.scaling.x = w / (HALF_MAX * 2 + 2);
    this.waterTex.uScale = w / 6;
    for (const m of this.chunkMeshes.values()) m.dispose();
    this.chunkMeshes.clear();
    this.chunkKey = "";
  }

  /** River Health 0..100 → water colour and murk. */
  setHealth(h: number) {
    this.healthNow = h;
    const t = Math.max(0, Math.min(1, h / 100));
    const clean = this.theme.water,
      murky = new Color3(0.3, 0.26, 0.14);
    const c = Color3.Lerp(murky, clean, t);
    this.waterMat.diffuseColor = c;
    this.waterMat.emissiveColor = c.scale(0.18);
    this.waterMat.alpha = 0.86 + (1 - t) * 0.12;
    this.scene.fogColor = Color3.Lerp(new Color3(0.62, 0.58, 0.48), new Color3(0.8, 0.84, 0.82), t);
  }

  /** Rebuild bank chunks around the camera when it moves into a new chunk. */
  private banks(camZ: number, open: number) {
    const first = Math.floor((camZ - 30) / CHUNK);
    const key = `${first}:${open}`;
    if (key === this.chunkKey) return;
    this.chunkKey = key;
    const wanted = new Set<number>();
    for (let i = first; i < first + AHEAD; i++) wanted.add(i);
    for (const [i, m] of this.chunkMeshes) {
      if (!wanted.has(i)) {
        m.dispose();
        this.chunkMeshes.delete(i);
      }
    }
    for (const i of wanted) if (!this.chunkMeshes.has(i)) this.chunkMeshes.set(i, this.bankChunk(i));
    for (const b of Object.values(this.bankBatches)) b.begin();
    for (const i of wanted) this.decorate(i, open);
    for (const b of Object.values(this.bankBatches)) b.commit();
  }

  private bankChunk(i: number): Mesh {
    const z0 = i * CHUNK;
    const xs = [0, 0.9, 1.6, 6, 14, 40];
    const ys = this.theme.ys;
    const [mud, path, land] = this.theme.bank;
    const parts: Mesh[] = [];
    for (const side of [-1, 1]) {
      const paths: Vector3[][] = [];
      const colors: Color4[] = [];
      for (let k = 0; k < xs.length; k++) {
        const row: Vector3[] = [];
        for (let s = 0; s <= 8; s++) {
          const z = z0 + (s / 8) * CHUNK;
          const n = fbm(z * 0.05 + side * 7, k * 1.3);
          row.push(new Vector3(side * (this.half + xs[k]), ys[k] + (k > 1 ? (n - 0.5) * 0.4 : 0), z));
          const base = k <= 1 ? mud : k === 2 ? path : land;
          const c = base.scale(0.85 + n * 0.3);
          colors.push(new Color4(c.r, c.g, c.b, 1));
        }
        paths.push(row);
      }
      // Mirrored side: reverse path order so the surface faces up on both banks.
      if (side < 0) {
        paths.reverse();
        const rows: Color4[][] = [];
        for (let k = 0; k < xs.length; k++) rows.push(colors.slice(k * 9, k * 9 + 9));
        colors.length = 0;
        for (const row of rows.reverse()) colors.push(...row);
      }
      parts.push(MeshBuilder.CreateRibbon("bank", { pathArray: paths, colors }, this.scene));
    }
    const m = Mesh.MergeMeshes(parts, true, true)!;
    m.material = this.bankMat;
    m.isPickable = false;
    m.freezeWorldMatrix();
    return m;
  }

  private decorate(i: number, open: number) {
    const r = seeded(i * 7919 + 13);
    const z0 = i * CHUNK;
    const b = this.bankBatches;
    const HALF = this.half;
    for (const side of [-1, 1]) {
      const edge = side * (HALF + 0.9);
      const th = this.theme;
      const top = th.ys[2];
      // Riverside stairs and lamps (towns only)
      if (th.houses && r() > 0.45) b.steps.add(side * (HALF + 0.2), 0, z0 + 8 + r() * 20, side > 0 ? 0 : Math.PI);
      if (th.houses) for (let k = 0; k < 3; k++) b.lamp.add(edge + side * 0.6, 0.62, z0 + k * 13 + 3);
      // Reeds at the water's edge
      for (let k = 0; k < 5; k++) b.reed.add(side * (HALF + 0.3 + r() * 0.5), -0.1, z0 + r() * CHUNK, r() * 6, 0.9 + r() * 0.6);
      // Row of houses with trees between
      let z = z0 + r() * 4;
      while (z < z0 + CHUNK) {
        const x = side * (HALF + 4.5 + r() * 3);
        const pick = r();
        if (!th.houses && r() > th.trees) {
          // gorge: boulders on the cliff tops instead of buildings
          b.rock.add(x, top, z, r() * 6, 3 + r() * 3);
          z += 3 + r() * 2;
        } else if (th.houses && pick < 0.55) {
          const key = (["houseA", "houseB", "houseC"] as const)[Math.floor(r() * 3)];
          b[key].add(x, 0.6, z, side > 0 ? 0 : Math.PI, 1);
          z += 3.2 + r() * 1.5;
        } else {
          const key = r() > 0.85 ? "fig" : r() > 0.5 ? "tree0" : "tree1";
          b[key].add(x + side * r() * 3, top, z, r() * 6, 2.6 + r() * 1.4);
          z += (th.houses ? 2.5 : 1.6) + r() * 2;
        }
      }
      for (let k = 0; k < 4; k++) b.grass.add(side * (HALF + 2 + r() * 8), th.ys[3] - 0.04, z0 + r() * CHUNK, r() * 6, 2);
      if (r() > 1 - th.rocks) b.rock.add(side * (HALF + 0.7), 0.1, z0 + r() * CHUNK, r() * 6, 2 + r());
      if (th.rocks >= 1) for (let k = 0; k < 3; k++) b.rock.add(side * (HALF + 1.2 + r() * 2), top * 0.6, z0 + r() * CHUNK, r() * 6, 4 + r() * 4);
    }
    // Dry season: the closed lanes on the right become mud flats with rocks.
    for (let lane = open; lane < this.laneCount; lane++) {
      for (let k = 0; k < 6; k++) b.rock.add(this.laneX(lane) + (r() - 0.5) * 1.6, -0.15, z0 + r() * CHUNK, r() * 6, 2.5 + r() * 2);
    }
    if (this.theme.houses && i % 6 === 3) b.footbridge.add(0, 0, z0 + 20, 0, 1, 0, 0, (HALF * 2 + 4) / (HALF3 * 2 + 4), 1);
  }

  /** Per-frame render of world objects (positions come from the server snapshot). */
  renderObjects(objs: RunObj[], open: number[], me: Viewer | null = null) {
    const HALF = this.half;
    const t = this.time;
    for (const bt of Object.values(this.objBatches)) bt.begin();
    const ob = this.objBatches;
    for (const o of objs) {
      if (o.kind !== "pipe" && o.kind !== "wave" && !open.includes(o.lane)) continue;
      const x = this.laneX(o.lane);
      const bob = Math.sin(t * 2.2 + o.id) * 0.06;
      switch (o.kind) {
        case "plastic":
          if (!o.alive) break;
          if (o.id % 3 === 0) ob.bag.add(x, 0.02 + bob, o.z, o.id, 1);
          else (o.id % 3 === 1 ? ob.bottle : ob.bottle2).add(x, 0.05 + bob, o.z, t * 0.8 + o.id, 1.4);
          break;
        case "weed":
          if (o.alive) ob.weed.add(x, -0.03 + bob * 0.4, o.z, o.id, 1.5);
          break;
        case "net":
          if (!o.alive) break;
          // spans its lane and the next; a cut net sags lower
          ob.net.add(x + LANE_W / 2, -0.02 + bob * 0.5, o.z, 0, 1, 0, 0, 1, (o.hp ?? 2) < 2 ? 0.55 : 1);
          break;
        case "current": {
          const d = o.dir ?? 1;
          const slide = ((t * 1.6 + o.id) % 1) * 0.5 * d;
          for (const dz of [-1.6, 0, 1.6]) ob.current.add(x + slide, 0.03, o.z + dz, d > 0 ? 0 : Math.PI, 1);
          break;
        }
        case "rock":
        case "log":
          if (!o.alive) break;
          if (o.kind === "rock") ob.boulder.add(x, -0.2, o.z, o.id, 6.5);
          else ob.log.add(x, 0.02 + bob, o.z, 0.15 * Math.sin(o.id), 1);
          if (me && o.lane === me.lane) {
            // A dark shadow grows under a log in your lane as it gets close.
            const secs = (o.z - me.z) / Math.max(me.speed, 1);
            if (secs > 0 && secs < WARN_SECONDS) {
              const k = 1 - secs / WARN_SECONDS;
              ob.danger.add(x, -0.02, o.z, 0, 1, 0, 0, 0.8 + 0.5 * k, 1.4 + 0.8 * k);
            }
          }
          break;
        case "wave":
          if (!o.alive) break;
          // The gap lane glows so it can be found at a glance.
          for (const g of [o.lane, o.gap2]) if (g !== undefined && g !== null) ob.gap.add(this.laneX(g), -0.02, o.z, 0, 1, 0, 0, 1.1 + 0.1 * Math.sin(t * 6), 3.5);
          for (const lane of open) if (lane !== o.lane && lane !== o.gap2) ob.debris.add(this.laneX(lane), 0.05 + bob, o.z, 0, 1);
          break;
        case "pipe": {
          // Big enough to spot from far away; the brown plume spreads well into the river.
          const px = o.lane * (HALF + 1.2);
          ob.pipe.add(px, 0.35, o.z, o.lane > 0 ? 0 : Math.PI, 1.6);
          if (o.plugged) ob.plug.add(px, 0.35, o.z, o.lane > 0 ? 0 : Math.PI, 1.6);
          else {
            ob.plume.add(o.lane * (HALF - 1.6), -0.04, o.z + 6, 0, 1, 0, 0, 2.4 + Math.sin(t * 1.5 + o.id) * 0.3, 9);
            // A grumpy smog cloud sulks over every open pipe, bobbing and puffing.
            const puff = 1 + Math.sin(t * 3 + o.id) * 0.06;
            ob.murk.add(px - o.lane * 0.6, 2.6 + Math.sin(t * 1.7 + o.id) * 0.25, o.z, Math.sin(t * 0.8 + o.id) * 0.15, 1.7 * puff);
          }
          break;
        }
      }
    }
    for (const bt of Object.values(ob)) bt.commit();
  }

  private skins: Record<string, string> = {};

  /** Equipped boats (seat → boat key). Boats already built are rebuilt. */
  setSkins(skins: Record<string, string>) {
    for (const [seat, key] of Object.entries(skins)) {
      if (this.skins[seat] !== key) {
        this.skins[seat] = key;
        this.dropBoat(Number(seat));
      }
    }
  }

  private dropBoat(seat: number) {
    this.built.get(seat)?.dispose();
    this.built.delete(seat);
    this.boats.get(seat)?.dispose();
    this.boats.delete(seat);
  }

  private boatFor(seat: number): TransformNode {
    let root = this.boats.get(seat);
    if (!root) {
      const node = new TransformNode(`seat-${seat}`, this.scene);
      root = node;
      const standIn = boatMesh(this.scene, SEAT_COLOURS[seat % SEAT_COLOURS.length], 40 + seat);
      standIn.material = this.vc;
      standIn.isPickable = false;
      standIn.parent = node;
      this.boats.set(seat, node);
      const key = this.skins[String(seat)] ?? DEFAULT_BOAT;
      buildBoat(this.scene, key, seat)
        .then((b) => {
          if (this.boats.get(seat) !== node) return b.dispose(); // replaced while loading
          standIn.dispose();
          b.root.parent = node;
          this.built.set(seat, b);
        })
        .catch((e) => console.warn("boat model failed, keeping stand-in", e));
    }
    return root;
  }

  /** Place boats, move camera and water. `me` is the local player's pose. */
  frame(dt: number, poses: BoatPose[], me: BoatPose | null, open: number) {
    this.time += dt;
    const seen = new Set<number>();
    for (const p of poses) {
      const m = this.boatFor(p.seat);
      m.position.set(p.x, -0.12 + p.y + Math.sin(this.time * 3 + p.seat) * 0.03, p.z);
      m.rotation.set(Math.sin(this.time * 2.3 + p.seat) * 0.03, p.yaw, p.roll);
      if (!p.ghost || Math.floor(this.time * 10) % 2 === 0) seen.add(p.seat);
      // Fixing a pipe: the crew member leans in and hammers.
      const person = this.built.get(p.seat)?.person;
      if (person) {
        person.rotation.x = p.fixing ? -0.35 + Math.abs(Math.sin(this.time * 16)) * 0.55 : 0;
        person.rotation.z = p.fixing ? Math.sin(this.time * 8) * 0.12 : 0;
      }
    }
    for (const [seat, m] of this.boats) m.setEnabled(seen.has(seat));

    const focus = me ?? poses[0];
    if (focus) {
      const cam = this.camera;
      const target = new Vector3(focus.x * 0.85, 0.9, focus.z + 9);
      const desired = new Vector3(focus.x * 0.7, 4.3, focus.z - 8.5);
      Vector3.LerpToRef(cam.position, desired, Math.min(1, dt * 6), cam.position);
      cam.position.z = desired.z; // never lag behind on the forward axis
      cam.setTarget(target);
      if (this.shake > 0) {
        // Trauma shake: strength is trauma², smooth summed sines, decays in real time.
        const k = this.shake * this.shake * this.shakeScale;
        const tt = this.time * 30;
        cam.position.x += 0.6 * k * Math.sin(tt * 1.7);
        cam.position.y += 0.4 * k * Math.sin(tt * 2.3);
        target.x += 0.25 * k * Math.sin(tt * 1.1);
        cam.setTarget(target);
        this.shake = Math.max(0, this.shake - dt * 1.4);
      }
      this.fovBoost += ((focus.boosting ? 1 : 0) - this.fovBoost) * Math.min(1, dt * 4);
      cam.fov = 0.95 + this.fovBoost * 0.18;
      this.water.position.set(0, -0.05, Math.floor(focus.z / 6) * 6 + 170);
      // Closed lanes (Dry Season) become dry mud from the last open lane to the right bank.
      const closed = this.laneCount - open;
      this.mud.setEnabled(closed > 0);
      if (closed > 0) {
        const left = this.laneX(open) - LANE_W / 2;
        const right = this.half + 0.2;
        this.mud.scaling.x = right - left;
        this.mud.position.set((left + right) / 2, 0.02, this.water.position.z);
      }
      this.waterTex.vOffset -= dt * 0.12;
      this.waterTex.uOffset = Math.sin(this.time * 0.3) * 0.02;
      this.banks(focus.z, open);
      this.wake.emitter = new Vector3(focus.x, 0.05, focus.z - 1.1);
      this.wake.emitRate = (focus.boosting ? 160 : 55) * (perf.low ? 0.5 : 1);
    }
  }

  burst(x: number, z: number, count = 24, colour?: Color4) {
    this.splash.emitter = new Vector3(x, 0.2, z);
    if (colour) this.splash.color1 = colour;
    this.splash.manualEmitCount = perf.low ? Math.ceil(count / 2) : count;
  }

  /** Player's shake setting, 0–1. */
  shakeScale = 1;

  /** Add camera trauma (0–1); hits stack, they don't reset. */
  kick(amount: number) {
    this.shake = Math.min(1, this.shake + amount);
  }

  worldToScreen(p: Vector3): { x: number; y: number; visible: boolean } {
    const engine = this.scene.getEngine();
    const w = engine.getRenderWidth(),
      h = engine.getRenderHeight();
    const v = Vector3.Project(p, Matrix.Identity(), this.scene.getTransformMatrix(), this.camera.viewport.toGlobal(w, h));
    const scale = engine.getHardwareScalingLevel();
    return { x: v.x * scale, y: v.y * scale, visible: v.z > 0 && v.z < 1 };
  }

  dispose() {
    this.scene.dispose();
  }
}

function particles(scene: Scene, tex: DynamicTexture, cap: number, colour: Color4, power: number): ParticleSystem {
  const ps = new ParticleSystem("p", cap, scene);
  ps.particleTexture = tex;
  ps.color1 = colour;
  ps.color2 = new Color4(colour.r, colour.g, colour.b, colour.a * 0.6);
  ps.colorDead = new Color4(1, 1, 1, 0);
  ps.minSize = 0.12;
  ps.maxSize = 0.35;
  ps.minLifeTime = 0.3;
  ps.maxLifeTime = 0.8;
  ps.direction1 = new Vector3(-1, power, -1);
  ps.direction2 = new Vector3(1, power * 1.6, 1);
  ps.gravity = new Vector3(0, -9, 0);
  ps.minEmitPower = 0.6;
  ps.maxEmitPower = 1.4;
  ps.emitRate = 0;
  ps.blendMode = ParticleSystem.BLENDMODE_STANDARD;
  ps.emitter = Vector3.Zero();
  return ps;
}

/** Tileable water normal map from fBm (generated, no download). */
function waterNormals(scene: Scene): Texture {
  const S = 128;
  const tex = new DynamicTexture("wn", { width: S, height: S }, scene, true);
  const ctx = tex.getContext() as CanvasRenderingContext2D;
  const img = ctx.createImageData(S, S);
  const h = (x: number, y: number) => {
    const a = (x / S) * Math.PI * 2,
      b = (y / S) * Math.PI * 2;
    return fbm(Math.cos(a) * 1.4 + 3, Math.sin(a) * 1.4 + Math.cos(b) * 2 + 7, 3) + 0.6 * fbm(Math.sin(b) * 2.2 + 1, Math.cos(b) * 2.2 + 5, 3);
  };
  for (let y = 0; y < S; y++)
    for (let x = 0; x < S; x++) {
      const dx = h(x + 1, y) - h(x - 1, y),
        dy = h(x, y + 1) - h(x, y - 1);
      const n = new Vector3(-dx * 4, -dy * 4, 1).normalize();
      const i = (y * S + x) * 4;
      img.data[i] = (n.x * 0.5 + 0.5) * 255;
      img.data[i + 1] = (n.y * 0.5 + 0.5) * 255;
      img.data[i + 2] = (n.z * 0.5 + 0.5) * 255;
      img.data[i + 3] = 255;
    }
  ctx.putImageData(img, 0, 0);
  tex.update();
  tex.wrapU = tex.wrapV = Texture.WRAP_ADDRESSMODE;
  tex.level = 0.45;
  return tex;
}

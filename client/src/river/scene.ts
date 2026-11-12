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
} from "./props";
import { LANE_W, SEAT_COLOURS, type RunObj } from "./types";

export const HALF = LANE_W * 1.5 + 1.0; // water edge (x)
const CHUNK = 40;
const AHEAD = 9;

/** Thin-instance batch rebuilt from a list of matrices. */
class Batch {
  private data: number[] = [];
  constructor(readonly mesh: Mesh) {
    mesh.isVisible = false;
    mesh.isPickable = false;
    mesh.alwaysSelectAsActiveMesh = true;
  }
  begin() {
    this.data.length = 0;
  }
  add(x: number, y: number, z: number, yaw = 0, s = 1, pitch = 0, roll = 0, sx = s, sz = s) {
    const m = Matrix.Compose(new Vector3(sx, s, sz), Quaternion.RotationYawPitchRoll(yaw, pitch, roll), new Vector3(x, y, z));
    for (const v of m.asArray()) this.data.push(v);
  }
  commit() {
    if (this.data.length) {
      this.mesh.thinInstanceSetBuffer("matrix", new Float32Array(this.data), 16, false);
      this.mesh.isVisible = true;
    } else {
      this.mesh.thinInstanceCount = 0;
      this.mesh.isVisible = false;
    }
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
}

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
  private boats = new Map<number, Mesh>();
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
    scene.fogEnd = 300;
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
    this.camera.maxZ = 400;
    this.camera.fov = 0.95;

    this.vc = new StandardMaterial("vc", scene);
    this.vc.specularColor = new Color3(0.08, 0.08, 0.08);
    this.bankMat = new StandardMaterial("bankMat", scene);
    this.bankMat.specularColor = Color3.Black();
    this.bankMat.backFaceCulling = false;

    // Water: one plane that follows the camera; colour tracks River Health.
    this.water = MeshBuilder.CreateGround("water", { width: HALF * 2 + 2, height: 420, subdivisions: 1 }, scene);
    this.waterMat = new StandardMaterial("waterMat", scene);
    this.waterMat.specularColor = new Color3(0.9, 0.95, 1);
    this.waterMat.specularPower = 80;
    this.waterMat.alpha = 0.92;
    this.waterTex = waterNormals(scene);
    this.waterTex.uScale = (HALF * 2 + 2) / 6;
    this.waterTex.vScale = 420 / 6;
    this.waterMat.bumpTexture = this.waterTex;
    this.water.material = this.waterMat;
    this.water.isPickable = false;

    this.plumeMat = new StandardMaterial("plume", scene);
    this.plumeMat.diffuseColor = new Color3(0.16, 0.12, 0.07);
    this.plumeMat.specularColor = Color3.Black();
    this.plumeMat.alpha = 0.6;

    const flora = buildPrototypes(scene);
    for (const m of Object.values(flora)) m.isVisible = false;
    const withMat = (m: Mesh, mat: StandardMaterial = this.vc) => {
      m.material = mat;
      return m;
    };
    const plume = MeshBuilder.CreateDisc("plume", { radius: 1, tessellation: 20 }, scene);
    plume.rotation.x = Math.PI / 2;
    plume.bakeCurrentTransformIntoVertices();
    this.objBatches = {
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
      footbridge: new Batch(withMat(bridgeMesh(scene, HALF * 2 + 4))),
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

  laneX(lane: number) {
    return lane * LANE_W;
  }

  /** River Health 0..100 → water colour and murk. */
  setHealth(h: number) {
    const t = Math.max(0, Math.min(1, h / 100));
    const clean = new Color3(0.1, 0.42, 0.46),
      murky = new Color3(0.3, 0.26, 0.14);
    const c = Color3.Lerp(murky, clean, t);
    this.waterMat.diffuseColor = c;
    this.waterMat.emissiveColor = c.scale(0.18);
    this.waterMat.alpha = 0.86 + (1 - t) * 0.12;
    this.scene.fogColor = Color3.Lerp(new Color3(0.62, 0.58, 0.48), new Color3(0.8, 0.84, 0.82), t);
  }

  /** Rebuild bank chunks around the camera when it moves into a new chunk. */
  private banks(camZ: number, dry: boolean) {
    const first = Math.floor((camZ - 30) / CHUNK);
    const key = `${first}:${dry}`;
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
    for (const i of wanted) this.decorate(i, dry);
    for (const b of Object.values(this.bankBatches)) b.commit();
  }

  private bankChunk(i: number): Mesh {
    const z0 = i * CHUNK;
    const xs = [0, 0.9, 1.6, 6, 14, 40];
    const ys = [-0.5, 0.45, 0.62, 0.66, 0.8, 1.6];
    const parts: Mesh[] = [];
    for (const side of [-1, 1]) {
      const paths: Vector3[][] = [];
      const colors: Color4[] = [];
      for (let k = 0; k < xs.length; k++) {
        const row: Vector3[] = [];
        for (let s = 0; s <= 8; s++) {
          const z = z0 + (s / 8) * CHUNK;
          const n = fbm(z * 0.05 + side * 7, k * 1.3);
          row.push(new Vector3(side * (HALF + xs[k]), ys[k] + (k > 1 ? (n - 0.5) * 0.4 : 0), z));
          const base = k <= 1 ? new Color3(0.55, 0.5, 0.42) : k === 2 ? new Color3(0.62, 0.56, 0.44) : new Color3(0.38, 0.5, 0.26);
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

  private decorate(i: number, dry: boolean) {
    const r = seeded(i * 7919 + 13);
    const z0 = i * CHUNK;
    const b = this.bankBatches;
    for (const side of [-1, 1]) {
      const edge = side * (HALF + 0.9);
      // Riverside stairs and lamps
      if (r() > 0.45) b.steps.add(side * (HALF + 0.2), 0, z0 + 8 + r() * 20, side > 0 ? 0 : Math.PI);
      for (let k = 0; k < 3; k++) b.lamp.add(edge + side * 0.6, 0.62, z0 + k * 13 + 3);
      // Reeds at the water's edge
      for (let k = 0; k < 5; k++) b.reed.add(side * (HALF + 0.3 + r() * 0.5), -0.1, z0 + r() * CHUNK, r() * 6, 0.9 + r() * 0.6);
      // Row of houses with trees between
      let z = z0 + r() * 4;
      while (z < z0 + CHUNK) {
        const x = side * (HALF + 4.5 + r() * 3);
        const pick = r();
        if (pick < 0.55) {
          const key = (["houseA", "houseB", "houseC"] as const)[Math.floor(r() * 3)];
          b[key].add(x, 0.6, z, side > 0 ? 0 : Math.PI, 1);
          z += 3.2 + r() * 1.5;
        } else {
          const key = r() > 0.85 ? "fig" : r() > 0.5 ? "tree0" : "tree1";
          b[key].add(x + side * r() * 3, 0.6, z, r() * 6, 2.6 + r() * 1.4);
          z += 2.5 + r() * 2;
        }
      }
      for (let k = 0; k < 4; k++) b.grass.add(side * (HALF + 2 + r() * 8), 0.62, z0 + r() * CHUNK, r() * 6, 2);
      if (r() > 0.7) b.rock.add(side * (HALF + 0.7), 0.1, z0 + r() * CHUNK, r() * 6, 2 + r());
    }
    if (dry) {
      // Dry season: the right lane becomes a mud flat with rocks
      for (let k = 0; k < 6; k++) b.rock.add(LANE_W + (r() - 0.5) * 1.6, -0.15, z0 + r() * CHUNK, r() * 6, 2.5 + r() * 2);
    }
    if (i % 6 === 3) b.footbridge.add(0, 0, z0 + 20);
  }

  /** Per-frame render of world objects (positions come from the server snapshot). */
  renderObjects(objs: RunObj[], dry: boolean) {
    const t = this.time;
    for (const bt of Object.values(this.objBatches)) bt.begin();
    const ob = this.objBatches;
    for (const o of objs) {
      if (dry && o.lane === 1 && o.kind !== "pipe") continue;
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
        case "log":
          if (o.alive) ob.log.add(x, 0.02 + bob, o.z, 0.15 * Math.sin(o.id), 1);
          break;
        case "wave":
          if (!o.alive) break;
          for (const lane of dry ? [-1, 0] : [-1, 0, 1]) if (lane !== o.lane) ob.debris.add(this.laneX(lane), 0.05 + bob, o.z, 0, 1);
          break;
        case "pipe": {
          const px = o.lane * (HALF + 0.9);
          ob.pipe.add(px, 0.2, o.z, o.lane > 0 ? 0 : Math.PI);
          if (o.plugged) ob.plug.add(px, 0.2, o.z, o.lane > 0 ? 0 : Math.PI);
          else ob.plume.add(o.lane * (HALF - 1.2), -0.06, o.z + 5, 0, 1, 0, 0, 1.6 + Math.sin(t * 1.5 + o.id) * 0.2, 7);
          break;
        }
      }
    }
    for (const bt of Object.values(ob)) bt.commit();
  }

  private skins: Record<string, string> = {};

  /** Equipped boat colours (seat → hex). Boats already built are rebuilt. */
  setSkins(skins: Record<string, string>) {
    for (const [seat, colour] of Object.entries(skins)) {
      if (this.skins[seat] !== colour) {
        this.skins[seat] = colour;
        this.boats.get(Number(seat))?.dispose();
        this.boats.delete(Number(seat));
      }
    }
  }

  private boatFor(seat: number): Mesh {
    let m = this.boats.get(seat);
    if (!m) {
      m = boatMesh(this.scene, this.skins[String(seat)] ?? SEAT_COLOURS[seat % SEAT_COLOURS.length], 40 + seat);
      m.material = this.vc;
      m.isPickable = false;
      this.boats.set(seat, m);
    }
    return m;
  }

  /** Place boats, move camera and water. `me` is the local player's pose. */
  frame(dt: number, poses: BoatPose[], me: BoatPose | null, dry: boolean) {
    this.time += dt;
    const seen = new Set<number>();
    for (const p of poses) {
      const m = this.boatFor(p.seat);
      m.position.set(p.x, -0.12 + p.y + Math.sin(this.time * 3 + p.seat) * 0.03, p.z);
      m.rotation.set(Math.sin(this.time * 2.3 + p.seat) * 0.03, p.yaw, p.roll);
      seen.add(p.seat);
    }
    for (const [seat, m] of this.boats) m.setEnabled(seen.has(seat));

    const focus = me ?? poses[0];
    if (focus) {
      const cam = this.camera;
      const target = new Vector3(focus.x * 0.85, 0.9, focus.z + 9);
      const desired = new Vector3(focus.x * 0.7, 4.3, focus.z - 8.5);
      Vector3.LerpToRef(cam.position, desired, Math.min(1, dt * 6), cam.position);
      cam.position.z = desired.z; // never lag behind on the forward axis
      if (this.shake > 0) {
        cam.position.x += (Math.random() - 0.5) * this.shake;
        cam.position.y += (Math.random() - 0.5) * this.shake;
        this.shake = Math.max(0, this.shake - dt * 1.6);
      }
      cam.setTarget(target);
      this.fovBoost += ((focus.boosting ? 1 : 0) - this.fovBoost) * Math.min(1, dt * 4);
      cam.fov = 0.95 + this.fovBoost * 0.18;
      this.water.position.set(0, -0.05, Math.floor(focus.z / 6) * 6 + 170);
      this.waterTex.vOffset -= dt * 0.12;
      this.waterTex.uOffset = Math.sin(this.time * 0.3) * 0.02;
      this.banks(focus.z, dry);
      this.wake.emitter = new Vector3(focus.x, 0.05, focus.z - 1.1);
      this.wake.emitRate = focus.boosting ? 160 : 55;
    }
  }

  burst(x: number, z: number, count = 24, colour?: Color4) {
    this.splash.emitter = new Vector3(x, 0.2, z);
    if (colour) this.splash.color1 = colour;
    this.splash.manualEmitCount = count;
  }

  kick(amount: number) {
    this.shake = Math.max(this.shake, amount);
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

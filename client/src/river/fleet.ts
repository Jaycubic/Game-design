// Boats and their crews as real models: Kenney "Watercraft Kit" boats with a
// Kenney "Mini Characters" person aboard (both CC0, in public/models).
// Each model is loaded once per scene and instanced per player.

import { AssetContainer, LoadAssetContainerAsync, PBRMaterial, Scene, StandardMaterial, TransformNode, Color3, type Material } from "@babylonjs/core";
import "@babylonjs/loaders/glTF";

/** Boat key (server catalog) → model file. Keep in step with crates/game-server/src/meta.rs. */
export const BOAT_MODELS: Record<string, string> = {
  boat_sun: "boat-row-small",
  boat_sky: "boat-fishing-small",
  boat_coral: "boat-speed-c",
  boat_lilac: "boat-speed-a",
  boat_leaf: "boat-speed-f",
  boat_night: "boat-speed-g",
  boat_lotus: "boat-tug-c",
};
export const DEFAULT_BOAT = "boat_sun";
const CREW = ["character-male-a", "character-female-b", "character-male-d", "character-female-e"];
const BASE = `${import.meta.env.BASE_URL}models/`;

/** Boat length in world units (a lane is 2.4 wide). */
const BOAT_LEN = 2.5;
const CREW_HEIGHT = 0.95;

const cache = new WeakMap<Scene, Map<string, Promise<AssetContainer>>>();

function load(scene: Scene, path: string): Promise<AssetContainer> {
  let byPath = cache.get(scene);
  if (!byPath) cache.set(scene, (byPath = new Map()));
  let p = byPath.get(path);
  if (!p) {
    p = LoadAssetContainerAsync(BASE + path, scene).then((c) => (matte(c, scene), c));
    byPath.set(path, p);
  }
  return p;
}

/** Swap glTF PBR materials for plain matte ones so models match the hand-shaded world. */
function matte(c: AssetContainer, scene: Scene) {
  const swapped = new Map<Material, StandardMaterial>();
  for (const m of c.meshes) {
    const pbr = m.material;
    if (!(pbr instanceof PBRMaterial)) continue;
    let s = swapped.get(pbr);
    if (!s) {
      s = new StandardMaterial(pbr.name + "-matte", scene);
      s.diffuseTexture = pbr.albedoTexture;
      s.diffuseColor = pbr.albedoTexture ? Color3.White() : pbr.albedoColor;
      s.specularColor = new Color3(0.06, 0.06, 0.06);
      s.backFaceCulling = pbr.backFaceCulling;
      swapped.set(pbr, s);
      c.materials.push(s);
    }
    m.material = s;
  }
}

/** Scale a model so its longest horizontal side is `size`, centred, sitting on y = 0. */
function fit(node: TransformNode, size: number, by: "length" | "height") {
  node.computeWorldMatrix(true);
  const { min, max } = node.getHierarchyBoundingVectors(true);
  const span = by === "height" ? max.y - min.y : Math.max(max.x - min.x, max.z - min.z);
  const k = size / (span || 1);
  node.scaling.scaleInPlace(k);
  node.position.set(-((min.x + max.x) / 2) * k, -min.y * k, -((min.z + max.z) / 2) * k);
  return { height: (max.y - min.y) * k, length: (max.z - min.z) * k };
}

export interface BuiltBoat {
  root: TransformNode;
  dispose(): void;
}

/** Build one player's boat with a crew member aboard. */
export async function buildBoat(scene: Scene, key: string, seat: number): Promise<BuiltBoat> {
  const model = BOAT_MODELS[key] ?? BOAT_MODELS[DEFAULT_BOAT];
  const [boatC, crewC] = await Promise.all([load(scene, `boats/${model}.glb`), load(scene, `crew/${CREW[seat % CREW.length]}.glb`)]);
  const root = new TransformNode(`boat-${seat}`, scene);

  const boat = boatC.instantiateModelsToScene((n) => `${n}-${seat}`, false, { doNotInstantiate: true });
  const hull = new TransformNode(`hull-${seat}`, scene);
  hull.parent = root;
  for (const n of boat.rootNodes) n.parent = hull;
  const size = fit(hull, BOAT_LEN, "length");
  hull.position.y -= size.height * 0.18; // sit in the water, not on it

  const crew = crewC.instantiateModelsToScene((n) => `${n}-c${seat}`, false, { doNotInstantiate: true });
  const person = new TransformNode(`crew-${seat}`, scene);
  person.parent = root;
  for (const n of crew.rootNodes) n.parent = person;
  fit(person, CREW_HEIGHT, "height");
  const rowing = model === "boat-row-small";
  person.position.y += size.height * (rowing ? 0.12 : 0.3);
  person.position.z += rowing ? 0 : -BOAT_LEN * 0.12;
  const pose = crew.animationGroups.find((a) => a.name === (rowing ? "sit" : "drive")) ?? crew.animationGroups.find((a) => a.name === "idle");
  for (const a of crew.animationGroups) if (a !== pose) a.stop();
  pose?.start(true);

  for (const m of root.getChildMeshes()) {
    m.isPickable = false;
    m.alwaysSelectAsActiveMesh = false;
  }
  return {
    root,
    dispose() {
      boat.dispose();
      crew.dispose();
      root.dispose();
    },
  };
}

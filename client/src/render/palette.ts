import { Color3 } from "@babylonjs/core";

const hex = (h: string) => Color3.FromHexString(h);

/** Shared colours for procedural vegetation and props. */
export const C = {
  bark: hex("#4b3524"),
  barkLight: hex("#6b4c32"),
  leafDeep: hex("#1f4a1e"),
  leaf: hex("#2f6b25"),
  leafLight: hex("#5a8f2e"),
  leafYoung: hex("#86b545"),
  fig: hex("#c2502a"),
  figLeaf: hex("#2d5e2a"),
  lantana: hex("#5c6b2c"),
  flowerPink: hex("#e0559b"),
  flowerOrange: hex("#f08a2b"),
  stump: hex("#6d5a47"),
  deadwood: hex("#8a7a68"),
  grass: hex("#7f9a3a"),
  grassGold: hex("#b9b25a"),
  rock: hex("#8c877c"),
  rope: hex("#b8925a"),
  plank: hex("#7a5534"),
  marking: hex("#e8e4d4"),
  stake: hex("#c8a165"),
  flag: hex("#f2c14e"),
};

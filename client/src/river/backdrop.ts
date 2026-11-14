// Calm river scene behind the menus: a few boats cruising, litter drifting, slow camera.

import type { AbstractEngine } from "@babylonjs/core";
import { RiverScene, type BoatPose } from "./scene";
import { LANE_W, type RunObj } from "./types";

export class MenuBackdrop {
  private scene: RiverScene;
  private z = 0;
  private t = 0;
  private last = performance.now();

  constructor(private engine: AbstractEngine) {
    this.scene = new RiverScene(engine);
    this.scene.setHealth(88);
    engine.runRenderLoop(() => this.frame());
  }

  private litter(): RunObj[] {
    // Deterministic litter every 9 units around the camera.
    const out: RunObj[] = [];
    const start = Math.floor(this.z / 9) * 9;
    for (let k = 0; k < 30; k++) {
      const z = start + k * 9;
      const id = Math.floor(z / 9);
      const lane = (id * 7) % 3;
      const kind = id % 11 === 0 ? "weed" : id % 17 === 0 ? "log" : "plastic";
      out.push({ id, kind, lane, z: z + 20, alive: true, plugged: false });
    }
    return out;
  }

  private frame() {
    const now = performance.now();
    const dt = Math.min(0.1, (now - this.last) / 1000);
    this.last = now;
    this.t += dt;
    this.z += dt * 6;
    const poses: BoatPose[] = [0, 1, 2].map((i) => ({
      seat: i,
      x: Math.sin(this.t * 0.35 + i * 2.1) * LANE_W,
      y: 0,
      z: this.z - i * 5 + Math.sin(this.t * 0.5 + i) * 1.5,
      roll: Math.cos(this.t * 0.35 + i * 2.1) * -0.06,
      yaw: Math.cos(this.t * 0.35 + i * 2.1) * 0.1,
      boosting: false,
    }));
    const me = { ...poses[0], x: poses[0].x * 0.4 };
    this.scene.frame(dt, poses, me, 3);
    this.scene.renderObjects(this.litter(), [0, 1, 2]);
    this.scene.scene.render();
  }

  dispose() {
    this.engine.stopRenderLoop();
    this.scene.dispose();
  }
}

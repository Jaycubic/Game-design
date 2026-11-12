import { AbstractEngine, Engine, WebGPUEngine } from "@babylonjs/core";

export type Backend = "webgpu" | "webgl2" | "webgl1";
export type Quality = "low" | "medium" | "high";

export interface EngineInfo {
  engine: AbstractEngine;
  backend: Backend;
  reason: string;
}

/**
 * WebGPU when the page is a secure context (HTTPS or localhost) and the
 * browser supports it; otherwise WebGL2. Over plain HTTP on the LAN the
 * browser hides navigator.gpu, so we fall back and say why.
 */
export async function createEngine(canvas: HTMLCanvasElement, preferWebGPU = true): Promise<EngineInfo> {
  const nav = navigator as Navigator & { gpu?: unknown };
  let reason = "";
  if (!preferWebGPU) reason = "WebGPU disabled in settings";
  else if (!window.isSecureContext) reason = "WebGPU needs HTTPS (open the game via https://)";
  else if (!nav.gpu) reason = "this browser does not expose WebGPU";
  else {
    try {
      const supported = await WebGPUEngine.IsSupportedAsync;
      if (supported) {
        const engine = new WebGPUEngine(canvas, { antialias: true, stencil: true, adaptToDeviceRatio: false, powerPreference: "high-performance" });
        await engine.initAsync();
        return { engine, backend: "webgpu", reason: "WebGPU" };
      }
      reason = "no compatible GPU adapter";
    } catch (e) {
      reason = "WebGPU failed to start (" + (e instanceof Error ? e.message : String(e)) + ")";
    }
  }
  const engine = new Engine(canvas, true, { stencil: true, preserveDrawingBuffer: false, powerPreference: "high-performance" }, false);
  return { engine, backend: engine.webGLVersion >= 2 ? "webgl2" : "webgl1", reason };
}

export interface QualitySettings {
  scaling: number;
  shadows: number; // shadow map size, 0 = off
  grassDensity: number;
  treesPerCell: number;
}

export function qualitySettings(q: Quality): QualitySettings {
  const dpr = Math.min(window.devicePixelRatio || 1, 2);
  switch (q) {
    case "low":
      return { scaling: 1.5, shadows: 0, grassDensity: 0.4, treesPerCell: 2 };
    case "medium":
      return { scaling: 1, shadows: 1024, grassDensity: 0.75, treesPerCell: 3 };
    case "high":
      return { scaling: 1 / dpr, shadows: 2048, grassDensity: 1, treesPerCell: 3 };
  }
}

const QKEY = "nt.quality";
export function loadQuality(): Quality {
  try {
    const q = localStorage.getItem(QKEY);
    if (q === "low" || q === "medium" || q === "high") return q;
  } catch {
    /* default */
  }
  return "medium";
}
export function saveQuality(q: Quality) {
  try {
    localStorage.setItem(QKEY, q);
  } catch {
    /* ignore */
  }
}

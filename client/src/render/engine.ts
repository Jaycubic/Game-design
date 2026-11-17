import { AbstractEngine, Engine, WebGPUEngine } from "@babylonjs/core";

export type Backend = "webgpu" | "webgl2" | "webgl1";
/** "auto" (the default) picks low or medium from the hardware, and may lower itself during a run. */
export type Quality = "auto" | "low" | "medium" | "high";
export type Level = Exclude<Quality, "auto">;

/** Read by the 3D scene: low-end mode draws less (shorter view, fewer particles). */
export const perf = { low: false };

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
export async function createEngine(canvas: HTMLCanvasElement, preferWebGPU = true, antialias = true): Promise<EngineInfo> {
  const nav = navigator as Navigator & { gpu?: unknown };
  let reason = "";
  if (!preferWebGPU) reason = "WebGPU disabled in settings";
  else if (!window.isSecureContext) reason = "WebGPU needs HTTPS (open the game via https://)";
  else if (!nav.gpu) reason = "this browser does not expose WebGPU";
  else {
    try {
      const supported = await WebGPUEngine.IsSupportedAsync;
      if (supported) {
        const engine = new WebGPUEngine(canvas, { antialias, stencil: true, adaptToDeviceRatio: false, powerPreference: "high-performance" });
        await engine.initAsync();
        return { engine, backend: "webgpu", reason: "WebGPU" };
      }
      reason = "no compatible GPU adapter";
    } catch (e) {
      reason = "WebGPU failed to start (" + (e instanceof Error ? e.message : String(e)) + ")";
    }
  }
  const engine = new Engine(canvas, antialias, { stencil: true, preserveDrawingBuffer: false, powerPreference: "high-performance" }, false);
  return { engine, backend: engine.webGLVersion >= 2 ? "webgl2" : "webgl1", reason };
}

export interface QualitySettings {
  scaling: number;
  shadows: number; // shadow map size, 0 = off
  grassDensity: number;
  treesPerCell: number;
}

export function qualitySettings(q: Level): QualitySettings {
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
    if (q === "auto" || q === "low" || q === "medium" || q === "high") return q;
  } catch {
    /* default */
  }
  return "auto";
}

/** The graphics chip's name, if the browser shares it (WebGPU adapter info, or WebGL's debug extension). */
async function gpuName(): Promise<string> {
  try {
    const gpu = (navigator as Navigator & { gpu?: { requestAdapter(): Promise<{ info?: Record<string, string> } | null> } }).gpu;
    const adapter = gpu ? await gpu.requestAdapter() : null;
    const info = adapter?.info;
    if (info && (info.description || info.vendor || info.architecture)) return `${info.vendor ?? ""} ${info.architecture ?? ""} ${info.description ?? ""}`;
  } catch {
    /* fall through */
  }
  try {
    const gl = document.createElement("canvas").getContext("webgl");
    const ext = gl?.getExtension("WEBGL_debug_renderer_info");
    const name = ext && gl ? String(gl.getParameter(ext.UNMASKED_RENDERER_WEBGL)) : "";
    gl?.getExtension("WEBGL_lose_context")?.loseContext();
    return name;
  } catch {
    return "";
  }
}

export interface HardwareGuess {
  level: Level;
  why: string;
}

/**
 * Pick a starting quality from the hardware: phones and tablets, integrated or
 * mobile graphics chips, software rendering, few CPU cores or little memory all
 * start on Low; everything else on Medium.
 */
export async function detectQuality(): Promise<HardwareGuess> {
  const nav = navigator as Navigator & { deviceMemory?: number };
  const mobile = /Android|iPhone|iPad|iPod|Mobile/i.test(navigator.userAgent) || (navigator.maxTouchPoints > 1 && Math.min(screen.width, screen.height) < 820);
  if (mobile) return { level: "low", why: "phone or tablet" };
  if (nav.deviceMemory !== undefined && nav.deviceMemory <= 4) return { level: "low", why: `${nav.deviceMemory} GB memory` };
  if (navigator.hardwareConcurrency && navigator.hardwareConcurrency <= 4) return { level: "low", why: `${navigator.hardwareConcurrency} CPU cores` };
  const gpu = (await gpuName()).toLowerCase();
  if (/swiftshader|llvmpipe|software|basic render/.test(gpu)) return { level: "low", why: "software graphics" };
  if (/intel|uhd|iris|hd graphics|mali|adreno|powervr|apple gpu|videocore/.test(gpu)) return { level: "low", why: "integrated graphics" };
  return { level: "medium", why: gpu ? "dedicated graphics" : "default" };
}
export function saveQuality(q: Quality) {
  try {
    localStorage.setItem(QKEY, q);
  } catch {
    /* ignore */
  }
}

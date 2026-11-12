#!/usr/bin/env node
// Re-format content JSON files: two-space indentation, with short arrays and
// small objects (coordinates, one-line records) kept on a single line.
//   node scripts/format-content.mjs [files...]   (default: every JSON file under content/)
import { readFileSync, writeFileSync, readdirSync, statSync } from "node:fs";
import { join } from "node:path";

const LINE_LIMIT = 150;

/** Single-line rendering built structurally (never by editing JSON text). */
function inline(value) {
  if (Array.isArray(value)) return "[" + value.map(inline).join(", ") + "]";
  if (value !== null && typeof value === "object") {
    const parts = Object.entries(value).map(([k, v]) => `${JSON.stringify(k)}: ${inline(v)}`);
    return parts.length ? "{ " + parts.join(", ") + " }" : "{}";
  }
  return JSON.stringify(value);
}

function fmt(value, indent = "") {
  if (value === null || typeof value !== "object") return JSON.stringify(value);
  const one = inline(value);
  if (indent.length + one.length <= LINE_LIMIT) return one;
  const next = indent + "  ";
  if (Array.isArray(value)) {
    return "[\n" + value.map((v) => next + fmt(v, next)).join(",\n") + "\n" + indent + "]";
  }
  const body = Object.entries(value).map(([k, v]) => `${next}${JSON.stringify(k)}: ${fmt(v, next)}`);
  return "{\n" + body.join(",\n") + "\n" + indent + "}";
}

function walk(dir) {
  return readdirSync(dir).flatMap((n) => {
    const p = join(dir, n);
    return statSync(p).isDirectory() ? walk(p) : p.endsWith(".json") ? [p] : [];
  });
}

const files = process.argv.length > 2 ? process.argv.slice(2) : walk("content");
for (const f of files) {
  const before = readFileSync(f, "utf8");
  const data = JSON.parse(before);
  const after = fmt(data) + "\n";
  if (JSON.stringify(JSON.parse(after)) !== JSON.stringify(data)) throw new Error(`formatter changed data in ${f}`);
  writeFileSync(f, after);
}
console.log(`formatted ${files.length} file(s)`);

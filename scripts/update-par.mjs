#!/usr/bin/env node
// Write solver-verified par values into the level files.
//   cargo run --release -p natures-thread-core --bin classify -- --json > par.json
//   node scripts/update-par.mjs par.json
// Only solvable puzzle/expedition results are written; deduction levels have no par.
import { readFileSync, writeFileSync, readdirSync } from "node:fs";
import { join } from "node:path";

const reports = JSON.parse(readFileSync(process.argv[2] ?? 0, "utf8"));
const dir = "content/levels";
const files = Object.fromEntries(
  readdirSync(dir)
    .filter((f) => f.endsWith(".json"))
    .map((f) => [JSON.parse(readFileSync(join(dir, f), "utf8")).number, join(dir, f)]),
);

const byLevel = new Map();
for (const [number, r] of reports) {
  if (!r.solvable || r.min_cost === null || r.answer) continue;
  if (!byLevel.has(number)) byLevel.set(number, {});
  byLevel.get(number)[r.rules] = r.min_cost;
}

for (const [number, par] of byLevel) {
  const path = files[number];
  const level = JSON.parse(readFileSync(path, "utf8"));
  level.par = par;
  writeFileSync(path, JSON.stringify(level) + "\n");
  console.log(`L${String(number).padStart(2, "0")} par ${JSON.stringify(par)}`);
}

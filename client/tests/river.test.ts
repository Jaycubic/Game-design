import { describe, expect, it } from "vitest";
import { FINALE_INFO, FINALE_ORDER, GLYPHS, TWIST_INFO, TWIST_ORDER } from "../src/river/types";

// The client's option order must match crates/river-core (TWISTS / FINALES),
// because votes are sent as option indices.
describe("river rule-change options", () => {
  it("lists the three twists and two finales in server order", () => {
    expect(TWIST_ORDER).toEqual(["monsoon_surge", "dry_season", "volunteer_day"]);
    expect(FINALE_ORDER).toEqual(["second_wind", "clean_sweep"]);
    for (const t of TWIST_ORDER) expect(TWIST_INFO[t].line.length).toBeGreaterThan(10);
    for (const f of FINALE_ORDER) expect(FINALE_INFO[f].name).toBeTruthy();
  });

  it("has six signal glyphs (keys 1–6)", () => {
    expect(GLYPHS).toHaveLength(6);
  });
});

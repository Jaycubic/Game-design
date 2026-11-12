// Tutorial coach: one instruction at a time, advanced by what the player
// actually does. Skip is always available.

import { h } from "../ui/dom";
import { isTouch } from "../input/touch";
import { hint } from "../input/controls";
import type { Intent } from "../input/controls";
import type { RunEvent, Snapshot } from "./types";

interface Step {
  title: string;
  keys: () => string;
  text: string;
  /** Return true when the step is complete. */
  done(c: Coach): boolean;
}

export class Coach {
  private step = 0;
  steers = 0;
  collected = 0;
  hops = 0;
  passedLog = false;
  cut = false;
  plugged = false;
  pluggingFailed = false;
  private card: HTMLElement;
  private finished = false;

  private steps: Step[] = [
    { title: "Steer", keys: () => `${hint("left")}  ${hint("right")}`, text: "Switch lanes. Your boat moves forward on its own.", done: (c) => c.steers >= 2 },
    { title: "Grab plastic", keys: () => "♻", text: "Drive into the bottles and bags. Plastic fills your boost.", done: (c) => c.collected >= 3 },
    { title: "Hop", keys: () => hint("hop"), text: "Logs block every lane — hop just before you reach one.", done: (c) => c.passedLog },
    { title: "Cut weed", keys: () => hint("action"), text: "Press just before a weed mat to cut it. Cut weed greens the bank.", done: (c) => c.cut },
    {
      title: "Plug the pipe",
      keys: () => `${hint("action")} then ${isTouch() ? "tap the symbols" : "1–6"}`,
      text: "Pull up beside the sewage pipe, press the action key, then type the 3-symbol gate code.",
      done: (c) => c.plugged || c.pluggingFailed,
    },
  ];

  constructor(
    host: HTMLElement,
    private onExit: () => void,
  ) {
    this.card = h("div", { class: "coach" });
    host.appendChild(this.card);
    this.draw();
  }

  input(i: Intent) {
    if (i.type === "steer") this.steers++;
    if (i.type === "hop") this.hops++;
    this.check();
  }

  events(events: RunEvent[], mySeat: number) {
    for (const e of events) {
      if ("seat" in e && e.seat !== mySeat) continue;
      if (e.type === "collected") this.collected++;
      if (e.type === "cut") this.cut = true;
      if (e.type === "plugged") this.plugged = true;
      if (e.type === "plug_failed") this.pluggingFailed = true;
    }
    this.check();
  }

  snapshot(s: Snapshot, mySeat: number) {
    const me = s.boats.find((b) => b.seat === mySeat);
    // A log is "passed" once the boat is beyond the first log row without capsizing.
    if (me && me.z > 196 && this.step === 2 && me.capsize <= 0) this.passedLog = true;
    if (me && this.step === 2 && me.z > 330) this.passedLog = true; // never stuck
    if (s.phase.phase === "finished" && !this.finished) this.finish();
    this.check();
  }

  private check() {
    let moved = false;
    while (this.step < this.steps.length && this.steps[this.step].done(this)) {
      this.step++;
      moved = true;
    }
    if (moved) {
      if (this.step >= this.steps.length) this.finish();
      else this.draw(true);
    }
  }

  private finish() {
    this.finished = true;
    this.step = this.steps.length;
    this.card.replaceChildren(
      h("div", { class: "coach-title" }, "🎉 You're ready!"),
      h(
        "p",
        { class: "coach-text" },
        "In real runs you race as a crew, the River Health is shared by everyone — and twice per run, the players vote to change the rules.",
      ),
      h("button", { class: "btn primary big", onclick: () => this.onExit() }, "Start playing"),
    );
    this.card.classList.add("done");
  }

  private draw(pulse = false) {
    const s = this.steps[this.step];
    this.card.replaceChildren(
      h(
        "div",
        { class: "coach-dots" },
        this.steps.map((_, i) => h("span", { class: "coach-dot" + (i < this.step ? " done" : i === this.step ? " now" : "") })),
      ),
      h("div", { class: "coach-title" }, `${this.step + 1}. ${s.title}`, h("kbd", { class: "coach-keys" }, s.keys())),
      h("p", { class: "coach-text" }, s.text),
      h("button", { class: "btn ghost coach-skip", onclick: () => this.onExit() }, "Skip tutorial"),
    );
    if (pulse) {
      this.card.classList.remove("pop");
      void this.card.offsetWidth;
      this.card.classList.add("pop");
    }
  }

  dispose() {
    this.card.remove();
  }
}

// Tutorial coach: one instruction at a time, advanced by what the player
// actually does. Each step also spotlights the control or meter it is about
// (dim, ring, tapping hand, bubble), so every button is introduced in place.
// Skip is always available.

import { h } from "../ui/dom";
import { Spotlight, type SpotTarget } from "../ui/spotlight";
import { touchPrefs, usesTouch } from "../input/touch";
import { hint } from "../input/controls";
import type { Intent } from "../input/controls";
import type { RunEvent, Snapshot } from "./types";

interface Step {
  title: string;
  keys: () => string;
  text: string;
  /** What to spotlight while this step is active (may change, e.g. once a gate code opens). */
  spot: (c: Coach) => SpotTarget | null;
  /** Return true when the step is complete. */
  done(c: Coach): boolean;
}

const tap = () => {
  const m = touchPrefs.get().mode;
  return m === "swipe" ? "Swipe" : m === "off" ? "Press" : "Tap";
};

export class Coach {
  private step = 0;
  private stepAt = performance.now();
  steers = 0;
  collected = 0;
  hops = 0;
  passedLog = false;
  cut = false;
  plugging = false;
  plugged = false;
  pluggingFailed = false;
  pings = 0;
  private card: HTMLElement;
  private spot: Spotlight;
  private finished = false;

  /** Seconds the current step has been showing. */
  get age() {
    return (performance.now() - this.stepAt) / 1000;
  }

  private steps: Step[] = [
    {
      title: "Steer",
      keys: () => `${hint("left")}  ${hint("right")}`,
      text: "Switch lanes. Your boat moves forward on its own.",
      spot: () => ({ name: "steer", title: "Steer", text: `${tap()} ◀ or ▶ to move one lane left or right.` }),
      done: (c) => c.steers >= 2,
    },
    {
      title: "Grab plastic",
      keys: () => "♻",
      text: "Drive into the bottles and bags.",
      spot: () => ({ name: "boost", title: "Boost meter", text: "Every piece of plastic fills it. When it's full, your boat speeds up by itself." }),
      done: (c) => c.collected >= 3,
    },
    {
      title: "River Health",
      keys: () => "🌊",
      text: "Plastic you leave behind hurts the river.",
      spot: () => ({
        name: "health",
        title: "River Health",
        text: "Shared by everyone on the river. Keep it at 60 % or more for a bonus. If it reaches 0, every score is halved.",
      }),
      done: (c) => c.age > 4.5,
    },
    {
      title: "Hop",
      keys: () => hint("hop"),
      text: "Logs block every lane, so hop just before you reach one.",
      spot: () => ({ name: "hop", title: "Hop", text: `${tap()} ▲ to jump. A red shadow appears under a log when it's time to hop.` }),
      done: (c) => c.passedLog,
    },
    {
      title: "Cut weed",
      keys: () => hint("action"),
      text: "Cut a weed mat just before you reach it. Cut weed greens the bank.",
      spot: () => ({ name: "action", title: "Cut / plug", text: `${tap()} ✂ near a weed mat to cut it. Hitting one slows you down.` }),
      done: (c) => c.cut,
    },
    {
      title: "Plug the pipe",
      keys: () => `${hint("action")} then ${usesTouch() ? "tap the symbols" : "1–6"}`,
      text: "Pull up beside the sewage pipe, use the same ✂ button, then enter the 3-symbol gate code.",
      spot: (c) =>
        c.plugging
          ? {
              name: "code",
              title: "Gate code",
              text: `Remember the order, then ${usesTouch() ? "tap the symbols" : "press their keys 1–6"}. In a crew, only your crewmate sees the right order.`,
            }
          : { name: "action", title: "Plug a pipe", text: `Steer into the lane next to the pipe and ${tap().toLowerCase()} ✂ when you're beside it.` },
      done: (c) => c.plugged || c.pluggingFailed,
    },
    {
      title: "Signal your crew",
      keys: () => (usesTouch() ? "🍃 💧 🐟" : "1–6"),
      text: "Send any symbol. It pops up over your boat for everyone to see.",
      spot: () => ({
        name: "signals",
        title: "Signals",
        text: "Six symbols with no fixed meaning: your crew decides what they mean. Use them to pass on gate codes, or to bluff the other crew.",
      }),
      done: (c) => c.pings >= 1 || c.age > 12,
    },
  ];

  constructor(
    private host: HTMLElement,
    private onExit: () => void,
  ) {
    host.classList.add("tutorial");
    this.card = h("div", { class: "coach" });
    host.appendChild(this.card);
    this.spot = new Spotlight(host);
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
      if (e.type === "ping") this.pings++;
      if (e.type === "cut") this.cut = true;
      if (e.type === "plug_started") this.plugging = true;
      if (e.type === "plugged") this.plugged = true;
      if (e.type === "plug_failed") this.pluggingFailed = true;
      if (e.type === "plugged" || e.type === "plug_failed") this.plugging = false;
    }
    this.check();
  }

  snapshot(s: Snapshot, mySeat: number) {
    const me = s.boats.find((b) => b.seat === mySeat);
    // A log is "passed" once the boat is beyond the first log row without capsizing.
    if (me && me.z > 196 && this.step === 3 && me.capsize <= 0) this.passedLog = true;
    if (me && this.step === 3 && me.z > 330) this.passedLog = true; // never stuck
    if (me && this.step === 4 && me.z > 470) this.cut = true; // missed both mats: move on
    if (me && this.step === 5 && me.z > 640 && !this.plugging) this.pluggingFailed = true; // passed both pipes
    if (s.phase.phase === "finished" && !this.finished) this.finish();
    this.check();
  }

  private check() {
    if (this.finished) return;
    let moved = false;
    while (this.step < this.steps.length && this.steps[this.step].done(this)) {
      this.step++;
      this.stepAt = performance.now();
      moved = true;
    }
    if (moved) {
      if (this.step >= this.steps.length) this.finish();
      else this.draw(true);
    } else if (this.step < this.steps.length) {
      this.spot.show(this.steps[this.step].spot(this));
    }
  }

  private finish() {
    this.finished = true;
    this.step = this.steps.length;
    this.spot.show(null);
    this.card.replaceChildren(
      h("div", { class: "coach-title" }, "🎉 You're ready!"),
      h(
        "p",
        { class: "coach-text" },
        "In real runs you race as a crew, the River Health is shared by everyone, and twice per run the players vote to change the rules.",
      ),
      h("button", { class: "btn primary big", onclick: () => this.onExit() }, "Start playing"),
    );
    this.card.classList.add("done");
  }

  private draw(pulse = false) {
    const s = this.steps[this.step];
    this.spot.show(s.spot(this));
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
    this.spot.dispose();
    this.host.classList.remove("tutorial");
    this.card.remove();
  }
}

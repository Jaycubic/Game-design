// In-run HUD: big, glanceable, minimal text. Rebuilt only when its inputs change.

import { h } from "../ui/dom";
import { hint } from "../input/controls";
import { actionVerb, usesTouch } from "../input/touch";
import type { MissionView, RankView, RiverResult } from "../net/api";
import { missionList, rankCard } from "../ui/hub";
import {
  CREW_NAMES,
  FINALE_INFO,
  FINALE_ORDER,
  RUN_SECONDS,
  SEAT_COLOURS,
  TWIST_INFO,
  TWIST_ORDER,
  type RunResults,
  type Snapshot,
} from "./types";

export interface HudCallbacks {
  choose(option: number): void;
  /** Press a lock number 1–9. */
  glyph(g: number): void;
  leave(): void;
  /** Leave the run (and the room) before it ends. */
  quit(): void;
  /** The pause menu opened (true) or closed (false). */
  pause(open: boolean): void;
}

export class RunHud {
  readonly root: HTMLElement;
  private top = h("div", { class: "rh-top" });
  private center = h("div", { class: "rh-center" });
  private code = h("div", { class: "rh-code" });
  private bottom = h("div", { class: "rh-bottom" });
  private fx = h("div", { class: "rh-fx" });
  private banners = h("div", { class: "rh-banners", "aria-live": "polite" });
  private sigTop = "";
  private sigCenter = "";
  private sigCode = "";
  private sigBottom = "";
  private lastPhase = "";
  // Steady "pipe ahead" card: fixed layout, only its text changes.
  private pipeCard = h("div", { class: "rh-pipe" });
  private pipeSide = h("span", { class: "rh-pipe-side" });
  private pipeDist = h("span", { class: "rh-pipe-dist" });
  private pipeStep = h("div", { class: "rh-pipe-step" });
  private pipeMark = h("div", { class: "rh-pipe-mark" }, "🔧", h("small", null, ""));
  private tags = new Map<number, HTMLElement>();

  constructor(
    host: HTMLElement,
    private mySeat: number,
    private names: Record<string, string>,
    private cb: HudCallbacks,
  ) {
    this.root = h(
      "div",
      { class: "run-ui" },
      // Layer 1: things pinned to the 3D world (name tags, +1 pops, pipe marker).
      this.fx,
      this.pipeMark,
      // Layer 2: the HUD slot grid (see run.css "HUD layout").
      h(
        "div",
        { class: "rhud" },
        h("div", { class: "rhud-slot rhud-topbar" }, this.pauseBtn, this.top),
        h("div", { class: "rhud-slot rhud-notices" }, this.pipeCard, this.banners),
        // Your number sits right above the lock card.
        h("div", { class: "rhud-slot rhud-lower" }, this.keyCard, this.code),
        h("div", { class: "rhud-slot rhud-bottom" }, this.bottom),
        h("div", { class: "rhud-slot rhud-bl" }, this.keysHint()),
      ),
      // Layer 3: full-screen moments (countdown, votes, reveal, results), then the pause menu.
      this.center,
      this.menu,
    );
    host.appendChild(this.root);
  }

  /** A HUD slot ("notices", "left", "lower", …) for pieces owned by other modules. */
  slot(name: string): HTMLElement | undefined {
    return this.root.querySelector<HTMLElement>(`.rhud-${name}`) ?? undefined;
  }

  /** Bottom-left keyboard hint (hidden with touch controls). */
  private keysHint() {
    return h(
        "div",
        { class: "rh-keys" },
        h("span", { "data-tut": "steer" }, `${hint("left")} ${hint("right")} steer`),
        " · ",
        h("span", { "data-tut": "hop" }, `${hint("hop")} hop`),
        " · ",
        h("span", { "data-tut": "action" }, `${hint("action")} cut / plug`),
        " · Esc pause",
      );
  }

  private name(seat: number) {
    return this.names[String(seat)] || `Player ${seat + 1}`;
  }

  update(s: Snapshot) {
    if (this.lastPhase === "countdown" && s.phase.phase === "running") this.banner("GO!", "go");
    this.lastPhase = s.phase.phase;
    const me = s.boats.find((b) => b.seat === this.mySeat);
    this.pipePrompt(s, me);
    const myCrew = me?.crew ?? 0;
    const left = Math.max(0, (s.duration || RUN_SECONDS) - s.t);
    const mm = Math.floor(left / 60),
      ss = Math.floor(left % 60);
    const twist = s.twist ? TWIST_INFO[s.twist] : null;
    const finale = s.finale ? FINALE_INFO[s.finale] : null;

    // ── top bar: scores · timer · River Health ──
    const topSig = JSON.stringify([s.crews, mm, ss, s.lives, s.endless ? Math.floor(s.distance ?? 0) : 0, Math.round(s.health), s.twist, s.finale, s.choked]);
    if (topSig !== this.sigTop) {
      this.sigTop = topSig;
      const hp = Math.round(s.health);
      this.top.replaceChildren(
        h(
          "div",
          { class: "rh-scores" },
          s.crews.map(([id, score, plastic, pipes, , nets]) =>
            h(
              "div",
              { class: "rh-crew" + (id === myCrew ? " mine" : "") },
              h("span", { class: "rh-crew-name" }, s.crews.length > 1 ? CREW_NAMES[id] : "Your crew"),
              h("span", { class: "rh-score" }, String(score), pipes > 0 ? h("span", { class: "rh-mult", title: "Every plastic is worth this much: +0.5 per pipe plugged" }, `×${1 + 0.5 * pipes}`) : null),
              h("span", { class: "rh-sub" }, `♻ ${plastic} · 🔧 ${pipes}` + (s.river === "mangrove" ? ` · 🕸 ${nets ?? 0}` : "")),
            ),
          ),
        ),
        h(
          "div",
          { class: "rh-clock" },
          s.endless
            ? h("div", { class: "rh-time" }, `${Math.floor(s.distance ?? 0)} m`)
            : h("div", { class: "rh-time" + (left < 15 ? " urgent" : "") }, `${mm}:${String(ss).padStart(2, "0")}`),
          h(
            "div",
            { class: "rh-badges" },
            s.endless ? h("span", { class: "rh-badge" }, "∞ Endless") : null,
            s.endless && s.boats.length > 1 ? h("span", { class: "rh-badge lives" }, `${"❤️".repeat(s.lives ?? 0)}${"🤍".repeat(Math.max(0, 3 - (s.lives ?? 0)))}`) : null,
            twist ? h("span", { class: "rh-badge", title: twist.line }, `${twist.icon} ${twist.name}`) : null,
            finale ? h("span", { class: "rh-badge", title: finale.line }, `${finale.icon} ${finale.name}`) : null,
          ),
        ),
        h(
          "div",
          { class: "rh-health" + (hp < 30 ? " low" : ""), "data-tut": "health" },
          h("span", { class: "rh-health-label" }, s.choked ? "River choked" : "River health"),
          h("div", { class: "rh-bar" }, h("div", { class: "rh-fill", style: `width:${hp}%` })),
          h("span", { class: "rh-hp" }, `${hp}%`),
        ),
      );
    }

    // ── centre: countdown, rule-change warning / vote / reveal ──
    const p = s.phase;
    const centerSig = JSON.stringify([p.phase, "left" in p ? Math.ceil(p.left) : 0, s.votes, s.twist, s.finale, s.results ? 1 : 0]);
    if (centerSig !== this.sigCenter) {
      this.sigCenter = centerSig;
      this.center.replaceChildren(...this.centerContent(s).filter((x): x is HTMLElement => !!x));
    }

    // ── your number for the pipe ahead ──
    this.keyUpdate(s);

    // ── pipe locks: your own first, otherwise a teammate's that needs your number ──
    const plug =
      s.plugs.find(([seat]) => seat === this.mySeat) ??
      s.plugs.find(([, pv]) => pv.team && !pv.tiles.find((t) => t.seat === this.mySeat)?.ready) ??
      s.plugs[0];
    const codeSig = plug ? JSON.stringify([plug[0], { ...plug[1], left: Math.ceil(plug[1].left) }]) : "";
    if (codeSig !== this.sigCode) {
      this.sigCode = codeSig;
      if (!plug) this.code.replaceChildren();
      else this.lockCard(plug[0], plug[1]);
    }

    // ── bottom: boost meter ──
    const boost = me ? (me.boosting > 0 ? 1 : me.boost) : 0;
    const bottomSig = `${Math.round(boost * 20)}|${me?.boosting ? 1 : 0}`;
    if (bottomSig !== this.sigBottom) {
      this.sigBottom = bottomSig;
      this.bottom.replaceChildren(
        h(
          "div",
          { class: "rh-boost" + (me?.boosting ? " on" : ""), "data-tut": "boost" },
          h("span", null, me?.boosting ? "BOOST!" : "Boost"),
          h("div", { class: "rh-bar" }, h("div", { class: "rh-fill", style: `width:${Math.round(boost * 100)}%` })),
        ),
      );
    }
  }

  // Your number: big, tappable, blinking as it is about to hide or while a crewmate waits for it.
  private keyDigit = 0;
  private keyCard = h("button", {
    class: "rh-key",
    type: "button",
    onpointerdown: (e: PointerEvent) => {
      e.preventDefault();
      e.stopPropagation();
      if (this.keyDigit) this.cb.glyph(this.keyDigit);
    },
  });
  // Pause: always on screen during a run. Opens a menu with Resume and Quit
  // (Quit asks once more, so a stray tap never ends a run).
  private pauseBtn = h(
    "button",
    { class: "rh-pause", title: "Pause (Esc)", "aria-label": "Pause menu", onclick: () => this.cb.pause(true) },
    h("span", { class: "rh-pause-icon", "aria-hidden": "true" }, "⏸"),
    h("span", { class: "rh-pause-label" }, "Pause"),
  );
  private menu = h("div", { class: "rh-menu", role: "dialog", "aria-modal": "true", "aria-label": "Paused" });
  menuOpen = false;

  /** Show the pause menu. `solo`: the run really stops; otherwise the crew keeps rowing. */
  showMenu(solo: boolean) {
    this.menuOpen = true;
    const resume = h("button", { class: "btn primary big", onclick: () => this.cb.pause(false) }, "▶ Resume");
    const quit = h("button", { class: "btn ghost" }, "Quit run");
    const note = h("p", { class: "rh-menu-note" });
    quit.onclick = () => {
      if (quit.classList.contains("danger")) {
        this.cb.quit();
        return;
      }
      quit.classList.add("danger");
      quit.textContent = "Yes, quit";
      note.textContent = solo
        ? "Quitting ends this run. You go back to the dock with no Seeds for it."
        : "You leave the run. Your crew finishes it without you; your keys fill in by themselves.";
    };
    this.menu.replaceChildren(
      h(
        "div",
        { class: "rh-menu-card" },
        h("div", { class: "rh-menu-title" }, solo ? "Paused" : "Menu"),
        h(
          "p",
          { class: "rh-menu-sub" },
          solo
            ? "The river is waiting for you."
            : "Your boat waits here while your crew keeps rowing (your keys fill in by themselves). Resume to rejoin them.",
        ),
        resume,
        quit,
        note,
      ),
    );
    this.menu.classList.add("open");
    resume.focus();
  }

  hideMenu() {
    this.menuOpen = false;
    this.menu.classList.remove("open");
    this.menu.replaceChildren();
  }
  private sigKey = "";
  private buzzed = new Set<string>();

  private keyUpdate(s: Snapshot) {
    const key = s.my_key ?? 0;
    const sent = !!s.key_sent;
    const call = !!s.key_call;
    const blink = !!key && !sent && (call || (s.key_left ?? 0) < 3);
    const sig = key ? `${key}|${sent}|${blink}|${call}` : "";
    if (sig === this.sigKey) return;
    this.sigKey = sig;
    this.keyDigit = key && !sent ? key : 0;
    this.keyCard.className = "rh-key" + (key ? " on" : "") + (sent ? " sent" : "") + (blink ? " blink" : "") + (call ? " call" : "");
    this.keyCard.disabled = sent;
    if (!key) {
      this.keyCard.replaceChildren();
      return;
    }
    if (sent) {
      this.keyCard.setAttribute("aria-label", `Your number ${key} is sent`);
      this.keyCard.replaceChildren(h("span", { class: "rh-key-ok" }, "✔"), h("span", { class: "rh-key-small" }, `Your ${key} is in`));
      return;
    }
    if (call && !this.buzzed.has(`call:${key}`)) {
      this.buzzed.add(`call:${key}`);
      navigator.vibrate?.([60, 40, 60]);
    }
    this.keyCard.setAttribute("aria-label", `Your number is ${key}. Press it to send it.`);
    this.keyCard.replaceChildren(
      h("span", { class: "rh-key-title" }, call ? "Your crew needs it!" : "Your number"),
      h("span", { class: "rh-key-digit" }, String(key)),
      h("span", { class: "rh-key-hint" }, usesTouch() ? "Tap to send" : `Press ${key} to send`),
    );
  }

  /** A pipe lock: the fixer's lock, a teammate's call for your number, or the solo lock. */
  private lockCard(fixer: number, pv: import("./types").PlugView) {
    const own = fixer === this.mySeat;
    const how = usesTouch() ? "tap them" : "press 1–9";
    const ready = pv.tiles.filter((t) => t.ready).length;
    const tile = (t: import("./types").TileView) => {
      const who = t.seat === undefined ? null : h("small", { class: "rh-tile-name" }, t.seat === this.mySeat ? "You" : this.name(t.seat));
      const colour = t.seat === undefined ? "" : `--c:${SEAT_COLOURS[t.seat % 4]}`;
      if (own && t.digit !== undefined) {
        const d = t.digit;
        return h(
          "button",
          {
            type: "button",
            class: "rh-tile" + (t.used ? " used" : ""),
            style: colour,
            disabled: !!t.used,
            "aria-label": `Number ${d}`,
            onpointerdown: (e: PointerEvent) => {
              e.preventDefault();
              e.stopPropagation();
              this.cb.glyph(d);
            },
          },
          h("b", null, t.used ? "✔" : String(d)),
          who,
        );
      }
      return h("div", { class: "rh-tile wait" + (t.ready ? " in" : ""), style: colour }, h("b", null, t.ready ? "✔" : "…"), who);
    };
    let title: string;
    let hint: string;
    let cls: string;
    if (!pv.team) {
      title = "Number lock — smallest first";
      hint = `Enter the 3 numbers from smallest to biggest: ${how}. Press ▲ to leave.`;
      cls = "fixer";
    } else if (own) {
      const mates = ready - 1;
      if (pv.locked) {
        title = `Enter them smallest first · ${pv.entered}/${pv.tiles.length}`;
        hint = `Your number and your crew's, from smallest to biggest: ${how}. Press ▲ to leave.`;
      } else if (mates < 1) {
        title = "Waiting for your crew's numbers…";
        hint = "Call out: “Press your number!” Their numbers appear here as they send them.";
      } else {
        title = "Enter them smallest first";
        hint = `Start when you're ready (numbers sent later won't count): ${how}. Press ▲ to leave.`;
      }
      cls = "fixer";
    } else {
      const mine = pv.tiles.find((t) => t.seat === this.mySeat);
      if (mine && !mine.ready) {
        title = `🔧 ${this.name(fixer)} is fixing a pipe — send your number!`;
        hint = usesTouch() ? "Tap your big number." : "Press your big number.";
        cls = "call";
      } else {
        title = `✔ Your number is in — ${this.name(fixer)} is entering the lock`;
        hint = "Keep sailing!";
        cls = "sent";
      }
    }
    this.code.replaceChildren(
      h(
        "div",
        { class: "rh-code-card lock " + cls, "data-tut": "code" },
        h("div", { class: "rh-code-title" }, title),
        h("div", { class: "rh-tiles" }, pv.tiles.map(tile)),
        h("div", { class: "rh-code-hint" }, hint),
        h("div", { class: "rh-code-time" }, `${Math.ceil(pv.left)} s`),
      ),
    );
  }

  /** A press that didn't count: shake the card it was meant for. */
  wrong() {
    for (const el of [this.code.firstElementChild, this.keyCard]) {
      if (!(el instanceof HTMLElement) || (el === this.keyCard && !this.keyDigit)) continue;
      el.classList.remove("shake");
      void el.offsetWidth;
      el.classList.add("shake");
    }
  }

  /** The nearest open pipe ahead: a steady card saying where it is and what to do next. */
  private pipePrompt(s: Snapshot, me: Snapshot["boats"][number] | undefined) {
    const racing = s.phase.phase === "running" || s.phase.phase === "warning";
    const pipe =
      me && racing && !me.plugging && !(me.fixing ?? 0)
        ? s.objs.filter((o) => o.kind === "pipe" && !o.plugged && o.z > me.z - 4 && o.z < me.z + 70).sort((a, b) => a.z - b.z)[0]
        : undefined;
    this.pipeCard.classList.toggle("on", !!pipe);
    if (!pipe || !me) return;
    const left = pipe.lane < 0;
    // Pipes are plugged from the outermost open lane on their bank.
    const edge = left ? s.lanes[0] : s.lanes[s.lanes.length - 1];
    const dz = Math.max(0, Math.round(pipe.z - me.z));
    const act = actionVerb();
    const step =
      me.lane !== edge
        ? `Move ${left ? "◀ left" : "right ▶"} to the edge lane`
        : dz < 7
          ? `${act.charAt(0).toUpperCase() + act.slice(1)} NOW to plug it!`
          : `Stay in this lane — ${act} when you reach it`;
    if (!this.pipeCard.firstChild) {
      this.pipeCard.append(
        h("div", { class: "rh-pipe-head" }, h("span", { class: "rh-pipe-ico" }, "🔧"), h("b", null, "Sewage pipe"), this.pipeSide, this.pipeDist),
        this.pipeStep,
      );
    }
    this.pipeSide.textContent = left ? "◀ LEFT bank" : "RIGHT bank ▶";
    this.pipeDist.textContent = `${dz} m`;
    if (this.pipeStep.textContent !== step) this.pipeStep.textContent = step;
    this.pipeCard.classList.toggle("now", me.lane === edge && dz < 7);
  }

  /** Floating marker over the nearest open pipe (screen position from the 3D scene). */
  pipeMarker(x: number, y: number, visible: boolean, metres: number, turn: number | null = null) {
    this.pipeMark.style.display = visible ? "" : "none";
    if (!visible) return;
    this.pipeMark.style.transform = `translate(${x}px, ${y}px) translate(-50%, -100%)`;
    const label = this.pipeMark.lastChild as HTMLElement;
    // Suggested fixer: whoever has fixed the fewest pipes so far (a hint, not a rule).
    const who = turn === null ? "" : turn === this.mySeat ? " · your turn?" : ` · ${this.name(turn)}'s turn?`;
    const text = `${Math.round(metres)} m${who}`;
    if (label.textContent !== text) label.textContent = text;
    this.pipeMark.style.setProperty("--c", turn === null ? "#b9853a" : SEAT_COLOURS[turn % 4]);
  }

  private centerContent(s: Snapshot): (HTMLElement | null)[] {
    const p = s.phase;
    if (p.phase === "countdown") return [h("div", { class: "rh-big pop", key: String(Math.ceil(p.left)) }, String(Math.ceil(p.left)))];
    if (p.phase === "warning") {
      return [h("div", { class: "rh-warning" }, p.change === "twist" ? "⚡ The river is about to change" : "🏁 Final twist incoming", h("strong", null, String(Math.ceil(p.left))))];
    }
    if (p.phase === "choosing" && p.change === "twist") {
      const myVote = s.votes.find(([seat]) => seat === this.mySeat)?.[1];
      return [
        h(
          "div",
          { class: "rh-choice" },
          h("div", { class: "rh-choice-title" }, "Change the river — vote!", h("span", { class: "rh-choice-time" }, `${Math.ceil(p.left)}`)),
          h(
            "div",
            { class: "rh-cards" },
            TWIST_ORDER.map((tw, i) => {
              const info = TWIST_INFO[tw];
              const voters = s.votes.filter(([, o]) => o === i).map(([seat]) => seat);
              return h(
                "button",
                { class: "rh-card" + (myVote === i ? " picked" : ""), onclick: () => this.cb.choose(i) },
                h("kbd", null, String(i + 1)),
                h("div", { class: "rh-card-icon" }, info.icon),
                h("div", { class: "rh-card-name" }, info.name),
                h("div", { class: "rh-card-line" }, info.line),
                h("div", { class: "rh-voters" }, voters.map((v) => h("span", { class: "rh-dot", style: `background:${SEAT_COLOURS[v % 4]}`, title: this.name(v) }))),
              );
            }),
          ),
          h("div", { class: "rh-choice-foot" }, "A tie goes to whoever voted first."),
        ),
      ];
    }
    if (p.phase === "choosing" && p.change === "finale") {
      // The Finale is a crew vote, like the Twist.
      const myVote = s.votes.find(([seat]) => seat === this.mySeat)?.[1];
      return [
        h(
          "div",
          { class: "rh-choice" },
          h("div", { class: "rh-choice-title" }, "Final twist — vote!", h("span", { class: "rh-choice-time" }, `${Math.ceil(p.left)}`)),
          h(
            "div",
            { class: "rh-cards two" },
            FINALE_ORDER.map((f, i) => {
              const info = FINALE_INFO[f];
              const voters = s.votes.filter(([, o]) => o === i).map(([seat]) => seat);
              return h(
                "button",
                { class: "rh-card" + (myVote === i ? " picked" : ""), onclick: () => this.cb.choose(i) },
                h("kbd", null, String(i + 1)),
                h("div", { class: "rh-card-icon" }, info.icon),
                h("div", { class: "rh-card-name" }, info.name),
                h("div", { class: "rh-card-line" }, info.line),
                h("div", { class: "rh-voters" }, voters.map((v) => h("span", { class: "rh-dot", style: `background:${SEAT_COLOURS[v % 4]}`, title: this.name(v) }))),
              );
            }),
          ),
          h("div", { class: "rh-choice-foot" }, "A tie goes to whoever voted first."),
        ),
      ];
    }
    if (p.phase === "reveal") {
      const info = p.change === "twist" ? (s.twist ? TWIST_INFO[s.twist] : null) : s.finale ? FINALE_INFO[s.finale] : null;
      if (!info) return [];
      return [
        h(
          "div",
          { class: "rh-reveal pop" },
          h(
            "div",
            { class: "rh-reveal-kicker" },
            p.spin
              ? "🎡 Nobody voted — the river picks"
              : p.tie_by !== undefined
                ? `⚖️ A tie — ${p.tie_by === this.mySeat ? "you" : this.name(p.tie_by)} voted first, so it's`
                : p.change === "twist"
                  ? "The river changes"
                  : "Final twist",
          ),
          h("div", { class: "rh-card-icon big" }, info.icon),
          h("div", { class: "rh-reveal-name" }, info.name),
          h("div", { class: "rh-card-line" }, info.line),
        ),
      ];
    }
    return [];
  }

  results(r: RunResults, s: Snapshot | null, host: boolean) {
    const myCrew = s?.boats.find((b) => b.seat === this.mySeat)?.crew ?? 0;
    const two = r.scores.length > 1;
    const title = s?.endless ? "Endless run over" : !two ? (r.revived ? "River revived! You saved it together" : r.choked ? "The river choked" : "Run complete") : r.winner === null ? "It's a draw!" : r.winner === myCrew ? "Your crew wins!" : `${CREW_NAMES[r.winner]} win!`;
    this.center.replaceChildren(
      h(
        "div",
        { class: "rh-results pop" },
        h("div", { class: "rh-results-title" }, title),
        h(
          "div",
          { class: "rh-results-scores" },
          r.scores.map(([id, score]) =>
            h("div", { class: "rh-result-crew" + (id === r.winner ? " win" : "") }, h("span", null, two ? CREW_NAMES[id] : "Your crew"), h("strong", null, String(score))),
          ),
        ),
        h(
          "div",
          { class: "rh-river-state " + (r.revived ? "good" : r.choked ? "bad" : "") },
          // Endless scores are metres + plastic, so the revive bonus and choke penalty don't apply there.
          s?.endless
            ? `🌊 River health ${Math.round(r.health)}%`
            : r.revived
              ? `🌊 River health ${Math.round(r.health)}% — the crew gets +20 for keeping it alive`
              : r.choked
                ? "🟤 The river choked — the crew's score was halved"
                : `🌊 River health ${Math.round(r.health)}%`,
        ),
        h("button", { class: "btn primary big", onclick: () => this.cb.leave() }, host ? "Back to the dock" : "Back to the dock"),
      ),
    );
    this.sigCenter = "results";
    if (this.pendingRiver) this.riverResult(this.pendingRiver);
    if (this.pendingSummary) this.runSummary(...this.pendingSummary);
  }

  private pendingRiver: RiverResult | null = null;
  private pendingSummary: [MissionView[], RankView | null] | null = null;

  /** Today's missions and the rank, under the stars on the results card. */
  runSummary(missions: MissionView[], rank: RankView | null) {
    const card = this.center.querySelector(".rh-results");
    if (!card) {
      this.pendingSummary = [missions, rank];
      return;
    }
    this.pendingSummary = null;
    card.querySelector(".rh-summary")?.remove();
    const block = h(
      "div",
      { class: "rh-summary" },
      missions.length ? h("div", { class: "rh-stars-title" }, "Daily missions") : null,
      missions.length ? missionList(missions) : null,
      rankCard(rank ?? undefined),
    );
    card.insertBefore(block, card.querySelector("button"));
  }

  /** Stars and best score on the results card (arrives just after the results). */
  riverResult(r: RiverResult) {
    const card = this.center.querySelector(".rh-results");
    if (!card) {
      this.pendingRiver = r;
      return;
    }
    this.pendingRiver = null;
    card.querySelector(".rh-stars")?.remove();
    if (r.endless) {
      const e = h(
        "div",
        { class: "rh-stars" },
        h("div", { class: "rh-stars-title" }, `∞ Endless · ${r.name}`),
        h("div", { class: "rh-best" }, `${r.distance ?? 0} m travelled · score ${r.score} (metres + 5 per plastic)`),
        h("div", { class: "rh-best" + (r.new_best ? " new" : "") }, r.new_best ? `🏆 New Endless best: ${r.best}` : `Endless best: ${r.best}`),
      );
      card.insertBefore(e, card.querySelector("button"));
      return;
    }
    const block = h(
      "div",
      { class: "rh-stars" },
      h("div", { class: "rh-stars-title" }, r.name),
      h(
        "ul",
        null,
        r.goals.map((g, i) =>
          h(
            "li",
            { class: (r.stars[i] ? "on" : "") + (r.new_stars[i] ? " new" : "") },
            h("span", { class: "star" + (r.stars[i] ? " on" : "") }, "★"),
            g,
            r.new_stars[i] ? h("span", { class: "chip good" }, "NEW") : null,
          ),
        ),
      ),
      h("div", { class: "rh-best" + (r.new_best ? " new" : "") }, r.new_best ? `🏆 New best score: ${r.best}` : `Best score: ${r.best}`),
    );
    card.insertBefore(block, card.querySelector("button"));
  }

  /** "+1" style popup at a screen position. */
  popup(x: number, y: number, text: string, kind = "") {
    const el = h("div", { class: "rh-pop " + kind, style: `left:${x}px;top:${y}px` }, text);
    this.fx.appendChild(el);
    setTimeout(() => el.remove(), 900);
  }

  /** Bubble over a boat ("✔" when its player sends their number). */
  bubble(x: number, y: number, text: string, seat: number) {
    const el = h("div", { class: "rh-bubble", style: `left:${x}px;top:${y}px;border-color:${SEAT_COLOURS[seat % 4]}` }, text);
    this.fx.appendChild(el);
    setTimeout(() => el.remove(), 1600);
  }

  /** Edge vignette on your own capsize: a red pulse, or a steady tint with reduced flashing. */
  hit(reduceFlash: boolean) {
    const el = h("div", { class: "rh-hit" + (reduceFlash ? " calm" : "") });
    this.fx.appendChild(el);
    setTimeout(() => el.remove(), 700);
  }

  banner(text: string, kind = "", ms = 1800) {
    // The same message twice in a row just restarts its timer.
    const same = [...this.banners.children].find((c) => c.textContent === text);
    same?.remove();
    const el = h("div", { class: "rh-banner " + kind }, text);
    this.banners.prepend(el);
    while (this.banners.children.length > 2) this.banners.lastElementChild!.remove();
    if (ms !== 1800) el.style.animationDuration = `${ms}ms`;
    setTimeout(() => el.remove(), ms);
  }

  /** Name tag above each boat (screen position from the 3D scene). */
  tag(seat: number, x: number, y: number, visible: boolean, parked = false) {
    let el = this.tags.get(seat);
    if (!el) {
      el = h("div", { class: "rh-tag" + (seat === this.mySeat ? " me" : ""), style: `--c:${SEAT_COLOURS[seat % 4]}` }, seat === this.mySeat ? "You" : this.name(seat));
      this.fx.appendChild(el);
      this.tags.set(seat, el);
    }
    el.style.display = visible ? "" : "none";
    el.classList.toggle("parked", parked);
    el.style.transform = `translate(${x}px, ${y}px) translate(-50%, -100%)`;
  }

  dispose() {
    this.root.remove();
  }
}

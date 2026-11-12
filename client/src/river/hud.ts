// In-run HUD: big, glanceable, minimal text. Rebuilt only when its inputs change.

import { h } from "../ui/dom";
import {
  CREW_NAMES,
  FINALE_INFO,
  FINALE_ORDER,
  GLYPHS,
  RUN_SECONDS,
  SEAT_COLOURS,
  TWIST_INFO,
  TWIST_ORDER,
  type RunResults,
  type Snapshot,
} from "./types";

export interface HudCallbacks {
  choose(option: number): void;
  glyph(g: number): void;
  leave(): void;
}

export class RunHud {
  readonly root: HTMLElement;
  private top = h("div", { class: "rh-top" });
  private center = h("div", { class: "rh-center" });
  private code = h("div", { class: "rh-code" });
  private bottom = h("div", { class: "rh-bottom" });
  private fx = h("div", { class: "rh-fx" });
  private sigTop = "";
  private sigCenter = "";
  private sigCode = "";
  private sigBottom = "";
  private lastPhase = "";
  private promptEl = h("div", { class: "rh-prompt" });
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
      this.top,
      this.center,
      this.code,
      this.bottom,
      this.fx,
      this.promptEl,
      h("div", { class: "rh-keys" }, "◀ ▶ steer · ▲ hop · ▼ cut / plug · 1–6 signal"),
    );
    host.appendChild(this.root);
  }

  private name(seat: number) {
    return this.names[String(seat)] || `Player ${seat + 1}`;
  }

  update(s: Snapshot) {
    if (this.lastPhase === "countdown" && s.phase.phase === "running") this.banner("GO!", "go");
    this.lastPhase = s.phase.phase;
    const me = s.boats.find((b) => b.seat === this.mySeat);
    // Pipe prompt: the nearest open pipe ahead, and which way to steer.
    const pipe = me ? s.objs.filter((o) => o.kind === "pipe" && !o.plugged && o.z > me.z - 4 && o.z < me.z + 45).sort((a, b) => a.z - b.z)[0] : undefined;
    let prompt = "";
    if (me && pipe && !me.plugging) {
      const side = pipe.lane < 0 ? "LEFT" : "RIGHT";
      prompt = me.lane === pipe.lane ? (pipe.z - me.z < 7 ? "🔧 Press ▼ now to plug the pipe!" : `🔧 Pipe ahead on the ${side} — get ready to press ▼`) : `🔧 Sewage pipe ahead on the ${side} — move ${pipe.lane < 0 ? "◀" : "▶"} and press ▼`;
    }
    if (this.promptEl.textContent !== prompt) {
      this.promptEl.textContent = prompt;
      this.promptEl.classList.toggle("on", !!prompt);
    }
    const myCrew = me?.crew ?? 0;
    const left = Math.max(0, RUN_SECONDS - s.t);
    const mm = Math.floor(left / 60),
      ss = Math.floor(left % 60);
    const twist = s.twist ? TWIST_INFO[s.twist] : null;
    const finale = s.finale ? FINALE_INFO[s.finale] : null;

    // ── top bar: scores · timer · River Health ──
    const topSig = JSON.stringify([s.crews, mm, ss, Math.round(s.health), s.twist, s.finale, s.choked]);
    if (topSig !== this.sigTop) {
      this.sigTop = topSig;
      const hp = Math.round(s.health);
      this.top.replaceChildren(
        h(
          "div",
          { class: "rh-scores" },
          s.crews.map(([id, score, plastic, pipes]) =>
            h(
              "div",
              { class: "rh-crew" + (id === myCrew ? " mine" : "") },
              h("span", { class: "rh-crew-name" }, s.crews.length > 1 ? CREW_NAMES[id] : "Your crew"),
              h("span", { class: "rh-score" }, String(score)),
              h("span", { class: "rh-sub" }, `♻ ${plastic} · 🔧 ${pipes}`),
            ),
          ),
        ),
        h(
          "div",
          { class: "rh-clock" },
          h("div", { class: "rh-time" + (left < 15 ? " urgent" : "") }, `${mm}:${String(ss).padStart(2, "0")}`),
          h(
            "div",
            { class: "rh-badges" },
            twist ? h("span", { class: "rh-badge", title: twist.line }, `${twist.icon} ${twist.name}`) : null,
            finale ? h("span", { class: "rh-badge", title: finale.line }, `${finale.icon} ${finale.name}`) : null,
          ),
        ),
        h(
          "div",
          { class: "rh-health" + (hp < 30 ? " low" : "") },
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
      this.center.replaceChildren(...this.centerContent(s, myCrew).filter((x): x is HTMLElement => !!x));
    }

    // ── gate code (private to the crew) ──
    const plug = s.plugs[0];
    const codeSig = JSON.stringify(plug ?? null);
    if (codeSig !== this.sigCode) {
      this.sigCode = codeSig;
      if (!plug) this.code.replaceChildren();
      else {
        const [seat, pv] = plug;
        const own = seat === this.mySeat;
        this.code.replaceChildren(
          h(
            "div",
            { class: "rh-code-card" + (pv.clear ? " clear" : " scrambled") },
            h("div", { class: "rh-code-title" }, own ? (pv.clear ? "Memorise the gate code!" : "Gate code — scrambled!") : `${this.name(seat)} is plugging a pipe`),
            h(
              "div",
              { class: "rh-code-glyphs" },
              pv.code.map((g, i) => h("span", { class: "rh-glyph" + (i < pv.entered ? " done" : "") }, GLYPHS[g], h("kbd", null, String(g + 1)))),
            ),
            h("div", { class: "rh-code-hint" }, own ? (pv.clear ? "Type it in order with keys 1–6." : "Right symbols, wrong order. Your crewmate sees the real order — watch their signals.") : "Only your crew sees this order. Signal it with keys 1–6 — the other crew can see your signals too."),
            h("div", { class: "rh-code-time" }, `${Math.ceil(pv.left)} s`),
          ),
        );
      }
    }

    // ── bottom: boost meter ──
    const boost = me ? (me.boosting > 0 ? 1 : me.boost) : 0;
    const bottomSig = `${Math.round(boost * 20)}|${me?.boosting ? 1 : 0}`;
    if (bottomSig !== this.sigBottom) {
      this.sigBottom = bottomSig;
      this.bottom.replaceChildren(
        h(
          "div",
          { class: "rh-boost" + (me?.boosting ? " on" : "") },
          h("span", null, me?.boosting ? "BOOST!" : "Boost"),
          h("div", { class: "rh-bar" }, h("div", { class: "rh-fill", style: `width:${Math.round(boost * 100)}%` })),
        ),
      );
    }
  }

  private centerContent(s: Snapshot, myCrew: number): (HTMLElement | null)[] {
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
          h("div", { class: "rh-choice-foot" }, "A tie is settled by a spin."),
        ),
      ];
    }
    if (p.phase === "choosing" && p.change === "finale") {
      const mine = p.pickers === myCrew;
      return [
        h(
          "div",
          { class: "rh-choice" },
          h("div", { class: "rh-choice-title" }, mine ? "You're behind — pick the final twist!" : `${s.crews.length > 1 ? CREW_NAMES[p.pickers ?? 0] : "Your crew"} picks the final twist…`, h("span", { class: "rh-choice-time" }, `${Math.ceil(p.left)}`)),
          h(
            "div",
            { class: "rh-cards two" },
            FINALE_ORDER.map((f, i) => {
              const info = FINALE_INFO[f];
              return h(
                "button",
                { class: "rh-card", disabled: !mine, onclick: () => this.cb.choose(i) },
                h("kbd", null, String(i + 1)),
                h("div", { class: "rh-card-icon" }, info.icon),
                h("div", { class: "rh-card-name" }, info.name),
                h("div", { class: "rh-card-line" }, info.line),
              );
            }),
          ),
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
          h("div", { class: "rh-reveal-kicker" }, p.spin ? "🎡 No clear choice — the spin decides" : p.change === "twist" ? "The river changes" : "Final twist"),
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
    const title = !two ? (r.revived ? "River revived!" : r.choked ? "The river choked" : "Run complete") : r.winner === null ? "It's a draw!" : r.winner === myCrew ? "Your crew wins!" : `${CREW_NAMES[r.winner]} win!`;
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
          r.revived ? `🌊 River health ${Math.round(r.health)}% — everyone +20 for keeping it alive` : r.choked ? "🟤 The river choked — everyone's score was halved" : `🌊 River health ${Math.round(r.health)}%`,
        ),
        h("button", { class: "btn primary big", onclick: () => this.cb.leave() }, host ? "Back to the dock" : "Back to the dock"),
      ),
    );
    this.sigCenter = "results";
  }

  /** "+1" style popup at a screen position. */
  popup(x: number, y: number, text: string, kind = "") {
    const el = h("div", { class: "rh-pop " + kind, style: `left:${x}px;top:${y}px` }, text);
    this.fx.appendChild(el);
    setTimeout(() => el.remove(), 900);
  }

  /** Signal bubble over a boat. */
  bubble(x: number, y: number, glyph: number, seat: number) {
    const el = h("div", { class: "rh-bubble", style: `left:${x}px;top:${y}px;border-color:${SEAT_COLOURS[seat % 4]}` }, GLYPHS[glyph]);
    this.fx.appendChild(el);
    setTimeout(() => el.remove(), 1600);
  }

  banner(text: string, kind = "") {
    const el = h("div", { class: "rh-banner " + kind }, text);
    this.fx.appendChild(el);
    setTimeout(() => el.remove(), 1800);
  }

  /** Name tag above each boat (screen position from the 3D scene). */
  tag(seat: number, x: number, y: number, visible: boolean) {
    let el = this.tags.get(seat);
    if (!el) {
      el = h("div", { class: "rh-tag" + (seat === this.mySeat ? " me" : ""), style: `--c:${SEAT_COLOURS[seat % 4]}` }, seat === this.mySeat ? "You" : this.name(seat));
      this.fx.appendChild(el);
      this.tags.set(seat, el);
    }
    el.style.display = visible ? "" : "none";
    el.style.transform = `translate(${x}px, ${y}px) translate(-50%, -100%)`;
  }

  dispose() {
    this.root.remove();
  }
}

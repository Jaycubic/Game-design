// In-run HUD: big, glanceable, minimal text. Rebuilt only when its inputs change.

import { h } from "../ui/dom";
import { hint } from "../input/controls";
import { actionVerb, usesTouch } from "../input/touch";
import { signalName } from "../ui/panels";
import type { MissionView, RankView, RiverResult } from "../net/api";
import { missionList, rankCard } from "../ui/hub";
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
      this.top,
      this.center,
      this.code,
      this.bottom,
      this.fx,
      this.pipeCard,
      this.pipeMark,
      h(
        "div",
        { class: "rh-keys" },
        h("span", { "data-tut": "steer" }, `${hint("left")} ${hint("right")} steer`),
        " · ",
        h("span", { "data-tut": "hop" }, `${hint("hop")} hop`),
        " · ",
        h("span", { "data-tut": "action" }, `${hint("action")} cut / plug`),
      ),
      // Desktop signal bar: the six symbols with their key and name (touch has its own buttons).
      h(
        "div",
        { class: "rh-signals", role: "group", "aria-label": "Signals", "data-tut": "signals" },
        GLYPHS.map((g, i) =>
          h(
            "button",
            { class: "rh-sig", title: `Signal ${i + 1}: ${signalName(i)}`, onclick: () => this.cb.glyph(i) },
            h("kbd", null, String(i + 1)),
            h("b", null, g),
            h("small", null, signalName(i)),
          ),
        ),
      ),
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
    this.pipePrompt(s, me);
    const myCrew = me?.crew ?? 0;
    const left = Math.max(0, (s.duration || RUN_SECONDS) - s.t);
    const mm = Math.floor(left / 60),
      ss = Math.floor(left % 60);
    const twist = s.twist ? TWIST_INFO[s.twist] : null;
    const finale = s.finale ? FINALE_INFO[s.finale] : null;

    // ── top bar: scores · timer · River Health ──
    const topSig = JSON.stringify([s.crews, mm, ss, s.endless ? Math.floor(s.distance ?? 0) : 0, Math.round(s.health), s.twist, s.finale, s.choked]);
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
        const how = usesTouch() ? "tap the symbol buttons" : "press keys 1–6";
        // Three cases: a lone boat's memory puzzle, the plugger (scrambled), a crewmate (real order).
        const title = !own
          ? `${this.name(seat)} is plugging a pipe — tell them the order!`
          : pv.memory
            ? pv.hidden
              ? "Now enter it from memory"
              : "Memorise the gate code!"
            : "Gate code — scrambled!";
        const hint = !own
          ? `You see the real order; they see it mixed up. Send the 3 symbols in order: ${how}. The other crew sees your signals too.`
          : pv.memory
            ? pv.hidden
              ? `Enter the 3 symbols in the order you saw: ${how}.`
              : "It hides in a moment. Remember the order!"
            : `Right symbols, wrong order. Your crewmate sees the real order — watch the symbols over their boat, then ${how}.`;
        const glyph = (g: number, i: number) =>
          pv.hidden
            ? h("span", { class: "rh-glyph hidden" + (i < pv.entered ? " done" : "") }, i < pv.entered ? "✔" : "?")
            : h("span", { class: "rh-glyph" + (i < pv.entered ? " done" : "") }, GLYPHS[g], h("kbd", null, String(g + 1)), h("small", { class: "rh-glyph-name" }, signalName(g)));
        this.code.replaceChildren(
          h(
            "div",
            { class: "rh-code-card" + (pv.clear ? " clear" : pv.hidden ? " memory" : " scrambled"), "data-tut": "code" },
            h("div", { class: "rh-code-title" }, title),
            h("div", { class: "rh-code-glyphs" }, pv.code.map(glyph)),
            h("div", { class: "rh-code-hint" }, hint),
            h("div", { class: "rh-code-time" }, `${Math.ceil(pv.left)} s`),
            pv.memory ? null : this.signalTip(),
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
          { class: "rh-boost" + (me?.boosting ? " on" : ""), "data-tut": "boost" },
          h("span", null, me?.boosting ? "BOOST!" : "Boost"),
          h("div", { class: "rh-bar" }, h("div", { class: "rh-fill", style: `width:${Math.round(boost * 100)}%` })),
        ),
      );
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
  pipeMarker(x: number, y: number, visible: boolean, metres: number) {
    this.pipeMark.style.display = visible ? "" : "none";
    if (!visible) return;
    this.pipeMark.style.transform = `translate(${x}px, ${y}px) translate(-50%, -100%)`;
    const label = this.pipeMark.lastChild as HTMLElement;
    const text = `${Math.round(metres)} m`;
    if (label.textContent !== text) label.textContent = text;
  }

  /** Shown the first time a gate code ever appears on this device. */
  private tipShown = false;
  private signalTip(): HTMLElement | null {
    if (this.tipShown) return h("div", { class: "rh-code-tip" }, "Your crewmate sees the real order. Tap the symbols to tell them — the other crew sees your signals too.");
    let seen = false;
    try {
      seen = localStorage.getItem("nt.tip.signals") === "1";
      localStorage.setItem("nt.tip.signals", "1");
    } catch {
      /* private mode: show once per run */
    }
    if (seen) return null;
    this.tipShown = true;
    return this.signalTip();
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
          h("div", { class: "rh-choice-title" }, mine ? (s.crews.length > 1 ? "You're behind — pick the final twist!" : "Pick the final twist!") : `${s.crews.length > 1 ? CREW_NAMES[p.pickers ?? 0] : "Your crew"} picks the final twist…`, h("span", { class: "rh-choice-time" }, `${Math.ceil(p.left)}`)),
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
    const title = s?.endless ? "Endless run over" : !two ? (r.revived ? "River revived!" : r.choked ? "The river choked" : "Run complete") : r.winner === null ? "It's a draw!" : r.winner === myCrew ? "Your crew wins!" : `${CREW_NAMES[r.winner]} win!`;
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

  /** Signal bubble over a boat. */
  bubble(x: number, y: number, glyph: number, seat: number) {
    const el = h("div", { class: "rh-bubble", style: `left:${x}px;top:${y}px;border-color:${SEAT_COLOURS[seat % 4]}` }, GLYPHS[glyph]);
    this.fx.appendChild(el);
    setTimeout(() => el.remove(), 1600);
  }

  /** Edge vignette on your own capsize: a red pulse, or a steady tint with reduced flashing. */
  hit(reduceFlash: boolean) {
    const el = h("div", { class: "rh-hit" + (reduceFlash ? " calm" : "") });
    this.fx.appendChild(el);
    setTimeout(() => el.remove(), 700);
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

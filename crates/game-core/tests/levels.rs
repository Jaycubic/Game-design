//! Content-level guarantees, proven by the solver on the shipped Forest Realm.

use nt_core::campaign::CampaignState;
use nt_core::content::forest_realm;
use nt_core::level::LevelKind;
use nt_core::rules::{Connectivity, Recovery, Ruleset};
use nt_core::sim::{Action, Sim, Status};
use nt_core::solver::{replay, solve, Limits, SolveResult};

fn level(n: u8) -> nt_core::LevelDef {
    forest_realm().levels.into_iter().find(|l| l.number == n).unwrap()
}

fn solve_level(n: u8, rules: Ruleset) -> nt_core::solver::Solution {
    let l = level(n);
    let sim = Sim::new(&l, rules).unwrap();
    match solve(&sim, &sim.initial_state(), &Limits::default()) {
        SolveResult::Solved { solution, .. } => solution,
        other => panic!("L{n} under {} is not solvable: {other:?}", rules.key()),
    }
}

/// Every level is solvable under every ruleset it can be played with, and the
/// par stored in the content file is the proven minimum.
fn assert_level_sound(n: u8) {
    let l = level(n);
    for rules in CampaignState::possible_rulesets(n) {
        let sol = solve_level(n, rules);
        let sim = Sim::new(&l, rules).unwrap();
        let spent = replay(&sim, &sol.plan).unwrap_or_else(|e| panic!("L{n} plan does not replay: {e}"));
        assert_eq!(spent, sol.cost);
        if l.kind != LevelKind::Deduction {
            assert_eq!(l.par_for(&rules), Some(sol.cost), "L{n} par for {}", rules.key());
        }
    }
}

#[test]
fn region_has_ten_levels_five_of_each_mode_and_two_amendments() {
    let r = forest_realm();
    assert_eq!(r.levels.len(), 10);
    let puzzles = r.levels.iter().filter(|l| l.mode == nt_core::Mode::Puzzle).count();
    assert_eq!(puzzles, 5);
    for (i, l) in r.levels.iter().enumerate() {
        let expect_puzzle = i % 2 == 0;
        assert_eq!(l.mode == nt_core::Mode::Puzzle, expect_puzzle, "levels alternate puzzle/expedition");
    }
    assert_eq!(r.amendments.len(), 2);
    assert_eq!(r.amendments[0].after_level, 2);
    assert_eq!(r.amendments[1].after_level, 4);
}

#[test]
fn every_journal_reward_has_an_entry_with_valid_sources() {
    let j = nt_core::content::journal();
    let keys: Vec<&str> = j.entries.iter().map(|e| e.key.as_str()).collect();
    for l in forest_realm().levels {
        for k in &l.reward.journal {
            assert!(keys.contains(&k.as_str()), "L{} unlocks missing journal entry '{k}'", l.number);
        }
    }
    for e in &j.entries {
        for s in &e.sources {
            assert!(j.sources.iter().any(|x| &x.id == s), "entry {} cites unknown source {s}", e.key);
        }
    }
}

#[test]
fn level_01_sound() {
    assert_level_sound(1);
}
#[test]
fn level_02_sound() {
    assert_level_sound(2);
}
#[test]
fn level_03_sound() {
    assert_level_sound(3);
}
#[test]
fn level_04_sound() {
    assert_level_sound(4);
}
#[test]
fn level_05_sound() {
    assert_level_sound(5);
}
#[test]
fn level_06_sound() {
    assert_level_sound(6);
}
#[test]
fn level_07_sound() {
    assert_level_sound(7);
}
#[test]
fn level_08_sound() {
    assert_level_sound(8);
}
#[test]
fn level_09_sound() {
    assert_level_sound(9);
}
#[test]
fn level_10_sound() {
    assert_level_sound(10);
}

fn plan_valid_under(n: u8, plan: &[Action], rules: Ruleset) -> Option<u16> {
    let l = level(n);
    let sim = Sim::new(&l, rules).unwrap();
    replay(&sim, plan).ok()
}

#[test]
fn connectivity_amendment_changes_solutions() {
    // For every level after amendment 1, the optimal plan under one connectivity
    // rule is invalid or more expensive under the other.
    let mut same = Vec::new();
    for n in [3u8, 4, 6, 7, 8, 9, 10] {
        let recovery = if n > 4 { Recovery::Succession } else { Recovery::Base };
        let cc = Ruleset::new(Connectivity::Continuous, recovery);
        let ss = Ruleset::new(Connectivity::SteppingStone, recovery);
        let a = solve_level(n, cc);
        let b = solve_level(n, ss);
        let a_under_b = plan_valid_under(n, &a.plan, ss);
        let b_under_a = plan_valid_under(n, &b.plan, cc);
        let differs = a_under_b != Some(b.cost) || b_under_a != Some(a.cost);
        if !differs {
            same.push(n);
        }
    }
    assert!(same.is_empty(), "connectivity rules produce interchangeable optimal plans in levels {same:?}");
}

#[test]
fn recovery_amendment_changes_solutions() {
    for n in [6u8, 7, 8, 9, 10] {
        for c in [Connectivity::Continuous, Connectivity::SteppingStone] {
            let s = Ruleset::new(c, Recovery::Succession);
            let a = Ruleset::new(c, Recovery::Active);
            let ps = solve_level(n, s);
            let pa = solve_level(n, a);
            assert_ne!(
                (plan_valid_under(n, &ps.plan, a), plan_valid_under(n, &pa.plan, s)),
                (Some(pa.cost), Some(ps.cost)),
                "L{n}: recovery rules produce interchangeable optimal plans"
            );
        }
    }
}

#[test]
fn deduction_answer_depends_on_connectivity_and_is_fair_for_2_to_4_players() {
    let l = level(5);
    let d = l.deduction.as_ref().unwrap();
    let board = l.board().unwrap();
    let mut answers = Vec::new();
    for rules in CampaignState::possible_rulesets(5) {
        for players in 2..=4 {
            d.fair_for(&board, &rules, players).unwrap_or_else(|e| panic!("{}: {e}", rules.key()));
        }
        answers.push((rules.connectivity, d.answer(&board, &rules).unwrap()));
    }
    let cc: Vec<_> = answers.iter().filter(|(c, _)| *c == Connectivity::Continuous).map(|x| x.1).collect();
    let ss: Vec<_> = answers.iter().filter(|(c, _)| *c == Connectivity::SteppingStone).map(|x| x.1).collect();
    assert!(cc.iter().all(|a| *a == cc[0]) && ss.iter().all(|a| *a == ss[0]));
    assert_ne!(cc[0], ss[0], "the corridor rule must change the deduction answer");
}

#[test]
fn optimal_play_earns_objective_and_efficiency_marks() {
    for n in 1..=10u8 {
        let l = level(n);
        let rules = CampaignState::possible_rulesets(n)[0];
        let sim = Sim::new(&l, rules).unwrap();
        let sol = solve_level(n, rules);
        let mut s = sim.initial_state();
        for a in &sol.plan {
            sim.apply(&mut s, a).unwrap();
        }
        assert!(matches!(s.status, Status::Won { .. }));
        let marks = sim.leaf_marks(&s);
        assert!(marks.objective && marks.efficient, "L{n}: optimal play must earn marks 1 and 2");
        // Determinism: replaying gives identical state and marks.
        let mut again = sim.initial_state();
        for a in &sol.plan {
            sim.apply(&mut again, a).unwrap();
        }
        assert_eq!(s, again);
        assert_eq!(marks, sim.leaf_marks(&again));
    }
}

#[test]
fn deduction_wrong_answers_then_loss() {
    let l = level(5);
    let rules = Ruleset::new(Connectivity::Continuous, Recovery::Succession);
    let sim = Sim::new(&l, rules).unwrap();
    let mut s = sim.initial_state();
    for _ in 0..l.max_attempts {
        let out = sim.apply(&mut s, &Action::Answer { candidate: "G".into() }).unwrap();
        assert_eq!(out, nt_core::sim::Outcome::Answered { correct: false });
    }
    assert!(matches!(s.status, Status::Lost { .. }));
    assert!(sim.apply(&mut s, &Action::Answer { candidate: "A".into() }).is_err());
}

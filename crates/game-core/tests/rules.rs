//! Rule-engine behaviour on small purpose-built levels.

use nt_core::level::LevelDef;
use nt_core::rules::{Connectivity, Recovery, Ruleset};
use nt_core::sim::{Action, Outcome, RuleError, Sim, Status};
use nt_core::{Pos, Species, Terrain};

fn level(json: &str) -> LevelDef {
    serde_json::from_str(json).unwrap()
}

const ROUTE: &str =
    r#"[{"kind":"connect","species":"malabar_giant_squirrel","from":[0,0],"to":[4,0],"label":"route"}]"#;

fn expedition(map: &[&str], extra: &str) -> LevelDef {
    expedition_with(map, ROUTE, extra)
}

fn expedition_with(map: &[&str], objectives: &str, extra: &str) -> LevelDef {
    let map = serde_json::to_string(map).unwrap();
    level(&format!(
        r#"{{"id":"t","number":9,"mode":"expedition","kind":"expedition","title":"t","area":"a","instruction":"i","intro":"i",
        "map":{map},"rounds":3,"capacity":3,
        "objectives":{objectives},
        "challenge":{{"text":"t","kind":"first_try"}} {extra} }}"#
    ))
}

const SS_BASE: Ruleset = Ruleset::new(Connectivity::SteppingStone, Recovery::Base);
const CC_BASE: Ruleset = Ruleset::new(Connectivity::Continuous, Recovery::Base);

#[test]
fn tile_placement_and_route_validation() {
    let l = expedition(&["F...F", "~~~~~"], "");
    let sim = Sim::new(&l, Ruleset::BASE).unwrap();
    let mut s = sim.initial_state();
    for x in 1..=3 {
        assert_eq!(sim.apply(&mut s, &Action::Plant { at: Pos::new(x, 0) }), Ok(Outcome::Applied { cost: 1 }));
    }
    // Planting on a river or on forest is rejected without changing state.
    let before = s.clone();
    assert!(matches!(sim.apply(&mut s, &Action::Plant { at: Pos::new(0, 1) }), Err(RuleError::WrongTerrain { .. })));
    assert!(matches!(sim.apply(&mut s, &Action::Plant { at: Pos::new(0, 0) }), Err(RuleError::WrongTerrain { .. })));
    assert_eq!(sim.apply(&mut s, &Action::Plant { at: Pos::new(9, 9) }), Err(RuleError::OffMap));
    assert_eq!(s, before, "rejected actions must not alter state");
    sim.apply(&mut s, &Action::EndRound).unwrap();
    assert_eq!(s.status, Status::Won { round: 1 });
}

#[test]
fn capacity_is_shared_and_enforced() {
    let l = expedition(&["F...F"], "");
    let sim = Sim::new(&l, Ruleset::BASE).unwrap();
    let mut s = sim.initial_state();
    for x in 1..=3 {
        sim.apply(&mut s, &Action::Plant { at: Pos::new(x, 0) }).unwrap();
    }
    assert_eq!(s.capacity_left, 0);
    let mut s = sim.initial_state();
    s.capacity_left = 0;
    assert_eq!(
        sim.apply(&mut s, &Action::Plant { at: Pos::new(1, 0) }),
        Err(RuleError::NotEnoughCapacity { need: 1, have: 0 })
    );
}

#[test]
fn charter_gates_bridges_and_groves() {
    let l = expedition(&["F.=.F"], "");
    let base = Sim::new(&l, Ruleset::BASE).unwrap();
    let mut s = base.initial_state();
    assert_eq!(base.apply(&mut s, &Action::Bridge { at: Pos::new(2, 0) }), Err(RuleError::NotAllowedByCharter));
    assert_eq!(base.apply(&mut s, &Action::Grove { at: Pos::new(1, 0) }), Err(RuleError::NotAllowedByCharter));

    // Continuous: plant, bridge, plant = 4 capacity.
    let cc = Sim::new(&l, CC_BASE).unwrap();
    let mut s = cc.initial_state();
    s.capacity_left = 9;
    cc.apply(&mut s, &Action::Plant { at: Pos::new(1, 0) }).unwrap();
    cc.apply(&mut s, &Action::Bridge { at: Pos::new(2, 0) }).unwrap();
    cc.apply(&mut s, &Action::Plant { at: Pos::new(3, 0) }).unwrap();
    assert_eq!(s.spent, 4);
    cc.apply(&mut s, &Action::EndRound).unwrap();
    assert!(matches!(s.status, Status::Won { .. }));

    // Stepping-stone: plant one side, grove the other, hop the road = 3 capacity.
    let ss = Sim::new(&l, SS_BASE).unwrap();
    let mut s = ss.initial_state();
    ss.apply(&mut s, &Action::Plant { at: Pos::new(1, 0) }).unwrap();
    ss.apply(&mut s, &Action::Grove { at: Pos::new(3, 0) }).unwrap();
    assert_eq!(s.spent, 3);
    ss.apply(&mut s, &Action::EndRound).unwrap();
    assert!(matches!(s.status, Status::Won { .. }));
}

#[test]
fn natural_succession_and_active_restoration_differ() {
    // A degraded cell surrounded by forest on three sides.
    let map = ["FFFFF", "FFDFF", "F...F"];
    let l = expedition_with(&map, r#"[{"kind":"restore","cells":[[2,1]],"label":"heal"}]"#, "");
    let succ = Sim::new(&l, Ruleset::new(Connectivity::Continuous, Recovery::Succession)).unwrap();
    let mut s = succ.initial_state();
    succ.apply(&mut s, &Action::EndRound).unwrap();
    assert!(matches!(s.status, Status::Won { .. }), "succession heals it for free");
    assert_eq!(s.spent, 0);

    let act = Sim::new(&l, Ruleset::new(Connectivity::Continuous, Recovery::Active)).unwrap();
    let mut s = act.initial_state();
    act.apply(&mut s, &Action::EndRound).unwrap();
    assert_eq!(s.status, Status::InProgress, "no passive regrowth under Active Restoration");

    // Active restoration heals the target and adjacent degraded cells for 2.
    let map = ["FDDDF", "..D.."];
    let l = expedition(&map, "");
    let act = Sim::new(&l, Ruleset::new(Connectivity::Continuous, Recovery::Active)).unwrap();
    let mut s = act.initial_state();
    assert_eq!(act.apply(&mut s, &Action::Restore { at: Pos::new(2, 0) }), Ok(Outcome::Applied { cost: 2 }));
    for x in 1..=3 {
        assert_eq!(s.board.tile(Pos::new(x, 0)).terrain, Terrain::Forest);
    }
    assert_eq!(s.board.tile(Pos::new(2, 1)).terrain, Terrain::Forest);
    let succ = Sim::new(&l, Ruleset::new(Connectivity::Continuous, Recovery::Succession)).unwrap();
    let mut s = succ.initial_state();
    assert_eq!(succ.apply(&mut s, &Action::Restore { at: Pos::new(2, 0) }), Ok(Outcome::Applied { cost: 1 }));
    assert_eq!(s.board.tile(Pos::new(1, 0)).terrain, Terrain::Degraded);
}

#[test]
fn lantana_degrades_unless_cleared() {
    let ev = r#","events":[{"round":1,"title":"L","text":"t","kind":"lantana","cells":[[1,0],[2,0]]}]"#;
    let l = expedition(&["FFFFF"], ev);
    let sim = Sim::new(&l, CC_BASE).unwrap();
    let mut s = sim.initial_state();
    assert_eq!(sim.apply(&mut s, &Action::Protect { at: Pos::new(3, 0) }), Err(RuleError::NotThreatened));
    sim.apply(&mut s, &Action::Protect { at: Pos::new(1, 0) }).unwrap();
    assert_eq!(sim.apply(&mut s, &Action::Protect { at: Pos::new(1, 0) }), Err(RuleError::AlreadyProtected));
    let out = sim.apply(&mut s, &Action::EndRound).unwrap();
    let Outcome::RoundResolved { report } = out else { panic!() };
    assert_eq!(report.degraded, vec![Pos::new(2, 0)]);
    assert_eq!(s.board.tile(Pos::new(1, 0)).terrain, Terrain::Forest);
    assert_eq!(s.status, Status::InProgress, "route broken by lantana");
}

#[test]
fn erosion_and_dry_spell() {
    let ev = r#","events":[
        {"round":1,"title":"Dry","text":"t","kind":"dry_spell","capacity_penalty":2},
        {"round":1,"title":"Rain","text":"t","kind":"erosion","cells":[[2,1],[3,1]]}]"#;
    let map = ["FFFFF", "FDDDF", "FFFFF"];
    let l = expedition_with(&map, r#"[{"kind":"restore","cells":[[1,1],[2,1],[3,1]],"label":"heal"}]"#, ev);
    let sim = Sim::new(&l, Ruleset::new(Connectivity::Continuous, Recovery::Succession)).unwrap();
    let mut s = sim.initial_state();
    assert_eq!(s.capacity_left, 1, "dry spell removes capacity");
    sim.apply(&mut s, &Action::Restore { at: Pos::new(2, 1) }).unwrap();
    sim.apply(&mut s, &Action::EndRound).unwrap();
    // (2,1) was restored in time; (3,1) washed out; (1,1) did not regrow during the dry spell.
    assert_eq!(s.board.tile(Pos::new(2, 1)).terrain, Terrain::Forest);
    assert_eq!(s.board.tile(Pos::new(3, 1)).terrain, Terrain::Open);
    assert_eq!(s.board.tile(Pos::new(1, 1)).terrain, Terrain::Degraded);
    // Next round regrowth resumes: (1,1) has 3+ forest neighbours.
    sim.apply(&mut s, &Action::EndRound).unwrap();
    assert_eq!(s.board.tile(Pos::new(1, 1)).terrain, Terrain::Forest);
}

#[test]
fn critical_deterioration_and_round_limit_fail_the_mission() {
    let ev = r#","critical_degraded":2,"events":[{"round":1,"title":"L","text":"t","kind":"lantana","cells":[[1,0],[2,0]]}]"#;
    let l = expedition(&["FFF.F"], ev);
    let sim = Sim::new(&l, CC_BASE).unwrap();
    let mut s = sim.initial_state();
    sim.apply(&mut s, &Action::EndRound).unwrap();
    assert!(matches!(s.status, Status::Lost { .. }));

    let l = expedition(&["F...F"], "");
    let sim = Sim::new(&l, CC_BASE).unwrap();
    let mut s = sim.initial_state();
    for _ in 0..3 {
        sim.apply(&mut s, &Action::EndRound).unwrap();
    }
    assert!(matches!(s.status, Status::Lost { .. }));
    assert_eq!(sim.apply(&mut s, &Action::EndRound), Err(RuleError::LevelOver));
}

#[test]
fn species_tokens_and_puzzle_checks() {
    let l = level(
        r#"{"id":"p","number":3,"mode":"puzzle","kind":"niche_match","title":"t","area":"a","instruction":"i","intro":"i",
        "map":["F~g^","FFgg"],"slots":[[0,0],[2,1]],"tokens":["malabar_gliding_frog","nilgiri_tahr"],
        "rounds":1,"capacity":1,
        "objectives":[{"kind":"habitat","species":"malabar_gliding_frog","label":"f"},{"kind":"habitat","species":"nilgiri_tahr","label":"t"}],
        "challenge":{"text":"t","kind":"first_try"}}"#,
    );
    let sim = Sim::new(&l, CC_BASE).unwrap();
    let mut s = sim.initial_state();
    assert_eq!(
        sim.apply(&mut s, &Action::PlaceSpecies { species: Species::GreatHornbill, at: Pos::new(0, 0) }),
        Err(RuleError::NoSuchToken)
    );
    assert_eq!(
        sim.apply(&mut s, &Action::PlaceSpecies { species: Species::NilgiriTahr, at: Pos::new(1, 1) }),
        Err(RuleError::NotASlot)
    );
    // Wrong placement: the check fails, the board stays, and the attempt is counted.
    sim.apply(&mut s, &Action::PlaceSpecies { species: Species::NilgiriTahr, at: Pos::new(0, 0) }).unwrap();
    sim.apply(&mut s, &Action::PlaceSpecies { species: Species::MalabarGlidingFrog, at: Pos::new(2, 1) }).unwrap();
    let out = sim.apply(&mut s, &Action::EndRound).unwrap();
    assert!(matches!(out, Outcome::CheckFailed { .. }));
    assert_eq!(s.attempts, 1);
    // Moving tokens fixes it.
    sim.apply(&mut s, &Action::RemoveSpecies { species: Species::NilgiriTahr }).unwrap();
    sim.apply(&mut s, &Action::PlaceSpecies { species: Species::MalabarGlidingFrog, at: Pos::new(0, 0) }).unwrap();
    sim.apply(&mut s, &Action::PlaceSpecies { species: Species::NilgiriTahr, at: Pos::new(2, 1) }).unwrap();
    sim.apply(&mut s, &Action::EndRound).unwrap();
    assert_eq!(s.status, Status::Won { round: 1 });
    assert!(!sim.leaf_marks(&s).challenge, "first-try challenge missed after a failed check");
}

#[test]
fn puzzle_take_back_refunds() {
    let l = level(
        r#"{"id":"p","number":1,"mode":"puzzle","kind":"connection","title":"t","area":"a","instruction":"i","intro":"i",
        "map":["F..F"],"rounds":1,"capacity":3,
        "objectives":[{"kind":"connect","species":"malabar_giant_squirrel","from":[0,0],"to":[3,0],"label":"r"}],
        "challenge":{"text":"t","kind":"first_try"}}"#,
    );
    let sim = Sim::new(&l, Ruleset::BASE).unwrap();
    let mut s = sim.initial_state();
    sim.apply(&mut s, &Action::Plant { at: Pos::new(1, 0) }).unwrap();
    assert_eq!(sim.apply(&mut s, &Action::Revert { at: Pos::new(1, 0) }), Ok(Outcome::Reverted { refund: 1 }));
    assert_eq!(s.capacity_left, 3);
    assert_eq!(s.board, sim.initial_board().clone());
    assert_eq!(sim.apply(&mut s, &Action::Revert { at: Pos::new(1, 0) }), Err(RuleError::NothingToRevert));
}

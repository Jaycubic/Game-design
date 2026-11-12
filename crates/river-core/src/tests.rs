use super::*;

fn idle(run: &Run) -> Vec<(u8, Input)> {
    run.boats.iter().map(|b| (b.seat, Input::default())).collect()
}

/// Run until a phase predicate holds (or the run ends), feeding idle input.
fn until(run: &mut Run, mut f: impl FnMut(&Run) -> bool) {
    for _ in 0..(TICK_HZ as usize * 400) {
        if f(run) || run.phase == Phase::Finished {
            return;
        }
        let i = idle(run);
        run.step(&i);
    }
    panic!("condition never reached");
}

#[test]
fn deterministic_for_same_seed_and_inputs() {
    let mut a = Run::new(42, &[0, 1, 2, 3]);
    let mut b = Run::new(42, &[0, 1, 2, 3]);
    assert_eq!(a.objs, b.objs);
    for k in 0..2000 {
        let inp: Vec<(u8, Input)> = (0..4)
            .map(|s| {
                (s, Input { steer: if k % 37 == s as usize { 1 } else { 0 }, hop: k % 50 == 0, ..Default::default() })
            })
            .collect();
        a.step(&inp);
        b.step(&inp);
    }
    assert_eq!(serde_json::to_string(&a).unwrap(), serde_json::to_string(&b).unwrap());
    assert_ne!(Run::new(43, &[0, 1]).objs, a.objs.clone(), "different seeds give different rivers");
}

#[test]
fn crews_two_players_coop_four_players_two_crews() {
    assert_eq!(Run::new(1, &[0, 1]).crews.len(), 1);
    let r = Run::new(1, &[0, 1, 2, 3]);
    assert_eq!(r.crews.len(), 2);
    assert_eq!(r.boats.iter().map(|b| b.crew).collect::<Vec<_>>(), vec![0, 1, 0, 1]);
}

#[test]
fn boats_move_forward_on_their_own_after_countdown() {
    let mut r = Run::new(5, &[0, 1]);
    let z0 = r.boats[0].z;
    until(&mut r, |r| r.phase == Phase::Running);
    for _ in 0..30 {
        let i = idle(&r);
        r.step(&i);
    }
    assert!(r.boats[0].z > z0 + 10.0, "about one second at base speed");
}

#[test]
fn plastic_is_collected_on_contact_and_fills_boost() {
    let mut r = Run::new(7, &[0, 1]);
    r.objs.clear();
    r.phase = Phase::Running;
    r.boats[0].lane = 0;
    let z = r.boats[0].z + 3.0;
    r.spawn(Kind::Plastic, 0, z);
    for _ in 0..15 {
        let i = idle(&r);
        r.step(&i);
    }
    assert_eq!(r.crews[0].plastic, 1);
    assert!(r.boats[0].boost > 0.0);
    assert!(r.drain_events().iter().any(|e| matches!(e, Event::Collected { seat: 0, .. })));
}

#[test]
fn logs_capsize_unless_hopped() {
    let mut r = Run::new(9, &[0, 1]);
    r.objs.clear();
    r.phase = Phase::Running;
    let z = r.boats[0].z + 4.0;
    r.spawn(Kind::Log, r.boats[0].lane, z);
    for _ in 0..20 {
        let i = idle(&r);
        r.step(&i);
    }
    assert!(r.boats[0].capsize > 0.0);

    let mut r = Run::new(9, &[0, 1]);
    r.objs.clear();
    r.phase = Phase::Running;
    let z = r.boats[0].z + 4.0;
    r.spawn(Kind::Log, r.boats[0].lane, z);
    for k in 0..20 {
        let mut i = idle(&r);
        if k == 3 {
            i[0].1.hop = true;
        }
        r.step(&i);
    }
    assert_eq!(r.boats[0].capsize, 0.0, "hopping clears the log");
}

#[test]
fn weed_slows_unless_cut_and_cutting_greens_a_bank() {
    let mut r = Run::new(11, &[0, 1]);
    r.objs.clear();
    r.phase = Phase::Running;
    let z = r.boats[0].z + 3.0;
    r.spawn(Kind::Weed, r.boats[0].lane, z);
    let mut i = idle(&r);
    i[0].1.action = true;
    r.step(&i);
    assert_eq!(r.crews[0].banks, 1);
    assert!(r.objs.iter().all(|o| !o.alive || o.kind != Kind::Weed));

    let mut r = Run::new(11, &[0, 1]);
    r.objs.clear();
    r.phase = Phase::Running;
    let z = r.boats[0].z + 3.0;
    r.spawn(Kind::Weed, r.boats[0].lane, z);
    for _ in 0..12 {
        let i = idle(&r);
        r.step(&i);
    }
    assert!(r.boats[0].slow > 0.0);
}

#[test]
fn gate_code_is_clear_only_to_crewmates_and_plugging_heals() {
    let mut r = Run::new(13, &[0, 1, 2, 3]);
    r.objs.clear();
    r.phase = Phase::Running;
    r.boats[0].lane = 1;
    let z = r.boats[0].z + 2.0;
    let pipe = r.spawn(Kind::Pipe, 1, z);
    let mut i = idle(&r);
    i[0].1.action = true;
    r.step(&i);
    let code = r.boats[0].plug.as_ref().expect("plugging").code;
    // Viewer 0 (plugger) sees scrambled; viewer 2 (crewmate) sees clear; viewer 1 (rival) sees nothing.
    let s0 = r.snapshot_for(0);
    let s2 = r.snapshot_for(2);
    let s1 = r.snapshot_for(1);
    assert!(!s0.plugs[0].1.clear);
    assert_eq!(s2.plugs[0].1.code, code);
    assert!(s2.plugs[0].1.clear);
    assert!(s1.plugs.is_empty());
    assert_eq!(r.boats[0].speed(), 0.0, "the boat stops while plugging");
    let before = r.health;
    for g in code {
        let mut i = idle(&r);
        i[0].1.glyph = Some(g);
        r.step(&i);
    }
    assert!(r.objs.iter().any(|o| o.id == pipe && o.plugged));
    assert_eq!(r.crews[0].pipes, 1);
    assert!(r.health > before + 5.0);
}

#[test]
fn wrong_code_fails_and_glyphs_are_pings_otherwise() {
    let mut r = Run::new(17, &[0, 1]);
    r.objs.clear();
    r.phase = Phase::Running;
    r.boats[0].lane = -1;
    let z = r.boats[0].z;
    r.spawn(Kind::Pipe, -1, z);
    let mut i = idle(&r);
    i[0].1.action = true;
    r.step(&i);
    let code = r.boats[0].plug.as_ref().unwrap().code;
    let wrong = (code[0] + 1) % GLYPHS;
    for g in [wrong, code[1], code[2]] {
        let mut i = idle(&r);
        i[0].1.glyph = Some(g);
        r.step(&i);
    }
    assert_eq!(r.crews[0].pipes, 0);
    assert!(r.drain_events().iter().any(|e| matches!(e, Event::PlugFailed { .. })));
    let mut i = idle(&r);
    i[1].1.glyph = Some(3);
    r.step(&i);
    assert!(r.drain_events().contains(&Event::Ping { seat: 1, glyph: 3 }));
}

#[test]
fn unplugged_pipes_drain_shared_health_and_choking_halves_scores() {
    let mut r = Run::new(19, &[0, 1, 2]);
    r.objs.clear();
    r.phase = Phase::Running;
    for k in 0..20 {
        r.spawn(Kind::Pipe, if k % 2 == 0 { 1 } else { -1 }, -50.0 - k as f32);
    }
    r.crews[0].plastic = 40;
    let before = r.health;
    for _ in 0..30 {
        let i = idle(&r);
        r.step(&i);
    }
    assert!(r.health < before);
    r.health = 0.5;
    r.heal(-1.0);
    assert!(r.choked);
    r.t = RUN_SECONDS;
    r.finish();
    assert_eq!(r.results.as_ref().unwrap().scores[0].1, 20, "40 plastic halved");
}

#[test]
fn exactly_two_rule_changes_per_run_chosen_by_players() {
    let mut r = Run::new(23, &[0, 1, 2, 3]);
    until(&mut r, |r| matches!(r.phase, Phase::Choosing { change: ChangeKind::Twist, .. }));
    assert!((r.t - TWIST_AT).abs() < 0.1, "twist opens at 35 %: {}", r.t);
    // Votes: 2 for Dry Season, 1 each for others → Dry Season wins outright.
    r.choose(0, 1).unwrap();
    r.choose(1, 1).unwrap();
    r.choose(2, 0).unwrap();
    r.choose(3, 2).unwrap();
    assert_eq!(r.twist, Some(Twist::DrySeason));
    assert!(matches!(r.phase, Phase::Reveal { spin: false, .. }));
    assert_eq!(r.lanes(), &[-1, 0]);
    assert!(r.boats.iter().all(|b| b.lane != 1));
    until(&mut r, |r| matches!(r.phase, Phase::Choosing { change: ChangeKind::Finale, .. }));
    let Phase::Choosing { pickers: Some(c), .. } = r.phase.clone() else { panic!() };
    let outsider = r.boats.iter().find(|b| b.crew != c).unwrap().seat;
    assert!(r.choose(outsider, 0).is_err(), "only the trailing crew picks");
    let picker = r.boats.iter().find(|b| b.crew == c).unwrap().seat;
    r.choose(picker, 1).unwrap();
    assert_eq!(r.finale, Some(Finale::CleanSweep));
    until(&mut r, |r| r.phase == Phase::Finished);
    assert_eq!(r.changes, 2);
    assert!(r.results.is_some());
    assert!(r.t >= RUN_SECONDS - 0.1);
}

#[test]
fn tied_or_missing_votes_resolve_with_a_visible_spin() {
    let mut r = Run::new(29, &[0, 1]);
    until(&mut r, |r| matches!(r.phase, Phase::Choosing { change: ChangeKind::Twist, .. }));
    r.choose(0, 0).unwrap();
    r.choose(1, 2).unwrap();
    assert!(matches!(r.phase, Phase::Reveal { spin: true, .. }));
    assert!(matches!(r.twist, Some(Twist::MonsoonSurge) | Some(Twist::VolunteerDay)));

    let mut r = Run::new(31, &[0, 1]);
    until(&mut r, |r| matches!(r.phase, Phase::Reveal { change: ChangeKind::Twist, .. }));
    assert!(r.twist.is_some(), "no votes still changes the rules (spin among all)");
}

#[test]
fn snapshot_stays_small() {
    let r = Run::new(37, &[0, 1, 2, 3]);
    let s = serde_json::to_string(&r.snapshot_for(0)).unwrap();
    assert!(s.len() < 12_000, "snapshot {} bytes", s.len());
}

/// Balance smoke test: simple bots that steer toward plastic, hop logs, cut weed
/// and plug pipes should keep the river alive and finish with positive scores.
#[test]
fn bots_can_finish_a_healthy_run() {
    let mut r = Run::new(41, &[0, 1, 2, 3]);
    for _ in 0..(TICK_HZ as usize * 400) {
        if r.phase == Phase::Finished {
            break;
        }
        if let Phase::Choosing { pickers, .. } = r.phase.clone() {
            for b in r.boats.clone() {
                if pickers.is_none_or(|c| c == b.crew) {
                    let _ = r.choose(b.seat, 0);
                }
            }
        }
        let inputs: Vec<(u8, Input)> = r.boats.iter().map(|b| (b.seat, bot(&r, b))).collect();
        r.step(&inputs);
    }
    let res = r.results.clone().expect("finished");
    assert!(res.scores.iter().all(|(_, s)| *s > 30), "{res:?}");
    assert!(!res.choked, "{res:?}");
}

fn bot(r: &Run, b: &Boat) -> Input {
    let mut i = Input::default();
    if let Some(p) = &b.plug {
        i.glyph = Some(p.code[p.entered.len()]);
        return i;
    }
    let ahead = |kind: Kind, lane: i8, d: f32| {
        r.objs.iter().any(|o| o.alive && o.kind == kind && o.lane == lane && o.z > b.z && o.z < b.z + d)
    };
    if ahead(Kind::Log, b.lane, 2.5)
        || r.objs
            .iter()
            .any(|o| o.alive && o.kind == Kind::Wave && wave_covers(o.lane, b.lane) && o.z > b.z && o.z < b.z + 2.5)
    {
        i.hop = true;
    }
    if ahead(Kind::Weed, b.lane, 4.0) {
        i.action = true;
    }
    if r.objs
        .iter()
        .any(|o| o.kind == Kind::Pipe && !o.plugged && o.lane == b.lane && b.z >= o.z - 6.0 && b.z <= o.z + 2.0)
    {
        i.action = true;
    }
    let pipe_side =
        r.objs.iter().find(|o| o.kind == Kind::Pipe && !o.plugged && o.z > b.z && o.z < b.z + 25.0).map(|o| o.lane);
    let target = pipe_side.or_else(|| {
        r.lanes().iter().copied().max_by_key(|l| {
            r.objs
                .iter()
                .filter(|o| o.alive && o.kind == Kind::Plastic && o.lane == *l && o.z > b.z && o.z < b.z + 20.0)
                .count()
        })
    });
    if let Some(t) = target {
        i.steer = (t - b.lane).signum();
    }
    i
}

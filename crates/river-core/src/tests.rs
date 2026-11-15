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
    r.gen_z = f32::MAX; // no generated rows in this test
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
    r.gen_z = f32::MAX; // no generated rows in this test
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
    r.gen_z = f32::MAX; // no generated rows in this test
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
fn capsize_is_short_spills_plastic_and_protects_briefly() {
    let mut r = Run::new(9, &[0]);
    r.objs.clear();
    r.gen_z = f32::MAX; // no generated rows in this test
    r.phase = Phase::Running;
    r.boats[0].plastic = 5;
    r.crews[0].plastic = 5;
    let (lane, z) = (r.boats[0].lane, r.boats[0].z);
    r.spawn(Kind::Log, lane, z + 1.0);
    // a second log right behind the first would hit again without protection
    r.spawn(Kind::Log, lane, z + 12.0);
    let i = idle(&r);
    r.step(&i);
    assert!((r.boats[0].capsize - CAPSIZE_TIME).abs() < 0.05);
    assert_eq!(r.boats[0].plastic, 2, "three pieces spill");
    assert_eq!(r.crews[0].plastic, 2);
    assert!(r.drain_events().iter().any(|e| matches!(e, Event::Capsized { seat: 0, dropped: 3 })));
    let spilled = r.objs.iter().filter(|o| o.kind == Kind::Plastic && o.alive && o.z > r.boats[0].z).count();
    assert_eq!(spilled, 3, "spilled plastic floats just ahead");
    // Stopped for 1.2 s, then protected for 1 s while it reaches the second log.
    let mut capsizes = 0;
    for _ in 0..(TICK_HZ as usize * 2) {
        let i = idle(&r);
        r.step(&i);
        capsizes += r.drain_events().iter().filter(|e| matches!(e, Event::Capsized { .. })).count();
    }
    assert_eq!(capsizes, 0, "no second capsize during protection");
    assert!(r.boats[0].z > z + 5.0, "moving again after 1.2 s");
}

#[test]
fn capsize_with_no_plastic_spills_nothing() {
    let mut r = Run::new(9, &[0]);
    r.objs.clear();
    r.gen_z = f32::MAX; // no generated rows in this test
    r.phase = Phase::Running;
    let (lane, z) = (r.boats[0].lane, r.boats[0].z);
    r.spawn(Kind::Log, lane, z + 1.0);
    let i = idle(&r);
    r.step(&i);
    assert!(r.boats[0].capsize > 0.0);
    assert_eq!(r.objs.iter().filter(|o| o.kind == Kind::Plastic).count(), 0);
}

#[test]
fn weed_slows_unless_cut_and_cutting_greens_a_bank() {
    let mut r = Run::new(11, &[0, 1]);
    r.objs.clear();
    r.gen_z = f32::MAX; // no generated rows in this test
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
    r.gen_z = f32::MAX; // no generated rows in this test
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
    r.gen_z = f32::MAX; // no generated rows in this test
    r.phase = Phase::Running;
    r.boats[0].lane = 5; // right edge of a 6-lane river
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
    r.gen_z = f32::MAX; // no generated rows in this test
    r.phase = Phase::Running;
    r.boats[0].lane = 0; // left edge
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
    r.gen_z = f32::MAX; // no generated rows in this test
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
    assert_eq!(r.lanes(), vec![0, 1, 2, 3], "Dry Season closes a third of 6 lanes");
    assert!(r.boats.iter().all(|b| b.lane <= 3));
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
            .any(|o| o.alive && o.kind == Kind::Wave && wave_covers(o, b.lane) && o.z > b.z && o.z < b.z + 2.5)
    {
        i.hop = true;
    }
    if ahead(Kind::Weed, b.lane, 4.0) {
        i.action = true;
    }
    if r.objs
        .iter()
        .any(|o| o.kind == Kind::Pipe && !o.plugged && r.pipe_lane(o.lane) == b.lane && b.z >= o.z - 6.0 && b.z <= o.z + 2.0)
    {
        i.action = true;
    }
    let pipe_side =
        r.objs.iter().find(|o| o.kind == Kind::Pipe && !o.plugged && o.z > b.z && o.z < b.z + 25.0).map(|o| r.pipe_lane(o.lane));
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

#[test]
fn tutorial_is_short_scripted_and_has_no_rule_changes() {
    let mut r = Run::tutorial(3, &[0]);
    assert!(r.objs.iter().any(|o| o.kind == Kind::Pipe));
    assert!(r.objs.iter().filter(|o| o.kind == Kind::Log).count() >= 9);
    until(&mut r, |r| r.phase == Phase::Finished);
    assert_eq!(r.changes, 0);
    assert!((r.t - TUTORIAL_SECONDS).abs() < 0.1);
    assert!(r.results.is_some());
}

#[test]
fn better_boats_are_faster_and_a_wide_net_reaches_the_next_lane() {
    assert_eq!(boat_stats("boat_sun"), Stats::default());
    let mut r = Run::new(9, &[0, 1]);
    r.objs.clear();
    r.gen_z = f32::MAX; // no generated rows in this test
    r.phase = Phase::Running;
    r.equip(1, boat_stats("boat_night"));
    r.equip(0, boat_stats("boat_lotus"));
    r.boats[0].lane = 1;
    r.boats[1].lane = 2;
    let (z0, z1) = (r.boats[0].z, r.boats[1].z);
    // Plastic one lane over from the wide-net boat.
    r.spawn(Kind::Plastic, 0, r.boats[0].z + 3.0);
    for _ in 0..30 {
        let i = idle(&r);
        r.step(&i);
    }
    assert!(r.boats[1].z - z1 > r.boats[0].z - z0, "Night Heron (speed 4) outruns Lotus (speed 3)");
    assert_eq!(r.boats[0].plastic, 1, "level-5 net grabs plastic in the next lane");
    // The upgrade is modest: at most +12 % speed.
    assert!(boat_stats("boat_night").speed_mult() <= 1.12);
}

#[test]
fn the_river_widens_with_more_boats_and_boats_start_spread_out() {
    assert_eq!(Run::new(1, &[0]).lane_count, 3);
    assert_eq!(Run::new(1, &[0, 1]).lane_count, 4);
    assert_eq!(Run::new(1, &[0, 1, 2]).lane_count, 6);
    let r = Run::new(1, &[0, 1, 2, 3]);
    assert_eq!(r.lanes(), vec![0, 1, 2, 3, 4, 5]);
    let mut starts: Vec<i8> = r.boats.iter().map(|b| b.lane).collect();
    starts.sort();
    starts.dedup();
    assert_eq!(starts.len(), 4, "four boats start in four different lanes: {starts:?}");
    assert_eq!(Run::new(1, &[0]).boats[0].lane, 1, "a lone boat starts in the middle");
    // Every object sits in a real lane (pipes use -1/+1 for the bank side).
    for o in &r.objs {
        if o.kind == Kind::Pipe {
            assert!(o.lane == -1 || o.lane == 1);
        } else {
            assert!((0..6).contains(&o.lane), "{o:?}");
        }
    }
}

#[test]
fn wider_rivers_carry_more_plastic_than_obstacles() {
    let count = |seats: &[u8], kind: Kind| -> f32 {
        (0..8u64)
            .map(|seed| {
                let mut r = Run::new(seed, seats);
                r.generate_to(COURSE_LEN);
                r.objs.iter().filter(|o| o.kind == kind).count() as f32
            })
            .sum()
    };
    let (p3, p6) = (count(&[0], Kind::Plastic), count(&[0, 1, 2, 3], Kind::Plastic));
    let (l3, l6) = (count(&[0], Kind::Log), count(&[0, 1, 2, 3], Kind::Log));
    let pr = p6 / p3;
    assert!(pr > 1.6 && pr < 2.0, "plastic x{pr}");
    assert!(l6 / l3 > 1.25, "obstacles grow too: x{}", l6 / l3);
    assert!(pr > l6 / l3 * 0.95 || pr > 1.6, "plastic grows at least as fast");
}

#[test]
fn waves_on_a_six_lane_river_leave_two_gaps() {
    let mut r = Run::new(31, &[0, 1, 2, 3]);
    r.objs.clear();
    r.gen_z = f32::MAX; // no generated rows in this test
    r.phase = Phase::Running;
    r.next_wave = 0.0;
    let i = idle(&r);
    r.step(&i);
    let w = r.objs.iter().find(|o| o.kind == Kind::Wave).expect("wave");
    let g2 = w.gap2.expect("second gap");
    assert!((g2 - w.lane).abs() > 1);
    let open = (0..6).filter(|l| !wave_covers(w, *l)).count();
    assert_eq!(open, 2);
    // three lanes: one gap only
    let mut r = Run::new(31, &[0]);
    r.objs.clear();
    r.gen_z = f32::MAX; // no generated rows in this test
    r.phase = Phase::Running;
    r.next_wave = 0.0;
    let i = idle(&r);
    r.step(&i);
    assert!(r.objs.iter().find(|o| o.kind == Kind::Wave).unwrap().gap2.is_none());
}

#[test]
fn pipes_are_plugged_from_the_edge_lane_only() {
    let mut r = Run::new(5, &[0, 1]);
    r.objs.clear();
    r.gen_z = f32::MAX; // no generated rows in this test
    r.phase = Phase::Running;
    let z = r.boats[0].z;
    r.spawn(Kind::Pipe, 1, z);
    r.boats[0].lane = 2; // one lane short of the right edge (3)
    let mut i = idle(&r);
    i[0].1.action = true;
    r.step(&i);
    assert!(r.boats[0].plug.is_none());
    r.boats[0].lane = 3;
    let mut i = idle(&r);
    i[0].1.action = true;
    r.step(&i);
    assert!(r.boats[0].plug.is_some());
}

fn full_course(seed: u64, d: f32) -> Run {
    let mut r = Run::with_difficulty(seed, &[0], d);
    r.generate_to(COURSE_LEN);
    r
}

#[test]
fn harder_rivers_have_more_obstacles_and_less_plastic() {
    let count = |d: f32, kind: Kind| -> usize {
        (0..6u64).map(|seed| full_course(seed, d).objs.iter().filter(|o| o.kind == kind).count()).sum()
    };
    let (easy_logs, hard_logs) = (count(0.8, Kind::Log), count(1.2, Kind::Log));
    let (easy_plastic, hard_plastic) = (count(0.8, Kind::Plastic), count(1.2, Kind::Plastic));
    assert!(hard_logs as f32 > easy_logs as f32 * 1.3, "logs {easy_logs} -> {hard_logs}");
    assert!((hard_plastic as f32) < easy_plastic as f32 * 0.8, "plastic {easy_plastic} -> {hard_plastic}");
    // Same seed and difficulty: same river.
    assert_eq!(full_course(4, 1.1).objs, full_course(4, 1.1).objs);
    // Out-of-range difficulty is clamped.
    assert_eq!(Run::with_difficulty(1, &[0], 3.0).difficulty, D_MAX);
}

#[test]
fn the_course_is_built_ahead_of_the_boats() {
    let mut r = Run::new(8, &[0]);
    let first = r.objs.iter().map(|o| o.z).fold(0.0, f32::max);
    assert!(first < GEN_AHEAD + 40.0, "only the first stretch exists at the start: {first}");
    until(&mut r, |r| r.leader_z() > 600.0);
    assert!(r.gen_z >= r.leader_z() + GEN_AHEAD - 1.0);
}

#[test]
fn director_eases_off_for_a_struggling_solo_player_but_only_so_far() {
    // An idle boat capsizes on every log: the river should get easier, by at most D_SWING.
    let mut r = Run::with_difficulty(12, &[0], 1.0);
    assert!(r.adaptive);
    until(&mut r, |r| r.t > 60.0);
    assert!(r.difficulty < 1.0, "eased: {}", r.difficulty);
    assert!(r.difficulty >= 1.0 - D_SWING - 1e-4);
}

#[test]
fn director_pushes_back_on_a_player_in_full_control() {
    // A bot that nets most plastic and never capsizes should see the river get harder.
    let mut r = Run::with_difficulty(14, &[0], 1.0);
    r.phase = Phase::Running;
    for _ in 0..(TICK_HZ as usize * 55) {
        if !matches!(r.phase, Phase::Running) {
            break;
        }
        let b = r.boats[0].clone();
        let i = vec![(0, bot(&r, &b))];
        r.step(&i);
    }
    assert!(r.difficulty > 1.0, "pushed back: {} (collected {}, missed {}, capsizes {})", r.difficulty, r.boats[0].collected, r.missed, r.boats[0].capsizes);
    assert!(r.difficulty <= 1.0 + D_SWING + 1e-4);
    assert!(r.boats[0].collected > 0);
}

#[test]
fn director_never_adapts_a_crew_versus_crew_run() {
    let mut r = Run::with_difficulty(16, &[0, 1, 2, 3], 1.1);
    assert!(!r.adaptive);
    until(&mut r, |r| r.t > 60.0);
    assert_eq!(r.difficulty, 1.1);
}

#[test]
fn performance_counters_track_each_boat() {
    let mut r = Run::new(9, &[0]);
    r.objs.clear();
    r.gen_z = f32::MAX;
    r.phase = Phase::Running;
    let (lane, z) = (r.boats[0].lane, r.boats[0].z);
    r.spawn(Kind::Plastic, lane, z + 2.0);
    r.spawn(Kind::Log, lane, z + 10.0);
    r.spawn(Kind::Plastic, (lane + 1) % 3, z + 3.0);
    for _ in 0..40 {
        let i = idle(&r);
        r.step(&i);
    }
    assert_eq!(r.boats[0].collected, 1);
    assert_eq!(r.boats[0].capsizes, 1);
    assert!(r.missed >= 1);
}

fn bare_on(river: River, seats: &[u8]) -> Run {
    let mut r = Run::create(3, seats, 1.0, river);
    r.objs.clear();
    r.gen_z = f32::MAX;
    r.phase = Phase::Running;
    r
}

#[test]
fn each_river_brings_its_own_mechanic() {
    let kinds = |river: River| -> Vec<Kind> {
        let mut r = Run::create(21, &[0, 1], 1.0, river);
        r.generate_to(COURSE_LEN);
        r.objs.iter().map(|o| o.kind).collect()
    };
    let (canal, mangrove, gorge) = (kinds(River::Canal), kinds(River::Mangrove), kinds(River::Gorge));
    assert!(!canal.iter().any(|k| matches!(k, Kind::Net | Kind::Current | Kind::Rock)));
    assert!(mangrove.contains(&Kind::Net));
    assert!(gorge.contains(&Kind::Current) && gorge.contains(&Kind::Rock));
    for river in RIVERS {
        assert_eq!(River::from_key(river.key()), Some(river));
    }
    assert_eq!(River::from_key("ocean"), None);
}

#[test]
fn a_ghost_net_needs_two_cuts_by_anyone_and_frees_plastic() {
    let mut r = bare_on(River::Mangrove, &[0, 1]);
    r.boats[0].lane = 1;
    r.boats[1].lane = 2;
    let z = r.boats[1].z + 4.0; // within reach of both boats
    let net = r.spawn(Kind::Net, 1, z); // covers lanes 1 and 2
    // One press from each crewmate.
    let mut i = idle(&r);
    i[0].1.action = true;
    r.step(&i);
    assert!(r.drain_events().iter().any(|e| matches!(e, Event::NetHit { left: 1, .. })));
    let mut i = idle(&r);
    i[1].1.action = true;
    r.step(&i);
    let n = r.objs.iter().find(|o| o.id == net).unwrap().clone();
    assert!(!n.alive, "net {n:?}, boats {:?}, events {:?}", r.boats.iter().map(|b| (b.lane, b.z)).collect::<Vec<_>>(), r.drain_events());
    assert_eq!(r.crews[0].nets, 1);
    let freed = r.objs.iter().filter(|o| o.kind == Kind::Plastic && o.alive).count() as u32;
    assert_eq!(freed, NET_PLASTIC);
}

#[test]
fn an_uncut_net_slows_like_weed() {
    let mut r = bare_on(River::Mangrove, &[0]);
    let (lane, z) = (r.boats[0].lane, r.boats[0].z);
    r.spawn(Kind::Net, lane - 1, z + 3.0); // its second lane is ours
    for _ in 0..12 {
        let i = idle(&r);
        r.step(&i);
    }
    assert!(r.boats[0].slow > 0.0);
}

#[test]
fn currents_push_once_unless_you_hop() {
    let mut r = bare_on(River::Gorge, &[0]);
    let (lane, z) = (r.boats[0].lane, r.boats[0].z);
    let id = r.spawn(Kind::Current, lane, z + 2.0);
    r.objs.iter_mut().find(|o| o.id == id).unwrap().dir = 1;
    for _ in 0..15 {
        let i = idle(&r);
        r.step(&i);
    }
    assert_eq!(r.boats[0].lane, lane + 1, "pushed exactly one lane");

    let mut r = bare_on(River::Gorge, &[0]);
    let id = r.spawn(Kind::Current, lane, z + 4.0);
    r.objs.iter_mut().find(|o| o.id == id).unwrap().dir = -1;
    for k in 0..15 {
        let mut i = idle(&r);
        i[0].1.hop = k == 2;
        r.step(&i);
    }
    assert_eq!(r.boats[0].lane, lane, "hopping rides over the current");
}

#[test]
fn rocks_cannot_be_hopped() {
    let mut r = bare_on(River::Gorge, &[0]);
    let (lane, z) = (r.boats[0].lane, r.boats[0].z);
    r.spawn(Kind::Rock, lane, z + 4.0);
    for k in 0..20 {
        let mut i = idle(&r);
        i[0].1.hop = k == 3;
        r.step(&i);
    }
    assert_eq!(r.boats[0].capsizes, 1);
}

#[test]
fn stars_for_clean_river_score_and_the_river_goal() {
    // Gorge: a clean, capsize-free run with a big score earns all three.
    let mut r = bare_on(River::Gorge, &[0]);
    r.crews[0].plastic = 200;
    r.health = 80.0;
    r.t = r.duration();
    let i = idle(&r);
    r.step(&i);
    assert_eq!(r.phase, Phase::Finished);
    assert_eq!(r.stars(0), [true, true, true]);
    // The same run with a capsize loses only the Gorge goal.
    r.boats[0].capsizes = 1;
    assert_eq!(r.stars(0), [true, true, false]);
    assert_eq!(r.stars(9), [false; 3], "unknown seat");

    // Canal: an open pipe the boats passed blocks the third star.
    let mut r = bare_on(River::Canal, &[0]);
    let z = r.boats[0].z;
    r.spawn(Kind::Pipe, 1, z - 5.0);
    r.crews[0].pipes = 1;
    r.t = r.duration();
    let i = idle(&r);
    r.step(&i);
    assert!(!r.stars(0)[2]);
    for o in r.objs.iter_mut().filter(|o| o.kind == Kind::Pipe) {
        o.plugged = true;
    }
    assert!(r.stars(0)[2]);

    // The tutorial never earns stars.
    let mut t = Run::tutorial(1, &[0]);
    until(&mut t, |r| r.phase == Phase::Finished);
    assert_eq!(t.stars(0), [false; 3]);
}

#[test]
fn solo_gorge_has_currents_near_the_start() {
    let mut r = Run::create(77, &[0], 1.0, River::Gorge);
    r.generate_to(1200.0);
    let n = r.objs.iter().filter(|o| o.kind == Kind::Current).count();
    assert!(n >= 3, "currents in the first 1200 m: {n}");
    let total: usize = (0..20u64)
        .map(|seed| {
            let mut r = Run::create(seed, &[0], 1.0, River::Gorge);
            r.generate_to(1200.0);
            r.objs.iter().filter(|o| o.kind == Kind::Current).count()
        })
        .sum();
    assert!(total >= 60, "about one row in ten has a current: {total} over 20 courses");
}

#[test]
fn endless_ends_on_the_first_capsize_and_scores_distance_plus_plastic() {
    let mut r = Run::endless(5, 0, 1.0, River::Canal);
    assert!(r.endless && !r.adaptive);
    r.objs.clear();
    r.gen_z = f32::MAX;
    r.phase = Phase::Running;
    r.boats[0].collected = 4;
    let (lane, z) = (r.boats[0].lane, r.boats[0].z);
    r.spawn(Kind::Log, lane, z + 30.0);
    until(&mut r, |r| r.phase == Phase::Finished);
    let res = r.results.clone().expect("results");
    let dist = r.distance() as i32;
    assert!(dist > 25);
    assert_eq!(res.scores[0].1, dist + 4 * ENDLESS_PLASTIC_POINTS);
    assert_eq!(r.stars(0), [false; 3], "Endless earns no stars");
}

#[test]
fn endless_speeds_up_and_still_has_exactly_two_player_chosen_changes() {
    let mut r = Run::endless(6, 0, 1.0, River::Canal);
    r.objs.clear();
    r.gen_z = f32::MAX; // open water: nothing to capsize on
    r.next_wave = f32::INFINITY;
    r.phase = Phase::Running;
    let mut changes = Vec::new();
    for _ in 0..(TICK_HZ as usize * 200) {
        if let Phase::Choosing { change, .. } = r.phase.clone() {
            changes.push((change, r.t));
            r.choose(0, 0).unwrap();
        }
        let i = idle(&r);
        r.step(&i);
        r.next_wave = f32::INFINITY;
    }
    assert_eq!(changes.len(), 2);
    assert!((changes[0].1 - ENDLESS_TWIST_AT).abs() < 0.2, "{changes:?}");
    assert!((changes[1].1 - ENDLESS_FINALE_AT).abs() < 0.2, "{changes:?}");
    assert_eq!(r.changes, 2);
    assert!(r.pace() > 1.2, "about x1.02 per 15 s: {}", r.pace());
    assert!(r.phase != Phase::Finished, "no time limit");
    // The course keeps going past the normal end.
    let mut e = Run::endless(6, 0, 1.0, River::Canal);
    e.generate_to(COURSE_LEN + 500.0);
    assert!(e.objs.iter().any(|o| o.z > COURSE_LEN));
}

//! `classify` — print the solver's classification of every Forest Realm level
//! under every Living Charter combination it can be played with.
//!
//! Usage: `cargo run --release -p natures-thread-core --bin classify [-- --json] [-- --level N]`

use nt_core::campaign::CampaignState;
use nt_core::content::forest_realm;
use nt_core::solver::{analyze, Limits};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let json = args.iter().any(|a| a == "--json");
    let only: Option<u8> =
        args.iter().position(|a| a == "--level").and_then(|i| args.get(i + 1)).and_then(|s| s.parse().ok());
    let region = forest_realm();
    let limits = Limits::default();
    let mut all = Vec::new();
    let mut failures = 0;
    for level in &region.levels {
        if only.is_some_and(|n| n != level.number) {
            continue;
        }
        for rules in CampaignState::possible_rulesets(level.number) {
            let t = std::time::Instant::now();
            let report = analyze(level, rules, &limits, args.iter().any(|a| a == "--moves")).expect("level validates");
            let ms = t.elapsed().as_millis();
            let par_ok = level.kind == nt_core::LevelKind::Deduction || level.par_for(&rules) == report.min_cost;
            if !report.solvable || !par_ok || !report.notes.is_empty() {
                failures += 1;
            }
            if !json {
                println!(
                    "L{:02} {:<24} {:<28} solvable={:<5} min_cost={:<4} rounds={:<4} first_moves legal={} optimal={} dead_end={} explored={} {}ms{}{}{}",
                    level.number,
                    level.title,
                    report.rules,
                    report.solvable,
                    report.min_cost.map(|c| c.to_string()).unwrap_or("-".into()),
                    report.rounds.map(|c| c.to_string()).unwrap_or("-".into()),
                    report.legal_first_moves,
                    report.optimal_first_moves,
                    report.dead_end_first_moves,
                    report.explored,
                    ms,
                    report.answer.as_ref().map(|a| format!(" answer={a}")).unwrap_or_default(),
                    if par_ok { String::new() } else { format!(" PAR MISMATCH (file {:?})", level.par_for(&rules)) },
                    if report.notes.is_empty() { String::new() } else { format!(" NOTES {:?}", report.notes) },
                );
                if let (Some(d), true) = (&level.deduction, args.iter().any(|a| a == "--clues")) {
                    let board = level.board().unwrap();
                    for c in &d.candidates {
                        let row: String = d
                            .clues
                            .iter()
                            .map(|k| if k.test.holds(&board, &rules, c.at) { 'Y' } else { '.' })
                            .collect();
                        println!("      {} {:?} {row}", c.id, c.at);
                    }
                }
                if args.iter().any(|a| a == "--plans") {
                    for a in &report.plan {
                        println!("      {}", serde_json::to_string(a).unwrap());
                    }
                }
            }
            all.push((level.number, report));
        }
    }
    if json {
        println!("{}", serde_json::to_string_pretty(&all).unwrap());
    }
    if failures > 0 {
        eprintln!("{failures} level/ruleset combination(s) need attention");
        std::process::exit(1);
    }
}

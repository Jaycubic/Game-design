//! Built-in content: the Western Ghats Forest Realm region and its ten levels.
//! Files live in `content/` at the repository root and are embedded at compile time
//! so the server and WebAssembly builds always agree on the data.

use crate::level::LevelDef;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Area {
    pub id: String,
    pub name: String,
    pub description: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Amendment {
    pub number: u8,
    pub after_level: u8,
    pub title: String,
    pub question: String,
    pub option_a: AmendmentOption,
    pub option_b: AmendmentOption,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AmendmentOption {
    pub name: String,
    pub summary: String,
    pub effects: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RegionDef {
    pub id: String,
    pub ecosystem: String,
    pub name: String,
    pub tagline: String,
    pub areas: Vec<Area>,
    pub amendments: Vec<Amendment>,
    #[serde(skip_deserializing, default)]
    pub levels: Vec<LevelDef>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct JournalEntry {
    pub key: String,
    pub title: String,
    pub kind: String,
    pub text: String,
    #[serde(default)]
    pub sources: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Source {
    pub id: String,
    pub title: String,
    pub url: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Journal {
    pub sources: Vec<Source>,
    pub entries: Vec<JournalEntry>,
}

const JOURNAL: &str = include_str!("../../../content/journal.json");

/// The Field Journal: every entry a level can unlock, with sources.
pub fn journal() -> Journal {
    serde_json::from_str(JOURNAL).expect("embedded journal is valid")
}

const REGION: &str = include_str!("../../../content/regions/forest_realm.json");
const LEVELS: [&str; 10] = [
    include_str!("../../../content/levels/01_missing_connection.json"),
    include_str!("../../../content/levels/02_restore_the_forest.json"),
    include_str!("../../../content/levels/03_who_belongs_here.json"),
    include_str!("../../../content/levels/04_forest_in_balance.json"),
    include_str!("../../../content/levels/05_missing_clue.json"),
    include_str!("../../../content/levels/06_under_pressure.json"),
    include_str!("../../../content/levels/07_ecosystem_puzzle.json"),
    include_str!("../../../content/levels/08_nature_under_threat.json"),
    include_str!("../../../content/levels/09_final_blueprint.json"),
    include_str!("../../../content/levels/10_living_forest.json"),
];

/// Load and validate the Forest Realm. Panics only if the embedded content is broken,
/// which the test-suite rules out.
pub fn forest_realm() -> RegionDef {
    try_forest_realm().expect("embedded Forest Realm content is valid")
}

pub fn try_forest_realm() -> Result<RegionDef, String> {
    let mut region: RegionDef = serde_json::from_str(REGION).map_err(|e| format!("region: {e}"))?;
    for (i, src) in LEVELS.iter().enumerate() {
        let level: LevelDef = serde_json::from_str(src).map_err(|e| format!("level {}: {e}", i + 1))?;
        level.validate()?;
        if level.number as usize != i + 1 {
            return Err(format!("level file {} declares number {}", i + 1, level.number));
        }
        region.levels.push(level);
    }
    if region.amendments.len() != 2 {
        return Err("a region must define exactly two Living Charter amendments".into());
    }
    Ok(region)
}

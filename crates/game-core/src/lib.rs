//! Nature's Thread — deterministic game core.
//!
//! Pure rules with no I/O: boards, species habitat needs, connectivity under the
//! Living Charter, the Environmental Pressure Engine, a level simulator, an
//! exhaustive level solver ("classifier"), the regional campaign with its two
//! amendment votes, and room membership rules. The authoritative server and the
//! browser (via WebAssembly) both run this exact code.

pub mod board;
pub mod campaign;
pub mod connectivity;
pub mod content;
pub mod deduction;
pub mod habitat;
pub mod level;
pub mod pos;
pub mod room;
pub mod rules;
pub mod sim;
pub mod solver;
pub mod species;

pub use board::{Board, Terrain, Tile};
pub use campaign::{AmendmentChoice, CampaignPhase, CampaignState, VoteOutcome, VoteState};
pub use level::{LevelDef, LevelKind, Mode};
pub use pos::Pos;
pub use rules::{ActionKind, Connectivity, Recovery, Ruleset};
pub use sim::{Action, RuleError, Sim, State, Status};
pub use species::Species;

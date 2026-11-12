use serde::{Deserialize, Serialize};

/// Living Charter Amendment 1 — how a habitat route is judged.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Connectivity {
    /// Before the first amendment: routes run only through adjacent forest.
    Base,
    /// Option A: unbroken corridors. Unlocks canopy bridges over roads.
    Continuous,
    /// Option B: a route may hop one gap next to a planted stepping-stone grove.
    SteppingStone,
}

/// Living Charter Amendment 2 — how habitat recovers.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Recovery {
    /// Before the second amendment: only player restoration heals forest.
    Base,
    /// Option A: degraded forest with 2+ healthy forest neighbours regrows at round end.
    Succession,
    /// Option B: no passive regrowth; a restoration heals the target and adjacent
    /// degraded forest immediately, at a higher capacity cost.
    Active,
}

/// Natural Succession: healthy forest neighbours a degraded cell needs to regrow.
pub const SUCCESSION_NEIGHBOURS: usize = 3;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Ruleset {
    pub connectivity: Connectivity,
    pub recovery: Recovery,
}

/// Player interventions that spend Restoration Capacity.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActionKind {
    /// Plant native saplings on open ground → forest.
    Plant,
    /// Restore degraded forest → forest.
    Restore,
    /// Clear Lantana from a threatened tile so the round's pressure event misses it.
    Protect,
    /// Build a canopy bridge across a road (Continuous Corridors).
    Bridge,
    /// Plant a stepping-stone grove on open ground (Stepping-Stone Networks).
    Grove,
}

impl ActionKind {
    pub const ALL: [ActionKind; 5] =
        [ActionKind::Plant, ActionKind::Restore, ActionKind::Protect, ActionKind::Bridge, ActionKind::Grove];
}

impl Ruleset {
    pub const BASE: Ruleset = Ruleset { connectivity: Connectivity::Base, recovery: Recovery::Base };

    pub const fn new(connectivity: Connectivity, recovery: Recovery) -> Self {
        Ruleset { connectivity, recovery }
    }

    /// Stable key used for per-ruleset data in level files, e.g. `"stepping_stone+active"`.
    pub fn key(&self) -> String {
        let c = match self.connectivity {
            Connectivity::Base => "base",
            Connectivity::Continuous => "continuous",
            Connectivity::SteppingStone => "stepping_stone",
        };
        let r = match self.recovery {
            Recovery::Base => "base",
            Recovery::Succession => "succession",
            Recovery::Active => "active",
        };
        format!("{c}+{r}")
    }

    /// Restoration Capacity cost of an action, or `None` when the active rules
    /// do not allow it at all.
    pub fn cost(&self, kind: ActionKind) -> Option<u8> {
        match kind {
            ActionKind::Plant => Some(1),
            ActionKind::Restore => Some(if self.recovery == Recovery::Active { 2 } else { 1 }),
            ActionKind::Protect => Some(1),
            ActionKind::Bridge => (self.connectivity == Connectivity::Continuous).then_some(2),
            ActionKind::Grove => (self.connectivity == Connectivity::SteppingStone).then_some(2),
        }
    }

    pub fn natural_regrowth(&self) -> bool {
        self.recovery == Recovery::Succession
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn amendment_unlocks_differ() {
        let cc = Ruleset::new(Connectivity::Continuous, Recovery::Base);
        let ss = Ruleset::new(Connectivity::SteppingStone, Recovery::Base);
        assert_eq!(cc.cost(ActionKind::Bridge), Some(2));
        assert_eq!(cc.cost(ActionKind::Grove), None);
        assert_eq!(ss.cost(ActionKind::Grove), Some(2));
        assert_eq!(ss.cost(ActionKind::Bridge), None);
        assert_eq!(Ruleset::BASE.cost(ActionKind::Bridge), None);
    }

    #[test]
    fn recovery_costs_differ() {
        let a = Ruleset::new(Connectivity::Base, Recovery::Active);
        let s = Ruleset::new(Connectivity::Base, Recovery::Succession);
        assert_eq!(a.cost(ActionKind::Restore), Some(2));
        assert_eq!(s.cost(ActionKind::Restore), Some(1));
        assert!(s.natural_regrowth() && !a.natural_regrowth());
    }
}

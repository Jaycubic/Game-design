//! Regional campaign progression and the Living Charter votes.
//!
//! A region has ten levels and exactly two amendment checkpoints: Connectivity
//! after level 2 and Recovery after level 4. The amendment slots are fixed in
//! the type, so a third amendment is unrepresentable as well as rejected.

use crate::rules::{Connectivity, Recovery, Ruleset};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const LEVELS_PER_REGION: u8 = 10;
pub const AMENDMENT_1_AFTER: u8 = 2;
pub const AMENDMENT_2_AFTER: u8 = 4;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum AmendmentChoice {
    A,
    B,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct VoteState {
    /// 1 = Connectivity, 2 = Recovery.
    pub amendment: u8,
    /// 1 = first vote, 2 = re-vote after a tie.
    pub ballot_round: u8,
    /// Seats allowed to vote (members present when the vote opened).
    pub eligible: Vec<u8>,
    pub ballots: BTreeMap<u8, AmendmentChoice>,
    /// Set after a second tie: only this seat decides.
    pub tie_breaker: Option<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "outcome", rename_all = "snake_case")]
pub enum VoteOutcome {
    Pending {
        remaining: usize,
    },
    /// First ballot tied: discuss briefly and vote again.
    Revote,
    /// Second ballot tied: the rotating tie-break player chooses.
    AwaitTieBreak {
        seat: u8,
    },
    Decided {
        choice: AmendmentChoice,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "code", rename_all = "snake_case")]
pub enum CampaignError {
    NoVoteOpen,
    NotEligible,
    NotTieBreaker,
    AmendmentAlreadyApplied,
    TooManyAmendments,
    WrongLevel { expected: u8 },
    CampaignComplete,
    NoVoters,
}

impl CampaignError {
    pub fn message(&self) -> String {
        match self {
            CampaignError::NoVoteOpen => "There is no Living Charter vote open.".into(),
            CampaignError::NotEligible => "You are not a voter in this Living Charter vote.".into(),
            CampaignError::NotTieBreaker => "Only the tie-break player can decide this vote.".into(),
            CampaignError::AmendmentAlreadyApplied => "That amendment has already been chosen.".into(),
            CampaignError::TooManyAmendments => "A region allows exactly two Living Charter amendments.".into(),
            CampaignError::WrongLevel { expected } => format!("The campaign is on level {expected}."),
            CampaignError::CampaignComplete => "This regional campaign is complete.".into(),
            CampaignError::NoVoters => "A vote needs at least one voter.".into(),
        }
    }
}

impl VoteState {
    pub fn open(amendment: u8, mut eligible: Vec<u8>) -> Result<Self, CampaignError> {
        eligible.sort();
        eligible.dedup();
        if eligible.is_empty() {
            return Err(CampaignError::NoVoters);
        }
        Ok(VoteState { amendment, ballot_round: 1, eligible, ballots: BTreeMap::new(), tie_breaker: None })
    }

    /// Cast or change a ballot. Resolution is deterministic regardless of arrival order.
    pub fn cast(&mut self, seat: u8, choice: AmendmentChoice, run_index: u32) -> Result<VoteOutcome, CampaignError> {
        if let Some(tb) = self.tie_breaker {
            if seat != tb {
                return Err(CampaignError::NotTieBreaker);
            }
            return Ok(VoteOutcome::Decided { choice });
        }
        if !self.eligible.contains(&seat) {
            return Err(CampaignError::NotEligible);
        }
        self.ballots.insert(seat, choice);
        Ok(self.tally(run_index))
    }

    /// A voter left before voting: shrink the electorate so the vote cannot stall.
    pub fn remove_voter(&mut self, seat: u8, run_index: u32) -> VoteOutcome {
        if !self.ballots.contains_key(&seat) && self.eligible.len() > 1 {
            self.eligible.retain(|s| *s != seat);
            if self.tie_breaker == Some(seat) {
                self.tie_breaker = Some(self.eligible[run_index as usize % self.eligible.len()]);
            }
        }
        self.tally(run_index)
    }

    pub fn tally(&mut self, run_index: u32) -> VoteOutcome {
        if let Some(seat) = self.tie_breaker {
            return VoteOutcome::AwaitTieBreak { seat };
        }
        let remaining = self.eligible.iter().filter(|s| !self.ballots.contains_key(s)).count();
        if remaining > 0 {
            return VoteOutcome::Pending { remaining };
        }
        let a = self.ballots.values().filter(|c| **c == AmendmentChoice::A).count();
        let b = self.ballots.len() - a;
        if a != b {
            let choice = if a > b { AmendmentChoice::A } else { AmendmentChoice::B };
            return VoteOutcome::Decided { choice };
        }
        if self.ballot_round == 1 {
            self.ballot_round = 2;
            self.ballots.clear();
            return VoteOutcome::Revote;
        }
        // Rotate the tie-break seat between regional runs.
        let seat = self.eligible[run_index as usize % self.eligible.len()];
        self.tie_breaker = Some(seat);
        VoteOutcome::AwaitTieBreak { seat }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "phase", rename_all = "snake_case")]
pub enum CampaignPhase {
    Level { number: u8 },
    CharterVote { vote: VoteState },
    Complete,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LevelBest {
    pub leaf_marks: u8,
    pub spent: u16,
    pub completions: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CampaignState {
    pub region: String,
    /// How many regional runs this room has started; rotates the tie-break seat.
    pub run_index: u32,
    pub phase: CampaignPhase,
    pub connectivity: Option<Connectivity>,
    pub recovery: Option<Recovery>,
    pub best: BTreeMap<u8, LevelBest>,
    pub insights: Vec<String>,
    /// Cells restored per level, kept so the regional map never resets.
    pub restored: BTreeMap<u8, u16>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LevelCompletion {
    pub number: u8,
    pub leaf_marks: u8,
    pub spent: u16,
    pub restored_cells: u16,
    pub insight: Option<String>,
}

impl CampaignState {
    pub fn new(region: &str, run_index: u32) -> Self {
        CampaignState {
            region: region.to_string(),
            run_index,
            phase: CampaignPhase::Level { number: 1 },
            connectivity: None,
            recovery: None,
            best: BTreeMap::new(),
            insights: Vec::new(),
            restored: BTreeMap::new(),
        }
    }

    pub fn current_level(&self) -> Option<u8> {
        match self.phase {
            CampaignPhase::Level { number } => Some(number),
            _ => None,
        }
    }

    pub fn amendments_applied(&self) -> u8 {
        self.connectivity.is_some() as u8 + self.recovery.is_some() as u8
    }

    pub fn ruleset(&self) -> Ruleset {
        Ruleset::new(self.connectivity.unwrap_or(Connectivity::Base), self.recovery.unwrap_or(Recovery::Base))
    }

    /// Every ruleset a level can be played under, given where the amendments fall.
    pub fn possible_rulesets(level: u8) -> Vec<Ruleset> {
        let cs: &[Connectivity] = if level > AMENDMENT_1_AFTER {
            &[Connectivity::Continuous, Connectivity::SteppingStone]
        } else {
            &[Connectivity::Base]
        };
        let rs: &[Recovery] =
            if level > AMENDMENT_2_AFTER { &[Recovery::Succession, Recovery::Active] } else { &[Recovery::Base] };
        cs.iter().flat_map(|c| rs.iter().map(move |r| Ruleset::new(*c, *r))).collect()
    }

    /// Record a won level and advance (opening a Charter vote at checkpoints).
    /// Replays of earlier levels only improve the best record.
    pub fn complete_level(&mut self, done: LevelCompletion, voters: Vec<u8>) -> Result<(), CampaignError> {
        let entry =
            self.best.entry(done.number).or_insert(LevelBest { leaf_marks: 0, spent: u16::MAX, completions: 0 });
        entry.leaf_marks = entry.leaf_marks.max(done.leaf_marks);
        entry.spent = entry.spent.min(done.spent);
        entry.completions += 1;
        let restored = self.restored.entry(done.number).or_insert(0);
        *restored = (*restored).max(done.restored_cells);
        if let Some(i) = done.insight {
            if !self.insights.contains(&i) {
                self.insights.push(i);
            }
        }
        let current = match self.phase {
            CampaignPhase::Level { number } => number,
            CampaignPhase::Complete => return Ok(()), // replay after completion
            CampaignPhase::CharterVote { .. } => return Ok(()),
        };
        if done.number != current {
            return if done.number < current { Ok(()) } else { Err(CampaignError::WrongLevel { expected: current }) };
        }
        self.phase = if current == AMENDMENT_1_AFTER && self.connectivity.is_none() {
            CampaignPhase::CharterVote { vote: VoteState::open(1, voters)? }
        } else if current == AMENDMENT_2_AFTER && self.recovery.is_none() {
            CampaignPhase::CharterVote { vote: VoteState::open(2, voters)? }
        } else if current >= LEVELS_PER_REGION {
            CampaignPhase::Complete
        } else {
            CampaignPhase::Level { number: current + 1 }
        };
        Ok(())
    }

    /// Cast a ballot in the open vote; applies the amendment when decided.
    pub fn vote(&mut self, seat: u8, choice: AmendmentChoice) -> Result<VoteOutcome, CampaignError> {
        let run = self.run_index;
        let CampaignPhase::CharterVote { vote } = &mut self.phase else { return Err(CampaignError::NoVoteOpen) };
        let outcome = vote.cast(seat, choice, run)?;
        if let VoteOutcome::Decided { choice } = outcome {
            let amendment = vote.amendment;
            self.apply_amendment(amendment, choice)?;
        }
        Ok(outcome)
    }

    pub fn voter_left(&mut self, seat: u8) -> Result<VoteOutcome, CampaignError> {
        let run = self.run_index;
        let CampaignPhase::CharterVote { vote } = &mut self.phase else { return Err(CampaignError::NoVoteOpen) };
        let outcome = vote.remove_voter(seat, run);
        if let VoteOutcome::Decided { choice } = outcome {
            let amendment = vote.amendment;
            self.apply_amendment(amendment, choice)?;
        }
        Ok(outcome)
    }

    pub fn apply_amendment(&mut self, amendment: u8, choice: AmendmentChoice) -> Result<(), CampaignError> {
        if self.amendments_applied() >= 2 {
            return Err(CampaignError::TooManyAmendments);
        }
        match amendment {
            1 => {
                if self.connectivity.is_some() {
                    return Err(CampaignError::AmendmentAlreadyApplied);
                }
                self.connectivity = Some(match choice {
                    AmendmentChoice::A => Connectivity::Continuous,
                    AmendmentChoice::B => Connectivity::SteppingStone,
                });
                self.phase = CampaignPhase::Level { number: AMENDMENT_1_AFTER + 1 };
            }
            2 => {
                if self.recovery.is_some() {
                    return Err(CampaignError::AmendmentAlreadyApplied);
                }
                if self.connectivity.is_none() {
                    return Err(CampaignError::WrongLevel { expected: AMENDMENT_1_AFTER });
                }
                self.recovery = Some(match choice {
                    AmendmentChoice::A => Recovery::Succession,
                    AmendmentChoice::B => Recovery::Active,
                });
                self.phase = CampaignPhase::Level { number: AMENDMENT_2_AFTER + 1 };
            }
            _ => return Err(CampaignError::TooManyAmendments),
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn win(c: &mut CampaignState, n: u8) {
        c.complete_level(
            LevelCompletion { number: n, leaf_marks: 1, spent: 3, restored_cells: 2, insight: None },
            vec![0, 1, 2],
        )
        .unwrap();
    }

    #[test]
    fn exactly_two_amendments_at_checkpoints() {
        let mut c = CampaignState::new("forest", 0);
        win(&mut c, 1);
        assert_eq!(c.current_level(), Some(2));
        win(&mut c, 2);
        assert!(matches!(c.phase, CampaignPhase::CharterVote { .. }));
        c.vote(0, AmendmentChoice::B).unwrap();
        c.vote(1, AmendmentChoice::B).unwrap();
        assert_eq!(c.vote(2, AmendmentChoice::A).unwrap(), VoteOutcome::Decided { choice: AmendmentChoice::B });
        assert_eq!(c.connectivity, Some(Connectivity::SteppingStone));
        assert_eq!(c.current_level(), Some(3));
        win(&mut c, 3);
        win(&mut c, 4);
        for s in 0..3 {
            c.vote(s, AmendmentChoice::A).unwrap();
        }
        assert_eq!(c.recovery, Some(Recovery::Succession));
        assert_eq!(c.amendments_applied(), 2);
        assert_eq!(c.apply_amendment(1, AmendmentChoice::A), Err(CampaignError::TooManyAmendments));
        assert_eq!(c.apply_amendment(3, AmendmentChoice::A), Err(CampaignError::TooManyAmendments));
        for n in 5..=10 {
            win(&mut c, n);
        }
        assert_eq!(c.phase, CampaignPhase::Complete);
        assert_eq!(c.amendments_applied(), 2);
    }

    #[test]
    fn tie_goes_to_revote_then_rotating_tie_breaker() {
        let mut v = VoteState::open(1, vec![0, 1, 2, 3]).unwrap();
        v.cast(0, AmendmentChoice::A, 5).unwrap();
        v.cast(1, AmendmentChoice::A, 5).unwrap();
        v.cast(2, AmendmentChoice::B, 5).unwrap();
        assert_eq!(v.cast(3, AmendmentChoice::B, 5).unwrap(), VoteOutcome::Revote);
        for (s, c) in [(0, AmendmentChoice::A), (1, AmendmentChoice::B), (2, AmendmentChoice::A)] {
            v.cast(s, c, 5).unwrap();
        }
        // run 5 with 4 voters → seat index 1
        assert_eq!(v.cast(3, AmendmentChoice::B, 5).unwrap(), VoteOutcome::AwaitTieBreak { seat: 1 });
        assert_eq!(v.cast(0, AmendmentChoice::A, 5), Err(CampaignError::NotTieBreaker));
        assert_eq!(v.cast(1, AmendmentChoice::B, 5).unwrap(), VoteOutcome::Decided { choice: AmendmentChoice::B });
    }

    #[test]
    fn absent_voter_cannot_stall_the_vote() {
        let mut v = VoteState::open(2, vec![0, 1, 2]).unwrap();
        v.cast(0, AmendmentChoice::A, 0).unwrap();
        v.cast(1, AmendmentChoice::A, 0).unwrap();
        assert_eq!(v.remove_voter(2, 0), VoteOutcome::Decided { choice: AmendmentChoice::A });
    }

    #[test]
    fn vote_order_does_not_change_result() {
        let order_a = [(0, AmendmentChoice::A), (1, AmendmentChoice::B), (2, AmendmentChoice::B)];
        let mut order_b = order_a;
        order_b.reverse();
        let run = |o: &[(u8, AmendmentChoice)]| {
            let mut v = VoteState::open(1, vec![0, 1, 2]).unwrap();
            o.iter().map(|(s, c)| v.cast(*s, *c, 0).unwrap()).last().unwrap()
        };
        assert_eq!(run(&order_a), run(&order_b));
    }

    #[test]
    fn possible_rulesets_follow_checkpoints() {
        assert_eq!(CampaignState::possible_rulesets(2), vec![Ruleset::BASE]);
        assert_eq!(CampaignState::possible_rulesets(3).len(), 2);
        assert_eq!(CampaignState::possible_rulesets(5).len(), 4);
    }
}

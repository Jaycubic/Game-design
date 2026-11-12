//! Shared-room membership rules (pure; the server persists and broadcasts).

use serde::{Deserialize, Serialize};

pub const MIN_PLAYERS: usize = 2;
pub const MAX_PLAYERS: usize = 4;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Member {
    pub player_id: String,
    pub seat: u8,
    pub ready: bool,
    pub connected: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "code", rename_all = "snake_case")]
pub enum RoomError {
    RoomFull,
    AlreadyMember,
    NotMember,
    NotHost,
    NotEnoughPlayers { have: usize, need: usize },
    NotAllReady,
    InvalidLimits,
}

impl RoomError {
    pub fn message(&self) -> String {
        match self {
            RoomError::RoomFull => format!("This room already has {MAX_PLAYERS} players."),
            RoomError::AlreadyMember => "You are already in this room.".into(),
            RoomError::NotMember => "You are not a member of this room.".into(),
            RoomError::NotHost => "Only the host can do that.".into(),
            RoomError::NotEnoughPlayers { have, need } => format!("{need} players are needed to start; {have} joined."),
            RoomError::NotAllReady => "Every connected player must be ready.".into(),
            RoomError::InvalidLimits => "Room limits must be between 2 and 4 players.".into(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Roster {
    pub host: String,
    pub min: usize,
    pub max: usize,
    pub members: Vec<Member>,
}

impl Roster {
    pub fn new(host: &str, min: usize, max: usize) -> Result<Self, RoomError> {
        if !(MIN_PLAYERS..=MAX_PLAYERS).contains(&min) || !(min..=MAX_PLAYERS).contains(&max) {
            return Err(RoomError::InvalidLimits);
        }
        Ok(Roster {
            host: host.into(),
            min,
            max,
            members: vec![Member { player_id: host.into(), seat: 0, ready: false, connected: true }],
        })
    }

    pub fn member(&self, player: &str) -> Option<&Member> {
        self.members.iter().find(|m| m.player_id == player)
    }

    /// Join with the lowest free seat. Rejoining an existing seat is a reconnect.
    pub fn join(&mut self, player: &str) -> Result<u8, RoomError> {
        if let Some(m) = self.members.iter_mut().find(|m| m.player_id == player) {
            m.connected = true;
            return Ok(m.seat);
        }
        if self.members.len() >= self.max {
            return Err(RoomError::RoomFull);
        }
        let seat = (0..self.max as u8).find(|s| !self.members.iter().any(|m| m.seat == *s)).expect("free seat");
        self.members.push(Member { player_id: player.into(), seat, ready: false, connected: true });
        self.members.sort_by_key(|m| m.seat);
        Ok(seat)
    }

    /// Leave permanently. If the host leaves, the next seat becomes host.
    pub fn leave(&mut self, player: &str) -> Result<(), RoomError> {
        let n = self.members.len();
        self.members.retain(|m| m.player_id != player);
        if self.members.len() == n {
            return Err(RoomError::NotMember);
        }
        if self.host == player {
            if let Some(m) = self.members.first() {
                self.host = m.player_id.clone();
            }
        }
        Ok(())
    }

    pub fn set_connected(&mut self, player: &str, connected: bool) -> Result<u8, RoomError> {
        let m = self.members.iter_mut().find(|m| m.player_id == player).ok_or(RoomError::NotMember)?;
        m.connected = connected;
        Ok(m.seat)
    }

    pub fn set_ready(&mut self, player: &str, ready: bool) -> Result<(), RoomError> {
        let m = self.members.iter_mut().find(|m| m.player_id == player).ok_or(RoomError::NotMember)?;
        m.ready = ready;
        Ok(())
    }

    pub fn connected_seats(&self) -> Vec<u8> {
        self.members.iter().filter(|m| m.connected).map(|m| m.seat).collect()
    }

    pub fn can_start(&self, by: &str) -> Result<(), RoomError> {
        if by != self.host {
            return Err(RoomError::NotHost);
        }
        let present: Vec<&Member> = self.members.iter().filter(|m| m.connected).collect();
        if present.len() < self.min {
            return Err(RoomError::NotEnoughPlayers { have: present.len(), need: self.min });
        }
        if present.iter().any(|m| !m.ready) {
            return Err(RoomError::NotAllReady);
        }
        Ok(())
    }

    /// Expedition turn order: next connected seat after `current`.
    pub fn next_turn(&self, current: u8) -> Option<u8> {
        let seats = self.connected_seats();
        seats.iter().copied().find(|s| *s > current).or_else(|| seats.first().copied())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enforces_two_to_four_players() {
        let mut r = Roster::new("a", 2, 4).unwrap();
        assert_eq!(r.can_start("a"), Err(RoomError::NotEnoughPlayers { have: 1, need: 2 }));
        assert_eq!(r.join("b"), Ok(1));
        assert_eq!(r.join("c"), Ok(2));
        assert_eq!(r.join("d"), Ok(3));
        assert_eq!(r.join("e"), Err(RoomError::RoomFull));
        assert_eq!(r.join("b"), Ok(1), "rejoin keeps the seat");
        assert!(Roster::new("x", 1, 4).is_err());
        assert!(Roster::new("x", 2, 5).is_err());
    }

    #[test]
    fn start_needs_host_and_ready() {
        let mut r = Roster::new("a", 2, 4).unwrap();
        r.join("b").unwrap();
        assert_eq!(r.can_start("b"), Err(RoomError::NotHost));
        assert_eq!(r.can_start("a"), Err(RoomError::NotAllReady));
        r.set_ready("a", true).unwrap();
        r.set_ready("b", true).unwrap();
        assert_eq!(r.can_start("a"), Ok(()));
    }

    #[test]
    fn host_hands_over_and_turns_skip_disconnected() {
        let mut r = Roster::new("a", 2, 4).unwrap();
        r.join("b").unwrap();
        r.join("c").unwrap();
        r.set_connected("b", false).unwrap();
        assert_eq!(r.next_turn(0), Some(2));
        assert_eq!(r.next_turn(2), Some(0));
        r.leave("a").unwrap();
        assert_eq!(r.host, "b");
    }
}

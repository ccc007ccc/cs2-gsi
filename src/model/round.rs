//! Round and bomb-state nodes.

use serde::{Deserialize, Serialize};

/// Per-round state.
#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
pub struct Round {
    /// Phase of the current round.
    #[serde(default)]
    pub phase: RoundPhase,
    /// State of the bomb during the current round.
    #[serde(default, rename = "bomb")]
    pub bomb: BombRoundState,
    /// Side that won the round (only set after the round ends).
    #[serde(default, rename = "win_team")]
    pub win_team: WinningTeam,
}

/// Phase of the active round.
#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum RoundPhase {
    /// Buy-time / freezetime — players cannot move yet.
    Freezetime,
    /// Round is live.
    Live,
    /// Round ended, end-of-round delay active.
    Over,
    /// Unrecognized / not-yet-mapped phase.
    #[serde(other)]
    #[default]
    Unknown,
}

/// Bomb state as reported via the `round.bomb` field. The richer per-bomb
/// position info is in [`crate::model::bomb::Bomb`] under the root `bomb`.
#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum BombRoundState {
    /// Bomb is being planted — defuse cycle has not yet started.
    Planted,
    /// Bomb was defused successfully.
    Defused,
    /// Bomb exploded.
    Exploded,
    /// No bomb event in the current round.
    #[serde(other)]
    #[default]
    None,
}

/// Side that won the latest round.
#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum WinningTeam {
    /// Counter-Terrorists.
    Ct,
    /// Terrorists.
    T,
    /// No winner yet (round still in progress).
    #[serde(other)]
    #[default]
    None,
}

//! Bomb node — root-level bomb position / countdown info.

use super::helpers::de_opt_num_or_str;
use serde::{Deserialize, Serialize};

/// Bomb status as a separate root key. More detailed than the per-round
/// state in [`crate::model::round::BombRoundState`].
#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq)]
pub struct Bomb {
    /// Lifecycle state of the bomb.
    #[serde(default)]
    pub state: BombState,
    /// World position, formatted by CS2 as `"x, y, z"`.
    #[serde(default)]
    pub position: String,
    /// SteamID of the player carrying / defusing the bomb (if any).
    #[serde(default)]
    pub player: String,
    /// Countdown until detonation / defuse, in seconds.
    #[serde(default, deserialize_with = "de_opt_num_or_str")]
    pub countdown: Option<f32>,
}

/// Lifecycle state of the bomb.
#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum BombState {
    /// Bomb is held by a T player.
    Carried,
    /// Bomb is dropped on the ground.
    Dropped,
    /// Bomb is being planted (animation in progress).
    Planting,
    /// Bomb planted and counting down.
    Planted,
    /// Bomb is being defused.
    Defusing,
    /// Bomb defused.
    Defused,
    /// Bomb exploded.
    Exploded,
    /// Unrecognized / no bomb info.
    #[serde(other)]
    #[default]
    Unknown,
}

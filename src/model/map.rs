//! Map / round-series statistics.

use super::helpers::{de_num_or_str, de_opt_num_or_str};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Map / series-level state.
#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
pub struct Map {
    /// Game mode, e.g. `"competitive"`, `"casual"`, `"deathmatch"`.
    #[serde(default)]
    pub mode: String,
    /// Map filename, e.g. `"de_dust2"`.
    #[serde(default)]
    pub name: String,
    /// Current map phase.
    #[serde(default)]
    pub phase: MapPhase,
    /// 1-based round number within the current map.
    #[serde(default, deserialize_with = "de_num_or_str")]
    pub round: u32,
    /// Counter-Terrorist team statistics.
    #[serde(default, alias = "team_ct")]
    pub team_ct: TeamStatistics,
    /// Terrorist team statistics.
    #[serde(default, alias = "team_t")]
    pub team_t: TeamStatistics,
    /// Number of round wins required to win a series in this map.
    #[serde(default, deserialize_with = "de_opt_num_or_str")]
    pub num_matches_to_win_series: Option<u32>,
    /// Spectator count for the live broadcast (only present in some modes).
    #[serde(default, deserialize_with = "de_opt_num_or_str")]
    pub current_spectators: Option<u32>,
    /// Souvenir packs awarded so far.
    #[serde(default, deserialize_with = "de_opt_num_or_str")]
    pub souvenirs_total: Option<u32>,
    /// Per-round winner side, keyed by `"<round_index>"`.
    /// Values are typically `"ct_win_..."` / `"t_win_..."`.
    #[serde(default)]
    pub round_wins: HashMap<String, String>,
}

/// Phase of the current map.
#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum MapPhase {
    /// Pre-match warmup.
    Warmup,
    /// Live round play.
    Live,
    /// Mid-match intermission (between halves).
    Intermission,
    /// Match has ended.
    Gameover,
    /// Unrecognized / not-yet-mapped phase.
    #[serde(other)]
    #[default]
    Unknown,
}

/// Statistics for a side (CT / T).
#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
pub struct TeamStatistics {
    /// Round score for this side.
    #[serde(default, deserialize_with = "de_num_or_str")]
    pub score: u32,
    /// Number of round losses incurred consecutively (for loss-bonus calc).
    #[serde(default, deserialize_with = "de_num_or_str")]
    pub consecutive_round_losses: u32,
    /// Tactical timeouts remaining for this side.
    #[serde(default, deserialize_with = "de_num_or_str")]
    pub timeouts_remaining: u32,
    /// Maps won so far in the current series (BoX).
    #[serde(default, deserialize_with = "de_num_or_str")]
    pub matches_won_this_series: u32,
    /// Optional team name.
    #[serde(default)]
    pub name: String,
    /// Optional team flag identifier.
    #[serde(default)]
    pub flag: String,
}

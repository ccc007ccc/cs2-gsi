//! Provider node — describes the game client that produced the payload.

use super::helpers::{de_num_or_str, de_opt_num_or_str};
use serde::{Deserialize, Serialize};

/// Identifies the producer of the game state — for CS2 this is always the
/// game client itself (`appid: 730`).
#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
pub struct Provider {
    /// Display name of the producing client (e.g. `"Counter-Strike 2"`).
    #[serde(default)]
    pub name: String,
    /// Steam app id. `730` for CS2.
    #[serde(default, deserialize_with = "de_num_or_str")]
    pub appid: u32,
    /// Game client build version.
    #[serde(default, deserialize_with = "de_num_or_str")]
    pub version: u64,
    /// 64-bit SteamID of the local player.
    #[serde(default)]
    pub steamid: String,
    /// Unix timestamp (seconds) at which the payload was emitted.
    #[serde(default, deserialize_with = "de_opt_num_or_str")]
    pub timestamp: Option<u64>,
}

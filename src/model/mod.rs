//! Top-level GameState model.
//!
//! The root JSON document POSTed by Counter-Strike 2 to its Game State
//! Integration endpoint. Every node is optional — the cfg file decides which
//! sections CS2 includes — so missing or null sub-objects deserialize to
//! their `Default` value rather than failing.

mod auth;
mod bomb;
mod grenade;
mod helpers;
mod map;
mod player;
mod provider;
mod round;
mod tournament;

pub use auth::Auth;
pub use bomb::{Bomb, BombState};
pub use grenade::{Grenade, GrenadeKind};
pub use map::{Map, MapPhase, TeamStatistics};
pub use player::{
    MatchStats, Player, PlayerActivity, PlayerState, PlayerTeam, Weapon, WeaponKind, WeaponState,
};
pub use provider::Provider;
pub use round::{BombRoundState, Round, RoundPhase, WinningTeam};
pub use tournament::{PhaseCountdowns, TournamentDraft};

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Root GSI document received from CS2.
#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq)]
pub struct GameState {
    /// Authentication block declared in the cfg file. CS2 itself never adds
    /// a token, but tokens you wrote into the cfg appear here.
    #[serde(default)]
    pub auth: Auth,

    /// Producer of the payload (the CS2 client itself).
    #[serde(default)]
    pub provider: Provider,

    /// Series / map state. Absent when the player is not in a match.
    #[serde(default)]
    pub map: Option<Map>,

    /// Per-round state. Absent outside of an active round.
    #[serde(default)]
    pub round: Option<Round>,

    /// Player snapshot from the local client's point-of-view (the player or
    /// the spectated entity). Absent on the main menu.
    #[serde(default)]
    pub player: Option<Player>,

    /// All known players keyed by SteamID. Only populated when the cfg file
    /// requests `allplayers_*` sections AND the local client is permitted to
    /// see them (e.g. spectator / replay / GOTV).
    #[serde(default)]
    pub allplayers: BTreeMap<String, Player>,

    /// Countdown for the current map / round phase.
    #[serde(default, rename = "phase_countdowns")]
    pub phase_countdowns: Option<PhaseCountdowns>,

    /// Detailed bomb info.
    #[serde(default)]
    pub bomb: Option<Bomb>,

    /// All in-flight / active grenades, keyed by entity index.
    #[serde(default, alias = "allgrenades")]
    pub grenades: BTreeMap<String, Grenade>,

    /// Tournament-mode draft state.
    #[serde(default, alias = "tournamentdraft")]
    pub tournament_draft: Option<TournamentDraft>,

    /// CS2 may include a `previously` block listing the values of fields
    /// that just changed. cs2-gsi performs its own diff and does not rely on
    /// this — it is exposed here as raw JSON for advanced consumers.
    #[serde(default)]
    pub previously: Option<serde_json::Value>,

    /// CS2 may include an `added` block listing fields that appeared for the
    /// first time. Exposed as raw JSON for advanced consumers.
    #[serde(default)]
    pub added: Option<serde_json::Value>,
}

impl GameState {
    /// Parse a GSI payload from raw bytes.
    pub fn from_slice(bytes: &[u8]) -> Result<Self, serde_json::Error> {
        serde_json::from_slice(bytes)
    }

    /// Parse a GSI payload from a string. Provided as an inherent method for
    /// discoverability — the conflicting [`std::str::FromStr`] impl has the
    /// same semantics.
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(s)
    }

    /// Convenience: returns `true` when the local player is dead (HP == 0
    /// AND in the `playing` activity). Returns `false` when no player block
    /// is present.
    pub fn local_is_dead(&self) -> bool {
        match &self.player {
            Some(p) => matches!(p.activity, PlayerActivity::Playing) && p.state.health == 0,
            None => false,
        }
    }
}

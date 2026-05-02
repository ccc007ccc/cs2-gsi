//! Phase-countdown and tournament-draft nodes.

use super::helpers::{de_num_or_str, de_opt_num_or_str};
use serde::{Deserialize, Serialize};

/// Countdown for the current `phase` (freezetime, live, planted, …).
#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq)]
pub struct PhaseCountdowns {
    /// The phase the countdown applies to.
    #[serde(default, rename = "phase")]
    pub phase: String,
    /// Remaining seconds until the phase ends. Stored as a string by CS2.
    #[serde(default, deserialize_with = "de_num_or_str", rename = "phase_ends_in")]
    pub phase_ends_in: f32,
}

/// Tournament draft / map veto data (only present in tournament-style modes).
#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
pub struct TournamentDraft {
    /// Phase of the draft (`"map_veto"`, `"side_pick"`, …).
    #[serde(default, rename = "state")]
    pub state: String,
    /// Tournament-side event id.
    #[serde(default, deserialize_with = "de_opt_num_or_str", rename = "event_id")]
    pub event_id: Option<u64>,
    /// Tournament-side stage id.
    #[serde(default, deserialize_with = "de_opt_num_or_str", rename = "stage_id")]
    pub stage_id: Option<u64>,
    /// Team id picking first.
    #[serde(
        default,
        deserialize_with = "de_opt_num_or_str",
        rename = "first_team_id"
    )]
    pub first_team_id: Option<u64>,
    /// Team id picking second.
    #[serde(
        default,
        deserialize_with = "de_opt_num_or_str",
        rename = "second_team_id"
    )]
    pub second_team_id: Option<u64>,
    /// Event display name.
    #[serde(default, rename = "event")]
    pub event: String,
    /// Stage display name.
    #[serde(default, rename = "stage")]
    pub stage: String,
    /// First team display name.
    #[serde(default, rename = "first_team_name")]
    pub first_team_name: String,
    /// Second team display name.
    #[serde(default, rename = "second_team_name")]
    pub second_team_name: String,
}

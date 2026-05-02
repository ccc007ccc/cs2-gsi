//! Grenade node — entries in the `allgrenades` map.

use super::helpers::{de_num_or_str, de_opt_num_or_str};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A single live grenade, keyed by entity index in the parent map.
#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq)]
pub struct Grenade {
    /// SteamID of the player who threw it.
    #[serde(default)]
    pub owner: String,
    /// World position, `"x, y, z"`.
    #[serde(default)]
    pub position: String,
    /// Velocity vector, `"vx, vy, vz"`.
    #[serde(default)]
    pub velocity: String,
    /// Time alive in seconds.
    #[serde(default, deserialize_with = "de_num_or_str")]
    pub lifetime: f32,
    /// Type of grenade.
    #[serde(default, rename = "type")]
    pub kind: GrenadeKind,
    /// For inferno (molotov / incendiary): per-flame position map.
    #[serde(default)]
    pub flames: HashMap<String, String>,
    /// For smoke / decoy: remaining effect time in seconds.
    #[serde(default, deserialize_with = "de_opt_num_or_str")]
    pub effecttime: Option<f32>,
}

/// Type of an in-flight or active grenade.
#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum GrenadeKind {
    /// Hand frag grenade.
    Frag,
    /// Smoke grenade.
    Smoke,
    /// Flashbang.
    Flashbang,
    /// Decoy grenade.
    Decoy,
    /// Molotov / Incendiary (inferno).
    Inferno,
    /// Unrecognized type.
    #[serde(other)]
    #[default]
    Unknown,
}

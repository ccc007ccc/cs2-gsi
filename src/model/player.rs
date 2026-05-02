//! Player node — both the local observer and entries inside `allplayers`.

use super::helpers::{de_num_or_str, de_opt_num_or_str};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// A single player snapshot.
#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq)]
pub struct Player {
    /// 64-bit SteamID. Empty for bots.
    #[serde(default)]
    pub steamid: String,
    /// In-game display name.
    #[serde(default)]
    pub name: String,
    /// Clan tag, may be empty.
    #[serde(default)]
    pub clan: String,
    /// Observer slot number (0..=9 in standard 5v5).
    #[serde(default, deserialize_with = "de_opt_num_or_str")]
    pub observer_slot: Option<u8>,
    /// XP overload level (CS2 progression).
    #[serde(default, deserialize_with = "de_opt_num_or_str")]
    pub xp_overload_level: Option<u32>,
    /// Side the player belongs to.
    #[serde(default)]
    pub team: PlayerTeam,
    /// Activity (menu / playing / textinput).
    #[serde(default)]
    pub activity: PlayerActivity,
    /// Volatile per-tick state (health, armor, money, …).
    #[serde(default)]
    pub state: PlayerState,
    /// Per-match aggregate statistics.
    #[serde(default, rename = "match_stats")]
    pub match_stats: MatchStats,
    /// Inventory keyed by weapon slot id (e.g. `"weapon_0"`).
    #[serde(default)]
    pub weapons: BTreeMap<String, Weapon>,
    /// SteamID of the player currently being spectated (when `activity = playing` is false).
    #[serde(default)]
    pub spectarget: String,
    /// World position vector, formatted by CS2 as `"x, y, z"`.
    #[serde(default)]
    pub position: String,
    /// Forward look direction vector, formatted by CS2 as `"x, y, z"`.
    #[serde(default)]
    pub forward: String,
}

/// Side the player is on.
#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq, Hash)]
pub enum PlayerTeam {
    /// Counter-Terrorists.
    #[serde(alias = "ct", alias = "CT")]
    CT,
    /// Terrorists.
    #[serde(alias = "t", alias = "T")]
    T,
    /// Player has not joined a side yet.
    #[serde(other)]
    #[default]
    Unassigned,
}

/// What the player is currently doing.
#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum PlayerActivity {
    /// Player is in a menu (e.g. team / loadout selection).
    Menu,
    /// Player is alive in-game.
    Playing,
    /// Player is typing in chat.
    Textinput,
    /// Unrecognized activity.
    #[serde(other)]
    #[default]
    Unknown,
}

/// Volatile per-tick state. Mirrors `player_state` block in the GSI cfg.
#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq)]
pub struct PlayerState {
    /// HP (0..=100). 0 means dead.
    #[serde(default, deserialize_with = "de_num_or_str")]
    pub health: i32,
    /// Armor value (0..=100).
    #[serde(default, deserialize_with = "de_num_or_str")]
    pub armor: i32,
    /// Whether the player has a helmet.
    #[serde(default)]
    pub helmet: bool,
    /// Whether the player carries a defuse kit (CT only).
    #[serde(default)]
    pub defusekit: bool,
    /// Flash blindness level, 0..=255.
    #[serde(default, deserialize_with = "de_num_or_str")]
    pub flashed: i32,
    /// Smoke obscurance level, 0..=255.
    #[serde(default, deserialize_with = "de_num_or_str")]
    pub smoked: i32,
    /// Molotov burning level, 0..=255.
    #[serde(default, deserialize_with = "de_num_or_str")]
    pub burning: i32,
    /// Cash on hand.
    #[serde(default, deserialize_with = "de_num_or_str")]
    pub money: i32,
    /// Kills accumulated in the current round.
    #[serde(default, deserialize_with = "de_num_or_str")]
    pub round_kills: i32,
    /// Headshot kills accumulated in the current round.
    #[serde(default, deserialize_with = "de_num_or_str")]
    pub round_killhs: i32,
    /// Total damage dealt in the current round.
    #[serde(default, deserialize_with = "de_num_or_str")]
    pub round_totaldmg: i32,
    /// Total equipment value carried.
    #[serde(default, deserialize_with = "de_num_or_str")]
    pub equip_value: i32,
}

/// Per-match aggregate stats. Mirrors `player_match_stats`.
#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
pub struct MatchStats {
    /// Match kills.
    #[serde(default, deserialize_with = "de_num_or_str")]
    pub kills: i32,
    /// Match assists.
    #[serde(default, deserialize_with = "de_num_or_str")]
    pub assists: i32,
    /// Match deaths.
    #[serde(default, deserialize_with = "de_num_or_str")]
    pub deaths: i32,
    /// Round-MVP awards earned this match.
    #[serde(default, deserialize_with = "de_num_or_str")]
    pub mvps: i32,
    /// Match score.
    #[serde(default, deserialize_with = "de_num_or_str")]
    pub score: i32,
}

/// A single weapon slot entry. Slot key is e.g. `"weapon_0"`.
#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
pub struct Weapon {
    /// Weapon class name, e.g. `"weapon_ak47"`.
    #[serde(default)]
    pub name: String,
    /// Paint-kit identifier (cosmetics).
    #[serde(default, deserialize_with = "de_opt_num_or_str")]
    pub paintkit: Option<String>,
    /// Weapon category.
    #[serde(default, rename = "type")]
    pub kind: WeaponKind,
    /// Rounds in the current magazine.
    #[serde(default, deserialize_with = "de_opt_num_or_str")]
    pub ammo_clip: Option<i32>,
    /// Magazine capacity.
    #[serde(default, deserialize_with = "de_opt_num_or_str")]
    pub ammo_clip_max: Option<i32>,
    /// Reserve ammo carried.
    #[serde(default, deserialize_with = "de_opt_num_or_str")]
    pub ammo_reserve: Option<i32>,
    /// Whether the weapon is `holstered`, `active`, or `reloading`.
    #[serde(default)]
    pub state: WeaponState,
}

/// Weapon category as reported by CS2.
#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq, Hash)]
pub enum WeaponKind {
    /// Knife.
    #[serde(alias = "Knife")]
    Knife,
    /// Pistol.
    #[serde(alias = "Pistol")]
    Pistol,
    /// Submachine gun.
    #[serde(alias = "Submachine Gun", alias = "SubmachineGun")]
    SubmachineGun,
    /// Rifle.
    #[serde(alias = "Rifle")]
    Rifle,
    /// Sniper rifle.
    #[serde(alias = "SniperRifle", alias = "Sniper Rifle")]
    SniperRifle,
    /// Shotgun.
    #[serde(alias = "Shotgun")]
    Shotgun,
    /// Heavy / machine gun.
    #[serde(alias = "Machine Gun", alias = "MachineGun")]
    MachineGun,
    /// Grenade.
    #[serde(alias = "Grenade")]
    Grenade,
    /// C4 explosive.
    #[serde(alias = "C4")]
    C4,
    /// Taser / Zeus.
    #[serde(alias = "StackableItem")]
    StackableItem,
    /// Unrecognized / future weapon class.
    #[serde(other)]
    #[default]
    Unknown,
}

/// Weapon slot state.
#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum WeaponState {
    /// In inventory but not equipped.
    Holstered,
    /// Currently equipped.
    Active,
    /// Reloading.
    Reloading,
    /// Unrecognized state.
    #[serde(other)]
    #[default]
    Unknown,
}

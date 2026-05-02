//! Strongly typed events emitted by [`GameStateListener`](crate::GameStateListener).
//!
//! Events are derived from a per-tick diff of two consecutive [`GameState`]
//! snapshots. Each event variant carries the relevant subject (`player`,
//! `team`, `bomb`, …) plus, where applicable, the `previous` and `new`
//! values to make reaction code straightforward.

use crate::model::{
    BombRoundState, GameState, MapPhase, MatchStats, Player, PlayerActivity, PlayerTeam, Provider,
    RoundPhase, WinningTeam,
};

// ------------------------- Player events ------------------------------------

/// A player record changed in some way (catch-all). Fired alongside any of
/// the more specific player events below.
#[derive(Clone, Debug, PartialEq)]
pub struct PlayerUpdated {
    /// Updated player snapshot.
    pub player: Player,
    /// Previous snapshot of the same player (`None` if just connected).
    pub previous: Option<Player>,
}

/// Player health dropped to zero — they were killed.
#[derive(Clone, Debug, PartialEq)]
pub struct PlayerDied {
    /// Player who died.
    pub player: Player,
    /// HP before the killing damage.
    pub previous_health: i32,
    /// HP after (always `0`).
    pub new_health: i32,
}

/// Player respawned (HP went from `0` back to `> 0`).
#[derive(Clone, Debug, PartialEq)]
pub struct PlayerRespawned {
    /// Player who respawned.
    pub player: Player,
    /// HP before respawn (`0`).
    pub previous_health: i32,
    /// HP after respawn.
    pub new_health: i32,
}

/// Player took non-lethal damage.
#[derive(Clone, Debug, PartialEq)]
pub struct PlayerTookDamage {
    /// Affected player.
    pub player: Player,
    /// HP before the damage.
    pub previous_health: i32,
    /// HP after the damage.
    pub new_health: i32,
}

/// Player's `round_kills` counter increased — they got a kill.
#[derive(Clone, Debug, PartialEq)]
pub struct PlayerGotKill {
    /// Player who got the kill.
    pub player: Player,
    /// Round-kill count before the kill.
    pub previous_round_kills: i32,
    /// Round-kill count after the kill.
    pub new_round_kills: i32,
    /// `true` if the kill was a headshot (derived from `round_killhs` delta).
    pub is_headshot: bool,
    /// Currently equipped weapon at the moment the kill was observed.
    pub weapon: Option<String>,
}

/// Player's HP changed (any direction).
#[derive(Clone, Debug, PartialEq)]
pub struct PlayerHealthChanged {
    /// Affected player.
    pub player: Player,
    /// HP before.
    pub previous: i32,
    /// HP after.
    pub new: i32,
}

/// Player's armor value changed.
#[derive(Clone, Debug, PartialEq)]
pub struct PlayerArmorChanged {
    /// Affected player.
    pub player: Player,
    /// Armor before.
    pub previous: i32,
    /// Armor after.
    pub new: i32,
}

/// Player's helmet status changed.
#[derive(Clone, Debug, PartialEq)]
pub struct PlayerHelmetChanged {
    /// Affected player.
    pub player: Player,
    /// Helmet status before.
    pub previous: bool,
    /// Helmet status after.
    pub new: bool,
}

/// Flash blindness level changed.
#[derive(Clone, Debug, PartialEq)]
pub struct PlayerFlashAmountChanged {
    /// Affected player.
    pub player: Player,
    /// Flash level before.
    pub previous: i32,
    /// Flash level after.
    pub new: i32,
}

/// Smoke obscurance level changed.
#[derive(Clone, Debug, PartialEq)]
pub struct PlayerSmokedAmountChanged {
    /// Affected player.
    pub player: Player,
    /// Smoke level before.
    pub previous: i32,
    /// Smoke level after.
    pub new: i32,
}

/// Burning level changed (molotov / incendiary).
#[derive(Clone, Debug, PartialEq)]
pub struct PlayerBurningAmountChanged {
    /// Affected player.
    pub player: Player,
    /// Burning level before.
    pub previous: i32,
    /// Burning level after.
    pub new: i32,
}

/// Cash on hand changed.
#[derive(Clone, Debug, PartialEq)]
pub struct PlayerMoneyAmountChanged {
    /// Affected player.
    pub player: Player,
    /// Money before.
    pub previous: i32,
    /// Money after.
    pub new: i32,
}

/// Equipment value carried changed.
#[derive(Clone, Debug, PartialEq)]
pub struct PlayerEquipmentValueChanged {
    /// Affected player.
    pub player: Player,
    /// Equipment value before.
    pub previous: i32,
    /// Equipment value after.
    pub new: i32,
}

/// Round-kill counter changed (any direction, including reset to 0).
#[derive(Clone, Debug, PartialEq)]
pub struct PlayerRoundKillsChanged {
    /// Affected player.
    pub player: Player,
    /// Round kills before.
    pub previous: i32,
    /// Round kills after.
    pub new: i32,
}

/// Round headshot-kill counter changed.
#[derive(Clone, Debug, PartialEq)]
pub struct PlayerRoundHeadshotKillsChanged {
    /// Affected player.
    pub player: Player,
    /// Headshot kills before.
    pub previous: i32,
    /// Headshot kills after.
    pub new: i32,
}

/// Round total-damage counter changed.
#[derive(Clone, Debug, PartialEq)]
pub struct PlayerRoundTotalDamageChanged {
    /// Affected player.
    pub player: Player,
    /// Round damage before.
    pub previous: i32,
    /// Round damage after.
    pub new: i32,
}

/// Player's [`PlayerActivity`] changed (e.g. began typing, opened menu).
#[derive(Clone, Debug, PartialEq)]
pub struct PlayerActivityChanged {
    /// Affected player.
    pub player: Player,
    /// Activity before.
    pub previous: PlayerActivity,
    /// Activity after.
    pub new: PlayerActivity,
}

/// Player switched sides (e.g. half-time swap, joined).
#[derive(Clone, Debug, PartialEq)]
pub struct PlayerTeamChanged {
    /// Affected player.
    pub player: Player,
    /// Team before.
    pub previous: PlayerTeam,
    /// Team after.
    pub new: PlayerTeam,
}

/// Player's actively held weapon changed.
#[derive(Clone, Debug, PartialEq)]
pub struct PlayerActiveWeaponChanged {
    /// Affected player.
    pub player: Player,
    /// Previous active weapon name.
    pub previous: Option<String>,
    /// Current active weapon name.
    pub new: Option<String>,
}

/// Per-match stats changed.
#[derive(Clone, Debug, PartialEq)]
pub struct PlayerStatsChanged {
    /// Affected player.
    pub player: Player,
    /// Stats before.
    pub previous: MatchStats,
    /// Stats after.
    pub new: MatchStats,
}

// ------------------------- Round / Match events -----------------------------

/// Any field of `round` changed.
#[derive(Clone, Debug, PartialEq)]
pub struct RoundUpdated {
    /// Previous round phase.
    pub previous_phase: RoundPhase,
    /// New round phase.
    pub new_phase: RoundPhase,
}

/// Round phase transitioned (`freezetime` → `live` → `over`).
#[derive(Clone, Debug, PartialEq)]
pub struct RoundPhaseUpdated {
    /// Previous round phase.
    pub previous: RoundPhase,
    /// New round phase.
    pub new: RoundPhase,
}

/// Round entered the `live` phase.
#[derive(Clone, Debug, PartialEq)]
pub struct RoundStarted;

/// Round entered the `over` phase.
#[derive(Clone, Debug, PartialEq)]
pub struct RoundConcluded {
    /// Side that won the round.
    pub winning_team: WinningTeam,
}

/// Round freezetime began.
#[derive(Clone, Debug, PartialEq)]
pub struct FreezetimeStarted;

/// Round freezetime ended.
#[derive(Clone, Debug, PartialEq)]
pub struct FreezetimeOver;

/// Match transitioned out of warmup into live play.
#[derive(Clone, Debug, PartialEq)]
pub struct MatchStarted;

/// Match ended.
#[derive(Clone, Debug, PartialEq)]
pub struct GameOver;

/// Bomb-state field of `round.bomb` changed.
#[derive(Clone, Debug, PartialEq)]
pub struct BombStateUpdated {
    /// State before.
    pub previous: BombRoundState,
    /// State after.
    pub new: BombRoundState,
}

// ------------------------- Map events ---------------------------------------

/// Any field of the `map` block changed.
#[derive(Clone, Debug, PartialEq)]
pub struct MapUpdated;

/// `map.mode` changed (e.g. competitive → wingman).
#[derive(Clone, Debug, PartialEq)]
pub struct GamemodeChanged {
    /// Previous mode string.
    pub previous: String,
    /// New mode string.
    pub new: String,
}

/// `map.name` changed.
#[derive(Clone, Debug, PartialEq)]
pub struct LevelChanged {
    /// Previous map filename.
    pub previous: String,
    /// New map filename.
    pub new: String,
}

/// Map phase changed.
#[derive(Clone, Debug, PartialEq)]
pub struct MapPhaseChanged {
    /// Previous map phase.
    pub previous: MapPhase,
    /// New map phase.
    pub new: MapPhase,
}

/// Either side's score changed.
#[derive(Clone, Debug, PartialEq)]
pub struct TeamScoreChanged {
    /// Side whose score changed.
    pub team: PlayerTeam,
    /// Score before.
    pub previous: u32,
    /// Score after.
    pub new: u32,
}

// ------------------------- Bomb (root-level) --------------------------------

/// Any field of the root `bomb` node changed.
#[derive(Clone, Debug, PartialEq)]
pub struct BombUpdated;

/// Bomb is being planted (planting animation in progress).
#[derive(Clone, Debug, PartialEq)]
pub struct BombPlanting;

/// Bomb has been planted and is counting down.
#[derive(Clone, Debug, PartialEq)]
pub struct BombPlanted;

/// Bomb is being defused.
#[derive(Clone, Debug, PartialEq)]
pub struct BombDefusing;

/// Bomb has been defused.
#[derive(Clone, Debug, PartialEq)]
pub struct BombDefused;

/// Bomb has exploded.
#[derive(Clone, Debug, PartialEq)]
pub struct BombExploded;

/// Bomb was dropped on the ground.
#[derive(Clone, Debug, PartialEq)]
pub struct BombDropped;

/// Bomb was picked up by a T player.
#[derive(Clone, Debug, PartialEq)]
pub struct BombPickedUp;

// ------------------------- Killfeed (synthesised) ---------------------------

/// A kill was attributed end-to-end. Synthesised by correlating
/// [`PlayerDied`] with [`PlayerGotKill`] within the same diff window.
#[derive(Clone, Debug, PartialEq)]
pub struct KillFeed {
    /// Player who got the kill.
    pub killer: Player,
    /// Player who died.
    pub victim: Player,
    /// Weapon used (best-effort — based on the killer's active weapon).
    pub weapon: Option<String>,
    /// `true` if the kill was a headshot.
    pub is_headshot: bool,
}

// ------------------------- Meta events --------------------------------------

/// Auth block in the cfg file changed (rarely fires — auth is set once).
#[derive(Clone, Debug, PartialEq)]
pub struct AuthUpdated;

/// Provider info changed (CS2 client version / appid).
#[derive(Clone, Debug, PartialEq)]
pub struct ProviderUpdated {
    /// Provider before.
    pub previous: Provider,
    /// Provider after.
    pub new: Provider,
}

/// A new GameState payload was received and parsed. Always fires before any
/// derived events.
#[derive(Clone, Debug, PartialEq)]
pub struct NewGameState {
    /// The freshly parsed state.
    pub state: GameState,
}

// ------------------------- Catch-all enum -----------------------------------/// Catch-all enum useful for storing heterogeneous events in a queue or
/// matching on every variant in one place. The dedicated `on_xxx` methods on
/// [`GameStateListener`](crate::GameStateListener) accept the strongly typed
/// event structs directly — most users should prefer those.
///
/// `NewGameState` carries a full [`GameState`] and is therefore boxed so the
/// total enum size stays small.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
#[allow(missing_docs)] // each variant is a thin wrapper around a documented struct of the same name
pub enum GameEvent {
    NewGameState(Box<NewGameState>),
    AuthUpdated(AuthUpdated),
    ProviderUpdated(ProviderUpdated),
    MapUpdated(MapUpdated),
    GamemodeChanged(GamemodeChanged),
    LevelChanged(LevelChanged),
    MapPhaseChanged(MapPhaseChanged),
    TeamScoreChanged(TeamScoreChanged),
    RoundUpdated(RoundUpdated),
    RoundPhaseUpdated(RoundPhaseUpdated),
    RoundStarted(RoundStarted),
    RoundConcluded(RoundConcluded),
    FreezetimeStarted(FreezetimeStarted),
    FreezetimeOver(FreezetimeOver),
    MatchStarted(MatchStarted),
    GameOver(GameOver),
    BombStateUpdated(BombStateUpdated),
    BombUpdated(BombUpdated),
    BombPlanting(BombPlanting),
    BombPlanted(BombPlanted),
    BombDefusing(BombDefusing),
    BombDefused(BombDefused),
    BombExploded(BombExploded),
    BombDropped(BombDropped),
    BombPickedUp(BombPickedUp),
    PlayerUpdated(PlayerUpdated),
    PlayerDied(PlayerDied),
    PlayerRespawned(PlayerRespawned),
    PlayerTookDamage(PlayerTookDamage),
    PlayerGotKill(PlayerGotKill),
    PlayerHealthChanged(PlayerHealthChanged),
    PlayerArmorChanged(PlayerArmorChanged),
    PlayerHelmetChanged(PlayerHelmetChanged),
    PlayerFlashAmountChanged(PlayerFlashAmountChanged),
    PlayerSmokedAmountChanged(PlayerSmokedAmountChanged),
    PlayerBurningAmountChanged(PlayerBurningAmountChanged),
    PlayerMoneyAmountChanged(PlayerMoneyAmountChanged),
    PlayerEquipmentValueChanged(PlayerEquipmentValueChanged),
    PlayerRoundKillsChanged(PlayerRoundKillsChanged),
    PlayerRoundHeadshotKillsChanged(PlayerRoundHeadshotKillsChanged),
    PlayerRoundTotalDamageChanged(PlayerRoundTotalDamageChanged),
    PlayerActivityChanged(PlayerActivityChanged),
    PlayerTeamChanged(PlayerTeamChanged),
    PlayerActiveWeaponChanged(PlayerActiveWeaponChanged),
    PlayerStatsChanged(PlayerStatsChanged),
    KillFeed(KillFeed),
}

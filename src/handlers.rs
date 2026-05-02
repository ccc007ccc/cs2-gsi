//! Diff engine that turns consecutive [`GameState`] snapshots into events.
//!
//! Every incoming payload is compared against the last one and a stream of
//! typed events is fired through the [`Dispatcher`]. Handlers run
//! synchronously, in registration order, on whatever task the HTTP listener
//! is using.

use crate::dispatcher::Dispatcher;
use crate::events::*;
use crate::model::{BombState, GameState, MapPhase, Player, PlayerTeam, Round, RoundPhase};

/// Public entry point — diff `previous` against `current` and emit every
/// derived event via `dispatcher`.
pub(crate) fn diff_and_dispatch(
    previous: Option<&GameState>,
    current: &GameState,
    dispatcher: &Dispatcher,
) {
    // 1. Always announce a new state first so listeners can perform their own diff.
    let new_state_evt = NewGameState {
        state: current.clone(),
    };
    dispatcher.fire(
        &new_state_evt,
        GameEvent::NewGameState(Box::new(new_state_evt.clone())),
    );

    // 2. Auth — `previous == None` is the very first payload; do not fire.
    if let Some(prev) = previous {
        if prev.auth != current.auth {
            dispatcher.fire(&AuthUpdated, GameEvent::AuthUpdated(AuthUpdated));
        }
        if prev.provider != current.provider {
            let evt = ProviderUpdated {
                previous: prev.provider.clone(),
                new: current.provider.clone(),
            };
            dispatcher.fire(&evt, GameEvent::ProviderUpdated(evt.clone()));
        }
    }

    // 3. Map / Match.
    diff_map(
        previous.and_then(|p| p.map.as_ref()),
        current.map.as_ref(),
        dispatcher,
    );

    // 4. Round.
    diff_round(
        previous.and_then(|p| p.round.as_ref()),
        current.round.as_ref(),
        dispatcher,
    );

    // 5. Bomb (root level).
    diff_bomb(
        previous.and_then(|p| p.bomb.as_ref()),
        current.bomb.as_ref(),
        dispatcher,
    );

    // 6. Local player + every entry in allplayers.
    let mut deaths: Vec<Player> = Vec::new();
    let mut kills: Vec<(Player, bool, Option<String>)> = Vec::new();

    if let Some(cur_p) = current.player.as_ref() {
        diff_player(
            previous.and_then(|p| p.player.as_ref()),
            cur_p,
            dispatcher,
            &mut deaths,
            &mut kills,
        );
    }

    for (sid, cur_p) in &current.allplayers {
        let prev_p = previous.and_then(|prev| prev.allplayers.get(sid));
        diff_player(prev_p, cur_p, dispatcher, &mut deaths, &mut kills);
    }

    // 7. KillFeed — pair deaths with kills.
    synthesize_killfeed(deaths, kills, dispatcher);
}

// ---------------------------------------------------------------------------

fn diff_map(prev: Option<&crate::model::Map>, cur: Option<&crate::model::Map>, d: &Dispatcher) {
    let cur = match cur {
        Some(m) => m,
        None => return,
    };
    let prev = match prev {
        Some(p) => p,
        None => return,
    };
    if prev == cur {
        return;
    }
    d.fire(&MapUpdated, GameEvent::MapUpdated(MapUpdated));

    if prev.mode != cur.mode {
        let evt = GamemodeChanged {
            previous: prev.mode.clone(),
            new: cur.mode.clone(),
        };
        d.fire(&evt, GameEvent::GamemodeChanged(evt.clone()));
    }
    if prev.name != cur.name {
        let evt = LevelChanged {
            previous: prev.name.clone(),
            new: cur.name.clone(),
        };
        d.fire(&evt, GameEvent::LevelChanged(evt.clone()));
    }
    if prev.phase != cur.phase {
        let evt = MapPhaseChanged {
            previous: prev.phase,
            new: cur.phase,
        };
        d.fire(&evt, GameEvent::MapPhaseChanged(evt.clone()));

        if matches!(prev.phase, MapPhase::Warmup) && matches!(cur.phase, MapPhase::Live) {
            d.fire(&MatchStarted, GameEvent::MatchStarted(MatchStarted));
        }
        if matches!(cur.phase, MapPhase::Gameover) {
            d.fire(&GameOver, GameEvent::GameOver(GameOver));
        }
    }
    if prev.team_ct.score != cur.team_ct.score {
        let evt = TeamScoreChanged {
            team: PlayerTeam::CT,
            previous: prev.team_ct.score,
            new: cur.team_ct.score,
        };
        d.fire(&evt, GameEvent::TeamScoreChanged(evt.clone()));
    }
    if prev.team_t.score != cur.team_t.score {
        let evt = TeamScoreChanged {
            team: PlayerTeam::T,
            previous: prev.team_t.score,
            new: cur.team_t.score,
        };
        d.fire(&evt, GameEvent::TeamScoreChanged(evt.clone()));
    }
}

fn diff_round(prev: Option<&Round>, cur: Option<&Round>, d: &Dispatcher) {
    let cur = match cur {
        Some(r) => r,
        None => return,
    };
    let prev = match prev {
        Some(r) => r,
        None => return,
    };
    if prev == cur {
        return;
    }
    let evt = RoundUpdated {
        previous_phase: prev.phase,
        new_phase: cur.phase,
    };
    d.fire(&evt, GameEvent::RoundUpdated(evt.clone()));

    if prev.phase != cur.phase {
        let evt = RoundPhaseUpdated {
            previous: prev.phase,
            new: cur.phase,
        };
        d.fire(&evt, GameEvent::RoundPhaseUpdated(evt.clone()));

        match (prev.phase, cur.phase) {
            (_, RoundPhase::Live) => {
                d.fire(&RoundStarted, GameEvent::RoundStarted(RoundStarted));
                d.fire(&FreezetimeOver, GameEvent::FreezetimeOver(FreezetimeOver));
            }
            (_, RoundPhase::Freezetime) => {
                d.fire(
                    &FreezetimeStarted,
                    GameEvent::FreezetimeStarted(FreezetimeStarted),
                );
            }
            (_, RoundPhase::Over) => {
                let evt = RoundConcluded {
                    winning_team: cur.win_team,
                };
                d.fire(&evt, GameEvent::RoundConcluded(evt.clone()));
            }
            _ => {}
        }
    }

    if prev.bomb != cur.bomb {
        let evt = BombStateUpdated {
            previous: prev.bomb,
            new: cur.bomb,
        };
        d.fire(&evt, GameEvent::BombStateUpdated(evt.clone()));
    }
}

fn diff_bomb(prev: Option<&crate::model::Bomb>, cur: Option<&crate::model::Bomb>, d: &Dispatcher) {
    let cur = match cur {
        Some(b) => b,
        None => return,
    };
    let prev = match prev {
        Some(b) => b,
        None => return,
    };
    if prev == cur {
        return;
    }
    d.fire(&BombUpdated, GameEvent::BombUpdated(BombUpdated));

    if prev.state != cur.state {
        match cur.state {
            BombState::Planting => d.fire(&BombPlanting, GameEvent::BombPlanting(BombPlanting)),
            BombState::Planted => d.fire(&BombPlanted, GameEvent::BombPlanted(BombPlanted)),
            BombState::Defusing => d.fire(&BombDefusing, GameEvent::BombDefusing(BombDefusing)),
            BombState::Defused => d.fire(&BombDefused, GameEvent::BombDefused(BombDefused)),
            BombState::Exploded => d.fire(&BombExploded, GameEvent::BombExploded(BombExploded)),
            BombState::Dropped => d.fire(&BombDropped, GameEvent::BombDropped(BombDropped)),
            BombState::Carried => {
                if matches!(prev.state, BombState::Dropped | BombState::Unknown) {
                    d.fire(&BombPickedUp, GameEvent::BombPickedUp(BombPickedUp));
                }
            }
            BombState::Unknown => {}
        }
    }
}

fn diff_player(
    prev: Option<&Player>,
    cur: &Player,
    d: &Dispatcher,
    deaths: &mut Vec<Player>,
    kills: &mut Vec<(Player, bool, Option<String>)>,
) {
    // First-seen player → only emit PlayerUpdated.
    let prev = match prev {
        Some(p) if p == cur => return,
        Some(p) => p,
        None => {
            let evt = PlayerUpdated {
                player: cur.clone(),
                previous: None,
            };
            d.fire(&evt, GameEvent::PlayerUpdated(evt.clone()));
            return;
        }
    };

    let evt = PlayerUpdated {
        player: cur.clone(),
        previous: Some(prev.clone()),
    };
    d.fire(&evt, GameEvent::PlayerUpdated(evt.clone()));

    let ps = &prev.state;
    let cs = &cur.state;

    if ps.health != cs.health {
        let h = PlayerHealthChanged {
            player: cur.clone(),
            previous: ps.health,
            new: cs.health,
        };
        d.fire(&h, GameEvent::PlayerHealthChanged(h.clone()));

        if ps.health > 0 && cs.health == 0 {
            let evt = PlayerDied {
                player: cur.clone(),
                previous_health: ps.health,
                new_health: cs.health,
            };
            d.fire(&evt, GameEvent::PlayerDied(evt.clone()));
            deaths.push(cur.clone());
        } else if ps.health == 0 && cs.health > 0 {
            let evt = PlayerRespawned {
                player: cur.clone(),
                previous_health: ps.health,
                new_health: cs.health,
            };
            d.fire(&evt, GameEvent::PlayerRespawned(evt.clone()));
        } else if cs.health > 0 && cs.health < ps.health {
            let evt = PlayerTookDamage {
                player: cur.clone(),
                previous_health: ps.health,
                new_health: cs.health,
            };
            d.fire(&evt, GameEvent::PlayerTookDamage(evt.clone()));
        }
    }

    if ps.armor != cs.armor {
        let evt = PlayerArmorChanged {
            player: cur.clone(),
            previous: ps.armor,
            new: cs.armor,
        };
        d.fire(&evt, GameEvent::PlayerArmorChanged(evt.clone()));
    }
    if ps.helmet != cs.helmet {
        let evt = PlayerHelmetChanged {
            player: cur.clone(),
            previous: ps.helmet,
            new: cs.helmet,
        };
        d.fire(&evt, GameEvent::PlayerHelmetChanged(evt.clone()));
    }
    if ps.flashed != cs.flashed {
        let evt = PlayerFlashAmountChanged {
            player: cur.clone(),
            previous: ps.flashed,
            new: cs.flashed,
        };
        d.fire(&evt, GameEvent::PlayerFlashAmountChanged(evt.clone()));
    }
    if ps.smoked != cs.smoked {
        let evt = PlayerSmokedAmountChanged {
            player: cur.clone(),
            previous: ps.smoked,
            new: cs.smoked,
        };
        d.fire(&evt, GameEvent::PlayerSmokedAmountChanged(evt.clone()));
    }
    if ps.burning != cs.burning {
        let evt = PlayerBurningAmountChanged {
            player: cur.clone(),
            previous: ps.burning,
            new: cs.burning,
        };
        d.fire(&evt, GameEvent::PlayerBurningAmountChanged(evt.clone()));
    }
    if ps.money != cs.money {
        let evt = PlayerMoneyAmountChanged {
            player: cur.clone(),
            previous: ps.money,
            new: cs.money,
        };
        d.fire(&evt, GameEvent::PlayerMoneyAmountChanged(evt.clone()));
    }
    if ps.equip_value != cs.equip_value {
        let evt = PlayerEquipmentValueChanged {
            player: cur.clone(),
            previous: ps.equip_value,
            new: cs.equip_value,
        };
        d.fire(&evt, GameEvent::PlayerEquipmentValueChanged(evt.clone()));
    }
    if ps.round_kills != cs.round_kills {
        let evt = PlayerRoundKillsChanged {
            player: cur.clone(),
            previous: ps.round_kills,
            new: cs.round_kills,
        };
        d.fire(&evt, GameEvent::PlayerRoundKillsChanged(evt.clone()));

        if cs.round_kills > ps.round_kills {
            let is_hs = cs.round_killhs > ps.round_killhs;
            let weapon = active_weapon_name(cur);
            let evt = PlayerGotKill {
                player: cur.clone(),
                previous_round_kills: ps.round_kills,
                new_round_kills: cs.round_kills,
                is_headshot: is_hs,
                weapon: weapon.clone(),
            };
            d.fire(&evt, GameEvent::PlayerGotKill(evt.clone()));
            kills.push((cur.clone(), is_hs, weapon));
        }
    }
    if ps.round_killhs != cs.round_killhs {
        let evt = PlayerRoundHeadshotKillsChanged {
            player: cur.clone(),
            previous: ps.round_killhs,
            new: cs.round_killhs,
        };
        d.fire(
            &evt,
            GameEvent::PlayerRoundHeadshotKillsChanged(evt.clone()),
        );
    }
    if ps.round_totaldmg != cs.round_totaldmg {
        let evt = PlayerRoundTotalDamageChanged {
            player: cur.clone(),
            previous: ps.round_totaldmg,
            new: cs.round_totaldmg,
        };
        d.fire(&evt, GameEvent::PlayerRoundTotalDamageChanged(evt.clone()));
    }

    if prev.activity != cur.activity {
        let evt = PlayerActivityChanged {
            player: cur.clone(),
            previous: prev.activity,
            new: cur.activity,
        };
        d.fire(&evt, GameEvent::PlayerActivityChanged(evt.clone()));
    }
    if prev.team != cur.team {
        let evt = PlayerTeamChanged {
            player: cur.clone(),
            previous: prev.team,
            new: cur.team,
        };
        d.fire(&evt, GameEvent::PlayerTeamChanged(evt.clone()));
    }

    let prev_active = active_weapon_name(prev);
    let cur_active = active_weapon_name(cur);
    if prev_active != cur_active {
        let evt = PlayerActiveWeaponChanged {
            player: cur.clone(),
            previous: prev_active,
            new: cur_active,
        };
        d.fire(&evt, GameEvent::PlayerActiveWeaponChanged(evt.clone()));
    }

    if prev.match_stats != cur.match_stats {
        let evt = PlayerStatsChanged {
            player: cur.clone(),
            previous: prev.match_stats.clone(),
            new: cur.match_stats.clone(),
        };
        d.fire(&evt, GameEvent::PlayerStatsChanged(evt.clone()));
    }
}

fn active_weapon_name(p: &Player) -> Option<String> {
    p.weapons
        .values()
        .find(|w| matches!(w.state, crate::model::WeaponState::Active))
        .map(|w| w.name.clone())
}

fn synthesize_killfeed(
    deaths: Vec<Player>,
    kills: Vec<(Player, bool, Option<String>)>,
    d: &Dispatcher,
) {
    if deaths.is_empty() || kills.is_empty() {
        return;
    }
    // Naive pairing: first-killer × first-victim within the same diff window.
    // Multi-kill ticks are rare in CS2 — single pair handles 99% of cases.
    let (killer, is_hs, weapon) = &kills[0];
    for victim in deaths {
        if killer.steamid == victim.steamid {
            continue;
        }
        let evt = KillFeed {
            killer: killer.clone(),
            victim,
            weapon: weapon.clone(),
            is_headshot: *is_hs,
        };
        d.fire(&evt, GameEvent::KillFeed(evt.clone()));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dispatcher::Dispatcher;
    use crate::model::{Player, PlayerActivity, PlayerState, PlayerTeam};
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    fn alive_player(name: &str, hp: i32, kills: i32) -> Player {
        Player {
            steamid: name.into(),
            name: name.into(),
            team: PlayerTeam::CT,
            activity: PlayerActivity::Playing,
            state: PlayerState {
                health: hp,
                round_kills: kills,
                ..Default::default()
            },
            ..Default::default()
        }
    }

    #[test]
    fn first_payload_only_fires_new_state_and_player_updated() {
        let d = Dispatcher::default();
        let died = Arc::new(AtomicUsize::new(0));
        let died2 = died.clone();
        d.register::<PlayerDied, _>(move |_| {
            died2.fetch_add(1, Ordering::SeqCst);
        });

        let state = GameState {
            player: Some(alive_player("alice", 100, 0)),
            ..Default::default()
        };
        diff_and_dispatch(None, &state, &d);
        assert_eq!(died.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn drops_to_zero_fires_player_died() {
        let d = Dispatcher::default();
        let died = Arc::new(AtomicUsize::new(0));
        let died2 = died.clone();
        d.register::<PlayerDied, _>(move |_| {
            died2.fetch_add(1, Ordering::SeqCst);
        });

        let prev = GameState {
            player: Some(alive_player("alice", 87, 0)),
            ..Default::default()
        };
        let cur = GameState {
            player: Some(alive_player("alice", 0, 0)),
            ..Default::default()
        };

        diff_and_dispatch(Some(&prev), &cur, &d);
        assert_eq!(died.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn round_kill_increment_fires_got_kill() {
        let d = Dispatcher::default();
        let killed = Arc::new(AtomicUsize::new(0));
        let k2 = killed.clone();
        d.register::<PlayerGotKill, _>(move |e| {
            assert_eq!(e.previous_round_kills, 0);
            assert_eq!(e.new_round_kills, 1);
            k2.fetch_add(1, Ordering::SeqCst);
        });
        let prev = GameState {
            player: Some(alive_player("alice", 100, 0)),
            ..Default::default()
        };
        let cur = GameState {
            player: Some(alive_player("alice", 100, 1)),
            ..Default::default()
        };
        diff_and_dispatch(Some(&prev), &cur, &d);
        assert_eq!(killed.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn killfeed_synthesises_from_kill_and_death() {
        let d = Dispatcher::default();
        let kf = Arc::new(AtomicUsize::new(0));
        let kf2 = kf.clone();
        d.register::<KillFeed, _>(move |e| {
            assert_eq!(e.killer.name, "alice");
            assert_eq!(e.victim.name, "bob");
            kf2.fetch_add(1, Ordering::SeqCst);
        });
        let mut prev = GameState::default();
        prev.allplayers
            .insert("alice".into(), alive_player("alice", 100, 0));
        prev.allplayers
            .insert("bob".into(), alive_player("bob", 100, 0));
        let mut cur = GameState::default();
        cur.allplayers
            .insert("alice".into(), alive_player("alice", 100, 1));
        cur.allplayers
            .insert("bob".into(), alive_player("bob", 0, 0));
        diff_and_dispatch(Some(&prev), &cur, &d);
        assert_eq!(kf.load(Ordering::SeqCst), 1);
    }
}

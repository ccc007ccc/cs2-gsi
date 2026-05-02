//! Integration tests: parse real-shaped GSI payloads and verify diff events
//! fire in the expected order.

use cs2_gsi::events::{PlayerDied, PlayerGotKill, RoundPhaseUpdated};
use cs2_gsi::GameStateListener;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

const SAMPLE_LIVE_ROUND: &str = include_str!("../fixtures/sample_live_round.json");

#[test]
fn fixture_parses_into_full_gamestate() {
    let state = cs2_gsi::GameState::from_str(SAMPLE_LIVE_ROUND).unwrap();
    assert_eq!(state.provider.appid, 730);
    let map = state.map.as_ref().unwrap();
    assert_eq!(map.name, "de_dust2");
    assert_eq!(map.team_ct.score, 3);
    let player = state.player.as_ref().unwrap();
    assert_eq!(player.name, "alice");
    assert_eq!(player.state.health, 100);
    assert_eq!(player.match_stats.kills, 5);
    assert!(player.weapons.contains_key("weapon_0"));
}

#[tokio::test]
async fn end_to_end_round_phase_transitions() {
    let listener = GameStateListener::with_addr(SocketAddr::from(([127, 0, 0, 1], 0)));
    let phase_updates = Arc::new(AtomicUsize::new(0));
    let p2 = phase_updates.clone();
    listener.on(move |e: &RoundPhaseUpdated| {
        // freezetime → live → over → freezetime → ... — count each transition.
        assert_ne!(e.previous, e.new);
        p2.fetch_add(1, Ordering::SeqCst);
    });

    listener.start().await.unwrap();
    let url = format!("http://{}/", listener.actual_addr().unwrap());
    let client = reqwest::Client::new();

    let payloads = [
        r#"{"round":{"phase":"freezetime"}}"#,
        r#"{"round":{"phase":"live"}}"#,
        r#"{"round":{"phase":"over","win_team":"CT"}}"#,
        r#"{"round":{"phase":"freezetime"}}"#,
    ];
    for body in payloads {
        client
            .post(&url)
            .body(body.to_string())
            .send()
            .await
            .unwrap();
    }

    tokio::time::sleep(Duration::from_millis(100)).await;
    listener.stop().await.unwrap();

    // 3 transitions between 4 payloads.
    assert_eq!(phase_updates.load(Ordering::SeqCst), 3);
}

#[tokio::test]
async fn end_to_end_kill_then_death() {
    let listener = GameStateListener::with_addr(SocketAddr::from(([127, 0, 0, 1], 0)));
    let kills = Arc::new(AtomicUsize::new(0));
    let deaths = Arc::new(AtomicUsize::new(0));
    let k2 = kills.clone();
    let d2 = deaths.clone();
    listener.on(move |_: &PlayerGotKill| {
        k2.fetch_add(1, Ordering::SeqCst);
    });
    listener.on(move |_: &PlayerDied| {
        d2.fetch_add(1, Ordering::SeqCst);
    });

    listener.start().await.unwrap();
    let url = format!("http://{}/", listener.actual_addr().unwrap());
    let c = reqwest::Client::new();

    let alive = r#"{"allplayers":{"alice":{"name":"alice","team":"CT","state":{"health":"100","round_kills":"0","round_killhs":"0"}},"bob":{"name":"bob","team":"T","state":{"health":"100","round_kills":"0","round_killhs":"0"}}}}"#;
    let alice_kills_bob = r#"{"allplayers":{"alice":{"name":"alice","team":"CT","state":{"health":"100","round_kills":"1","round_killhs":"1"}},"bob":{"name":"bob","team":"T","state":{"health":"0","round_kills":"0","round_killhs":"0"}}}}"#;
    c.post(&url).body(alive.to_string()).send().await.unwrap();
    c.post(&url)
        .body(alice_kills_bob.to_string())
        .send()
        .await
        .unwrap();

    tokio::time::sleep(Duration::from_millis(100)).await;
    listener.stop().await.unwrap();
    assert_eq!(kills.load(Ordering::SeqCst), 1);
    assert_eq!(deaths.load(Ordering::SeqCst), 1);
}

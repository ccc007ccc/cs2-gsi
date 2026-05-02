# cs2-gsi

[![crates.io](https://img.shields.io/crates/v/cs2-gsi.svg)](https://crates.io/crates/cs2-gsi)
[![docs.rs](https://docs.rs/cs2-gsi/badge.svg)](https://docs.rs/cs2-gsi)
[![CI](https://github.com/ccc007ccc/cs2-gsi/actions/workflows/ci.yml/badge.svg)](https://github.com/ccc007ccc/cs2-gsi/actions)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](#license)

**English** · [简体中文](README.zh-CN.md)

Async **Counter-Strike 2 Game State Integration** listener for Rust.

> A Rust port (and re-design) of [`antonpup/CounterStrike2GSI`][upstream].

```rust
use cs2_gsi::{cfg::GsiCfg, events::PlayerDied, GameStateListener};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    GsiCfg::for_localhost("MyApp", 4000).write_to_cs2()?;

    let gsl = GameStateListener::new(4000);
    gsl.on(|e: &PlayerDied| println!("☠ {} died", e.player.name));
    gsl.start().await?;

    tokio::signal::ctrl_c().await?;
    gsl.stop().await?;
    Ok(())
}
```

---

## Why this crate?

CS2 ships a [Game State Integration][gsi-docs] feature that POSTs JSON
documents about the live match to whichever HTTP endpoint you configure.
Doing anything useful with those payloads requires four pieces:

1. An HTTP listener.
2. A model that mirrors the JSON CS2 actually sends (with all its quirks —
   numeric fields encoded as strings, casing inconsistencies, optional sub-objects).
3. A diff engine that turns *snapshots* into actionable events
   (`PlayerDied`, `BombPlanted`, `RoundConcluded`, `KillFeed`, ...).
4. A way to drop the right `gamestate_integration_*.cfg` into the right place
   so CS2 actually starts pushing.

`cs2-gsi` does all four — typed, async, and with a single clean API surface.

| Feature                                    | Status |
|--------------------------------------------|:-:|
| HTTP listener (hyper 1.x + tokio)          | ✅ |
| Strongly typed `GameState` model           | ✅ |
| 40+ derived events with `previous`/`new`   | ✅ |
| Synthesised `KillFeed` events              | ✅ |
| Auto-write `gamestate_integration_*.cfg`   | ✅ |
| Steam library + `appmanifest_730.acf` discovery (Win/Linux/macOS) | ✅ |
| `auth { token "..." }` blocks              | ✅ |
| Graceful start/stop, hot handler registration | ✅ |

---

## Install

```toml
[dependencies]
cs2-gsi = "0.1"
tokio   = { version = "1", features = ["macros", "rt-multi-thread", "signal"] }
```

Cargo features (all enabled by default):

| Feature           | Purpose |
|-------------------|---------|
| `cfg-writer`      | `GsiCfg` builder + writer for `gamestate_integration_*.cfg` |
| `steam-discover`  | Locate the CS2 install through Steam's `libraryfolders.vdf` |

Disable both for a tiny build that only contains the HTTP listener and the
data model:

```toml
cs2-gsi = { version = "0.1", default-features = false }
```

---

## Architecture

```text
┌────────────┐   POST JSON    ┌──────────────────┐
│ CS2 client │───────────────▶│ GameStateListener│
└────────────┘                │   (this crate)   │
                              │   • hyper server │
                              │   • parse → State│
                              │   • diff w/ prev │
                              └────────┬─────────┘
                                       │ typed events
              ┌────────────────────────┼─────────────────────────┐
              ▼                        ▼                         ▼
       PlayerDied{..}          RoundPhaseUpdated{..}      KillFeed{killer,victim,..}
       PlayerGotKill{..}       BombPlanted{..}            ...
```

Every payload is parsed into a [`GameState`], compared against the previous
one, and turned into a stream of typed events. Handlers fire **synchronously**
on the listener's tokio task (matching the upstream library's behaviour) — if
your handler needs to do heavy work, `tokio::spawn` from inside it.

---

## Subscribing to events

```rust
use cs2_gsi::{events::*, GameEvent, GameStateListener};

let gsl = GameStateListener::new(4000);

// Strongly typed callbacks.
gsl.on(|e: &PlayerDied| { /* ... */ });
gsl.on(|e: &BombPlanted| { /* ... */ });
gsl.on(|e: &KillFeed| {
    println!(
        "{} ({}{}) killed {}",
        e.killer.name,
        e.weapon.as_deref().unwrap_or("unknown"),
        if e.is_headshot { ", HS" } else { "" },
        e.victim.name,
    );
});

// Or one catch-all handler for every event:
gsl.on_any(|evt: &GameEvent| match evt {
    GameEvent::RoundPhaseUpdated(p) => println!("phase {:?} → {:?}", p.previous, p.new),
    _ => {}
});
```

A non-exhaustive list of events the diff engine fires:

- **Player**: `PlayerUpdated`, `PlayerDied`, `PlayerRespawned`,
  `PlayerTookDamage`, `PlayerGotKill`, `PlayerHealthChanged`,
  `PlayerArmorChanged`, `PlayerHelmetChanged`, `PlayerFlashAmountChanged`,
  `PlayerSmokedAmountChanged`, `PlayerBurningAmountChanged`,
  `PlayerMoneyAmountChanged`, `PlayerEquipmentValueChanged`,
  `PlayerRoundKillsChanged`, `PlayerRoundHeadshotKillsChanged`,
  `PlayerRoundTotalDamageChanged`, `PlayerActivityChanged`,
  `PlayerTeamChanged`, `PlayerActiveWeaponChanged`, `PlayerStatsChanged`
- **Round / Match**: `RoundUpdated`, `RoundPhaseUpdated`, `RoundStarted`,
  `RoundConcluded`, `FreezetimeStarted`, `FreezetimeOver`, `MatchStarted`,
  `GameOver`, `BombStateUpdated`
- **Map**: `MapUpdated`, `GamemodeChanged`, `LevelChanged`, `MapPhaseChanged`,
  `TeamScoreChanged`
- **Bomb (root)**: `BombUpdated`, `BombPlanting`, `BombPlanted`,
  `BombDefusing`, `BombDefused`, `BombExploded`, `BombDropped`,
  `BombPickedUp`
- **Synthesised**: `KillFeed` (kill ↔ death pairing within one diff window)
- **Meta**: `NewGameState`, `AuthUpdated`, `ProviderUpdated`

---

## Generating the cfg file

CS2 has to be told where to send payloads. `cs2-gsi` builds an integration
file identical in shape to the upstream CounterStrike2GSI library:

```rust
use cs2_gsi::cfg::GsiCfg;

// 1. Auto-discover and place into ...\Counter-Strike Global Offensive\game\csgo\cfg\
GsiCfg::for_localhost("MyApp", 4000)
    .with_auth("token", "secret-shared-with-myself")
    .write_to_cs2()?;

// 2. Or render to a string and place it yourself.
let kv = GsiCfg::for_localhost("MyApp", 4000).render();
println!("{kv}");
```

Output (snipped):

```text
"MyApp Integration Configuration"
{
    "uri"          "http://localhost:4000/"
    "timeout"      "5.0"
    "buffer"       "0.1"
    "throttle"     "0.1"
    "heartbeat"    "10.0"
    "data"
    {
        "allgrenades              "        "1"
        "allplayers_id            "        "1"
        "allplayers_match_stats   "        "1"
        ...
    }
}
```

---

## Examples

```bash
cargo run --example quickstart   # PlayerDied + round phase
cargo run --example full_dump    # every event, debug-printed
```

---

## MSRV & platforms

- **Rust 1.75** or newer.
- Tested on Windows 10/11 (primary target — that's where CS2 lives).
  Linux & macOS are supported for the listener / parser / Steam discovery,
  using `~/.steam/steam` and `~/Library/Application Support/Steam`
  respectively.

---

## Differences from upstream

`cs2-gsi` is **not** a 1:1 binding of the upstream C# library — it's a
re-implementation that targets idiomatic Rust:

| Upstream (C#)                          | This crate                              |
|----------------------------------------|-----------------------------------------|
| `event` / `+= handler`                 | `gsl.on(|e: &PlayerDied| ...)`          |
| `PascalCase` event types               | `PascalCase` types preserved            |
| 70+ events                             | ~45 events covering all common cases    |
| `EventDispatcher` calls handlers sync  | Same — handlers run sync on the listener task |
| `GenerateGSIConfigFile(name)`          | `GsiCfg::for_localhost(name, port).write_to_cs2()` |
| `.NET HttpListener` (single-threaded)  | `hyper` 1.x + `tokio` (multi-threaded accept) |
| Numeric fields awkward to consume      | Auto-coerced from JSON strings          |

If you need an upstream event that isn't yet exposed, open an issue —
adding more variants is mechanical (one diff branch, one event struct).

---

## License

Licensed under either of

- **Apache License, Version 2.0** ([LICENSE-APACHE](LICENSE-APACHE) or
  http://www.apache.org/licenses/LICENSE-2.0)
- **MIT license** ([LICENSE-MIT](LICENSE-MIT) or
  http://opensource.org/licenses/MIT)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally
submitted for inclusion in the work by you, as defined in the Apache-2.0
license, shall be dual-licensed as above, without any additional terms or
conditions.

## Acknowledgements

Inspired by and behaviour-compatible with [`antonpup/CounterStrike2GSI`][upstream].

[upstream]: https://github.com/antonpup/CounterStrike2GSI
[gsi-docs]: https://developer.valvesoftware.com/wiki/Counter-Strike_Global_Offensive_Game_State_Integration

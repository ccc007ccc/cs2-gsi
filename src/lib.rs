//! # cs2-gsi
//!
//! Counter-Strike 2 **Game State Integration** listener for Rust.
//!
//! - **Async, single dependency surface** — built on `tokio` + `hyper` 1.x.
//! - **Strongly typed** model and events (no `serde_json::Value` in your code).
//! - **Drop-in cfg writer** — generate the right `gamestate_integration_*.cfg`
//!   into the right place via Steam path discovery.
//! - **Event diffing** done for you: subscribe to `PlayerDied`,
//!   `BombPlanted`, `RoundPhaseUpdated`, `KillFeed`, … instead of comparing
//!   payloads by hand.
//!
//! ## Quick start
//!
//! ```no_run
//! use cs2_gsi::{events::PlayerDied, cfg::GsiCfg, GameStateListener};
//!
//! # async fn run() -> Result<(), Box<dyn std::error::Error>> {
//! // 1. Drop a gamestate_integration_*.cfg into the CS2 cfg dir.
//! #[cfg(feature = "steam-discover")]
//! GsiCfg::for_localhost("ImLag", 4000).write_to_cs2()?;
//!
//! // 2. Stand up the listener.
//! let listener = GameStateListener::new(4000);
//! listener.on(|e: &PlayerDied| {
//!     println!("☠ {} died", e.player.name);
//! });
//! listener.start().await?;
//!
//! // ... run the rest of your app ...
//!
//! listener.stop().await?;
//! # Ok(()) }
//! ```
//!
//! ## How it works
//!
//! ```text
//!   CS2 client                                Your app
//!  ┌──────────┐                             ┌───────────────┐
//!  │ cfg file │── HTTP POST JSON ──────────▶│ GameStateList │
//!  └──────────┘                             │   (this lib)  │
//!         ▲                                 │     diff       │
//!         │           generate cfg          │      ▼         │
//!         └─────────────────────────────────│  typed events  │
//!                                           └───────────────┘
//! ```
//!
//! Every payload arrives as JSON, is parsed into a [`model::GameState`],
//! diffed against the previous one and turned into a stream of typed
//! events. Handlers run synchronously on the listener task — keep them
//! light or `tokio::spawn` from inside.

#![cfg_attr(docsrs, feature(doc_cfg))]
#![warn(missing_docs)]
#![warn(rust_2018_idioms)]

pub mod cfg;
mod dispatcher;
pub mod error;
pub mod events;
mod handlers;
mod listener;
pub mod model;
pub mod steam;

pub use crate::error::{Error, Result};
pub use crate::events::GameEvent;
pub use crate::listener::GameStateListener;
pub use crate::model::GameState;

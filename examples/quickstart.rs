//! Minimal cs2-gsi quickstart — listen for `PlayerDied` and print to stdout.
//!
//! Run with:
//!
//! ```text
//! cargo run --example quickstart
//! ```
//!
//! Then launch CS2. On the very first run the example also writes the
//! integration cfg file into the CS2 `cfg/` directory (no-op afterwards).

use cs2_gsi::cfg::GsiCfg;
use cs2_gsi::events::{PlayerDied, RoundPhaseUpdated};
use cs2_gsi::GameStateListener;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "cs2_gsi=info,quickstart=info".into()),
        )
        .init();

    // Drop the cfg into CS2 — best-effort, fall back to a hint if it fails.
    match GsiCfg::for_localhost("Quickstart", 4000).write_to_cs2() {
        Ok(p) => println!("✔ wrote integration cfg at {}", p.display()),
        Err(e) => eprintln!(
            "✘ couldn't auto-place cfg ({e}); copy it manually:\n{}",
            GsiCfg::for_localhost("Quickstart", 4000).render()
        ),
    }

    let listener = GameStateListener::new(4000);

    listener.on(|e: &PlayerDied| {
        println!("☠ {} died (HP {} → 0)", e.player.name, e.previous_health);
    });

    listener.on(|e: &RoundPhaseUpdated| {
        println!("⏱ round phase: {:?} → {:?}", e.previous, e.new);
    });

    listener.start().await?;
    println!("listening on http://localhost:4000/  — launch CS2 and join a match");
    println!("press Ctrl-C to quit");

    tokio::signal::ctrl_c().await?;
    listener.stop().await?;
    Ok(())
}

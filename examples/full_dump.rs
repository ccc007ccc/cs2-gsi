//! Dump every event raised by cs2-gsi in `Debug` form. Useful for exploring
//! the data model and verifying behaviour against real CS2 sessions.

use cs2_gsi::cfg::GsiCfg;
use cs2_gsi::GameStateListener;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter("cs2_gsi=debug")
        .init();

    if let Err(e) = GsiCfg::for_localhost("FullDump", 4000).write_to_cs2() {
        eprintln!("(cfg auto-write failed: {e}; copy it manually if needed)");
    }

    let listener = GameStateListener::new(4000);
    listener.on_any(|evt| {
        println!("{evt:?}");
    });

    listener.start().await?;
    println!("listening — Ctrl-C to quit");
    tokio::signal::ctrl_c().await?;
    listener.stop().await?;
    Ok(())
}

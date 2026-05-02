//! Error types for cs2-gsi.

use std::path::PathBuf;
use thiserror::Error;

/// Result alias used throughout the library.
pub type Result<T, E = Error> = std::result::Result<T, E>;

/// Errors that can be returned by the public API.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum Error {
    /// Failed to bind the HTTP listener.
    #[error("failed to bind HTTP listener on {addr}: {source}")]
    Bind {
        /// Socket address we tried to bind.
        addr: String,
        /// Underlying I/O error.
        #[source]
        source: std::io::Error,
    },

    /// Generic I/O error (file system, sockets, …).
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    /// Failed to parse JSON received from CS2.
    #[error("failed to parse GSI payload: {0}")]
    Parse(#[from] serde_json::Error),

    /// CS2 install directory could not be located.
    #[error("could not locate Counter-Strike 2 installation: {0}")]
    SteamDiscovery(String),

    /// `gamestate_integration_*.cfg` could not be written.
    #[error("failed to write GSI cfg file at {path}: {source}")]
    CfgWrite {
        /// Target path we tried to write.
        path: PathBuf,
        /// Underlying I/O error.
        #[source]
        source: std::io::Error,
    },

    /// Listener was already started.
    #[error("listener has already been started")]
    AlreadyStarted,

    /// Listener was not started yet.
    #[error("listener is not running")]
    NotRunning,
}

//! Authentication node.
//!
//! CS2 itself does not push an auth token by default — the `auth` block is
//! still present (often empty) and exposed here so consumers can inspect any
//! custom tokens they themselves added to the gamestate integration cfg file.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// The `auth` root node of a GSI payload. Holds any token / key/value pairs
/// the cfg file declares under `"auth" {}`.
#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(transparent)]
pub struct Auth(pub HashMap<String, String>);

impl Auth {
    /// Lookup a token by name (e.g. `"token"`).
    pub fn get(&self, key: &str) -> Option<&str> {
        self.0.get(key).map(String::as_str)
    }

    /// Returns `true` if the auth block is empty.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

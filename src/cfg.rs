//! Auto-generate `gamestate_integration_*.cfg` files for Counter-Strike 2.
//!
//! CS2 reads any file matching `cfg/gamestate_integration_*.cfg` on launch
//! and starts POSTing payloads to the URI declared inside. This module
//! produces files identical in shape to the ones emitted by the upstream
//! CounterStrike2GSI C# library — every data section enabled, sensible
//! throttle / heartbeat values — and lets callers tweak only the bits they
//! care about.

use crate::error::{Error, Result};
use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

/// Builder for a single `gamestate_integration_*.cfg` file.
#[derive(Debug, Clone)]
pub struct GsiCfg {
    /// User-visible name. Becomes part of the file name and the top-level
    /// KeyValues key (`"<service> Integration Configuration" { ... }`).
    pub service_name: String,
    /// Endpoint URI. Defaults to `http://localhost:<port>/`.
    pub uri: String,
    /// Network timeout in seconds.
    pub timeout: f32,
    /// Coalesce window in seconds (CS2 will sit on a payload up to this long
    /// before sending it out).
    pub buffer: f32,
    /// Minimum seconds between subsequent payloads — prevents overload.
    pub throttle: f32,
    /// Liveness payloads — CS2 emits an unchanged payload every N seconds.
    pub heartbeat: f32,
    /// Optional `auth { ... }` block. CS2 echoes these fields back to your
    /// listener verbatim. Useful to authenticate multiple co-hosted clients.
    pub auth: BTreeMap<String, String>,
    /// Subscribed data sections. Each value is `"1"` to enable, `"0"` to
    /// disable.
    pub data: BTreeMap<String, String>,
}

impl GsiCfg {
    /// Create a default config that points at `http://localhost:<port>/`
    /// with every known data section enabled. This matches the upstream
    /// CounterStrike2GSI library byte-for-byte.
    pub fn for_localhost(service_name: impl Into<String>, port: u16) -> Self {
        Self {
            service_name: service_name.into(),
            uri: format!("http://localhost:{port}/"),
            timeout: 5.0,
            buffer: 0.1,
            throttle: 0.1,
            heartbeat: 10.0,
            auth: BTreeMap::new(),
            data: default_data_sections(),
        }
    }

    /// Override the listener URI.
    pub fn with_uri(mut self, uri: impl Into<String>) -> Self {
        self.uri = uri.into();
        self
    }

    /// Add a single auth `key = value` token. The same value will be echoed
    /// back inside `GameState.auth` on every payload.
    pub fn with_auth(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.auth.insert(key.into(), value.into());
        self
    }

    /// Disable a previously enabled data section.
    pub fn without_section(mut self, name: &str) -> Self {
        self.data.remove(name);
        self
    }

    /// Render the file content as a string (without writing it to disk).
    pub fn render(&self) -> String {
        let mut out = String::with_capacity(512);
        let _ = writeln!(out, "\"{} Integration Configuration\"", self.service_name);
        let _ = writeln!(out, "{{");
        let _ = writeln!(out, "    \"uri\"          \"{}\"", self.uri);
        let _ = writeln!(out, "    \"timeout\"      \"{:.1}\"", self.timeout);
        let _ = writeln!(out, "    \"buffer\"       \"{:.1}\"", self.buffer);
        let _ = writeln!(out, "    \"throttle\"     \"{:.1}\"", self.throttle);
        let _ = writeln!(out, "    \"heartbeat\"    \"{:.1}\"", self.heartbeat);
        if !self.auth.is_empty() {
            let _ = writeln!(out, "    \"auth\"");
            let _ = writeln!(out, "    {{");
            for (k, v) in &self.auth {
                let _ = writeln!(out, "        \"{k}\"        \"{v}\"");
            }
            let _ = writeln!(out, "    }}");
        }
        let _ = writeln!(out, "    \"data\"");
        let _ = writeln!(out, "    {{");
        for (k, v) in &self.data {
            // Pad the *quoted* key (not the key itself) so CS2's KeyValues
            // parser sees the bare key name. Putting padding inside the
            // quotes turned the key into `"allgrenades             "`,
            // which CS2 silently ignored — and dropped the whole data
            // block with it.
            let quoted = format!("\"{k}\"");
            let _ = writeln!(out, "        {quoted:<26}        \"{v}\"");
        }
        let _ = writeln!(out, "    }}");
        let _ = writeln!(out, "}}");
        out
    }

    /// Sanitise the service name into a filename-safe slug:
    /// `gamestate_integration_<slug>.cfg`.
    pub fn file_name(&self) -> PathBuf {
        let slug: String = self
            .service_name
            .chars()
            .map(|c| if c.is_alphanumeric() { c } else { '_' })
            .collect();
        PathBuf::from(format!("gamestate_integration_{slug}.cfg"))
    }

    /// Write the cfg file into the supplied CS2 `cfg/` directory and return
    /// the full path. Creates the directory if it is missing.
    pub fn write_to_cfg_dir(&self, cfg_dir: &Path) -> Result<PathBuf> {
        std::fs::create_dir_all(cfg_dir).map_err(|source| Error::CfgWrite {
            path: cfg_dir.to_path_buf(),
            source,
        })?;
        let path = cfg_dir.join(self.file_name());
        std::fs::write(&path, self.render()).map_err(|source| Error::CfgWrite {
            path: path.clone(),
            source,
        })?;
        Ok(path)
    }

    /// Auto-discover the CS2 cfg directory via [`crate::steam`] and write
    /// the file there. Requires the `steam-discover` feature (enabled by
    /// default).
    #[cfg(feature = "steam-discover")]
    pub fn write_to_cs2(&self) -> Result<PathBuf> {
        let cfg_dir = crate::steam::find_cs2_cfg_dir()?;
        self.write_to_cfg_dir(&cfg_dir)
    }
}

fn default_data_sections() -> BTreeMap<String, String> {
    let mut m = BTreeMap::new();
    for k in [
        "provider",
        "tournamentdraft",
        "map",
        "map_round_wins",
        "round",
        "player_id",
        "player_state",
        "player_weapons",
        "player_match_stats",
        "player_position",
        "phase_countdowns",
        "allplayers_id",
        "allplayers_state",
        "allplayers_match_stats",
        "allplayers_weapons",
        "allplayers_position",
        "allgrenades",
        "bomb",
    ] {
        m.insert(k.into(), "1".into());
    }
    m
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_produces_expected_skeleton() {
        let cfg = GsiCfg::for_localhost("Demo", 4000);
        let rendered = cfg.render();
        assert!(rendered.starts_with("\"Demo Integration Configuration\""));
        assert!(rendered.contains("\"uri\"          \"http://localhost:4000/\""));
        assert!(rendered.contains("\"throttle\"     \"0.1\""));
        assert!(rendered.contains("\"heartbeat\"    \"10.0\""));
        // 18 default data keys.
        assert_eq!(rendered.matches("\"1\"").count(), 18);
    }

    #[test]
    fn data_keys_have_no_trailing_whitespace_inside_quotes() {
        // CS2's KeyValues parser is intolerant of stray whitespace inside
        // quoted keys — silently ignoring the whole `data` block. This test
        // pins the fix for that regression: padding must sit *outside* the
        // closing quote.
        let cfg = GsiCfg::for_localhost("Demo", 4000);
        let rendered = cfg.render();
        for key in [
            "provider",
            "map",
            "round",
            "bomb",
            "allgrenades",
            "allplayers_id",
            "allplayers_match_stats",
            "allplayers_position",
            "allplayers_state",
            "allplayers_weapons",
            "map_round_wins",
            "phase_countdowns",
            "player_id",
            "player_match_stats",
            "player_position",
            "player_state",
            "player_weapons",
            "tournamentdraft",
        ] {
            // The exact token `"<key>"` (closing quote immediately after
            // the name, no spaces) must appear in the output.
            assert!(
                rendered.contains(&format!("\"{key}\"")),
                "expected exact `\"{key}\"` token in rendered cfg",
            );
            // And the buggy variant `"<key>   "` (any space-then-quote
            // sequence) must NOT appear anywhere in the file.
            assert!(
                !rendered.contains(&format!("{key} ")) || !rendered.contains(&format!("{key}\"")),
                "found a quoted key with internal trailing whitespace",
            );
        }
    }

    #[test]
    fn file_name_slugifies_service() {
        let cfg = GsiCfg::for_localhost("ImLag Rust", 1234);
        assert_eq!(
            cfg.file_name(),
            PathBuf::from("gamestate_integration_ImLag_Rust.cfg")
        );
    }

    #[test]
    fn auth_block_is_emitted_when_provided() {
        let cfg = GsiCfg::for_localhost("Demo", 4000).with_auth("token", "abc123");
        let rendered = cfg.render();
        assert!(rendered.contains("\"auth\""));
        assert!(rendered.contains("\"token\"        \"abc123\""));
    }
}

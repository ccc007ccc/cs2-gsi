//! Steam / Counter-Strike 2 install path discovery.
//!
//! Replicates what the upstream C# library does:
//!  1. Find Steam's install directory:
//!     * Windows — registry under `HKLM\SOFTWARE\Valve\Steam` (and the WOW6432
//!       node), with a fallback to `HKCU`.
//!     * Linux   — `~/.steam/steam` or `~/.local/share/Steam`.
//!     * macOS   — `~/Library/Application Support/Steam`.
//!  2. Parse `<steam>/steamapps/libraryfolders.vdf` to enumerate library paths.
//!  3. In each library, look for `steamapps/appmanifest_730.acf`. The
//!     `installdir` value tells us the leaf folder name (`Counter-Strike 2` or
//!     `Counter-Strike Global Offensive`) under `steamapps/common/`.
//!
//! Failures are non-fatal — the result is wrapped in `Result` and the caller
//! is expected to fall back to a user-supplied path.

#![allow(clippy::result_large_err)]

use crate::error::{Error, Result};
use std::path::{Path, PathBuf};

/// Steam app id for Counter-Strike 2.
pub const CS2_APP_ID: u32 = 730;

/// Locate the root of the Steam installation.
pub fn find_steam_root() -> Result<PathBuf> {
    #[cfg(windows)]
    {
        if let Some(p) = win::registry_steam_path() {
            if p.is_dir() {
                return Ok(p);
            }
        }
    }
    #[cfg(target_os = "linux")]
    {
        if let Some(home) = dirs_home() {
            for candidate in [home.join(".steam/steam"), home.join(".local/share/Steam")] {
                if candidate.is_dir() {
                    return Ok(candidate);
                }
            }
        }
    }
    #[cfg(target_os = "macos")]
    {
        if let Some(home) = dirs_home() {
            let candidate = home.join("Library/Application Support/Steam");
            if candidate.is_dir() {
                return Ok(candidate);
            }
        }
    }
    Err(Error::SteamDiscovery(
        "Steam install directory not found".into(),
    ))
}

/// Resolve the absolute path of the CS2 game directory (the folder that
/// contains `game/csgo/pak01_dir.vpk`).
pub fn find_cs2_install_dir() -> Result<PathBuf> {
    let steam = find_steam_root()?;
    for library in steam_libraries(&steam)? {
        let manifest = library.join("steamapps").join("appmanifest_730.acf");
        if !manifest.is_file() {
            continue;
        }
        let installdir = parse_acf_installdir(&std::fs::read_to_string(&manifest)?)
            .unwrap_or_else(|| "Counter-Strike Global Offensive".into());
        let candidate = library.join("steamapps").join("common").join(installdir);
        if candidate.join("game/csgo/pak01_dir.vpk").is_file() {
            return Ok(candidate);
        }
    }
    Err(Error::SteamDiscovery(
        "CS2 (app id 730) is not installed in any Steam library".into(),
    ))
}

/// Resolve the path of the CS2 cfg directory: `<install>/game/csgo/cfg`.
pub fn find_cs2_cfg_dir() -> Result<PathBuf> {
    Ok(find_cs2_install_dir()?
        .join("game")
        .join("csgo")
        .join("cfg"))
}

/// Enumerate every Steam library folder configured on this machine.
pub fn steam_libraries(steam_root: &Path) -> Result<Vec<PathBuf>> {
    let mut libraries = vec![steam_root.to_path_buf()];
    let vdf = steam_root.join("steamapps").join("libraryfolders.vdf");
    if vdf.is_file() {
        let content = std::fs::read_to_string(&vdf)?;
        for path in parse_vdf_paths(&content) {
            let p = PathBuf::from(path);
            if p.is_dir() && !libraries.contains(&p) {
                libraries.push(p);
            }
        }
    }
    Ok(libraries)
}

/// Extract every `"path"  "..."` value from a Steam `libraryfolders.vdf`.
///
/// VDF (Valve KeyValues) is a small recursive format. We only need the leaf
/// `"path"` key here, so a regex-free linear scan is enough.
fn parse_vdf_paths(content: &str) -> Vec<String> {
    let mut paths = Vec::new();
    for line in content.lines() {
        let line = line.trim();
        // Match: "path"   "X:\\some\\path"
        let mut it = line.split('"').filter(|s| !s.trim().is_empty());
        let key = match it.next() {
            Some(k) => k,
            None => continue,
        };
        if key.trim() != "path" {
            continue;
        }
        if let Some(value) = it.next() {
            // Steam VDF uses doubled backslashes — collapse them.
            paths.push(value.replace("\\\\", "\\"));
        }
    }
    paths
}

/// Pull `"installdir"` out of an appmanifest ACF file.
fn parse_acf_installdir(content: &str) -> Option<String> {
    for line in content.lines() {
        let line = line.trim();
        let mut it = line.split('"').filter(|s| !s.trim().is_empty());
        let key = match it.next() {
            Some(k) => k,
            None => continue,
        };
        if key.trim() != "installdir" {
            continue;
        }
        return it.next().map(|s| s.replace("\\\\", "\\"));
    }
    None
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
fn dirs_home() -> Option<PathBuf> {
    std::env::var_os("HOME").map(PathBuf::from)
}

#[cfg(windows)]
mod win {
    use std::path::PathBuf;
    use windows::core::PCWSTR;
    use windows::Win32::Foundation::ERROR_SUCCESS;
    use windows::Win32::System::Registry::{
        RegCloseKey, RegOpenKeyExW, RegQueryValueExW, HKEY, HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE,
        KEY_READ, REG_VALUE_TYPE,
    };

    /// Look up `SteamPath` / `InstallPath` from the Windows registry.
    pub(super) fn registry_steam_path() -> Option<PathBuf> {
        // Prefer per-user, then 64-bit machine-wide, then the WOW6432 node.
        for (root, sub, value) in [
            (HKEY_CURRENT_USER, r"Software\Valve\Steam", "SteamPath"),
            (HKEY_LOCAL_MACHINE, r"SOFTWARE\Valve\Steam", "InstallPath"),
            (
                HKEY_LOCAL_MACHINE,
                r"SOFTWARE\WOW6432Node\Valve\Steam",
                "InstallPath",
            ),
        ] {
            if let Some(s) = read_reg_string(root, sub, value) {
                return Some(PathBuf::from(s));
            }
        }
        None
    }

    fn read_reg_string(root: HKEY, sub: &str, name: &str) -> Option<String> {
        let sub_w = wide(sub);
        let name_w = wide(name);
        let mut key = HKEY::default();
        unsafe {
            if RegOpenKeyExW(root, PCWSTR(sub_w.as_ptr()), 0, KEY_READ, &mut key) != ERROR_SUCCESS {
                return None;
            }
        }
        // First pass — discover required size.
        let mut ty = REG_VALUE_TYPE::default();
        let mut len: u32 = 0;
        let status = unsafe {
            RegQueryValueExW(
                key,
                PCWSTR(name_w.as_ptr()),
                None,
                Some(&mut ty),
                None,
                Some(&mut len),
            )
        };
        if status != ERROR_SUCCESS {
            unsafe {
                let _ = RegCloseKey(key);
            }
            return None;
        }
        let mut buf = vec![0u8; len as usize];
        let status = unsafe {
            RegQueryValueExW(
                key,
                PCWSTR(name_w.as_ptr()),
                None,
                Some(&mut ty),
                Some(buf.as_mut_ptr()),
                Some(&mut len),
            )
        };
        unsafe {
            let _ = RegCloseKey(key);
        }
        if status != ERROR_SUCCESS {
            return None;
        }
        // Reinterpret as wide chars.
        let wlen = (len as usize) / 2;
        let words: &[u16] = unsafe { std::slice::from_raw_parts(buf.as_ptr().cast::<u16>(), wlen) };
        let trimmed = words.split(|c| *c == 0).next().unwrap_or(words);
        Some(String::from_utf16_lossy(trimmed))
    }

    fn wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(std::iter::once(0)).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_libraryfolders_vdf() {
        let sample = r#"
"libraryfolders"
{
    "0"
    {
        "path"        "C:\\Program Files (x86)\\Steam"
        "label"        ""
    }
    "1"
    {
        "path"        "D:\\SteamLibrary"
        "label"        ""
    }
}
"#;
        let paths = parse_vdf_paths(sample);
        assert_eq!(
            paths,
            vec![
                "C:\\Program Files (x86)\\Steam".to_string(),
                "D:\\SteamLibrary".to_string(),
            ]
        );
    }

    #[test]
    fn parses_appmanifest_installdir() {
        let sample = r#"
"AppState"
{
    "appid"        "730"
    "installdir"   "Counter-Strike Global Offensive"
}
"#;
        assert_eq!(
            parse_acf_installdir(sample).as_deref(),
            Some("Counter-Strike Global Offensive")
        );
    }
}

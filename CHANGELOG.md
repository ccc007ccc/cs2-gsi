# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.1] - 2026-05-03

### Fixed
- `steam.rs` (Windows): registry value buffer is now allocated as `Vec<u16>`
  instead of `Vec<u8>` then re-cast — eliminates a latent alignment-UB on the
  `*const u8 → *const u16` reinterpret. The `HKEY` is also released through a
  small RAII guard so a panic between open and close cannot leak it.
- `Cargo.toml`: examples (`quickstart`, `full_dump`) now declare
  `required-features = ["cfg-writer", "steam-discover"]`. Building with
  `--no-default-features --examples` previously failed with `E0599` because
  `GsiCfg::write_to_cs2` is gated on `steam-discover`.
- `listener.rs`: incoming POST bodies are now capped at 1 MiB. Real GSI
  payloads stay well under 100 KB; the cap prevents a misbehaving local
  sender from feeding the listener arbitrary memory (returns `413 Payload
  Too Large`).
- `listener.rs`: corrected the bind-retry docstring (was claiming
  "6 × 250ms = 1.5s", actual budget is 3 × 250 ms ≈ 750 ms) and removed
  the redundant trailing sleep on the final retry attempt.

## [0.1.0] - 2026-05-02

### Added
- Initial release: HTTP listener for CS2 GSI POST payloads
- Strongly typed `GameState` model (provider, map, round, player, allplayers,
  phase_countdowns, bomb, grenades, tournament_draft, auth, previously)
- Event diffing engine with 30+ typed events across player, round, map, bomb,
  killfeed and meta categories
- `gamestate_integration_*.cfg` auto-writer (`cfg-writer` feature)
- Cross-platform Steam / CS2 install path discovery via `libraryfolders.vdf`
  and `appmanifest_730.acf` (`steam-discover` feature)
- Synchronous handler registration that mirrors the upstream C# library
- `quickstart` and `full_dump` examples

[Unreleased]: https://github.com/ccc007ccc/cs2-gsi/compare/v0.1.1...HEAD
[0.1.1]: https://github.com/ccc007ccc/cs2-gsi/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/ccc007ccc/cs2-gsi/releases/tag/v0.1.0

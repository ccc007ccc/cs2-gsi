# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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

[Unreleased]: https://github.com/ccc007ccc/cs2-gsi/compare/v0.1.0...HEAD

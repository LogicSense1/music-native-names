# Changelog

## 1.0.3 — 2026-09-09

### Added

- Symbol-and-color menu bar states with accessible descriptions and detailed tooltips.
- About dialog with version, MIT license, and the GitHub project link.
- English-default and Chinese README files, contributor/developer documentation, and a donation section.
- CI checks, versioned release packaging, SHA-256 checksums, and tag-triggered GitHub Release drafts.

### Changed

- Consolidated the maintained Rust implementation into a clean repository layout.
- Excluded prototypes, recorded responses, private diagnostics, caches, and app bundles from source control.
- Removed unused native helpers, unused installation arguments, and discarded native debug logging.
- Moved the native menu test driver behind the optional `ui-test` Cargo feature.
- Derived bundle versions from Cargo.toml and checked installation prerequisites before stopping an older helper.

## 1.0.2 — 2026-09-09

- Fixed original song names in the mini player and expanded Now Playing when MPModelSong uses
  whole-second or decoded-asset duration instead of catalog milliseconds.
- Kept exact cross-catalog recording validation and introduced bounded playback-only duration handling.
- Supported replacing the previous model hook in an existing Music process.

## 1.0.1 — 2026-09-09

- Improved artist origin inference when biographies are empty and genres are generic, using
  unambiguous solo-credited releases. Conflicts and collaborations still fall back to the existing name.
- Corrected affected album artist headers without hardcoded artist IDs or names.

## 1.0.0 — 2026-09-09

- Initial local Rust/AppKit helper with bounded catalog resolution, name caching, log retention,
  native runtime hooks, and login startup.

Versions 1.0.0–1.0.2 were local builds, not previously published GitHub releases.

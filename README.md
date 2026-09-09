# Music Native Names

**Original-language song, artist, and album names in the native Apple Music app.**

English · [简体中文](README.zh-CN.md)

A small macOS menu bar helper built with Rust and AppKit. It checks Apple’s catalog
and displays original-language names when it can verify the same recording or release.
Your existing Apple Music account and storefront stay in place.

[Download](https://github.com/LogicSense1/music-native-names/releases) ·
[Changelog](CHANGELOG.md) · [Report an issue](https://github.com/LogicSense1/music-native-names/issues)

> **Experimental compatibility:** Apple Silicon only. The current bridge requires an
> **already SIP-disabled system** and Apple Command Line Tools. It uses private Music
> runtime interfaces; this is not an official Apple plugin. The application and installer
> do not change SIP or other system protections. Runtime behavior has been verified on
> macOS 27.0 (26A5425a); compatibility with other macOS/Music versions is not established.

## What it does

- Restores names in supported catalog pages, album details, the mini player, and Now Playing.
- Uses official catalog equivalents, recording duration, content rating, and available
  artist identity to avoid replacing a name with another recording’s name.
- Keeps original English titles when appropriate; uncertain matches keep the existing display.
- Provides pause/resume, reconnect, login startup, cache clearing, and bounded daily logs.
- Runs one Rust background process. No persistent Python process, browser, or debugger is needed.

Coverage currently starts from the **US storefront**. Japanese, Korean, mainland Chinese,
and Taiwanese catalog metadata are considered where the artist/release evidence supports it.
The app does not translate lyrics, playlists, or precomposed editorial headings. Artist origin
is inferred from catalog evidence; Apple does not expose a universal “original language” field.

## Install

1. Download `MusicNativeNames-<version>-macos-arm64.zip` from [Releases](https://github.com/LogicSense1/music-native-names/releases).
2. Extract the complete ZIP and open `install.command`.
3. Open Music. The helper connects automatically; initial connection briefly pauses Music while
   the native bridge is installed.

The installer checks prerequisites before replacing an existing helper, installs to
`~/Applications/Music Native Names.app`, and enables login startup. Previous installations
are retained beside the app with a timestamped `.previous-…` suffix.

Release bundles are locally ad-hoc signed, **not Developer ID signed or notarized**. macOS may
block a downloaded bundle. Building from source is available below. Do not weaken system
protections simply to run this experimental helper.

## Menu bar

The menu bar uses a symbol and a semantic color, with no text characters. Shape and accessible
labels distinguish states without relying on color alone.

| Symbol | Color | State |
| --- | --- | --- |
| Music note | Green | Connected and enabled |
| Circular arrows | Orange | Connecting |
| Clock | Orange | Waiting for Music |
| Pause circle | Gray | Paused |
| Exclamation circle | Red | Connection or catalog error |

Hover for details. The menu currently uses Chinese labels:

| Menu item | Action |
| --- | --- |
| 启用原文显示 | Enable/pause replacement |
| 打开 Music | Open Music |
| 登录时自动启动 | Toggle login startup |
| Log | Open logs; retain 1, 7, 14, or 30 days (default: 7) |
| 清空名称缓存 | Clear the name cache |
| 重新连接 Music | Retry connection |
| 关于 Music Native Names… | Version, MIT license, and GitHub project link |
| 退出 | Restore hooked methods and quit |

## Build and test

Requires an Apple Silicon Mac, Rust via rustup, Apple Command Line Tools, and Python 3 for
packaging checks. `rust-toolchain.toml` pins the compiler; `Cargo.lock` pins dependencies.

```sh
git clone https://github.com/LogicSense1/music-native-names.git
cd music-native-names
./build.command
./scripts/package.sh --no-build
```

The app is written to `dist/Music Native Names.app`; the ZIP and `SHA256SUMS` are also in `dist/`.
To install a source build, run `./install.command`.

```sh
./scripts/test.sh                         # formatting, Rust tests, Clippy, native checks
```

Verify a downloaded archive from the directory containing both files:

```sh
cd dist
shasum -a 256 -c SHA256SUMS
```

Builds and CI do **not** attach to Music or require disabled SIP. Actual bridge/UI acceptance
requires a compatible local Music installation. See [Development](docs/DEVELOPMENT.md) for
isolated diagnostics and [Architecture](docs/ARCHITECTURE.md) for matching and runtime details.

## Data and privacy

Only name fields are replaced. Track IDs, playback parameters, audio URLs, artwork, lyrics,
and account data are not rewritten. The helper does not modify Music.app on disk or write
name changes to your music library.

Catalog queries use Apple’s web-distributed public catalog credential, held in memory. The
helper does not read account tokens or browser cookies. Public catalog access is an unsupported
integration and may stop working. There is no analytics endpoint.

| Local data | Location |
| --- | --- |
| Preferences and cache | `~/Library/Application Support/MusicNativeNames/` |
| Daily diagnostic logs | `~/Library/Logs/MusicNativeNames/` |
| Login startup entry | `~/Library/LaunchAgents/local.music-native-names.rust.plist` |

A loopback service listens on `127.0.0.1:18090`. It is not exposed to the network and has no
per-client authentication; do not expose or forward this port. Logs contain operational
counts and errors, not full catalog responses or credentials. Default retention is the last
seven calendar dates, with approximately 4 MiB per day.

## Troubleshooting and uninstall

- **Waiting/connecting:** open Music and allow the initial bridge installation to finish.
- **Red indicator:** inspect Log, confirm prerequisites, then select **重新连接 Music**.
- **Some names remain romanized:** the resource may be cached upstream, unsupported, ambiguous,
  or unavailable in the relevant catalog. Open the page again after resolving the error.
- **A macOS update breaks connection:** disable the helper and report the OS/Music version.
  Private interfaces are not guaranteed across system updates.

Run `uninstall.command` to disable login startup, restore active hooks, and move the helper
into Trash. Preferences, cache, and logs remain for recovery.

## Contributing and license

See [CONTRIBUTING.md](CONTRIBUTING.md). The project is licensed under [MIT](LICENSE).
Dependency notices are included in release bundles. Apple Music and related marks belong to
Apple Inc.; this project is independent and is not affiliated with or endorsed by Apple.

## Support / Donate

If this project helps you, a star, a useful bug report, or a contribution is appreciated.

**Financial support is not enabled yet.** Verified donation links and payment QR codes will
appear here when available. See [Support](docs/SUPPORT.md) for the supported funding setup.

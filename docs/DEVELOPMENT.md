# Development

Run `./scripts/test.sh` before a pull request. It runs Rust formatting, all feature tests,
Clippy with warnings as errors, the actual native duration policy assertions, and arm64e
bridge syntax checks. `./build.command` also runs these checks before packaging the app.
Native code follows `.clang-format`; formatting must not reorder the syntax harness imports.

## Isolated diagnostics

Developer flags are retained because they make troubleshooting possible without changing a
Music process. Always use isolated state, log, and port settings so a test cannot clear the
installed helper's cache or conflict with its listener.

```sh
scratch="$(mktemp -d)"
MNN_STATE_DIR="$scratch/state" MNN_LOG_DIR="$scratch/logs" MNN_PORT=18091 \
  cargo run --locked -- --no-hooks --headless
```

Use Ctrl-C for normal shutdown. The headless service still makes public catalog requests.
It never attaches to Music when `--no-hooks` is present.

For an opt-in native menu smoke test, run from an unlocked interactive desktop:

```sh
scratch="$(mktemp -d)"
MNN_STATE_DIR="$scratch/state" MNN_LOG_DIR="$scratch/logs" MNN_PORT=18091 \
  cargo run --locked --features ui-test -- --no-hooks --self-test-menu
```

This exercises native menu dispatch for retention, pause/resume, and quit. It writes snapshots
only in the isolated state directory. The `ui-test` feature is excluded from standard releases.
It is not a substitute for visually checking symbol colors, tooltips, About, or Music metadata.

Other diagnostics:

| Argument | Purpose |
| --- | --- |
| `--status` | Read the helper's latest status JSON |
| `--bridge-status PID` | Briefly attach to the official Music process and inspect installed hooks |
| `--resolve-file INPUT OUTPUT` | Transform an explicit catalog fixture without installing hooks |
| `--install-login` / `--remove-login` | Manage this helper's login startup entry |

## Runtime acceptance

For a bridge or UI change, record the exact macOS build and Music version. Check the real
catalog/album page, mini player, expanded Now Playing, pause/resume, and normal helper exit.
Check that playback IDs, audio URLs, album artwork, and other non-name metadata are unchanged.
Keep real response dumps, account data, logs, and machine paths outside the public repository.

Use synthetic fixtures in automated tests. Never publish Apple catalog credentials, cookies,
private listening history, debugger dumps, or copied third-party artwork.

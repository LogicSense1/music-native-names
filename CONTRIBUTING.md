# Contributing

Issues and pull requests are welcome in English or Chinese.

For a bug report, include your macOS build, Music version, storefront, expected and actual
names, and the affected UI surface. A public Apple Music URL is useful; private listening
history, credentials, and complete diagnostic dumps are not needed.

For code changes:

1. Keep name matching evidence-based. Do not add artist name/ID allowlists or translations by guesswork.
2. Preserve non-name fields and playback behavior. Add a focused regression test when fixing
   matching, concurrency, duration, or state transitions.
3. Run `./scripts/test.sh` and the relevant local UI/runtime checks.
4. Keep English and Chinese README instructions in sync when user-facing behavior changes.
5. Describe the final behavior and validation in the pull request. Report unverified environments clearly.

Code contributions are made under the project's MIT license. Keep generated builds, caches,
recorded responses, and local diagnostics out of Git.

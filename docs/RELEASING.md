# Releasing

## Local package

1. Update the package version in `Cargo.toml` and its package entry in `Cargo.lock`.
2. Update `CHANGELOG.md` and add `docs/releases/vX.Y.Z.md` in English with a Chinese summary.
3. Run `./scripts/package.sh`. The script tests, builds, verifies, and stages only the release payload.
4. Extract the ZIP into a fresh directory, verify `SHA256SUMS`, inspect the app signature, and
   perform the relevant local UI/runtime acceptance checks before publication.

The package version is the single source for the bundle version and asset filename. The
`--no-build` option verifies the bundle version and source fingerprint before reusing a build.
Release archives include dependency license texts generated from the locked Cargo metadata.
`dist/` and `target/` are excluded from Git.

## GitHub workflow

The public repository is [LogicSense1/music-native-names](https://github.com/LogicSense1/music-native-names).
Push the reviewed main branch, then push an annotated version tag:

```sh
git tag -a v1.0.3 -m "Music Native Names 1.0.3"
git push origin main
git push origin v1.0.3
```

The Release workflow validates the tag against Cargo.toml, builds on the documented
[macOS ARM64 runner](https://docs.github.com/en/actions/reference/runners/github-hosted-runners),
and creates a draft containing the ZIP and `SHA256SUMS`. It uses the repository-scoped
`GITHUB_TOKEN`; no personal token is required. Checkout and artifact actions are pinned by SHA.

Inspect the successful workflow, download/verify the exact generated artifact, and publish
the draft from the Releases page. The draft uses [GitHub CLI release creation](https://cli.github.com/manual/gh_release_create).
Tag creation or a running workflow does not mean the release has passed acceptance.

Do not overwrite a published version with a different binary. Publish a new patch version
if an existing release needs a code or packaging fix.

## Donation links

Funding remains inactive until the account owner supplies verified payment destinations.
Follow [SUPPORT.md](SUPPORT.md) to enable links or QR codes in both README files.

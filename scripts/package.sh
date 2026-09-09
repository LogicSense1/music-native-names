#!/bin/bash
set -euo pipefail
cd "$(dirname "$0")/.."
if [[ "${1:-}" == '--no-build' ]]; then
  [[ $# == 1 ]] || { echo 'Usage: package.sh [--no-build]' >&2; exit 1; }
elif [[ $# == 0 ]]; then
  ./build.command
else
  echo 'Usage: package.sh [--no-build]' >&2
  exit 1
fi
python3 scripts/check_release.py --bundle
version="$(./scripts/version.sh)"
name="MusicNativeNames-$version-macos-arm64"
stage="$(mktemp -d "$PWD/dist/stage.XXXXXX")"
trap 'rm -rf "$stage"' EXIT
payload="$stage/$name"
mkdir -p "$payload"
ditto 'dist/Music Native Names.app' "$payload/Music Native Names.app"
cp install.command uninstall.command README.md README.zh-CN.md LICENSE CHANGELOG.md CONTRIBUTING.md "$payload/"
cp -R docs "$payload/docs"
python3 scripts/dependency_notices.py "$payload"
ditto -c -k --norsrc --noextattr --keepParent "$payload" "dist/$name.zip"
(cd dist && shasum -a 256 "$name.zip" > SHA256SUMS)
python3 - "dist/$name.zip" <<'PY'
import sys,zipfile
from pathlib import PurePosixPath
with zipfile.ZipFile(sys.argv[1]) as archive:
    assert archive.testzip() is None
    for entry in archive.namelist():
        parts = PurePosixPath(entry).parts
        assert not entry.startswith('/') and '..' not in parts, entry
        assert '.DS_Store' not in parts and '.git' not in parts, entry
        assert not any(p in ('work', 'target', 'evidence') for p in parts), entry
print('Release archive integrity and contents checked.')
PY
printf 'Release ready: dist/%s.zip\n' "$name"

#!/bin/bash
set -euo pipefail
root="$(cd "$(dirname "$0")/.." && pwd)"
version="$(awk '/^\[package\]/{section=1;next} /^\[/{section=0} section && /^version = /{gsub(/"/, "", $3); print $3; exit}' "$root/Cargo.toml")"
[[ "$version" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]] || { echo 'Expected a numeric X.Y.Z package version.' >&2; exit 1; }
printf '%s\n' "$version"

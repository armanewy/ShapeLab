#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "$script_dir/.." && pwd)"
skip_build=0

for arg in "$@"; do
  case "$arg" in
    --skip-build)
      skip_build=1
      ;;
    *)
      echo "usage: $0 [--skip-build]" >&2
      exit 2
      ;;
  esac
done

if [[ "$skip_build" -eq 0 ]]; then
  cargo build -p shape-app --release
fi

binary="$repo_root/target/release/shape-app"
plist="$repo_root/packaging/macos/Info.plist"
app="$repo_root/target/release/Shape Lab.app"
contents="$app/Contents"
macos="$contents/MacOS"
resources="$contents/Resources"

if [[ ! -x "$binary" ]]; then
  echo "missing release binary: $binary" >&2
  echo "run cargo build -p shape-app --release first, or omit --skip-build" >&2
  exit 1
fi

rm -rf "$app"
mkdir -p "$macos" "$resources"
cp "$binary" "$macos/shape-app"
chmod 755 "$macos/shape-app"
cp "$plist" "$contents/Info.plist"
printf 'APPL????' > "$contents/PkgInfo"

plutil -lint "$contents/Info.plist" >/dev/null
echo "$app"

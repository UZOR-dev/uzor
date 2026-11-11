#!/bin/bash
# Stamp out a new uzor mobile app from uzor-window-mobile's host templates.
#
#   new-app.sh <name> [dest] [--title Title] [--package com.example.name] [--uzor-path DIR]
#
#   <name>        crate / library name: lower case, digits, underscores (my_app)
#   dest          where to create it (default ./<name>); must not exist
#   --title       display name and Xcode target (default: <name> in CamelCase)
#   --package     Android package and iOS bundle id (default dev.uzor.<name>)
#   --uzor-path   use a local uzor checkout (path dependencies) instead of crates.io
#
# The result builds on the desktop at once (`cargo run`); build-ios.sh and
# build-android.sh build the phone hosts.
set -euo pipefail
here="$(cd "$(dirname "$0")/.." && pwd)"
tpl="$here/templates"

name="" dest="" title="" pkg="" uzor_path=""
while [ $# -gt 0 ]; do
  case "$1" in
    --title) title="$2"; shift 2 ;;
    --package) pkg="$2"; shift 2 ;;
    --uzor-path) uzor_path="$(cd "$2" && pwd)"; shift 2 ;;
    -h|--help) sed -n '2,15p' "$0"; exit 0 ;;
    -*) echo "unknown option $1" >&2; exit 2 ;;
    *) if [ -z "$name" ]; then name="$1"; elif [ -z "$dest" ]; then dest="$1"; else echo "extra argument $1" >&2; exit 2; fi; shift ;;
  esac
done
[ -n "$name" ] || { sed -n '2,15p' "$0"; exit 2; }
[[ "$name" =~ ^[a-z][a-z0-9_]*$ ]] || { echo "name must match [a-z][a-z0-9_]*: $name" >&2; exit 2; }
dest="${dest:-./$name}"
[ -e "$dest" ] && { echo "$dest exists" >&2; exit 2; }
if [ -z "$title" ]; then
  title="$(echo "$name" | sed -E 's/(^|_)([a-z])/\U\2/g')"
fi
[[ "$title" =~ ^[A-Za-z][A-Za-z0-9]*$ ]] || { echo "title must be letters and digits: $title" >&2; exit 2; }
pkg="${pkg:-dev.uzor.$name}"
[[ "$pkg" =~ ^[a-z][a-z0-9_]*(\.[a-z][a-z0-9_]*)+$ ]] || { echo "bad package: $pkg" >&2; exit 2; }
prefix="${pkg%.*}"
version="$(sed -n 's/^version = "\(.*\)"/\1/p' "$here/../Cargo.toml" 2>/dev/null | head -1)"
version="${version:-$(sed -n 's/^version = "\(.*\)"/\1/p' "$here/Cargo.toml" | head -1)}"

mkdir -p "$dest"
cp -r "$tpl/rust/." "$dest/"
cp -r "$tpl/ios" "$tpl/android" "$tpl/build-ios.sh" "$tpl/build-android.sh" "$tpl/README.md" "$dest/"
cp "$tpl/gitignore" "$dest/.gitignore"
pkgdir="$(echo "$pkg" | tr . /)"
mkdir -p "$dest/android/app/src/main/java/$pkgdir"
mv "$dest/android/app/src/main/java/__PACKAGE_DIR__/MainActivity.kt" "$dest/android/app/src/main/java/$pkgdir/"
rmdir "$dest/android/app/src/main/java/__PACKAGE_DIR__"

find "$dest" -type f | while read -r f; do
  sed -i \
    -e "s|__APP_TITLE__|$title|g" \
    -e "s|__APP__|$name|g" \
    -e "s|__BUNDLE_PREFIX__|$prefix|g" \
    -e "s|__BUNDLE_ID__|$pkg|g" \
    -e "s|__PACKAGE__|$pkg|g" \
    -e "s|__UZOR_VERSION__|$version|g" \
    "$f"
done

if [ -n "$uzor_path" ]; then
  sed -i -E \
    -e "s|^uzor = \{ version = \"[^\"]*\"|uzor = { path = \"$uzor_path/uzor\"|" \
    -e "s|^uzor-framework = \{ version = \"[^\"]*\"|uzor-framework = { path = \"$uzor_path/uzor-framework\"|" \
    -e "s|^uzor-window-mobile = \{ version = \"[^\"]*\"|uzor-window-mobile = { path = \"$uzor_path/uzor-window-mobile\"|" \
    "$dest/Cargo.toml"
  printf '\n[workspace]\n' >> "$dest/Cargo.toml"
fi
echo "created $dest ($title, $pkg, uzor ${uzor_path:-$version})"

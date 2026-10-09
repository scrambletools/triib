#!/usr/bin/env bash
# Assembles triib's website into OUT: the pages in site/, plus the icon
# and the screenshots, from the repository.
#
#   scripts/build-site.sh [OUT]
set -euo pipefail

root=$(cd "$(dirname "$0")/.." && pwd)
out=${1:-$root/target/site}

rm -rf "$out"
mkdir -p "$out/screenshots"
cp "$root"/site/* "$out/"
cp "$root/data/icons/hicolor/256x256/apps/io.github.scrambletools.triib.png" "$out/icon.png"
cp "$root/data/icons/hicolor/32x32/apps/io.github.scrambletools.triib.png" "$out/favicon.png"
cp -r "$root/docs/screenshots/." "$out/screenshots/"
echo "Built the site in $out"

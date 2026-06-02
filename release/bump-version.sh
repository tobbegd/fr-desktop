#!/usr/bin/env bash
set -e

CARGO_TOML="../fr-app/src-tauri/Cargo.toml"
TAURI_CONF="../fr-app/src-tauri/tauri.conf.json"
APP_DIR="../fr-app"

current=$(grep '^version = ' "$CARGO_TOML" | head -1 | sed 's/version = "\(.*\)"/\1/')

if [[ "$1" == "--manual" ]]; then
  if [[ -z "$2" ]]; then
    echo "Användning: $0 --manual <version>  (t.ex. 0.3.0)"
    exit 1
  fi
  new_version="$2"
else
  major=$(echo "$current" | cut -d. -f1)
  minor=$(echo "$current" | cut -d. -f2)
  patch=$(echo "$current" | cut -d. -f3)
  new_version="${major}.${minor}.$((patch + 1))"
fi

echo "Bumpar: $current → $new_version"

sed -i "0,/^version = \"$current\"/s//version = \"$new_version\"/" "$CARGO_TOML"
sed -i "s/\"version\": \"$current\"/\"version\": \"$new_version\"/" "$TAURI_CONF"

echo "→ Committar och pushar..."
git -C "$APP_DIR" add src-tauri/Cargo.toml src-tauri/tauri.conf.json
git -C "$APP_DIR" commit -m "Bumpa till $new_version"
git -C "$APP_DIR" push

echo "✓ $new_version klar att bygga — kör: ./build.sh --github --notes \"...\""

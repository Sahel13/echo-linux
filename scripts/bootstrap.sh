#!/bin/sh
set -eu

require_command() {
    if ! command -v "$1" >/dev/null 2>&1; then
        printf 'Missing required command: %s\n' "$1" >&2
        exit 1
    fi
}

require_command cargo
require_command pkg-config

for library in gtk4 libadwaita-1; do
    if ! pkg-config --exists "$library"; then
        printf 'Missing required development library: %s\n' "$library" >&2
        exit 1
    fi
    printf '%s %s\n' "$library" "$(pkg-config --modversion "$library")"
done

cargo fetch --locked
cargo build --locked

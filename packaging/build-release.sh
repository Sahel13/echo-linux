#!/bin/sh
set -eu

require_command() {
    if ! command -v "$1" >/dev/null 2>&1; then
        printf 'Missing required command: %s\n' "$1" >&2
        exit 1
    fi
}

require_command cargo
require_command install
require_command sha256sum
require_command tar

version=$(sed -n 's/^version = "\([^"]*\)"$/\1/p' Cargo.toml | head -n 1)
case "$version" in
    ''|*[!0-9A-Za-z.+-]*)
        printf 'Could not read a safe package version from Cargo.toml\n' >&2
        exit 1
        ;;
esac

case "$(uname -m)" in
    x86_64) ;;
    *)
        printf 'Release builds require an x86_64 host; found %s\n' "$(uname -m)" >&2
        exit 1
        ;;
esac

archive_name="echo-linux-x86_64-$version"
output_dir=${OUTPUT_DIR:-dist}
staging_dir=$(mktemp -d)
trap 'rm -rf "$staging_dir"' EXIT HUP INT TERM

cargo build --release --locked

root="$staging_dir/$archive_name"
install -d "$root/share/icons/hicolor/256x256/apps"
install -m 755 target/release/echo "$root/echo"
install -m 644 packaging/README.txt "$root/README.txt"
install -m 644 LICENSE "$root/LICENSE"
install -m 644 assets/echo.png "$root/share/icons/hicolor/256x256/apps/echo.png"

mkdir -p "$output_dir"
tar -C "$staging_dir" -czf "$output_dir/$archive_name.tar.gz" "$archive_name"
(
    cd "$output_dir"
    sha256sum "$archive_name.tar.gz" > "$archive_name.tar.gz.sha256"
)

printf 'Created %s/%s.tar.gz and matching SHA-256 checksum\n' "$output_dir" "$archive_name"

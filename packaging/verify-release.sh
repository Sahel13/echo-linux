#!/bin/sh
set -eu

if [ "$#" -ne 1 ]; then
    printf 'Usage: %s path/to/echo-linux-x86_64-VERSION.tar.gz\n' "$0" >&2
    exit 2
fi

archive=$1
checksum="$archive.sha256"
case "$(basename "$archive")" in
    echo-linux-x86_64-*.tar.gz) ;;
    *)
        printf 'Unexpected release archive name: %s\n' "$archive" >&2
        exit 1
        ;;
esac

if [ ! -f "$checksum" ]; then
    printf 'Missing checksum: %s\n' "$checksum" >&2
    exit 1
fi

(
    cd "$(dirname "$archive")"
    sha256sum -c "$(basename "$checksum")"
)

root=$(basename "$archive" .tar.gz)
entries=$(tar -tzf "$archive" | LC_ALL=C sort)
for required in \
    "$root/echo" \
    "$root/README.txt" \
    "$root/LICENSE" \
    "$root/share/icons/hicolor/256x256/apps/echo.png"; do
    if ! printf '%s\n' "$entries" | grep -Fqx "$required"; then
        printf 'Archive is missing %s\n' "$required" >&2
        exit 1
    fi
done

staging_dir=$(mktemp -d)
trap 'rm -rf "$staging_dir"' EXIT HUP INT TERM
tar -C "$staging_dir" -xzf "$archive"
ldd "$staging_dir/$root/echo" | tee "$staging_dir/ldd.txt"
if grep -F 'not found' "$staging_dir/ldd.txt" >/dev/null; then
    printf 'Release has unresolved shared libraries\n' >&2
    exit 1
fi

printf 'Verified archive layout, SHA-256 checksum, and resolvable shared libraries.\n'

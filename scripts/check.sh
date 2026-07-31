#!/bin/sh
set -eu

cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test --all-targets --locked
sh -n packaging/build-release.sh packaging/verify-release.sh

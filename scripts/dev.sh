#!/bin/sh
set -eu

export RUST_BACKTRACE="${RUST_BACKTRACE:-1}"
export G_MESSAGES_DEBUG="${G_MESSAGES_DEBUG:-all}"

exec cargo run --locked

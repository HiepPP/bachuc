#!/bin/sh
# Runs cargo in this workspace with the rustup toolchain. Homebrew's older rustc
# on PATH cannot build gpui-pre.
# Usage: cargo.sh <cargo args>
set -eu
cd "$(dirname "$0")"
export PATH="$HOME/.cargo/bin:$PATH"
export RUSTUP_TOOLCHAIN="${RUSTUP_TOOLCHAIN:-1.97.1}"
exec cargo "$@"

#!/usr/bin/env bash
# Runs once, when the dev container is created (onCreateCommand in
# devcontainer.json). In a Codespaces prebuild it runs during the prebuild,
# so the learner does not wait for it.
#
# It compiles every chapter's tests without running them, and the coach, so
# that the first lesson and the first `cargo progress` start at once. The
# root package uses only the standard library, so no network is needed.
set -euo pipefail

cd "$(dirname "$0")/.."

echo "Rust in this container:"
rustc --version
cargo --version

echo
echo "Compiling the chapters (the tests are not run)..."
cargo test --locked --offline --all-targets --no-run

echo
echo "Compiling the coach (cargo progress, cargo drill)..."
cargo build --locked --offline --quiet --example coach

echo
echo "Done. Every exercise starts unfinished: tests fail at todo!() until you fill them in."

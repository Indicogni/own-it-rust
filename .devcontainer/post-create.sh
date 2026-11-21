#!/usr/bin/env bash
# Runs once, when the container is first given to the learner
# (postCreateCommand in devcontainer.json). Keep it short: the heavy build
# is in on-create.sh.
#
# It checks the setup and says where to start. A failed check is reported
# but does not stop the container from opening.
set -uo pipefail

cd "$(dirname "$0")/.."

cargo progress doctor || echo "cargo progress doctor reported a problem; see the lines above."

cat <<'EOF'

Welcome to Own It: Hands-On Mastery of Rust.
Start with README.md, then exercises/chapter01/CONCEPTS.md.
Run `cargo progress` in this terminal to see where you are.
The browser desktop for the game capstone is on port 6080 (password: vscode).
EOF

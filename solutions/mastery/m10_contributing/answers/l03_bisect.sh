#!/bin/sh
# Lesson 03, task B: the script for `git bisect run`.
#
# git reads the exit code: 0 means "good", 125 means "cannot test this commit, skip
# it", and any other code from 1 to 127 means "bad".
#
# A commit whose library does not compile says nothing about the regression.
cargo build --quiet 2>/dev/null || exit 125

# The probe test decides: exit code 0 is good, 101 (a failed test) is bad.
cargo test --quiet --test probe

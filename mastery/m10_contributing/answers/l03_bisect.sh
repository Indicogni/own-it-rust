#!/bin/sh
# Lesson 03, task B: the script for `git bisect run`.
#
# git reads the exit code: 0 means "good", 125 means "cannot test this commit, skip
# it", and any other code from 1 to 127 means "bad".
#
# TODO: build the library first. If that fails, the commit cannot be tested.
# TODO: then run the probe test. Its exit code is the verdict.

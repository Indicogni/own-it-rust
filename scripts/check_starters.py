#!/usr/bin/env python3
"""Cheap checks of the starter state, the state a learner receives.

Static checks (fast, no compiler):

  - every chapter directory has a test target in Cargo.toml, and every
    lesson file is declared in its mod.rs;
  - every lesson and review file (`lNN_*.rs`, `rNN_*.rs`; capstones are
    exempt) has a `p_predictions` test that still uses `guess()`;
  - every drill has exactly one `EXPECT: ?` line and at least one line
    ending in `// KEEP`;
  - no file contains a forbidden phrase. The phrases come from --forbid
    and the COURSE_FORBIDDEN environment variable (comma-separated); exempt
    a phrase with --allow or COURSE_ALLOWED. A match is reported by file
    and line without printing the phrase.

With --strict, a chapter must also have README.md, CONCEPTS.md, HINTS.md and
NOTES.md, and every drill set must be listed in its solutions/expected.tsv.

With --run, the script also builds and runs every chapter's starter tests
and requires that each `p_predictions` test fails at `guess()` (its output
contains `PREDICT:`), and that no test program crashes or hangs.

With --compile-packages, it also compiles the starter tests of every
standalone package (project, ecosystem, mastery modules, capstones,
checkpoints, chapter labs) without running them. A package can set
`test-args` and `skip-os` under `[package.metadata.course]` in its
Cargo.toml; see check_solutions.py.

  python3 scripts/check_starters.py
  python3 scripts/check_starters.py --strict --run
  python3 scripts/check_starters.py --no-phrases --compile-packages

Exit status: 0 when every check passed, 1 otherwise.
"""
import argparse
import os
import pathlib
import re
import sys

import course

# The phrases live outside the repository (--forbid, COURSE_FORBIDDEN), so
# that this file names none of them.
DEFAULT_PHRASES = []


def phrase_pattern(phrases):
    """One case-insensitive pattern; each phrase must stand as a whole word."""
    parts = []
    for phrase in sorted(phrases, key=len, reverse=True):
        words = [re.escape(word) for word in phrase.split()]
        parts.append(r"(?<![A-Za-z0-9])" + r"[\s_.-]*".join(words) + r"(?![A-Za-z0-9])")
    return re.compile("|".join(parts), re.I) if parts else None


def check_phrases(repo, pattern, problems):
    if pattern is None:
        return 0
    scanned = 0
    for path in course.course_files(repo):
        if path.name == "Cargo.lock" or path.stat().st_size > 2_000_000:
            continue
        try:
            text = path.read_text(encoding="utf-8")
        except (UnicodeDecodeError, OSError):
            continue  # not a text file
        scanned += 1
        relative = path.relative_to(repo).as_posix()
        if pattern.search(relative):
            problems.append("%s: forbidden phrase in the file name" % relative)
        for number, line in enumerate(text.splitlines(), 1):
            if pattern.search(line):
                # The phrase itself is not printed: this output may be public.
                problems.append("%s:%d: forbidden phrase" % (relative, number))
    return scanned


def check_chapters(repo, strict, problems):
    manifest = course.read(repo / "Cargo.toml")
    lessons = 0
    for number, folder in sorted(course.chapters(repo).items()):
        name = folder.name
        if not re.search(r'name\s*=\s*"%s"' % name, manifest):
            problems.append("%s: no [[test]] target named %s in Cargo.toml" % (name, name))
        modules = set(re.findall(r"^\s*(?:pub\s+)?mod\s+(\w+)\s*;", course.read(folder / "mod.rs"), re.M))
        if strict:
            for required in ("README.md", "CONCEPTS.md", "HINTS.md", "NOTES.md"):
                if not (folder / required).is_file():
                    problems.append("exercises/%s: %s is missing" % (name, required))
        for path in course.lesson_files(folder):
            lessons += 1
            relative = path.relative_to(repo).as_posix()
            if path.stem not in modules:
                problems.append("%s: not declared with `mod %s;` in mod.rs" % (relative, path.stem))
            if path.name[0] == "c" or "capstone" in path.name:
                continue
            text = course.read(path)
            code = course.blank_non_code(text)
            start = re.search(r"\bfn\s+p_predictions\s*\(", code)
            if not start:
                problems.append("%s: no `p_predictions` test" % relative)
                continue
            brace = code.find("{", start.end())
            body = code[brace : course.matching_brace(code, brace)] if brace >= 0 else ""
            if not re.search(r"\bguess\s*\(\s*\)", body):
                problems.append("%s: `p_predictions` does not use guess()" % relative)
    return lessons


def check_drills(repo, strict, problems):
    drills = 0
    for key, files in course.drill_sets(repo).items():
        expected = repo / "solutions" / "drills" / key / "expected.tsv"
        listed = None
        if expected.is_file():
            listed = {line.split("\t")[0].strip() for line in course.read(expected).splitlines() if line.strip()}
        elif strict:
            problems.append("drills/%s: no solutions/drills/%s/expected.tsv" % (key, key))
        for path in files:
            drills += 1
            relative = path.relative_to(repo).as_posix()
            lines = course.read(path).splitlines()
            blank = [line for line in lines if re.search(r"EXPECT:\s*\?\s*$", line)]
            every = [line for line in lines if re.search(r"^\s*//[/!]?\s*EXPECT:", line)]
            if len(blank) != 1 or len(every) != 1:
                problems.append(
                    "%s: needs exactly one `EXPECT: ?` line (found %d EXPECT lines, %d unanswered)"
                    % (relative, len(every), len(blank))
                )
            if not course.keep_lines("\n".join(lines)):
                problems.append("%s: no line ends in `// KEEP`" % relative)
            if listed is not None and strict and path.name not in listed:
                problems.append("%s: not listed in solutions/drills/%s/expected.tsv" % (relative, key))
    return drills


def run_starters(repo, timeout, problems):
    """Build every chapter, run it, and check how the predictions fail."""
    numbers = sorted(course.chapters(repo))
    env = dict(os.environ, CARGO_TERM_COLOR="never")
    build = ["cargo", "build", "--locked", "--offline", "--keep-going"]
    for number in numbers:
        build += ["--test", "chapter%02d" % number]
    course.run(build, cwd=repo, env=env, timeout=3600)
    checked = 0
    for number in numbers:
        name = "chapter%02d" % number
        command = ["cargo", "test", "--locked", "--offline", "--no-fail-fast", "--test", name]
        code, out, err = course.run(command + ["--", "--color", "never"], cwd=repo, env=env, timeout=timeout)
        if code is None:
            problems.append("%s: the starter tests did not finish in %d s" % (name, timeout))
            continue
        if "test result:" not in out:
            last = [line for line in err.splitlines() if line.strip()][-3:]
            problems.append("%s: the starter tests did not compile or crashed:\n  %s" % (name, "\n  ".join(last)))
            continue
        results = dict(re.findall(r"^test (\S+) \.\.\. (\w+)", out, re.M))
        for test, result in sorted(results.items()):
            if not test.endswith("::p_predictions"):
                continue
            checked += 1
            if result != "FAILED":
                problems.append("%s %s: passes in the starter; it must fail at guess()" % (name, test))
                continue
            block = re.search(r"^---- %s stdout ----\n(.*?)(?=^---- |\Z)" % re.escape(test), out, re.M | re.S)
            if not block or "PREDICT:" not in block.group(1):
                problems.append("%s %s: fails, but not at guess() (no PREDICT: message)" % (name, test))
    return checked


def compile_packages(repo, timeout, problems):
    """Compile (do not run) the starter tests of every standalone package."""
    compiled = 0
    env = dict(os.environ, CARGO_TERM_COLOR="never")
    for package in course.packages(repo):
        manifest = repo / package / "Cargo.toml"
        if course.os_name() in course.course_metadata(manifest, "skip-os"):
            print("      skipped %s: its Cargo.toml skips %s" % (package, course.os_name()), flush=True)
            continue
        command = ["cargo", "test", "--no-run", "--all-targets"]
        if (repo / package / "Cargo.lock").is_file():
            command.append("--locked")
        command += course.course_metadata(manifest, "test-args")
        code, out, err = course.run(command, cwd=repo / package, env=env, timeout=timeout)
        compiled += 1
        if code != 0:
            why = "timed out" if code is None else "does not compile"
            last = [line for line in err.splitlines() if line.strip()][-12:]
            problems.append("%s: %s (%s)\n  %s" % (package, why, " ".join(command), "\n  ".join(last)))
        else:
            print("      compiled %s" % package, flush=True)
    return compiled


def main():
    course.utf8_output()
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--repo", type=pathlib.Path, default=course.REPO)
    parser.add_argument("--strict", action="store_true", help="also require the chapter documents and expected.tsv entries")
    parser.add_argument("--run", action="store_true", help="also run the starter tests and check how the predictions fail")
    parser.add_argument("--forbid", action="append", default=[], metavar="PHRASE", help="one more forbidden phrase. May be repeated.")
    parser.add_argument("--allow", action="append", default=[], metavar="PHRASE",
                        help="never treat this phrase as forbidden (it may come from COURSE_FORBIDDEN). May be repeated.")
    parser.add_argument("--no-phrases", action="store_true", help="skip the forbidden-phrase check")
    parser.add_argument("--compile-packages", action="store_true",
                        help="also compile the starter tests of every standalone package")
    parser.add_argument("--package-timeout", type=int, default=3600, help="seconds for one package (default 3600)")
    parser.add_argument("--timeout", type=int, default=600, help="seconds for one chapter with --run (default 600)")
    args = parser.parse_args()
    repo = args.repo.resolve()

    failed = False

    def section(title, problems):
        nonlocal failed
        print("%s  %s" % ("FAIL" if problems else "PASS", title))
        for problem in problems[:200]:
            print("        " + problem.replace("\n", "\n        "))
        if len(problems) > 200:
            print("        ... and %d more" % (len(problems) - 200))
        failed = failed or bool(problems)
        sys.stdout.flush()

    problems = []
    lessons = check_chapters(repo, args.strict, problems)
    section("%d chapters, %d lesson files" % (len(course.chapters(repo)), lessons), problems)

    problems = []
    drills = check_drills(repo, args.strict, problems)
    section("%d drill sets, %d drills" % (len(course.drill_sets(repo)), drills), problems)

    if not args.no_phrases:
        allowed = {phrase.strip().lower() for phrase in args.allow if phrase.strip()}
        allowed |= {p.strip().lower() for p in os.environ.get("COURSE_ALLOWED", "").split(",") if p.strip()}
        phrases = set(DEFAULT_PHRASES) | set(args.forbid)
        phrases |= {p.strip() for p in os.environ.get("COURSE_FORBIDDEN", "").split(",") if p.strip()}
        phrases = {phrase for phrase in phrases if phrase.lower() not in allowed}
        problems = []
        scanned = check_phrases(repo, phrase_pattern(phrases), problems)
        section("forbidden phrases: %d files scanned for %d phrases" % (scanned, len(phrases)), problems)

    if args.run:
        problems = []
        checked = run_starters(repo, args.timeout, problems)
        section("starter run: %d p_predictions tests fail at guess()" % checked, problems)

    if args.compile_packages:
        problems = []
        compiled = compile_packages(repo, args.package_timeout, problems)
        section("standalone packages: %d starters compile" % (compiled - len(problems)), problems)

    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main())

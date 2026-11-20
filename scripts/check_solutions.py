#!/usr/bin/env python3
"""Check that the model solutions in solutions/ really solve the course.

The script never changes the repository. It copies the course to a temporary
directory, lays solutions/ over the copy (a solution may also add files that
have no starter), and then runs these checks:

  tests     every solution keeps the starter's tests: its `mod tests` block
            and every other module compiled only for tests (or, for a file
            under a tests/ directory, the whole file) equals the starter's,
            except for the answers to `guess()` predictions.
  drills    every starter drill produces the outcome recorded in
            solutions/drills/<key>/expected.tsv, and every solved drill
            compiles, exits with 0 and still has every `// KEEP` line.
  chapters  the root package: `cargo test --test chapterNN` passes for
            every chapter, in the debug and the release profile.
  packages  every standalone package that has files under solutions/
            (chapter labs, the project, the ecosystem track, mastery
            modules, capstones and checkpoints): `cargo test` passes in both
            profiles, plus the package's extra runs (see QUIRKS below).

Optional checks (add them with --starters, --miri, --loom or --checks):

  starters  the unchanged starters compile (`cargo test --no-run`) in both
            profiles, built in the repository with a target directory of
            their own, so a starter build never reuses a solved binary.
  miri      the solved unsafe code under Miri, with the nightly toolchain
            named by MIRI_TOOLCHAIN (default: nightly-2026-11-20,
            the dated nightly that CI and the README name).
  loom      the solved concurrency module under loom.

A unit is a chapter (chNN: its tests, drills and labs), the project, the
ecosystem track, a mastery module (m2_unsafe or m2: the package and drill
set m2), a capstone (cap_game, or game) or a checkpoint (cp1_after_ch07, or
cp1). Groups: core (all chapters), mastery, capstones, checkpoints, labs,
packages (every standalone package), drills (only the drill sets).

  python3 scripts/check_solutions.py --list            # the units
  python3 scripts/check_solutions.py                   # everything
  python3 scripts/check_solutions.py --only ch04       # one chapter, its drills and labs
  python3 scripts/check_solutions.py --only project,cap_game --starters
  python3 scripts/check_solutions.py --only m2_unsafe --checks miri
  python3 scripts/check_solutions.py --only m1 --profile debug --repeat 3 --clippy

A package can pass extra arguments to `cargo test`, skip an operating system,
or let learners change some of its tests, in its Cargo.toml:

  [package.metadata.course]
  test-args = ["--no-default-features"]
  skip-os = ["windows"]          # linux, macos or windows
  editable-tests = ["fixture/"]  # tests under these paths may differ from the starter

Separate CARGO_TARGET_DIRs: the starter build, the solved copy, each Miri and
each loom run build into directories of their own. Cargo decides freshness by
file times, and a copy keeps the times of the original, so a shared target
directory could silently run a binary built from the other tree.

Exit status: 0 when every check passed, 1 otherwise.
"""
import argparse
import concurrent.futures
import difflib
import json
import os
import pathlib
import re
import shutil
import stat
import sys
import tempfile
import time

import course

# Per-package settings that the package's Cargo.toml does not carry (yet).
# Lists from [package.metadata.course] are added to these.
#
#   variants   replaces the single `cargo test` run: (label, extra arguments)
#   extra      more runs on the solved copy, once, in the debug profile:
#              (label, cargo arguments, environment, sub-directory, when)
#              `when` is "always", "clippy" (only with --clippy) or
#              "target:<triple>" (only when that target is installed)
#   miri       Miri runs on the solved copy: (label, arguments after
#              `cargo +<nightly>`, environment, sub-directory)
#   loom       loom runs on the solved copy: (label, cargo arguments, environment)
#   needs      what the machine needs besides the pinned toolchain; CI reads it
#   solved-by  repository paths outside the package whose solutions it
#              compiles (#[path] modules); the package is tested when any
#              of them has a file under solutions/
WINDOW_LIBS = "window-libs"
QUIRKS = {
    "exercises/chapter22/export_lab": {
        # The library includes the chapter's l06_exports.rs with #[path]:
        # the real check that its macros use `$crate` from another crate.
        "solved-by": ["exercises/chapter22/l06_exports.rs"],
    },
    "exercises/chapter08/label_lab": {
        # The lab has tests behind a feature.
        "variants": [("default features", []), ("all features", ["--all-features"])],
    },
    "project": {
        "extra": [
            ("m23 with the optional tests", ["test", "--test", "m23_unsafe", "--", "--include-ignored"], {}, "", "always"),
        ],
        "miri": [
            ("m23_unsafe", ["miri", "test", "--test", "m23_unsafe", "--", "--include-ignored"], {}, ""),
        ],
    },
    "mastery/m2_unsafe": {
        "miri": [
            ("tests", ["miri", "test"], {}, ""),
            ("tests, tree borrows", ["miri", "test"], {"MIRIFLAGS": "-Zmiri-tree-borrows"}, ""),
            # The Miri labs: every solved examples/miri_lNN_*.rs is added at
            # run time (see miri_runs); aliasing.rs and miri_l06_queue.rs
            # contain undefined behaviour on purpose and have no solution.
        ],
        "miri-examples": True,
        "miri-example-env": {"miri_l04_drain": {"MIRIFLAGS": "-Zmiri-ignore-leaks"}},
    },
    "mastery/m3_concurrency": {
        "loom": [("loom models", None, {})],  # None: the package's `cargo model` alias, see loom_runs
    },
    "mastery/m4_async": {
        # Only the lessons with unsafe code; the others use sockets and
        # timers that Miri cannot run.
        "miri": [
            ("l01_", ["miri", "test", "--lib", "l01_"], {}, ""),
            ("l05_", ["miri", "test", "--lib", "l05_"], {}, ""),
            ("l06_", ["miri", "test", "--lib", "l06_"], {}, ""),
            ("c01_", ["miri", "test", "--lib", "c01_"], {}, ""),
        ],
    },
    "mastery/m5_proc_macros": {"needs": ["network"]},
    "mastery/m6_ffi": {
        "needs": ["c-compiler"],
        # Not plain `cargo miri test`: the doc tests call C functions.
        "miri": [
            ("lib", ["miri", "test", "--lib"], {}, ""),
            ("miri_export", ["miri", "test", "--test", "miri_export"], {}, ""),
        ],
    },
    "mastery/m7_perf": {
        "needs": ["c-compiler", "network"],
        # Every criterion benchmark once, as a test: it must still build and run.
        "extra": [("benchmarks run once", ["test", "--benches"], {}, "", "always")],
    },
    "mastery/m9_toolbox": {
        "needs": ["network"],
        "extra": [
            ("lab answers", ["test", "--no-fail-fast", "--", "--ignored", "lab_"], {}, "", "always"),
        ],
    },
    "mastery/m10_contributing": {
        # `editable-tests = ["fixture/"]` is in its Cargo.toml.
        "needs": ["git"],
    },
    "mastery/m11_services": {"needs": ["c-compiler", "network"]},
    "capstones/game": {
        "variants": [("window", []), ("no window", ["--no-default-features"])],
        "needs": [WINDOW_LIBS],
    },
    "capstones/kernel": {
        "needs": ["qemu", "aarch64"],
        "extra": [
            ("bare-metal kernel clippy",
             ["clippy", "--target", "aarch64-unknown-none", "--", "-D", "warnings"], {}, "kernel", "clippy"),
        ],
        "miri": [("kcore", ["miri", "test", "-p", "kcore"], {}, "")],
    },
    "capstones/nostd_collections": {
        "needs": ["aarch64"],
        "extra": [
            ("bare-metal build", ["build", "--lib", "--target", "aarch64-unknown-none"], {},
             "", "target:aarch64-unknown-none"),
        ],
        "miri": [("tests", ["miri", "test"], {}, "")],
    },
}

# Miri runs of chapters (root package), by chapter number. The tests of
# chapter 24's lesson 08 that call C library functions carry
# #[cfg_attr(miri, ignore)], so no test needs to be skipped by name here.
CHAPTER_MIRI = {
    23: [("chapter23", ["miri", "test", "--offline", "--test", "chapter23"], {})],
    24: [("chapter24", ["miri", "test", "--offline", "--test", "chapter24"], {})],
}
CHAPTER_NEEDS = {}


class Report:
    def __init__(self):
        self.rows = []

    def add(self, status, unit, check, detail="", seconds=None):
        self.rows.append((status, unit, check, seconds))
        took = "" if seconds is None else "  (%.1f s)" % seconds
        print("%-4s  %-22s %s%s" % (status, unit, check, took), flush=True)
        if detail:
            for line in detail.rstrip().splitlines():
                print("        " + line)
            sys.stdout.flush()

    def count(self, status):
        return sum(1 for row in self.rows if row[0] == status)

    def table(self):
        """The summary: one line per unit, then every row that did not pass."""
        units = {}
        for status, unit, _, seconds in self.rows:
            entry = units.setdefault(unit, {"PASS": 0, "FAIL": 0, "SKIP": 0, "seconds": 0.0})
            entry[status] += 1
            entry["seconds"] += seconds or 0.0
        width = max([len("Unit")] + [len(unit) for unit in units])
        lines = ["%-*s  %-6s %5s %5s %5s %8s" % (width, "Unit", "Result", "pass", "fail", "skip", "time")]
        lines.append("-" * len(lines[0]))
        for unit, entry in units.items():
            result = "FAIL" if entry["FAIL"] else ("PASS" if entry["PASS"] else "SKIP")
            lines.append(
                "%-*s  %-6s %5d %5d %5d %7.0fs"
                % (width, unit, result, entry["PASS"], entry["FAIL"], entry["SKIP"], entry["seconds"])
            )
        problems = [row for row in self.rows if row[0] != "PASS"]
        if problems:
            lines += ["", "Not passed:"]
            lines += ["  %-4s  %-*s  %s" % (status, width, unit, check) for status, unit, check, _ in problems]
        return "\n".join(lines)


def tail(text, lines=60):
    rows = text.rstrip().splitlines()
    if len(rows) > lines:
        rows = ["... (%d earlier lines not shown)" % (len(rows) - lines)] + rows[-lines:]
    return "\n".join(rows)


def failure_summary(out, err):
    """The useful part of a failed `cargo test`: the failures section when
    tests ran, the compiler's messages otherwise."""
    marker = out.find("\nfailures:\n")
    if marker >= 0:
        return tail(out[marker + 1 :], 60) + "\n" + tail(err, 6)
    return tail(err, 50)


def setting(repo, package, key):
    """A list setting of a package: QUIRKS first, then its Cargo.toml."""
    value = list(QUIRKS.get(package, {}).get(key, []))
    manifest = repo / package / "Cargo.toml"
    if manifest.is_file():
        for item in course.course_metadata(manifest, key):
            if item not in value:
                value.append(item)
    return value


def dedupe(arguments):
    seen, out = set(), []
    for argument in arguments:
        if argument.startswith("-") and argument in seen:
            continue
        seen.add(argument)
        out.append(argument)
    return out


# ------------------------------------------------------------------ the units


class Unit:
    """A named bundle of chapters, packages and drill sets."""

    def __init__(self, key, group):
        self.key, self.group = key, group
        self.chapters, self.packages, self.drills = [], [], []

    def needs(self, repo):
        found = []
        for number in self.chapters:
            found += CHAPTER_NEEDS.get(number, [])
        for package in self.packages:
            found += QUIRKS.get(package, {}).get("needs", [])
        return sorted(set(found))

    def has_miri(self):
        return any(n in CHAPTER_MIRI for n in self.chapters) or any(
            QUIRKS.get(p, {}).get("miri") or QUIRKS.get(p, {}).get("miri-examples") for p in self.packages
        )

    def has_loom(self):
        return any(QUIRKS.get(p, {}).get("loom") for p in self.packages)


def package_key(package):
    parts = package.split("/")
    if parts[0] == "capstones" and len(parts) == 2:
        return "cap_" + parts[1]
    if parts[0] in ("mastery", "checkpoints") and len(parts) == 2:
        return parts[1]
    return package.replace("/", "_")


def discover_units(repo):
    chapters = course.chapters(repo)
    packages = course.packages(repo)
    drills = course.drill_sets(repo)
    units, used_packages, used_drills = [], set(), set()
    for number in sorted(chapters):
        unit = Unit("ch%02d" % number, "core")
        unit.chapters.append(number)
        key = "ch%02d" % number
        if key in drills:
            unit.drills.append(key)
            used_drills.add(key)
        prefix = "exercises/chapter%02d/" % number
        for package in packages:
            if package.startswith(prefix):
                unit.packages.append(package)
                used_packages.add(package)
        units.append(unit)
    for package in packages:
        if package in used_packages:
            continue
        top = package.split("/")[0]
        group = top if top in ("mastery", "capstones", "checkpoints") else "tracks"
        unit = Unit(package_key(package), group)
        unit.packages.append(package)
        module = re.match(r"mastery/(m\d+)_", package)
        if module and module.group(1) in drills:
            unit.drills.append(module.group(1))
            used_drills.add(module.group(1))
        units.append(unit)
    for key in drills:
        if key not in used_drills:
            unit = Unit("drills_" + key, "drills")
            unit.drills.append(key)
            units.append(unit)
    # Course order: chapters, the project and the ecosystem track,
    # checkpoints, mastery modules, capstones.
    rank = {"core": 0, "tracks": 1, "checkpoints": 2, "mastery": 3, "capstones": 4, "drills": 5}
    units.sort(key=lambda unit: rank[unit.group])  # stable: natural order inside a group
    return units, chapters, packages, drills


class Selection:
    """Which chapters, packages and drill sets the --only options choose."""

    def __init__(self, repo, only):
        self.units, self.all_chapters, self.all_packages, self.all_drills = discover_units(repo)
        self.chapters, self.packages, self.drills = [], [], []
        if not only:
            for unit in self.units:
                self.take(unit)
            return
        for selector in only:
            if not self.choose(selector.strip().strip("/")):
                raise SystemExit(
                    "error: --only %s matches nothing. Units: %s. Groups: core mastery capstones "
                    "checkpoints labs packages drills." % (selector, " ".join(u.key for u in self.units))
                )

    def add(self, bucket, items):
        for item in items:
            if item not in bucket:
                bucket.append(item)
        return bool(items)

    def take(self, unit):
        self.add(self.chapters, unit.chapters)
        self.add(self.packages, unit.packages)
        self.add(self.drills, unit.drills)
        return True

    def unit_of(self, kind, item):
        for unit in self.units:
            if item in getattr(unit, kind):
                return unit.key
        return "?"

    def chosen_units(self):
        """Units that have at least one selected part, in course order."""
        return [
            unit
            for unit in self.units
            if set(unit.chapters) & set(self.chapters)
            or set(unit.packages) & set(self.packages)
            or set(unit.drills) & set(self.drills)
        ]

    def choose(self, selector):
        lower = selector.lower()
        if lower in ("core", "chapters"):
            return all(self.take(u) for u in self.units if u.group == "core") or True
        if lower in ("mastery", "capstones", "checkpoints"):
            return any([self.take(u) for u in self.units if u.group == lower])
        if lower == "packages":
            return self.add(self.packages, self.all_packages) or True
        if lower == "labs":
            return self.add(self.packages, [p for p in self.all_packages if p.startswith("exercises/")])
        if lower == "drills":
            return self.add(self.drills, list(self.all_drills)) or True
        chapter = re.fullmatch(r"(?:chapter|ch)?(\d{1,2})", lower)
        if chapter:
            key = "ch%02d" % int(chapter.group(1))
            return any([self.take(u) for u in self.units if u.key == key])
        found = False
        for unit in self.units:
            if unit.group == "core":
                # A chapter lab on its own: exercises/chapter17/regex-lab or regex-lab.
                labs = [p for p in unit.packages if lower in (p.lower(), p.split("/")[-1].lower())]
                found = self.add(self.packages, labs) or found
                continue
            names = {unit.key.lower()} | {d.lower() for d in unit.drills}
            for package in unit.packages:
                last = package.split("/")[-1].lower()
                names |= {package.lower(), last}
                if lower == last.split("_")[0]:
                    names.add(lower)  # m2 for m2_unsafe, cp1 for cp1_after_ch07
            if lower in names:
                found = self.take(unit) or found
        return found


# ------------------------------------------------------------- the solved copy


def remove_tree(path):
    # Read-only files (Windows marks some build output so) are made writable
    # and removed on a second try.
    def retry(function, target, _info):
        os.chmod(target, stat.S_IWRITE)
        function(target)

    if sys.version_info >= (3, 12):
        shutil.rmtree(path, onexc=retry)
    else:
        shutil.rmtree(path, onerror=retry)


def solution_files(solutions):
    """Files to lay over the course: everything under solutions/ except the
    drills (checked separately) and files directly in solutions/ itself."""
    files = []
    for path in course.walk(solutions):
        relative = path.relative_to(solutions)
        if len(relative.parts) > 1 and relative.parts[0] != "drills":
            files.append(relative)
    return files


def build_solved_copy(repo, solutions, destination):
    """Copy the course to `destination` and lay the solutions over it."""
    course.copy_course(repo, destination)
    files = solution_files(solutions)
    for relative in files:
        target = destination / relative
        target.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(solutions / relative, target)
    return files


# ------------------------------------------------------------------ the checks


def normal(text):
    """Test text as lines, without differences a finished lesson must have:
    once every prediction is answered, the `guess` import is unused, so a
    solution may drop it (`use practice::{alloc::count, guess};` becomes
    `use practice::alloc::count;`)."""
    lines = []
    for line in text.splitlines():
        line = line.rstrip()
        if re.match(r"\s*use\b.*\bguess\b.*;$", line):
            line = re.sub(r"\{\s*guess\s*,\s*", "{", line)
            line = re.sub(r",\s*guess\s*(?=[,}])", "", line)
            line = re.sub(r"::\{\s*([\w:]+)\s*\}", r"::\1", line)
            if re.match(r"\s*use\s+[\w:]*\bguess\s*;$", line):
                continue
        elif re.match(r"\s*use\b.*::\{\s*[\w:]+\s*\};$", line):
            line = re.sub(r"::\{\s*([\w:]+)\s*\}", r"::\1", line)
        lines.append(line)
    return lines


def editable(repo, selection, posix):
    """True when the package of this file lets learners change its tests
    (`editable-tests`, path prefixes inside the package). Used by modules
    whose task is to write or extend tests."""
    for package in selection.all_packages:
        if posix.startswith(package + "/"):
            inside = posix[len(package) + 1 :]
            prefixes = setting(repo, package, "editable-tests")
            return any(inside.startswith(prefix.lstrip("./")) for prefix in prefixes)
    return False


def check_test_identity(repo, solutions, selection, report):
    """A solution may change bodies and prediction answers, not tests."""
    for unit in selection.chosen_units():
        chapter_dirs = {"exercises/chapter%02d/" % n for n in unit.chapters if n in selection.chapters}
        packages = [p for p in unit.packages if p in selection.packages]
        if not chapter_dirs and not packages:
            continue
        checked, problems = 0, []
        for relative in solution_files(solutions):
            posix = relative.as_posix()
            if relative.suffix != ".rs":
                continue
            in_package = any(posix.startswith(p + "/") for p in packages)
            in_chapter = any(posix.startswith(prefix) for prefix in chapter_dirs) and not any(
                posix.startswith(p + "/") for p in unit.packages
            )
            if not (in_chapter or in_package):
                continue
            starter = repo / relative
            if not starter.is_file():
                # New files are normal in blank-file projects, but a lesson
                # file without a starter is a misspelled name.
                if re.fullmatch(r"exercises/chapter\d\d/[^/]+\.rs", posix):
                    problems.append("%s: no starter file with this name" % posix)
                continue
            if in_package and editable(repo, selection, posix):
                continue
            whole = "tests" in relative.parts[:-1]
            starter_text, solved_text = course.read(starter), course.read(solutions / relative)
            starter_regions = course.test_regions(starter_text, whole)
            solved_regions = course.test_regions(solved_text, whole)
            if not starter_regions:
                continue
            checked += 1
            if len(starter_regions) != len(solved_regions):
                problems.append(
                    "%s: the starter has %d test module(s), the solution has %d"
                    % (posix, len(starter_regions), len(solved_regions))
                )
                continue
            for (a, b), (c, d) in zip(starter_regions, solved_regions):
                heads = course.prediction_heads(starter_text[a:b])
                want, _ = course.mask_predictions(starter_text[a:b], heads)
                have, missing = course.mask_predictions(solved_text[c:d], heads)
                if missing:
                    problems.append("%s: prediction statement not found in the solution: %s" % (posix, missing[0]))
                    break
                if re.search(r"\bguess\s*\(\s*\)", course.blank_non_code(solved_text[c:d])):
                    problems.append("%s: the solution still contains guess()" % posix)
                    break
                if normal(want) != normal(have):
                    diff = difflib.unified_diff(
                        normal(want), normal(have), "starter tests", "solution tests", lineterm="", n=1
                    )
                    problems.append("%s: tests differ from the starter:\n%s" % (posix, "\n".join(list(diff)[:24])))
                    break
        name = "tests unchanged (%d files)" % checked
        if problems:
            report.add("FAIL", unit.key, name, "\n".join(problems))
        elif checked:
            report.add("PASS", unit.key, name)


class Targets:
    """Build output directories. Every tree and every kind of run gets its
    own, under --target-dir when given, else under the work directory."""

    def __init__(self, base):
        self.base = pathlib.Path(base).resolve()

    def env(self, kind, slug, extra=None):
        env = dict(os.environ)
        env.pop("CARGO_TARGET_DIR", None)
        env["CARGO_TARGET_DIR"] = str(self.base / kind / slug)
        env["CARGO_TERM_COLOR"] = "never"
        env.update(extra or {})
        return env


def slug(text):
    return re.sub(r"\W+", "_", text)


def profiles(args):
    return {"debug": [[]], "release": [["--release"]], "both": [[], ["--release"]]}[args.profile]


def locked(directory):
    return ["--locked"] if (directory / "Cargo.lock").is_file() else []


def timed(report, unit, name, command, cwd, env, timeout, ok=None):
    """Run one command and report it. Returns True when it passed."""
    started = time.time()
    code, out, err = course.run(command, cwd=cwd, env=env, timeout=timeout)
    passed = code == 0 if ok is None else ok(code, out)
    if passed:
        report.add("PASS", unit, name, seconds=time.time() - started)
    else:
        why = "timed out after %d s\n" % timeout if code is None else ""
        why += "$ %s\n" % " ".join(str(part) for part in command)
        report.add("FAIL", unit, name, why + failure_summary(out, err), time.time() - started)
    return passed


def check_starters(repo, selection, targets, args, report):
    """The unchanged starters compile in both profiles. They are built in
    the repository, into a target directory that no solved copy uses."""
    for number in selection.chapters:
        unit = selection.unit_of("chapters", number)
        env = targets.env("starter", "root")
        for profile in profiles(args):
            label = "release" if profile else "debug"
            command = ["cargo", "test", "--locked", "--offline", "--no-run"] + profile
            command += ["--test", "chapter%02d" % number]
            timed(report, unit, "starter compiles (%s)" % label, command, repo, env, args.timeout)
    for package in selection.packages:
        unit = selection.unit_of("packages", package)
        directory = repo / package
        if course.os_name() in setting(repo, package, "skip-os"):
            report.add("SKIP", unit, "%s starter: its Cargo.toml skips %s" % (package, course.os_name()))
            continue
        env = targets.env("starter", slug(package))
        for variant, variant_args in QUIRKS.get(package, {}).get("variants", [("", [])]):
            for profile in profiles(args):
                label = ", ".join(x for x in ("release" if profile else "debug", variant) if x)
                command = ["cargo", "test", "--no-run", "--all-targets"] + locked(directory) + profile
                command = dedupe(command + setting(repo, package, "test-args") + variant_args)
                timed(report, unit, "%s starter compiles (%s)" % (package, label), command, directory, env,
                      args.package_timeout)


def check_chapters(repo, solutions, work, selection, targets, args, report):
    """The root package: every chapter's tests in both profiles."""
    ready = []
    for number in selection.chapters:
        unit = selection.unit_of("chapters", number)
        name = "chapter%02d" % number
        solved_dir = solutions / "exercises" / name
        lessons = course.lesson_files(repo / "exercises" / name)
        missing = [f.name for f in lessons if not (solved_dir / f.name).is_file()]
        if not solved_dir.is_dir() or len(missing) == len(lessons):
            report.add("FAIL" if args.strict else "SKIP", unit, "no solutions under solutions/exercises/%s" % name)
        elif missing:
            report.add("FAIL", unit, "lessons without a solution file", " ".join(missing))
        else:
            ready.append(number)
    if not ready:
        return
    # Lessons whose starter has tests: each must have a passing test in the
    # solved copy, so a test module that is no longer compiled is noticed.
    tested = {
        number: [f.stem for f in course.lesson_files(repo / "exercises" / ("chapter%02d" % number))
                 if "#[test]" in course.read(f)]
        for number in ready
    }
    env = targets.env("solved", "root")
    for profile in profiles(args):
        label = "release" if profile else "debug"
        # One parallel build first; a chapter that does not compile is then
        # reported by its own test run below.
        build = ["cargo", "build", "--locked", "--offline", "--keep-going"] + profile
        for number in ready:
            build += ["--test", "chapter%02d" % number]
        course.run(build, cwd=work, env=env, timeout=args.timeout)
        for number in ready:
            unit = selection.unit_of("chapters", number)
            command = ["cargo", "test", "--locked", "--offline", "--no-fail-fast"] + profile
            command += ["--test", "chapter%02d" % number]
            if args.clippy and not profile:
                lint = ["cargo", "clippy", "--locked", "--offline", "--test", "chapter%02d" % number,
                        "--", "-D", "warnings"]
                timed(report, unit, "clippy", lint, work, env, args.timeout)
            for round_number in range(args.repeat):
                started = time.time()
                code, out, err = course.run(command, cwd=work, env=env, timeout=args.timeout)
                name = "tests (%s%s)" % (label, ", run %d" % (round_number + 1) if args.repeat > 1 else "")
                ran = set(re.findall(r"^test (\w+)::.* \.\.\. ok\s*$", out, re.M))
                silent = [stem for stem in tested[number] if stem not in ran]
                if code == 0 and "test result: ok." in out and not silent:
                    report.add("PASS", unit, name, seconds=time.time() - started)
                elif code == 0 and "test result: ok." in out:
                    report.add(
                        "FAIL", unit, name,
                        "no test of these lessons ran and passed: %s\n"
                        "Is the lesson still declared in mod.rs, and is its `mod tests` still under `#[cfg(test)]`?"
                        % " ".join(silent),
                        time.time() - started,
                    )
                    break
                else:
                    why = "timed out after %d s\n" % args.timeout if code is None else ""
                    report.add("FAIL", unit, name, why + failure_summary(out, err), time.time() - started)
                    break


def has_solutions(solutions, package):
    solved_dir = solutions / package
    if solved_dir.is_dir() and any(course.walk(solved_dir)):
        return True
    return any((solutions / path).is_file() for path in QUIRKS.get(package, {}).get("solved-by", []))


def target_installed(cwd, triple):
    """True when the toolchain that rust-toolchain.toml selects in `cwd`
    has the standard library for `triple`."""
    code, out, _ = course.run(["rustc", "--print", "sysroot"], cwd=cwd, timeout=120)
    return code == 0 and (pathlib.Path(out.strip()) / "lib" / "rustlib" / triple).is_dir()


def check_packages(repo, solutions, work, selection, targets, args, report):
    """Standalone packages that have solutions."""
    for package in selection.packages:
        unit = selection.unit_of("packages", package)
        if not has_solutions(solutions, package):
            report.add("SKIP", unit, "%s: no solutions under solutions/%s" % (package, package))
            continue
        if course.os_name() in setting(repo, package, "skip-os"):
            report.add("SKIP", unit, "%s: its Cargo.toml skips %s" % (package, course.os_name()))
            continue
        directory = work / package
        lock = locked(directory)
        if not lock:
            print("note  %s has no Cargo.lock; testing without --locked" % package, flush=True)
        extra = setting(repo, package, "test-args")
        env = targets.env("solved", slug(package))
        quirks = QUIRKS.get(package, {})
        for variant, variant_args in quirks.get("variants", [("", [])]):
            arguments = dedupe(lock + extra + variant_args)
            for profile in profiles(args):
                label = ", ".join(x for x in ("release" if profile else "debug", variant) if x)
                if args.clippy and not profile:
                    lint = ["cargo", "clippy", "--all-targets"] + arguments + ["--", "-D", "warnings"]
                    timed(report, unit, "%s clippy%s" % (package, " (%s)" % variant if variant else ""),
                          lint, directory, env, args.package_timeout)
                command = ["cargo", "test", "--no-fail-fast"] + profile + arguments
                for round_number in range(args.repeat):
                    name = "%s tests (%s%s)" % (package, label, ", run %d" % (round_number + 1) if args.repeat > 1 else "")
                    if not timed(report, unit, name, command, directory, env, args.package_timeout):
                        break
        for label, cargo_args, extra_env, sub, when in quirks.get("extra", []):
            cwd = directory / sub if sub else directory
            if when == "clippy" and not args.clippy:
                continue
            if when.startswith("target:") and not target_installed(cwd, when[len("target:"):]):
                report.add("SKIP", unit, "%s %s: target %s is not installed (rustup target add %s)"
                           % (package, label, when[7:], when[7:]))
                continue
            command = ["cargo"] + cargo_args[:1] + locked(cwd) + cargo_args[1:]
            # A test run with a filter must run something: a filter that
            # matches no test would pass silently.
            ran = None
            if cargo_args[0] == "test" and "--" in cargo_args:
                ran = lambda code, out: code == 0 and sum(
                    int(n) for n in re.findall(r"^test result: \w+\. (\d+) passed", out, re.M)) > 0
            timed(report, unit, "%s %s" % (package, label), command, cwd,
                  targets.env("solved", slug(package + "/" + sub) if sub else slug(package), extra_env),
                  args.package_timeout, ran)


def miri_runs(repo, solutions, selection):
    """(unit, label, cwd relative to the copy, arguments after `cargo +<MIRI_TOOLCHAIN>`, env, target slug)."""
    runs = []
    for number in selection.chapters:
        if not (solutions / "exercises" / ("chapter%02d" % number)).is_dir():
            continue
        for label, arguments, env in CHAPTER_MIRI.get(number, []):
            runs.append((selection.unit_of("chapters", number), label, ".", arguments[:2] + ["--locked"] + arguments[2:],
                         env, "root"))
    for package in selection.packages:
        quirks = QUIRKS.get(package, {})
        if not has_solutions(solutions, package):
            continue
        unit = selection.unit_of("packages", package)
        for label, arguments, env, sub in quirks.get("miri", []):
            cwd = package + ("/" + sub if sub else "")
            runs.append((unit, "%s %s" % (package, label), cwd, arguments[:2] + ["--locked"] + arguments[2:], env,
                         slug(package)))
        if quirks.get("miri-examples"):
            for example in sorted((solutions / package / "examples").glob("miri_*.rs")):
                env = quirks.get("miri-example-env", {}).get(example.stem, {})
                runs.append((unit, "%s example %s" % (package, example.stem), package,
                             ["miri", "run", "--locked", "--example", example.stem], env, slug(package)))
    return runs


def check_miri(repo, solutions, work, selection, targets, args, report):
    toolchain = os.environ.get("MIRI_TOOLCHAIN", "nightly-2026-11-20")
    for unit, label, cwd, arguments, env, target_slug in miri_runs(repo, solutions, selection):
        command = ["cargo", "+" + toolchain] + arguments
        timed(report, unit, "miri: %s" % label, command, work / cwd,
              targets.env("miri", target_slug, env), args.package_timeout)


def check_loom(repo, solutions, work, selection, targets, args, report):
    for package in selection.packages:
        runs = QUIRKS.get(package, {}).get("loom", [])
        if not runs or not has_solutions(solutions, package):
            continue
        unit = selection.unit_of("packages", package)
        directory = work / package
        config = directory / ".cargo" / "config.toml"
        manifest = course.read(directory / "Cargo.toml")
        for label, arguments, env in runs:
            env = dict(env)
            if arguments is None:
                # The module's own alias `cargo model` is what its README
                # teaches. Without it: a `loom` feature, or the cfg flag.
                if config.is_file() and re.search(r"^model\s*=", course.read(config), re.M):
                    arguments = ["model"] + locked(directory)
                elif re.search(r"^loom\s*=\s*\[", manifest, re.M):
                    arguments = ["test", "--release", "--features", "loom"] + locked(directory)
                else:
                    arguments = ["test", "--release", "--lib"] + locked(directory)
                    env["RUSTFLAGS"] = (os.environ.get("RUSTFLAGS", "") + " --cfg loom").strip()
            timed(report, unit, "loom: %s %s" % (package, label), ["cargo"] + arguments, directory,
                  targets.env("loom", slug(package), env), args.package_timeout)


OUTCOME = re.compile(r"ok|panic|error|E\d{4}(,E\d{4})*")


def read_expected(path):
    """Read an expected.tsv. Returns ({drill file: outcome}, problems)."""
    expected, problems = {}, []
    for number, line in enumerate(course.read(path).splitlines(), 1):
        if not line.strip() or line.startswith("#"):
            continue
        name, tab, outcome = line.partition("\t")
        name, outcome = name.strip(), outcome.strip()
        where = "expected.tsv line %d" % number
        if not tab:
            problems.append("%s: no tab between the file name and the outcome: %r" % (where, line))
        elif not OUTCOME.fullmatch(outcome):
            problems.append(
                "%s: outcome %r is not ok, panic, error or error codes joined by commas (E0382 or E0277,E0599)"
                % (where, outcome)
            )
        elif name in expected:
            problems.append("%s: %s is listed twice" % (where, name))
        else:
            expected[name] = outcome
    return expected, problems


def check_drills(repo, solutions, work, selection, args, report):
    """Starter drills behave as recorded; solved drills run cleanly."""
    jobs = []  # (key, kind, file)
    expected_by_key, table_problems = {}, {}
    for key in selection.drills:
        unit = selection.unit_of("drills", key)
        files = selection.all_drills[key]
        table = solutions / "drills" / key / "expected.tsv"
        if not table.is_file():
            report.add("FAIL" if args.strict else "SKIP", unit, "drills/%s: no solutions/drills/%s/expected.tsv" % (key, key))
            continue
        expected_by_key[key], table_problems[key] = read_expected(table)
        for drill in files:
            jobs.append((key, "starter", work / "drills" / key / drill.name))
            solved = solutions / "drills" / key / drill.name
            if solved.is_file():
                jobs.append((key, "solved", solved))
    bin_dir = work / "target" / "drill-check"

    def outcome(job):
        key, kind, path = job
        try:
            return job, course.drill_outcome(path, bin_dir / kind, work, args.drill_timeout)
        except course.ToolError as error:
            return job, ("tool error", str(error))

    started = time.time()
    with concurrent.futures.ThreadPoolExecutor(max_workers=args.jobs) as pool:
        results = dict(pool.map(outcome, jobs))
    for key, expected in expected_by_key.items():
        problems = list(table_problems[key])
        files = selection.all_drills[key]
        names = {drill.name for drill in files}
        for name in sorted(set(expected) - names):
            problems.append("%s: listed in expected.tsv but there is no such drill" % name)
        for drill in files:
            if drill.name not in expected:
                problems.append("%s: not listed in expected.tsv" % drill.name)
                continue
            actual, output = results[(key, "starter", work / "drills" / key / drill.name)]
            if not course.same_outcome(expected[drill.name], actual):
                problems.append(
                    "%s: expected.tsv says %s, the starter gives %s\n%s"
                    % (drill.name, expected[drill.name], actual, tail(output, 12))
                )
            solved = solutions / "drills" / key / drill.name
            if not solved.is_file():
                problems.append("%s: no solved drill in solutions/drills/%s" % (drill.name, key))
                continue
            actual, output = results[(key, "solved", solved)]
            if actual != "ok":
                problems.append("%s: the solved drill gives %s, not ok\n%s" % (drill.name, actual, tail(output, 12)))
            kept = {line.strip() for line in course.read(solved).splitlines()}
            for line in course.keep_lines(course.read(drill)):
                if line not in kept:
                    problems.append("%s: the solved drill lost a KEEP line: %s" % (drill.name, line))
        name = "drills/%s: %d drills" % (key, len(files))
        unit = selection.unit_of("drills", key)
        if problems:
            report.add("FAIL", unit, name, "\n".join(problems), time.time() - started)
        else:
            report.add("PASS", unit, name)
    if expected_by_key:
        print("      (drills took %.1f s)" % (time.time() - started), flush=True)


# ----------------------------------------------------------------------- main


def list_units(repo, solutions, selection, as_json, packages_only):
    units = selection.chosen_units()
    if packages_only:
        for package in selection.packages:
            print(package)
        return
    rows = []
    for unit in units:
        parts = ["ch%02d" % n for n in unit.chapters if n in selection.chapters]
        parts += [p for p in unit.packages if p in selection.packages]
        parts += ["drills/" + d for d in unit.drills if d in selection.drills]
        solved = [p for p in unit.packages if has_solutions(solutions, p)]
        solved += ["ch%02d" % n for n in unit.chapters if (solutions / "exercises" / ("chapter%02d" % n)).is_dir()]
        rows.append({
            "unit": unit.key,
            "group": unit.group,
            "parts": parts,
            "solved": bool(solved),
            "needs": " ".join(unit.needs(repo)),
            "miri": unit.has_miri(),
            "loom": unit.has_loom(),
        })
    if as_json:
        print(json.dumps(rows))
        return
    width = max([4] + [len(row["unit"]) for row in rows])
    print("%-*s  %-11s %-7s %-22s %s" % (width, "Unit", "Group", "Solved", "Extra", "Parts"))
    for row in rows:
        extra = " ".join(x for x in (row["needs"], "miri" if row["miri"] else "", "loom" if row["loom"] else "") if x)
        print("%-*s  %-11s %-7s %-22s %s" % (width, row["unit"], row["group"], "yes" if row["solved"] else "no",
                                             extra or "-", " ".join(row["parts"])))
    print("\n%d units. Select with --only UNIT (comma-separated or repeated)." % len(rows))


ALL_CHECKS = ["tests", "drills", "chapters", "packages", "starters", "miri", "loom"]
DEFAULT_CHECKS = ["tests", "drills", "chapters", "packages"]


def main():
    course.utf8_output()
    parser = argparse.ArgumentParser(
        description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter
    )
    parser.add_argument("--only", action="append", default=[], metavar="UNIT",
                        help="check these units or groups (see above). Comma-separated; may be repeated.")
    parser.add_argument("--list", action="store_true", help="print the selected units and stop")
    parser.add_argument("--json", action="store_true", help="with --list: print the units as JSON (for CI)")
    parser.add_argument("--packages", action="store_true", help="with --list: print only the package paths")
    parser.add_argument("--strict", action="store_true",
                        help="a chapter or drill set without solutions is a failure, not a skip")
    parser.add_argument("--profile", choices=["debug", "release", "both"], default="both")
    parser.add_argument("--repeat", type=int, default=1, metavar="N", help="run each test command N times")
    parser.add_argument("--clippy", action="store_true", help="also require `cargo clippy -- -D warnings` on the solved code")
    parser.add_argument("--starters", action="store_true", help="also compile the unchanged starters in both profiles")
    parser.add_argument("--miri", action="store_true", help="also run the Miri checks (needs MIRI_TOOLCHAIN, default nightly-2026-11-20)")
    parser.add_argument("--loom", action="store_true", help="also run the loom checks")
    parser.add_argument("--checks", metavar="LIST",
                        help="run exactly these checks, comma-separated: " + ",".join(ALL_CHECKS))
    parser.add_argument("--skip", action="append", default=[], choices=ALL_CHECKS,
                        help="leave out one kind of check. May be repeated.")
    parser.add_argument("--repo", type=pathlib.Path, default=course.REPO, help="the course checkout (default: this one)")
    parser.add_argument("--solutions", type=pathlib.Path, help="the solutions tree (default: <repo>/solutions)")
    parser.add_argument("--work-dir", type=pathlib.Path, help="where to put the solved copy (default: a new temporary directory)")
    parser.add_argument("--keep", action="store_true", help="do not delete the solved copy afterwards")
    parser.add_argument("--emit-solved", type=pathlib.Path, metavar="DIR",
                        help="only write the solved copy to DIR (which must not exist) and stop")
    parser.add_argument("--target-dir", type=pathlib.Path,
                        help="keep build output here between runs (subdirectories per tree and kind of run)")
    parser.add_argument("--jobs", type=int, default=os.cpu_count() or 2, help="drills compiled at the same time")
    parser.add_argument("--timeout", type=int, default=900, help="seconds for one chapter command (default 900)")
    parser.add_argument("--package-timeout", type=int, default=3600, help="seconds for one package command (default 3600)")
    parser.add_argument("--drill-timeout", type=int, default=60, help="seconds for one drill program (default 60)")
    args = parser.parse_args()

    repo = args.repo.resolve()
    solutions = (args.solutions or repo / "solutions").resolve()
    if not solutions.is_dir():
        raise SystemExit("error: no solutions directory at %s" % solutions)
    only = [part for value in args.only for part in value.split(",") if part.strip()]
    selection = Selection(repo, only)
    if args.list:
        list_units(repo, solutions, selection, args.json, args.packages)
        return 0

    if args.checks:
        checks = [c.strip() for c in args.checks.split(",") if c.strip()]
        unknown = [c for c in checks if c not in ALL_CHECKS]
        if unknown:
            parser.error("unknown check %s (choose from %s)" % (", ".join(unknown), ", ".join(ALL_CHECKS)))
    else:
        checks = list(DEFAULT_CHECKS)
        checks += [name for name in ("starters", "miri", "loom") if getattr(args, name)]
    checks = [c for c in checks if c not in args.skip]

    if args.emit_solved:
        if args.emit_solved.exists():
            raise SystemExit("error: %s already exists" % args.emit_solved)
        files = build_solved_copy(repo, solutions, args.emit_solved.resolve())
        print("Wrote the solved course (%d solution files laid over it) to %s" % (len(files), args.emit_solved))
        return 0

    if args.work_dir:
        args.work_dir.mkdir(parents=True, exist_ok=True)
    holder = pathlib.Path(tempfile.mkdtemp(prefix="solution-check-", dir=str(args.work_dir) if args.work_dir else None))
    work = holder / "course"
    targets = Targets(args.target_dir if args.target_dir else holder / "target")
    report = Report()
    started = time.time()
    print("Units: %s" % " ".join(unit.key for unit in selection.chosen_units()))
    print("Checks: %s\n" % " ".join(checks), flush=True)
    try:
        if "starters" in checks:
            check_starters(repo, selection, targets, args, report)
        if set(checks) - {"starters"}:
            files = build_solved_copy(repo, solutions, work)
            print("Solved copy: %s (%d solution files laid over the course)\n" % (work, len(files)), flush=True)
        if "tests" in checks:
            check_test_identity(repo, solutions, selection, report)
        if "drills" in checks:
            check_drills(repo, solutions, work, selection, args, report)
        if "chapters" in checks:
            check_chapters(repo, solutions, work, selection, targets, args, report)
        if "packages" in checks:
            check_packages(repo, solutions, work, selection, targets, args, report)
        if "loom" in checks:
            check_loom(repo, solutions, work, selection, targets, args, report)
        if "miri" in checks:
            check_miri(repo, solutions, work, selection, targets, args, report)
    finally:
        if args.keep:
            print("\nKept the solved copy at %s" % work)
        else:
            try:
                remove_tree(holder)
            except OSError as error:
                print("\nnote: could not delete %s: %s" % (holder, error))

    failed = report.count("FAIL")
    print("\n== Summary ==")
    if report.rows:
        print(report.table())
    print(
        "\n%d passed, %d failed, %d skipped in %.0f s"
        % (report.count("PASS"), failed, report.count("SKIP"), time.time() - started)
    )
    if failed == 0 and report.count("PASS") == 0:
        print("Nothing was checked: the selected units have no solutions yet.")
        return 1 if args.strict else 0
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main())

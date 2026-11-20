#!/usr/bin/env python3
"""Compile and run the code blocks of every std-only CONCEPTS.md.

Runs `rustdoc --edition 2024 --test <file>` on each CONCEPTS.md whose code
needs only the standard library: the chapter files under exercises/, and the
files of standalone packages (mastery modules, capstones, ...) whose code
blocks name none of the package's dependencies. A file whose blocks use a
dependency (`serde::...`) is listed as skipped; its code blocks are checked
by the package's own tests (`cargo test --doc`).

  python3 scripts/check_concepts.py            # every std-only CONCEPTS.md
  python3 scripts/check_concepts.py ch04 m1    # only these units
  python3 scripts/check_concepts.py path/to/CONCEPTS.md

Exit status: 0 when every file passed, 1 otherwise.
"""
import argparse
import concurrent.futures
import os
import pathlib
import re
import sys

import course


def package_manifest(repo, path):
    """The Cargo.toml of the standalone package that holds `path`, or None
    when the file belongs to the root package."""
    for folder in path.parents:
        if folder == repo:
            return None
        if (folder / "Cargo.toml").is_file():
            return folder / "Cargo.toml"
    return None


def dependency_names(manifest):
    """Crate names a Cargo.toml depends on, as used in Rust code."""
    text = course.read(manifest)
    names, section = set(), None
    for line in text.splitlines():
        line = line.split("#", 1)[0].strip()
        header = re.match(r"\[+\s*([^\]]+?)\s*\]+$", line)
        if header:
            section = header.group(1)
            dotted = re.search(r"dependencies\.([\w-]+)$", section)
            if dotted:
                names.add(dotted.group(1))
            continue
        if section and re.search(r"(^|\.)(dev-|build-)?dependencies$", section):
            key = re.match(r"([\w-]+)\s*=", line)
            if key:
                names.add(key.group(1))
                package = re.search(r"package\s*=\s*\"([\w-]+)\"", line)
                if package:
                    names.add(package.group(1))
    return {name.replace("-", "_") for name in names}


def code_blocks(text):
    """Rust code blocks of a Markdown file that rustdoc compiles."""
    blocks, block, fence = [], None, None
    for line in text.splitlines():
        stripped = line.strip()
        if block is None:
            match = re.match(r"(`{3,}|~{3,})\s*(.*)$", stripped)
            if match:
                info = [word.strip() for word in match.group(2).split(",") if word.strip()]
                fence = match.group(1)
                rust = not info or info[0] == "rust" or info[0].startswith("compile_fail") or info[0] in (
                    "should_panic", "no_run", "edition2024")
                block = [] if rust and "ignore" not in info and "text" not in info else "skip"
            continue
        if stripped.startswith(fence):
            if block != "skip":
                blocks.append("\n".join(block))
            block = None
            continue
        if block != "skip":
            block.append(line)
    return blocks


def uses_dependency(path, manifest):
    """True when a code block of `path` names a dependency of the package."""
    names = dependency_names(manifest)
    for block in code_blocks(course.read(path)):
        for name in names:
            if re.search(r"(?<![\w:])%s\s*::|extern\s+crate\s+%s\b" % (re.escape(name), re.escape(name)), block):
                return True
    return False


def wanted(repo, path, selectors):
    if not selectors:
        return True
    relative = path.relative_to(repo).as_posix()
    for selector in selectors:
        lower = selector.lower().strip("/")
        chapter = re.fullmatch(r"(?:chapter|ch)?(\d{1,2})", lower)
        if chapter and relative.startswith("exercises/chapter%02d/" % int(chapter.group(1))):
            return True
        parts = relative.split("/")[:-1]
        if relative == selector or relative.startswith(lower + "/"):
            return True
        if any(part == lower or part.startswith(lower + "_") for part in parts):
            return True
    return False


def main():
    course.utf8_output()
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("units", nargs="*", help="chNN, a package name such as m1, or a path (default: all)")
    parser.add_argument("--repo", type=pathlib.Path, default=course.REPO)
    parser.add_argument("--jobs", type=int, default=min(4, os.cpu_count() or 1))
    parser.add_argument("--timeout", type=int, default=900, help="seconds per file (default 900)")
    args = parser.parse_args()
    repo = args.repo.resolve()

    files, skipped = [], []
    for path in course.course_files(repo, "CONCEPTS.md"):
        if path.name != "CONCEPTS.md" or path.relative_to(repo).parts[0] == "solutions":
            continue
        if not wanted(repo, path, args.units):
            continue
        manifest = package_manifest(repo, path)
        if manifest is not None and course.manifest_has_dependencies(manifest) and uses_dependency(path, manifest):
            skipped.append(path)
        else:
            files.append(path)
    if args.units and not files and not skipped:
        raise SystemExit("error: no CONCEPTS.md matches %s" % " ".join(args.units))

    rustdoc = os.environ.get("RUSTDOC", "rustdoc")

    def test(path):
        # The working directory is the repository, so rust-toolchain.toml
        # selects the pinned compiler.
        # The path is given relative to it, which keeps the messages short.
        code, out, err = course.run(
            [rustdoc, "--edition", "2024", "--test", "--color", "never", path.relative_to(repo)],
            cwd=repo,
            timeout=args.timeout,
        )
        return path, code, out, err

    failed = 0
    with concurrent.futures.ThreadPoolExecutor(max_workers=max(1, args.jobs)) as pool:
        for path, code, out, err in pool.map(test, files):
            relative = path.relative_to(repo).as_posix()
            if code == 0:
                blocks = sum(int(n) for n in re.findall(r"^test result: .*?(\d+) passed", out, re.M))
                print("PASS  %s (%d code blocks)" % (relative, blocks))
            else:
                failed += 1
                why = "timed out after %d s" % args.timeout if code is None else "rustdoc exit code %d" % code
                print("FAIL  %s (%s)" % (relative, why))
                shown = [line for line in (out + "\n" + err).splitlines() if not re.match(r"test .* \.\.\. ok$", line)]
                for line in shown[-80:]:
                    print("        " + line)
            sys.stdout.flush()
    for path in skipped:
        print("SKIP  %s (its code uses the package's crates; `cargo test --doc` checks it)" % path.relative_to(repo).as_posix())
    print("\n%d passed, %d failed, %d skipped" % (len(files) - failed, failed, len(skipped)))
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main())

#!/usr/bin/env python3
"""Check that every relative Markdown link in the course resolves.

For every *.md file of the course (tracked by Git, or new and not ignored)
the script checks inline links `[text](target)`, images, reference links
`[text][label]` with their `[label]: target` definitions, and `href="..."`
and `src="..."` attributes in embedded HTML:

  - a relative target must name an existing file or directory;
  - a `#fragment` on a Markdown target must be a heading (or an explicit
    `id`/`name` anchor) of that file, using GitHub's rules for heading ids.

Links to other sites are not fetched; `--external` lists them instead.
Code blocks and inline code are ignored.

  python3 scripts/check_links.py
  python3 scripts/check_links.py --external

Exit status: 0 when every relative link resolves, 1 otherwise.
"""
import argparse
import os
import pathlib
import re
import sys
import urllib.parse

import course

FENCE = re.compile(r"\s{0,3}(`{3,}|~{3,})")
HEADING = re.compile(r"\s{0,3}(#{1,6})\s+(.*?)\s*#*\s*$")
INLINE = re.compile(r"!?\[(?:[^\]\[]|\[[^\]]*\])*\]\(\s*(<[^>]*>|[^)\s]*)(?:\s+(?:\"[^\"]*\"|'[^']*'))?\s*\)")
# Not preceded by a word character, so `v[i][j]` in prose is not a link.
REFERENCE_USE = re.compile(r"(?<![\w\])])\[((?:[^\]\[]|\[[^\]]*\])+)\]\[([^\]]*)\]")
REFERENCE_DEFINITION = re.compile(r"\s{0,3}\[([^\]]+)\]:\s*(<[^>]*>|\S+)")
ATTRIBUTE = re.compile(r"\b(?:href|src)\s*=\s*[\"']([^\"']+)[\"']")
ANCHOR = re.compile(r"\b(?:id|name)\s*=\s*[\"']([^\"']+)[\"']")


def prose_lines(text):
    """(line number, line) for lines outside fenced code blocks, with inline
    code removed."""
    fence = None
    for number, line in enumerate(text.splitlines(), 1):
        match = FENCE.match(line)
        if match:
            marker = match.group(1)
            if fence is None:
                fence = marker
            elif marker[0] == fence[0] and len(marker) >= len(fence):
                fence = None
            continue
        if fence is None:
            yield number, re.sub(r"(`+)(?:(?!\1).)+?\1", "", line)


def slug(heading):
    """GitHub's id for a heading: lower case, punctuation removed, spaces
    become hyphens."""
    heading = re.sub(r"<[^>]+>", "", heading)
    heading = re.sub(r"!?\[([^\]]*)\]\([^)]*\)", r"\1", heading)
    heading = heading.replace("`", "").replace("*", "").strip().lower()
    return "".join("-" if c == " " else c for c in heading if c.isalnum() or c in "-_ ")


_anchor_cache = {}


def anchors(path):
    if path not in _anchor_cache:
        found, seen = set(), {}
        text = course.read(path)
        fence = None
        for line in text.splitlines():
            match = FENCE.match(line)
            if match:
                marker = match.group(1)
                if fence is None:
                    fence = marker
                elif marker[0] == fence[0] and len(marker) >= len(fence):
                    fence = None
                continue
            if fence is not None:
                continue
            heading = HEADING.match(line)
            if heading:
                base = slug(heading.group(2))
                count = seen.get(base, 0)
                seen[base] = count + 1
                found.add(base if count == 0 else "%s-%d" % (base, count))
            found.update(anchor.lower() for anchor in ANCHOR.findall(line))
        _anchor_cache[path] = found
    return _anchor_cache[path]


_listings = {}


def spelled_as_on_disk(repo, lexical):
    """True when every part of `lexical` (a path below repo, not resolved)
    has the same upper and lower case as the name on disk. macOS and Windows
    open `Readme.md` for `README.md`; Linux and the web do not."""
    try:
        parts = lexical.relative_to(repo).parts
    except ValueError:
        return True
    current = repo
    for part in parts:
        names = _listings.get(current)
        if names is None:
            try:
                names = set(os.listdir(current))
            except OSError:
                names = set()
            _listings[current] = names
        if part not in names:
            return False
        current = current / part
    return True


def targets(text):
    """(line number, target) for every link target in a Markdown text."""
    definitions = {}
    lines = list(prose_lines(text))
    for number, line in lines:
        definition = REFERENCE_DEFINITION.match(line)
        if definition:
            definitions[definition.group(1).strip().lower()] = (number, definition.group(2))
    for number, line in lines:
        if REFERENCE_DEFINITION.match(line):
            continue
        for match in INLINE.finditer(line):
            yield number, match.group(1)
        for match in ATTRIBUTE.finditer(line):
            yield number, match.group(1)
        for match in REFERENCE_USE.finditer(line):
            label = (match.group(2) or match.group(1)).strip().lower()
            if label not in definitions:
                yield number, "[%s]" % label
    for number, target in definitions.values():
        yield number, target


def main():
    course.utf8_output()
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--repo", type=pathlib.Path, default=course.REPO)
    parser.add_argument("--external", action="store_true", help="also list the links to other sites")
    args = parser.parse_args()
    repo = args.repo.resolve()

    files = course.course_files(repo, ".md")
    broken, external, checked = [], {}, 0
    for path in files:
        relative = path.relative_to(repo).as_posix()
        for number, target in targets(course.read(path)):
            target = target.strip("<>").strip()
            if target.startswith("[") and target.endswith("]"):
                broken.append((relative, number, target, "no definition for this reference label"))
                continue
            if re.match(r"[a-zA-Z][a-zA-Z0-9+.-]*:", target) or target.startswith("//"):
                external.setdefault(target, (relative, number))
                continue
            if not target:
                continue
            checked += 1
            file_part, _, fragment = target.partition("#")
            file_part = urllib.parse.unquote(file_part.split("?", 1)[0])
            if not file_part:
                destination = path
            elif file_part.startswith("/"):
                destination = repo / file_part.lstrip("/")
            else:
                destination = path.parent / file_part
            lexical = pathlib.Path(os.path.normpath(str(destination)))
            try:
                destination = destination.resolve()
                inside = repo == destination or repo in destination.parents
            except OSError:
                inside = False
            if inside and not destination.exists() and relative.startswith("solutions/") and not file_part.startswith("/"):
                # solutions/ mirrors the course: a file there may link to a
                # neighbour that exists only next to the starter.
                mirrored = (repo / pathlib.Path(relative).relative_to("solutions")).parent / file_part
                if mirrored.exists():
                    destination = mirrored.resolve()
            if not inside:
                broken.append((relative, number, target, "points outside the repository"))
            elif not destination.exists():
                broken.append((relative, number, target, "no such file or directory"))
            elif destination.resolve() == lexical.resolve() and not spelled_as_on_disk(repo, lexical):
                broken.append((relative, number, target, "upper/lower case differs from the name on disk"))
            elif fragment and destination.suffix.lower() == ".md":
                if urllib.parse.unquote(fragment).lower() not in anchors(destination):
                    broken.append((relative, number, target, "no such heading in the target file"))

    for relative, number, target, why in broken:
        print("%s:%d: %s -> %s" % (relative, number, target, why))
    if args.external:
        print("\nLinks to other sites (not fetched):")
        for target in sorted(external):
            print("  %s  (%s:%d)" % ((target,) + external[target]))
    print(
        "%d Markdown files, %d relative links checked, %d broken, %d distinct external links"
        % (len(files), checked, len(broken), len(external))
    )
    return 1 if broken else 0


if __name__ == "__main__":
    sys.exit(main())

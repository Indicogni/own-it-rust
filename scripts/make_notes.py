#!/usr/bin/env python3
"""Generate exercises/chapterNN/NOTES.md, the learner's workbook.

The workbook is built from the lesson headers and from the "Mastery check"
section of the chapter's README.md. This is a maintainer tool: run it after
adding or renaming a lesson.

  python3 scripts/make_notes.py 04            # write chapter 4's NOTES.md if it is missing
  python3 scripts/make_notes.py 04 --force    # regenerate an untouched workbook
  python3 scripts/make_notes.py --all --force # the same for every chapter
  python3 scripts/make_notes.py --all --check # exit 1 if any workbook is out of date

Learners write their answers into NOTES.md, so the script protects it:

  - without --force an existing NOTES.md is never changed;
  - with --force it is regenerated only when it holds no answers (every
    answer slot is still empty);
  - a NOTES.md that holds answers is replaced only with --discard-my-notes;
  - whenever a file is replaced, the old one is first saved next to it as
    NOTES.md.bak (or NOTES.md.bak.2, ...). A backup is never overwritten.

Exit status: 0 on success, 1 when a file was refused or --check found a
difference, 2 for a wrong command line.
"""
import argparse
import pathlib
import re
import sys

ROOT = pathlib.Path(__file__).resolve().parent.parent / "exercises"

KEYS = ("PREDICT", "EXPLAIN", "IMPLEMENT", "COMPILER", "TRANSFER", "BEFORE CODING",
        "Use examples", "Run:", "Read first", "Then:", "Do this")
INTRO = ("Your workbook. Write answers in your own words, *before* checking the hints. "
         "`cargo progress` tells you when this chapter is due for a review and which part "
         "of these notes that review uses.")
DRILLS = "Drills whose verdict surprised me, and the rule I learned:"
SEPARATOR = r"\s*[—–·:-]\s*"  # em dash, en dash, middle dot, colon, hyphen


def read(path):
    return path.read_text(encoding="utf-8")


def write(path, text):
    with open(path, "w", encoding="utf-8", newline="\n") as handle:
        handle.write(text)


def header(path):
    lines = []
    for line in read(path).splitlines():
        if not line.startswith("//!"):
            break
        lines.append(line[3:].strip())
    return lines


def section(lines, key):
    out, on = [], False
    for line in lines:
        starts = any(line.startswith(k) for k in KEYS)
        if line.startswith(key):
            on = True
            text = re.sub(r"^%s[^:]*:\s*" % key, "", line)
            if text:
                out.append(text)
            continue
        if on and (not line or starts):
            break
        if on:
            out.append(line)
    return " ".join(out)


def generate(folder):
    """The NOTES.md text for one chapter directory."""
    number = int(folder.name[-2:])
    readme = read(folder / "README.md")
    first = readme.splitlines()[0].lstrip("#").strip()
    title = re.sub(r"^Chapter\s+\d+" + SEPARATOR, "", first)
    out = ["# Chapter %d notes — %s" % (number, title), "", INTRO, ""]
    order = {"l": 0, "r": 1, "c": 2}
    files = [f for f in folder.glob("*.rs") if re.match(r"[lrc]\d\d_", f.name)]
    # Study order: lessons by number with the capstone last among them (a
    # lesson added after the capstone still comes before it), then the
    # review, then the code review. The coach uses the same order.
    for path in sorted(files, key=lambda f: (order[f.name[0]], "capstone" in f.name, f.name)):
        lines = header(path)
        if not lines:
            continue
        name = re.sub(r"^(LESSON|REVIEW|CODE REVIEW)\s*\d*" + SEPARATOR, "", lines[0])
        out += ["## %s · %s" % (path.stem.split("_")[0], name.strip()), ""]
        if path.name.startswith("c"):
            out += ["**My review, written before running the tests:**", "", "- ", ""]
        else:
            out += ["**Predictions I got wrong, and why:**", "", "- ", ""]
        explain = section(lines, "EXPLAIN")
        if explain:
            out += ["**Explain:** %s" % explain, "", "> ", ""]
        out += ["**Edge case I added:**", "", "- ", ""]
    out += ["## Drills", "", DRILLS, "", "- ", ""]
    match = re.search(r"## Mastery check\n(.*?)(\n## |\Z)", readme, re.S)
    if match:
        out += ["## Mastery check (answer from memory)", ""]
        for item in re.findall(r"^(?:- \[ \]|\d+\.)\s*(.+)$", match.group(1), re.M):
            out += ["**%s**" % item.strip(), "", "> ", ""]
    return "\n".join(out).rstrip() + "\n"


def holds_answers(existing, generated):
    """True when a NOTES.md contains something a learner wrote.

    A workbook made by any version of this script consists of headings,
    bold prompts, two fixed sentences and empty answer slots (`- ` and
    `> `). Anything else is the learner's.
    """
    template = set(line.rstrip() for line in generated.splitlines())
    for line in existing.splitlines():
        line = line.rstrip()
        if not line or line in template or line in ("-", ">"):
            continue
        if line.startswith("#") or line.startswith("**") or line.startswith("Your workbook."):
            continue
        if line == DRILLS:
            continue
        return True
    return False


def backup(notes):
    """Copy NOTES.md to NOTES.md.bak, or the first free NOTES.md.bak.N."""
    count = 1
    while True:
        name = notes.name + ".bak" + ("" if count == 1 else ".%d" % count)
        target = notes.with_name(name)
        if not target.exists():
            target.write_bytes(notes.read_bytes())
            return target
        count += 1


def chapter(folder, args):
    """Handle one chapter. Returns "ok" or "problem"."""
    label = "exercises/%s/NOTES.md" % folder.name
    notes = folder / "NOTES.md"
    if not (folder / "README.md").is_file():
        print("skipped  %s: the chapter has no README.md yet" % label)
        return "problem" if args.check else "ok"
    generated = generate(folder)
    if not notes.exists():
        if args.check:
            print("missing  %s" % label)
            return "problem"
        write(notes, generated)
        print("written  %s (new)" % label)
        return "ok"
    existing = read(notes)
    if existing == generated:
        print("current  %s" % label)
        return "ok"
    if args.check:
        print("stale    %s: differs from what this script generates" % label)
        return "problem"
    if not args.force and not args.discard_my_notes:
        print("kept     %s: it exists and differs from the template (use --force to regenerate it)" % label)
        return "ok"
    if holds_answers(existing, generated) and not args.discard_my_notes:
        print("REFUSED  %s: it holds written answers. Nothing was changed. "
              "Pass --discard-my-notes to replace it (a backup is kept)." % label)
        return "problem"
    saved = backup(notes)
    write(notes, generated)
    print("written  %s (the previous file is in %s)" % (label, saved.name))
    return "ok"


def main():
    for stream in (sys.stdout, sys.stderr):
        try:
            stream.reconfigure(encoding="utf-8", errors="replace")
        except (AttributeError, ValueError):
            pass
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("chapters", nargs="*", metavar="NN", help="chapter numbers, for example 04 or chapter04")
    parser.add_argument("--all", action="store_true", help="every chapter (must be given explicitly)")
    parser.add_argument("--force", action="store_true", help="regenerate a NOTES.md that holds no answers")
    parser.add_argument("--discard-my-notes", action="store_true",
                        help="replace a NOTES.md even if it holds answers (a .bak copy is kept)")
    parser.add_argument("--check", action="store_true", help="change nothing; exit 1 if a NOTES.md is missing or out of date")
    args = parser.parse_args()

    existing = {int(d.name[-2:]): d for d in sorted(ROOT.glob("chapter[0-9][0-9]")) if d.is_dir()}
    if args.all and args.chapters:
        parser.error("give chapter numbers or --all, not both")
    if not args.all and not args.chapters:
        parser.error("name the chapters (for example 04), or pass --all for every chapter")
    if args.check and (args.force or args.discard_my_notes):
        parser.error("--check changes nothing; do not combine it with --force or --discard-my-notes")
    if args.all:
        folders = list(existing.values())
    else:
        folders = []
        for text in args.chapters:
            match = re.fullmatch(r"(?:chapter|ch)?(\d{1,2})", text.lower())
            if not match or int(match.group(1)) not in existing:
                parser.error("no chapter %r (chapters here: %s)" % (text, " ".join("%02d" % n for n in existing)))
            folders.append(existing[int(match.group(1))])

    results = [chapter(folder, args) for folder in folders]
    return 1 if "problem" in results else 0


if __name__ == "__main__":
    sys.exit(main())

"""Helpers shared by the check scripts in this directory.

Standard library only. Works with Python 3.9 or newer on Linux, macOS and
Windows. Every file is read and written as UTF-8 with "\n" line endings.
"""
import os
import pathlib
import re
import shutil
import subprocess
import sys

REPO = pathlib.Path(__file__).resolve().parent.parent
EXE = ".exe" if os.name == "nt" else ""

# Directories that never hold course content.
SKIP_DIRS = {".git", "target", ".progress", "node_modules", "__pycache__", ".idea", ".vscode"}


def utf8_output():
    """Print UTF-8 even where the console default is something else."""
    for stream in (sys.stdout, sys.stderr):
        try:
            stream.reconfigure(encoding="utf-8", errors="replace")
        except (AttributeError, ValueError):
            pass


def read(path):
    return pathlib.Path(path).read_text(encoding="utf-8")


def write(path, text):
    with open(path, "w", encoding="utf-8", newline="\n") as handle:
        handle.write(text)


def natural(name):
    """Sort key that puts ch04 before ch10 and m2 before m10."""
    return [int(part) if part.isdigit() else part for part in re.split(r"(\d+)", str(name))]


def walk(root, suffix=None):
    """Every file under root, skipping SKIP_DIRS, in a stable order."""
    root = pathlib.Path(root)
    for folder, dirs, files in os.walk(root):
        dirs[:] = sorted(d for d in dirs if d not in SKIP_DIRS)
        for name in sorted(files):
            if suffix is None or name.endswith(suffix):
                yield pathlib.Path(folder) / name


def course_files(root, suffix=None):
    """Files that belong to the course: tracked by Git, or new and not ignored.

    Falls back to walking the directory when root is not a Git checkout.
    """
    root = pathlib.Path(root)
    try:
        listed = subprocess.run(
            ["git", "-C", str(root), "ls-files", "-z", "--cached", "--others", "--exclude-standard"],
            stdout=subprocess.PIPE,
            stderr=subprocess.DEVNULL,
            check=True,
        ).stdout.decode("utf-8", errors="replace")
        inside = subprocess.run(
            ["git", "-C", str(root), "rev-parse", "--show-toplevel"],
            stdout=subprocess.PIPE,
            stderr=subprocess.DEVNULL,
            check=True,
        ).stdout.decode("utf-8", errors="replace").strip()
        if pathlib.Path(inside).resolve() != root.resolve():
            raise ValueError("root is inside another checkout")
        files = [root / name for name in listed.split("\0") if name]
        files = [f for f in files if f.is_file() and not (set(f.relative_to(root).parts) & SKIP_DIRS)]
    except (OSError, subprocess.CalledProcessError, ValueError):
        files = list(walk(root))
    if suffix is not None:
        files = [f for f in files if f.name.endswith(suffix)]
    return sorted(files, key=lambda f: natural(f.relative_to(root).as_posix()))


def chapters(root):
    """{4: Path('exercises/chapter04'), ...} for chapters that have a mod.rs."""
    found = {}
    for folder in sorted((pathlib.Path(root) / "exercises").glob("chapter[0-9][0-9]")):
        if (folder / "mod.rs").is_file():
            found[int(folder.name[-2:])] = folder
    return found


def lesson_files(chapter_dir):
    """Lesson, review and code-review files of a chapter, in study order:
    lessons by number with the capstone last among them, then the review,
    then the code review."""
    order = {"l": 0, "r": 1, "c": 2}
    files = [f for f in chapter_dir.glob("*.rs") if re.match(r"[lrc]\d\d_", f.name)]
    return sorted(files, key=lambda f: (order[f.name[0]], "capstone" in f.name, f.name))


def drill_sets(root):
    """{'ch04': [Path, ...], 'm1': [...]} for every directory under drills/."""
    found = {}
    base = pathlib.Path(root) / "drills"
    if base.is_dir():
        for folder in sorted(base.iterdir(), key=lambda p: natural(p.name)):
            files = sorted(folder.glob("*.rs")) if folder.is_dir() else []
            if files:
                found[folder.name] = files
    return found


def packages(root):
    """Standalone Cargo packages: the outermost Cargo.toml directories below
    the repository root, as POSIX paths relative to root. `solutions/` is not
    searched, because it mirrors the packages instead of adding new ones."""
    root = pathlib.Path(root)
    manifests = sorted(
        (m.parent.relative_to(root) for m in walk(root, "Cargo.toml") if m.parent != root),
        key=lambda p: len(p.parts),
    )
    found = []
    for folder in manifests:
        if folder.parts[0] == "solutions":
            continue
        if not any(folder.parts[: len(p.parts)] == p.parts for p in found):
            found.append(folder)
    return sorted((p.as_posix() for p in found), key=natural)


def keep_lines(text):
    """Lines of a drill that end in `// KEEP`, without indentation."""
    return [line.strip() for line in text.splitlines() if line.rstrip().endswith("// KEEP")]


def kill_tree(process):
    """Stop a child process and the processes it started."""
    try:
        if os.name == "nt":
            subprocess.run(
                ["taskkill", "/T", "/F", "/PID", str(process.pid)],
                stdout=subprocess.DEVNULL,
                stderr=subprocess.DEVNULL,
            )
        else:
            import signal

            os.killpg(os.getpgid(process.pid), signal.SIGKILL)
    except (OSError, ProcessLookupError):
        pass
    try:
        process.kill()
    except OSError:
        pass


def run(command, cwd=None, env=None, timeout=None):
    """Run a command and capture its output as text.

    Returns (exit code, stdout, stderr). On timeout the whole process tree is
    killed and the exit code is None.
    """
    options = {}
    if os.name != "nt":
        options["start_new_session"] = True
    process = subprocess.Popen(
        [str(part) for part in command],
        cwd=str(cwd) if cwd else None,
        env=env,
        stdin=subprocess.DEVNULL,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        **options,
    )
    try:
        out, err = process.communicate(timeout=timeout)
        code = process.returncode
    except subprocess.TimeoutExpired:
        kill_tree(process)
        try:
            out, err = process.communicate(timeout=10)
        except subprocess.TimeoutExpired:
            out, err = b"", b""
        code = None
    return code, out.decode("utf-8", errors="replace"), err.decode("utf-8", errors="replace")


class ToolError(Exception):
    """The compiler or the disk failed; this says nothing about the program."""


def drill_outcome(source, out_dir, cwd, timeout=60):
    """Compile and run one drill the way AUTHORING.md prescribes.

    Returns (outcome, output). The outcome is "ok", "panic", "timeout",
    "error" (a compile error without an error code) or the error codes,
    sorted and joined by commas, for example "E0382" or "E0277,E0599".
    """
    source = pathlib.Path(source)
    out_dir = pathlib.Path(out_dir)
    out_dir.mkdir(parents=True, exist_ok=True)
    exe = out_dir / (source.parent.name + "_" + source.stem + EXE)
    rustc = os.environ.get("RUSTC", "rustc")
    command = [rustc, "--edition", "2024", "-A", "warnings", "--color", "never"]
    command += ["-C", "debuginfo=0", "-o", exe, source]
    code, _, err = run(command, cwd=cwd, timeout=600)
    try:
        if code != 0:
            codes = sorted(set(re.findall(r"^error\[(E\d{4})\]", err, re.M)))
            if not codes and "-->" not in err:
                raise ToolError("rustc failed without a diagnostic for %s:\n%s" % (source, err))
            return (",".join(codes) or "error"), err
        code, out, err = run([exe], cwd=cwd, timeout=timeout)
        if code is None:
            return "timeout", out + err
        return ("ok" if code == 0 else "panic"), out + err
    finally:
        for leftover in (exe, exe.with_suffix(".pdb")):
            try:
                leftover.unlink()
            except OSError:
                pass


def same_outcome(expected, actual):
    """Compare an expected.tsv outcome with a real one. Error codes are
    compared as a set, so their order and separators do not matter."""
    codes = lambda text: set(re.findall(r"E\d{4}", text.upper()))
    if codes(expected) or codes(actual):
        return codes(expected) == codes(actual)
    return expected.strip().lower() == actual.strip().lower()


def manifest_has_dependencies(manifest):
    """True when a Cargo.toml declares any dependency (also dev, build,
    target-specific or workspace dependencies)."""
    text = read(manifest)
    try:
        import tomllib  # Python 3.11+
    except ImportError:
        tomllib = None
    if tomllib is not None:
        data = tomllib.loads(text)
        tables = [data, data.get("workspace", {})] + list(data.get("target", {}).values())
        keys = ("dependencies", "dev-dependencies", "build-dependencies")
        return any(table.get(key) for table in tables for key in keys)
    section = None
    for line in text.splitlines():
        line = line.split("#", 1)[0].strip()
        header = re.match(r"\[+\s*([^\]]+?)\s*\]+$", line)
        if header:
            section = header.group(1)
            if re.search(r"dependencies\.[\w-]+$", section):
                return True  # [dependencies.name]
        elif line and section and re.search(r"(^|\.)(dev-|build-)?dependencies$", section):
            return True
    return False


def course_metadata(manifest, key):
    """Read a list of strings from `[package.metadata.course]` or
    `[workspace.metadata.course]` in a Cargo.toml, for example

        [package.metadata.course]
        test-args = ["--no-default-features"]
        skip-os = ["windows"]

    Returns [] when the key is absent.
    """
    text = read(manifest)
    try:
        import tomllib
    except ImportError:
        tomllib = None
    if tomllib is not None:
        data = tomllib.loads(text)
        for top in ("package", "workspace"):
            value = data.get(top, {}).get("metadata", {}).get("course", {}).get(key)
            if value:
                return [str(item) for item in value]
        return []
    section = None
    for line in text.splitlines():
        stripped = line.strip()
        header = re.match(r"\[\s*([^\]]+?)\s*\]$", stripped)
        if header:
            section = header.group(1)
        elif section in ("package.metadata.course", "workspace.metadata.course"):
            match = re.match(r"%s\s*=\s*\[(.*)\]\s*(#.*)?$" % re.escape(key), stripped)
            if match:
                return re.findall(r'"([^"]*)"', match.group(1))
    return []


def os_name():
    if sys.platform.startswith("win"):
        return "windows"
    if sys.platform == "darwin":
        return "macos"
    return "linux"


# ----------------------------------------------------------- Rust source text


_CHAR = re.compile(r"'(?:\\(?:u\{[0-9a-fA-F_]+\}|x[0-9a-fA-F]{2}|.)|[^\\'\n])'")
_RAW = re.compile(r"(?:b|c)?r(#*)\"")


def blank_non_code(text):
    """Return text with comments and string/char literal contents replaced
    by spaces. The result has the same length, so positions in it are
    positions in the original; braces and semicolons that remain are code."""
    out = list(text)
    i, n = 0, len(text)

    def blank(start, end):
        for k in range(start, min(end, n)):
            if out[k] != "\n":
                out[k] = " "

    while i < n:
        c = text[i]
        if text.startswith("//", i):
            end = text.find("\n", i)
            end = n if end < 0 else end
            blank(i, end)
            i = end
        elif text.startswith("/*", i):
            depth, j = 1, i + 2
            while j < n and depth:
                if text.startswith("/*", j):
                    depth, j = depth + 1, j + 2
                elif text.startswith("*/", j):
                    depth, j = depth - 1, j + 2
                else:
                    j += 1
            blank(i, j)
            i = j
        elif c in "rbc" and (i == 0 or not (text[i - 1].isalnum() or text[i - 1] == "_")) and _RAW.match(text, i):
            match = _RAW.match(text, i)
            close = '"' + match.group(1)
            end = text.find(close, match.end())
            end = n if end < 0 else end + len(close)
            blank(match.end(), end - len(close))
            i = end
        elif c == '"':
            j = i + 1
            while j < n and text[j] != '"':
                j += 2 if text[j] == "\\" else 1
            blank(i + 1, j)
            i = j + 1
        elif c == "'":
            match = _CHAR.match(text, i)
            if match:
                blank(i + 1, match.end() - 1)
                i = match.end()
            else:
                i += 1  # a lifetime such as 'a
        else:
            i += 1
    return "".join(out)


def matching_brace(code, open_index):
    """Index just after the brace that closes code[open_index] == '{'.
    `code` must come from blank_non_code."""
    depth = 0
    for k in range(open_index, len(code)):
        if code[k] == "{":
            depth += 1
        elif code[k] == "}":
            depth -= 1
            if depth == 0:
                return k + 1
    return len(code)


def attributes_start(code, index):
    """Index where the attributes in front of code[index] begin, for example
    the `#[cfg(test)]` in front of `mod tests`. Returns `index` itself when
    there is none. `code` must come from blank_non_code."""
    start = index
    while True:
        end = start
        while end > 0 and code[end - 1].isspace():
            end -= 1
        if end == 0 or code[end - 1] != "]":
            return start
        depth, k = 0, end - 1
        while k >= 0:
            if code[k] == "]":
                depth += 1
            elif code[k] == "[":
                depth -= 1
                if depth == 0:
                    break
            k -= 1
        if k < 1 or code[k - 1] != "#":
            return start
        start = k - 1


# Test modules that the learner writes. They are part of the task, so a
# solution's version differs from the starter's.
LEARNER_TEST_MODULES = {"my_tests"}


def test_regions(text, whole_file=False):
    """The parts of a Rust file that hold its tests, as (start, end) pairs:
    the whole file for an integration test; otherwise every `mod tests { }`
    block, and every other inline module that is compiled only for tests
    (`#[cfg(test)] mod predictions`, `#[cfg(all(test, loom))] mod model`).
    A region starts at the attributes in front of the module, so that
    `#[cfg(test)]` cannot quietly become something else. Modules named in
    LEARNER_TEST_MODULES are left out."""
    if whole_file:
        return [(0, len(text))]
    code = blank_non_code(text)
    regions = []
    for match in re.finditer(r"\bmod\s+(\w+)\s*\{", code):
        if regions and match.start() < regions[-1][1]:
            continue
        name = match.group(1)
        start = attributes_start(code, match.start())
        attributes = code[start : match.start()]
        for_tests = (
            re.search(r"\bcfg\b", attributes)
            and re.search(r"\btest\b", attributes)
            and not re.search(r"\bnot\s*\(\s*test\b", attributes)
        )
        if name in LEARNER_TEST_MODULES or not (name == "tests" or for_tests):
            continue
        regions.append((start, matching_brace(code, match.end() - 1)))
    return regions


_GUESS = re.compile(r"=\s*(?:\w+\s*::\s*)*guess\s*\(\s*\)\s*;")


def _one_statement(code):
    """True when `code` has balanced brackets and no `;` outside them, so
    it can be the start of a single `let` statement. A `;` inside brackets,
    as in the type `[u8; 3]`, is fine."""
    depth = 0
    for c in code:
        if c in "([{":
            depth += 1
        elif c in ")]}":
            depth -= 1
            if depth < 0:
                return False
        elif c == ";" and depth == 0:
            return False
    return depth == 0


def prediction_heads(text):
    """For each `let NAME: TYPE = guess();` statement, the text from `let`
    up to (not including) the `=`, with whitespace collapsed."""
    code = blank_non_code(text)
    heads = []
    for match in _GUESS.finditer(code):
        window = max(0, match.start() - 4000)
        starts = [window + m.start() for m in re.finditer(r"\blet\b", code[window : match.start()])]
        for start in reversed(starts):
            if _one_statement(code[start : match.start()]):
                heads.append(" ".join(text[start : match.start()].split()))
                break
    return heads


def mask_predictions(text, heads):
    """Replace the initializer of each prediction statement by a marker, so
    a starter (`= guess();`) and a solution (`= 42; // because ...`) compare
    equal.

    Returns (masked text, heads that were not found).
    """
    code = blank_non_code(text)
    pieces, position, missing = [], 0, []
    for head in heads:
        pattern = r"\s*".join(re.escape(token) for token in re.findall(r"\w+|\S", head))
        match = re.compile(pattern + r"\s*=").search(code, position)
        if not match:
            missing.append(head)
            continue
        depth, end = 0, None
        for k in range(match.end(), len(code)):
            if code[k] in "([{":
                depth += 1
            elif code[k] in ")]}":
                depth -= 1
            elif code[k] == ";" and depth == 0:
                end = k + 1
                break
        if end is None:
            missing.append(head)
            continue
        # A comment after the answer, on the same line, explains the answer
        # and belongs to it.
        comment = re.compile(r"[ \t]*//[^\n]*").match(text, end)
        if comment:
            end = comment.end()
        pieces.append(text[position : match.start()])
        pieces.append(head + " = <PREDICTION>;")
        position = end
    pieces.append(text[position:])
    return "".join(pieces), missing


def copy_course(source, destination):
    """Copy the course without build output, Git data and solutions."""
    source = pathlib.Path(source)

    def ignore(folder, names):
        skipped = {name for name in names if name in SKIP_DIRS}
        if pathlib.Path(folder) == source:
            skipped.add("solutions")
        return skipped

    shutil.copytree(source, destination, ignore=ignore, symlinks=True)

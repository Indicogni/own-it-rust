//! The steps that ship with tasklane.
//!
//! | Step | Arguments | Effect |
//! | --- | --- | --- |
//! | `append` | `PATH TEXT...` | adds a line to a file, creating it if needed |
//! | `concat` | `OUT PATTERN` | joins all files that match the pattern, in path order |
//! | `copy` | `FROM TO` | copies a file |
//! | `count` | `PATH` | logs the number of lines and words |
//! | `echo` | `TEXT...` | logs the text |
//! | `fail` | `TEXT...` | always fails with the text as the message |
//! | `remove` | `PATTERN` | deletes all files that match the pattern |
//! | `slice` | `OUT IN START LEN` | writes `LEN` bytes of `IN`, from byte `START`, to `OUT` |
//! | `upper` | `PATH` | converts a file to upper case |
//! | `write` | `PATH TEXT...` | creates or replaces a file |
//!
//! `TEXT...` means all remaining arguments, joined with one space.

use tasklane_plugin::{Step, StepContext, StepError, Workspace};

use crate::glob;

/// Every built-in step, in alphabetical order.
pub fn all() -> Vec<Box<dyn Step>> {
    vec![
        Box::new(Append),
        Box::new(Concat),
        Box::new(Copy),
        Box::new(Count),
        Box::new(Echo),
        Box::new(Fail),
        Box::new(Remove),
        Box::new(Slice),
        Box::new(Upper),
        Box::new(Write),
    ]
}

fn missing(step: &dyn Step, path: &str) -> StepError {
    StepError::new(step.name(), format!("no such file `{path}`"))
}

/// `append PATH TEXT...`
pub struct Append;

impl Step for Append {
    fn name(&self) -> &str {
        "append"
    }
    fn usage(&self) -> &str {
        "append PATH TEXT..."
    }
    fn run(
        &self,
        args: &[String],
        ws: &mut Workspace,
        _: &StepContext<'_>,
    ) -> Result<(), StepError> {
        let [path, text @ ..] = args else {
            return Err(StepError::usage(self));
        };
        let line = text.join(" ");
        let content = match ws.read(path) {
            Some(old) if !old.is_empty() => format!("{old}\n{line}"),
            _ => line,
        };
        ws.write(path, content);
        Ok(())
    }
}

/// `concat OUT PATTERN`
pub struct Concat;

impl Step for Concat {
    fn name(&self) -> &str {
        "concat"
    }
    fn usage(&self) -> &str {
        "concat OUT PATTERN"
    }
    fn run(
        &self,
        args: &[String],
        ws: &mut Workspace,
        _: &StepContext<'_>,
    ) -> Result<(), StepError> {
        let [out, pattern] = args else {
            return Err(StepError::usage(self));
        };
        let parts: Vec<&str> = ws
            .paths()
            .filter(|path| glob::matches(pattern, path))
            .filter_map(|path| ws.read(path))
            .collect();
        if parts.is_empty() {
            return Err(StepError::new(
                self.name(),
                format!("no file matches `{pattern}`"),
            ));
        }
        let joined = parts.join("\n");
        ws.write(out, joined);
        Ok(())
    }
}

/// `copy FROM TO`
pub struct Copy;

impl Step for Copy {
    fn name(&self) -> &str {
        "copy"
    }
    fn usage(&self) -> &str {
        "copy FROM TO"
    }
    fn run(
        &self,
        args: &[String],
        ws: &mut Workspace,
        _: &StepContext<'_>,
    ) -> Result<(), StepError> {
        let [from, to] = args else {
            return Err(StepError::usage(self));
        };
        let content = ws
            .read(from)
            .ok_or_else(|| missing(self, from))?
            .to_string();
        ws.write(to, content);
        Ok(())
    }
}

/// `count PATH`
pub struct Count;

impl Step for Count {
    fn name(&self) -> &str {
        "count"
    }
    fn usage(&self) -> &str {
        "count PATH"
    }
    fn run(
        &self,
        args: &[String],
        ws: &mut Workspace,
        _: &StepContext<'_>,
    ) -> Result<(), StepError> {
        let [path] = args else {
            return Err(StepError::usage(self));
        };
        let text = ws.read(path).ok_or_else(|| missing(self, path))?;
        let lines = text.lines().count();
        let words = text.split_whitespace().count();
        ws.log(format!("{path}: {lines} lines, {words} words"));
        Ok(())
    }
}

/// `echo TEXT...`
pub struct Echo;

impl Step for Echo {
    fn name(&self) -> &str {
        "echo"
    }
    fn usage(&self) -> &str {
        "echo TEXT..."
    }
    fn run(
        &self,
        args: &[String],
        ws: &mut Workspace,
        _: &StepContext<'_>,
    ) -> Result<(), StepError> {
        ws.log(args.join(" "));
        Ok(())
    }
}

/// `fail TEXT...`
pub struct Fail;

impl Step for Fail {
    fn name(&self) -> &str {
        "fail"
    }
    fn usage(&self) -> &str {
        "fail TEXT..."
    }
    fn run(
        &self,
        args: &[String],
        _: &mut Workspace,
        _: &StepContext<'_>,
    ) -> Result<(), StepError> {
        Err(StepError::new(self.name(), args.join(" ")))
    }
}

/// `remove PATTERN`
pub struct Remove;

impl Step for Remove {
    fn name(&self) -> &str {
        "remove"
    }
    fn usage(&self) -> &str {
        "remove PATTERN"
    }
    fn run(
        &self,
        args: &[String],
        ws: &mut Workspace,
        _: &StepContext<'_>,
    ) -> Result<(), StepError> {
        let [pattern] = args else {
            return Err(StepError::usage(self));
        };
        let doomed: Vec<String> = ws
            .paths()
            .filter(|path| glob::matches(pattern, path))
            .map(str::to_string)
            .collect();
        for path in &doomed {
            ws.remove(path);
        }
        Ok(())
    }
}

/// `slice OUT IN START LEN`
pub struct Slice;

impl Step for Slice {
    fn name(&self) -> &str {
        "slice"
    }
    fn usage(&self) -> &str {
        "slice OUT IN START LEN"
    }
    fn run(
        &self,
        args: &[String],
        ws: &mut Workspace,
        _: &StepContext<'_>,
    ) -> Result<(), StepError> {
        let [out, input, start, len] = args else {
            return Err(StepError::usage(self));
        };
        let number = |text: &String| {
            text.parse::<usize>()
                .map_err(|_| StepError::new(self.name(), format!("`{text}` is not a number")))
        };
        let (start, len) = (number(start)?, number(len)?);
        if !ws.contains(input) {
            return Err(missing(self, input));
        }
        let part = ws
            .read_range(input, start, len)
            .ok_or_else(|| {
                StepError::new(
                    self.name(),
                    format!("bytes {start}+{len} are outside `{input}`"),
                )
            })?
            .to_string();
        ws.write(out, part);
        Ok(())
    }
}

/// `upper PATH`
pub struct Upper;

impl Step for Upper {
    fn name(&self) -> &str {
        "upper"
    }
    fn usage(&self) -> &str {
        "upper PATH"
    }
    fn run(
        &self,
        args: &[String],
        ws: &mut Workspace,
        _: &StepContext<'_>,
    ) -> Result<(), StepError> {
        let [path] = args else {
            return Err(StepError::usage(self));
        };
        let upper = ws
            .read(path)
            .ok_or_else(|| missing(self, path))?
            .to_uppercase();
        ws.write(path, upper);
        Ok(())
    }
}

/// `write PATH TEXT...`
pub struct Write;

impl Step for Write {
    fn name(&self) -> &str {
        "write"
    }
    fn usage(&self) -> &str {
        "write PATH TEXT..."
    }
    fn run(
        &self,
        args: &[String],
        ws: &mut Workspace,
        _: &StepContext<'_>,
    ) -> Result<(), StepError> {
        let [path, text @ ..] = args else {
            return Err(StepError::usage(self));
        };
        ws.write(path, text.join(" "));
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tasklane_plugin::TaskFlags;

    fn run(step: &dyn Step, args: &[&str], ws: &mut Workspace) -> Result<(), StepError> {
        let args: Vec<String> = args.iter().map(|arg| arg.to_string()).collect();
        step.run(&args, ws, &StepContext::new("test", TaskFlags::empty()))
    }

    #[test]
    fn names_are_sorted_and_match_the_usage_line() {
        let steps = all();
        let names: Vec<&str> = steps.iter().map(|step| step.name()).collect();
        let mut sorted = names.clone();
        sorted.sort_unstable();
        assert_eq!(names, sorted);
        for step in &steps {
            assert!(step.usage().starts_with(step.name()), "{}", step.usage());
        }
    }

    #[test]
    fn write_joins_the_text_arguments() {
        let mut ws = Workspace::new();
        run(&Write, &["a.txt", "one", "two  three"], &mut ws).unwrap();
        assert_eq!(ws.read("a.txt"), Some("one two  three"));
        run(&Write, &["a.txt"], &mut ws).unwrap();
        assert_eq!(ws.read("a.txt"), Some(""));
        assert_eq!(
            run(&Write, &[], &mut ws).unwrap_err().to_string(),
            "write: usage: write PATH TEXT..."
        );
    }

    #[test]
    fn append_adds_lines() {
        let mut ws = Workspace::new();
        run(&Append, &["log", "first"], &mut ws).unwrap();
        run(&Append, &["log", "second", "line"], &mut ws).unwrap();
        assert_eq!(ws.read("log"), Some("first\nsecond line"));
    }

    #[test]
    fn copy_and_upper_need_the_file() {
        let mut ws = Workspace::new();
        ws.write("a", "abc");
        run(&Copy, &["a", "b"], &mut ws).unwrap();
        run(&Upper, &["b"], &mut ws).unwrap();
        assert_eq!(ws.read("a"), Some("abc"));
        assert_eq!(ws.read("b"), Some("ABC"));
        assert_eq!(
            run(&Upper, &["c"], &mut ws).unwrap_err().to_string(),
            "upper: no such file `c`"
        );
        assert!(run(&Copy, &["c", "d"], &mut ws).is_err());
    }

    #[test]
    fn concat_joins_matches_in_path_order() {
        let mut ws = Workspace::new();
        ws.write("pages/b.txt", "B");
        ws.write("pages/a.txt", "A");
        ws.write("pages/skip.md", "no");
        run(&Concat, &["all.txt", "pages/*.txt"], &mut ws).unwrap();
        assert_eq!(ws.read("all.txt"), Some("A\nB"));
        assert_eq!(
            run(&Concat, &["all.txt", "none/*"], &mut ws)
                .unwrap_err()
                .message(),
            "no file matches `none/*`"
        );
    }

    #[test]
    fn remove_deletes_matches_and_accepts_none() {
        let mut ws = Workspace::new();
        ws.write("build/a.tmp", "");
        ws.write("build/keep.txt", "");
        run(&Remove, &["build/*.tmp"], &mut ws).unwrap();
        run(&Remove, &["build/*.tmp"], &mut ws).unwrap();
        assert_eq!(ws.paths().collect::<Vec<_>>(), ["build/keep.txt"]);
    }

    #[test]
    fn count_and_echo_write_to_the_log() {
        let mut ws = Workspace::new();
        ws.write("a", "one two\nthree\n");
        run(&Count, &["a"], &mut ws).unwrap();
        run(&Echo, &["hello", "there"], &mut ws).unwrap();
        assert_eq!(ws.log_lines(), ["a: 2 lines, 3 words", "hello there"]);
    }

    #[test]
    fn slice_copies_a_byte_range() {
        let mut ws = Workspace::new();
        ws.write("in", "0123456789");
        run(&Slice, &["out", "in", "2", "3"], &mut ws).unwrap();
        assert_eq!(ws.read("out"), Some("234"));
        assert_eq!(
            run(&Slice, &["out", "in", "x", "3"], &mut ws)
                .unwrap_err()
                .message(),
            "`x` is not a number"
        );
        assert_eq!(
            run(&Slice, &["out", "gone", "0", "1"], &mut ws)
                .unwrap_err()
                .message(),
            "no such file `gone`"
        );
    }

    #[test]
    fn fail_always_fails() {
        let mut ws = Workspace::new();
        assert_eq!(
            run(&Fail, &["not", "today"], &mut ws)
                .unwrap_err()
                .to_string(),
            "fail: not today"
        );
    }
}

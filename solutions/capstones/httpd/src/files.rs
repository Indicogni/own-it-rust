//! Static files: from a request path to an open file (SPEC section 8).
//!
//! The security rule lives in [`resolve`], and it is a rule about text: a
//! request path is turned into a list of plain names, and a name that could
//! mean anything but "an entry of this directory" is refused before the file
//! system is asked. The function never looks at the disk, so its tests can
//! list attacks without creating a single file.

use std::fs::{self, File};
use std::io;
use std::path::{Path, PathBuf};

use crate::response::{Body, Response, Status, TEXT_PLAIN};

/// Why a request path was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PathError {
    /// A broken percent escape, or bytes that are not UTF-8 after decoding: 400.
    Malformed,
    /// A segment the server never serves: 403.
    Forbidden,
}

/// Turns the path part of a request target into a file-system path below
/// `root`.
///
/// The path is split at `/` first and each segment is decoded afterwards.
/// The order matters: an encoded slash (`%2F`) must not become a separator.
/// Segments are checked from left to right and the first bad one decides.
pub fn resolve(root: &Path, path: &str) -> Result<PathBuf, PathError> {
    let mut resolved = root.to_path_buf();
    // Empty segments come from the leading slash, from `//` and from a
    // trailing slash. They name nothing.
    for segment in path.split('/').filter(|segment| !segment.is_empty()) {
        let name = percent_decode(segment)?;
        if is_forbidden(&name) {
            return Err(PathError::Forbidden);
        }
        resolved.push(name);
    }
    Ok(resolved)
}

/// True for a name that must not reach the file system.
///
/// - A leading dot covers `.`, `..` and hidden files such as `.git` or
///   `.env` in one rule. `..` is refused, never resolved: `/a/../b` is 403
///   although it would stay inside the root.
/// - `/` and `\` would make one segment into several path components
///   (`\` is a separator on Windows).
/// - `:` selects a drive or an alternate data stream on Windows.
/// - NUL ends the name early in C APIs.
fn is_forbidden(name: &str) -> bool {
    name.starts_with('.') || name.contains(['/', '\\', ':', '\0'])
}

/// Replaces each `%XX` by the byte it stands for. `+` is not special in a
/// path (it means a space only in form data).
fn percent_decode(segment: &str) -> Result<String, PathError> {
    let mut decoded = Vec::with_capacity(segment.len());
    let mut rest = segment.as_bytes();
    while let [first, tail @ ..] = rest {
        if *first == b'%' {
            let [high, low, tail @ ..] = tail else {
                return Err(PathError::Malformed);
            };
            decoded.push(hex_value(*high)? << 4 | hex_value(*low)?);
            rest = tail;
        } else {
            decoded.push(*first);
            rest = tail;
        }
    }
    String::from_utf8(decoded).map_err(|_| PathError::Malformed)
}

fn hex_value(digit: u8) -> Result<u8, PathError> {
    // `to_digit(16)` accepts both cases, as the SPEC requires.
    char::from(digit)
        .to_digit(16)
        .map(|value| value as u8)
        .ok_or(PathError::Malformed)
}

/// Builds the response for `GET` (or `HEAD`) of a resolved path: the file,
/// or `index.html` when the path names a directory.
pub fn respond(mut path: PathBuf) -> Response {
    match open(&mut path) {
        Ok((file, len)) => Response::new(Status::Ok, content_type(&path), Body::File { file, len }),
        Err(status) => Response::status_only(status),
    }
}

fn open(path: &mut PathBuf) -> Result<(File, u64), Status> {
    // `File::open` cannot be used to ask "is this a directory?": it opens
    // directories on Unix and fails with "access denied" on Windows.
    if fs::metadata(&*path).map_err(status_for)?.is_dir() {
        path.push("index.html");
    }
    let file = match File::open(&*path) {
        Ok(file) => file,
        // Windows reports "access denied" when a directory is opened as a
        // file. Here that can only be a directory called `index.html`,
        // which is "not a regular file" (SPEC 8.3), not a refusal.
        Err(error) if error.kind() == io::ErrorKind::PermissionDenied && path.is_dir() => {
            return Err(Status::NotFound);
        }
        Err(error) => return Err(status_for(error)),
    };
    // Ask the open file, not the path again: the answer then describes the
    // very file whose bytes will be sent.
    let metadata = file.metadata().map_err(|_| Status::InternalServerError)?;
    if metadata.is_file() {
        Ok((file, metadata.len()))
    } else {
        // A directory called `index.html`, a device, a pipe.
        Err(Status::NotFound)
    }
}

/// "The file system refused" is 403; everything else means that there is no
/// such file as far as a client is concerned. That includes names the
/// platform cannot even represent (too long, or with characters it forbids).
fn status_for(error: io::Error) -> Status {
    match error.kind() {
        io::ErrorKind::PermissionDenied => Status::Forbidden,
        _ => Status::NotFound,
    }
}

/// The media type for a file, from its extension (SPEC 8.4).
fn content_type(path: &Path) -> &'static str {
    let extension = path
        .extension()
        .and_then(|extension| extension.to_str())
        .map(str::to_ascii_lowercase);
    match extension.as_deref() {
        Some("html" | "htm") => "text/html; charset=utf-8",
        Some("css") => "text/css; charset=utf-8",
        Some("js") => "text/javascript; charset=utf-8",
        Some("txt") => TEXT_PLAIN,
        Some("json") => "application/json",
        Some("svg") => "image/svg+xml",
        Some("png") => "image/png",
        Some("jpg" | "jpeg") => "image/jpeg",
        Some("gif") => "image/gif",
        Some("ico") => "image/x-icon",
        Some("wasm") => "application/wasm",
        Some("pdf") => "application/pdf",
        _ => "application/octet-stream",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn resolved(path: &str) -> Result<PathBuf, PathError> {
        resolve(Path::new("root"), path)
    }

    fn below_root(parts: &[&str]) -> Result<PathBuf, PathError> {
        Ok(parts
            .iter()
            .fold(PathBuf::from("root"), |path, part| path.join(part)))
    }

    #[test]
    fn plain_paths_and_empty_segments() {
        assert_eq!(resolved("/"), below_root(&[]));
        assert_eq!(resolved("/a.txt"), below_root(&["a.txt"]));
        assert_eq!(resolved("/a/b/c.txt"), below_root(&["a", "b", "c.txt"]));
        assert_eq!(resolved("//a///b/"), below_root(&["a", "b"]));
    }

    #[test]
    fn escapes_are_decoded_per_segment() {
        assert_eq!(resolved("/my%20file.txt"), below_root(&["my file.txt"]));
        assert_eq!(resolved("/caf%C3%A9"), below_root(&["caf\u{e9}"]));
        assert_eq!(resolved("/caf%c3%a9"), below_root(&["caf\u{e9}"]));
        assert_eq!(resolved("/100%25"), below_root(&["100%"]));
        assert_eq!(resolved("/a+b"), below_root(&["a+b"]));
        assert_eq!(resolved("/%61/%62"), below_root(&["a", "b"]));
    }

    #[test]
    fn broken_escapes_and_non_utf8_are_malformed() {
        for path in [
            "/%", "/%4", "/a%", "/a%2", "/%zz", "/%G0", "/%0g", "/%ff", "/%C3", "/%C3%28",
        ] {
            assert_eq!(resolved(path), Err(PathError::Malformed), "{path}");
        }
    }

    #[test]
    fn names_that_could_leave_the_root_are_forbidden() {
        for path in [
            "/..",
            "/../secret",
            "/a/../../secret",
            "/a/../b",
            "/.",
            "/./a",
            "/...",
            "/.hidden",
            "/.git/config",
            "/a/.env",
            "/%2e%2e/secret",
            "/%2E%2E/secret",
            "/.%2e/secret",
            "/%2e/a",
            "/..%2fsecret",
            "/a%2fb",
            "/a%2Fb",
            "/..%5csecret",
            "/a%5Cb",
            "/a\\b",
            "/C:",
            "/c%3A",
            "/a.txt::$DATA",
            "/a%00",
            "/%00",
        ] {
            assert_eq!(resolved(path), Err(PathError::Forbidden), "{path}");
        }
    }

    #[test]
    fn the_first_bad_segment_decides() {
        assert_eq!(resolved("/../%zz"), Err(PathError::Forbidden));
        assert_eq!(resolved("/%zz/.."), Err(PathError::Malformed));
    }

    /// The only unit test that touches the disk: the cases of SPEC 8.3 that
    /// depend on what a path names.
    #[test]
    fn what_a_path_names_decides_the_status() {
        let root = std::env::temp_dir().join(format!("httpd-unit-{}", std::process::id()));
        let made = (|| {
            fs::create_dir_all(root.join("docs"))?;
            fs::create_dir_all(root.join("odd").join("index.html"))?;
            fs::create_dir_all(root.join("bare"))?;
            fs::write(root.join("docs").join("index.html"), "<h1>docs</h1>")?;
            fs::write(root.join("note.txt"), "note")
        })();
        let status = |path: &str| respond(resolve(&root, path).expect("a safe path")).status;
        let seen = [
            status("/note.txt"),
            status("/docs"),
            status("/bare"),
            // A directory called `index.html` is not a regular file.
            status("/odd"),
            status("/odd/index.html"),
            status("/note.txt/more"),
            status("/missing"),
        ];
        let _ = fs::remove_dir_all(&root);
        made.expect("the temporary directory can be written");
        assert_eq!(
            seen,
            [
                Status::Ok,
                Status::Ok,
                Status::NotFound,
                Status::NotFound,
                Status::NotFound,
                Status::NotFound,
                Status::NotFound,
            ]
        );
    }

    #[test]
    fn content_types() {
        for (name, expected) in [
            ("a.html", "text/html; charset=utf-8"),
            ("a.HTM", "text/html; charset=utf-8"),
            ("a.css", "text/css; charset=utf-8"),
            ("a.js", "text/javascript; charset=utf-8"),
            ("a.txt", "text/plain; charset=utf-8"),
            ("a.json", "application/json"),
            ("a.Svg", "image/svg+xml"),
            ("a.png", "image/png"),
            ("a.jpg", "image/jpeg"),
            ("a.jpeg", "image/jpeg"),
            ("a.gif", "image/gif"),
            ("a.ico", "image/x-icon"),
            ("a.wasm", "application/wasm"),
            ("a.pdf", "application/pdf"),
            ("a.tar.gz", "application/octet-stream"),
            ("noext", "application/octet-stream"),
            ("html", "application/octet-stream"),
        ] {
            assert_eq!(content_type(Path::new(name)), expected, "{name}");
        }
    }
}

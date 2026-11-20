//! Milestone M3: static files and path safety. SPEC section 8.
//!
//! Every [`Site`] has a file `secret.txt` next to the served directory,
//! outside it. Whatever a request says, its content must never come back.

mod common;

use common::{Conn, SECRET, Server, Site, get, head, pattern_bytes};

const FILES: &[(&str, &str)] = &[
    ("index.html", "<h1>home</h1>\n"),
    ("hello.txt", "hello\n"),
    ("empty.txt", ""),
    ("css/site.css", "body { margin: 0 }\n"),
    ("js/app.js", "console.log('hi');\n"),
    ("data/table.json", "{\"rows\": []}\n"),
    ("docs/index.html", "<h1>docs</h1>\n"),
    ("docs/guide/intro.txt", "introduction\n"),
    ("docs/guide/index.html", "<h1>guide</h1>\n"),
    ("bare/", ""),
    ("my file.txt", "a name with a space\n"),
    ("caf\u{e9}.txt", "a name that is not ASCII\n"),
    ("100%.txt", "a name with a percent sign\n"),
    ("plus+sign.txt", "a name with a plus sign\n"),
    ("a b/c d.txt", "spaces in two segments\n"),
    ("archive.tar.gz", "not really an archive\n"),
    ("README", "no extension\n"),
    ("page.HTML", "<p>upper-case extension</p>\n"),
    ("pic.Svg", "<svg xmlns=\"http://www.w3.org/2000/svg\"/>\n"),
    (".hidden", "a hidden file\n"),
    (".git/config", "[core]\n"),
    ("sub/.env", "PASSWORD=hunter2\n"),
    ("sub/visible.txt", "visible\n"),
];

#[track_caller]
fn server() -> Server {
    Server::start(FILES)
}

/// Sends a GET for each target on one connection and checks the status and
/// the body.
#[track_caller]
fn expect_bodies(server: &Server, cases: &[(&str, &str)]) {
    let mut conn = server.connect();
    for (target, body) in cases {
        conn.send(get(target));
        let response = conn.response();
        response.expect_status(200, &format!("GET {target}"));
        assert_eq!(
            response.text(),
            *body,
            "GET {target}: the body must be the content of the right file"
        );
    }
}

/// Sends a GET for each target on one connection and expects the standard
/// error response `status` every time, never the secret, and a connection
/// that stays usable (SPEC 7.2: the request was complete).
#[track_caller]
fn expect_errors(server: &Server, status: u16, targets: &[&str]) {
    let mut conn = server.connect();
    for target in targets {
        conn.send(get(target));
        let response = conn.response();
        assert!(
            !response.text().contains(SECRET),
            "GET {target} returned the file outside the root directory. This is the bug that SPEC 8.2 exists \
             to prevent."
        );
        assert!(
            !response.text().contains("hunter2") && !response.text().contains("[core]"),
            "GET {target} returned a hidden file (SPEC 8.2)"
        );
        response.expect_error(status, &format!("GET {target}"));
        response.expect_keep_alive(true, &format!("GET {target}"));
    }
    conn.send(get("/hello.txt"));
    conn.response().expect_status(
        200,
        "a normal GET after the refused ones, on the same connection",
    );
}

#[test]
fn content_type_follows_the_extension() {
    let site = Site::new(FILES);
    for name in [
        "a.htm", "a.jpg", "a.jpeg", "a.gif", "a.png", "a.ico", "a.wasm", "a.pdf", "a.bin", "a.TXT",
    ] {
        site.write(name, "x");
    }
    let server = Server::start_site(site, &[]);
    let cases = [
        ("/index.html", "text/html; charset=utf-8"),
        ("/a.htm", "text/html; charset=utf-8"),
        ("/page.HTML", "text/html; charset=utf-8"),
        ("/css/site.css", "text/css; charset=utf-8"),
        ("/js/app.js", "text/javascript; charset=utf-8"),
        ("/hello.txt", "text/plain; charset=utf-8"),
        ("/a.TXT", "text/plain; charset=utf-8"),
        ("/data/table.json", "application/json"),
        ("/pic.Svg", "image/svg+xml"),
        ("/a.png", "image/png"),
        ("/a.jpg", "image/jpeg"),
        ("/a.jpeg", "image/jpeg"),
        ("/a.gif", "image/gif"),
        ("/a.ico", "image/x-icon"),
        ("/a.wasm", "application/wasm"),
        ("/a.pdf", "application/pdf"),
        ("/a.bin", "application/octet-stream"),
        ("/archive.tar.gz", "application/octet-stream"),
        ("/README", "application/octet-stream"),
    ];
    let mut conn = server.connect();
    for (target, expected) in cases {
        conn.send(get(target));
        let response = conn.response();
        response.expect_status(200, &format!("GET {target}"));
        assert_eq!(
            response.header("content-type"),
            Some(expected),
            "Content-Type of {target} (SPEC 8.4; the extension is compared without regard to case)"
        );
    }
}

#[test]
fn a_directory_is_answered_with_its_index_html() {
    let server = server();
    expect_bodies(
        &server,
        &[
            ("/", "<h1>home</h1>\n"),
            ("/index.html", "<h1>home</h1>\n"),
            ("/docs/", "<h1>docs</h1>\n"),
            ("/docs", "<h1>docs</h1>\n"),
            ("/docs/guide", "<h1>guide</h1>\n"),
            ("/docs/guide/", "<h1>guide</h1>\n"),
        ],
    );
    let response = server.get("/docs");
    assert_eq!(
        response.header("content-type"),
        Some("text/html; charset=utf-8"),
        "the type comes from the file that is sent (index.html), not from the request path"
    );
    expect_errors(&server, 404, &["/bare", "/bare/", "/sub", "/sub/"]);
}

#[test]
fn nested_paths_empty_segments_and_trailing_slashes() {
    let server = server();
    expect_bodies(
        &server,
        &[
            ("/docs/guide/intro.txt", "introduction\n"),
            ("//hello.txt", "hello\n"),
            ("/docs//guide///intro.txt", "introduction\n"),
            ("/hello.txt/", "hello\n"),
            ("///", "<h1>home</h1>\n"),
        ],
    );
    expect_errors(
        &server,
        404,
        &[
            "/hello.txt/more",
            "/docs/guide/missing.txt",
            "/Docs-not-here/index.html",
            "/index.html/index.html",
        ],
    );
}

#[test]
fn the_query_string_is_not_part_of_the_path() {
    let server = server();
    expect_bodies(
        &server,
        &[
            ("/hello.txt?", "hello\n"),
            ("/hello.txt?a=1&b=2", "hello\n"),
            ("/hello.txt?/../../secret.txt", "hello\n"),
            ("/hello.txt?%zz", "hello\n"),
            ("/hello.txt??", "hello\n"),
            ("/?page=2", "<h1>home</h1>\n"),
            ("/docs?x", "<h1>docs</h1>\n"),
        ],
    );
    // An encoded question mark is part of the name, not a separator.
    expect_errors(&server, 404, &["/hello.txt%3Fa=1"]);
}

#[test]
fn percent_escapes_are_decoded() {
    let server = server();
    expect_bodies(
        &server,
        &[
            ("/my%20file.txt", "a name with a space\n"),
            ("/caf%C3%A9.txt", "a name that is not ASCII\n"),
            ("/caf%c3%a9.txt", "a name that is not ASCII\n"),
            ("/100%25.txt", "a name with a percent sign\n"),
            ("/a%20b/c%20d.txt", "spaces in two segments\n"),
            ("/%68%65%6C%6c%6F.txt", "hello\n"),
            ("/hello%2Etxt", "hello\n"),
            ("/plus+sign.txt", "a name with a plus sign\n"),
            ("/plus%2Bsign.txt", "a name with a plus sign\n"),
        ],
    );
    // `+` means a space only in form data, never in a path.
    expect_errors(&server, 404, &["/my+file.txt", "/my%2520file.txt"]);
}

#[test]
fn broken_escapes_and_invalid_utf8_are_400() {
    let server = server();
    expect_errors(
        &server,
        400,
        &[
            "/%",
            "/%4",
            "/hello%",
            "/hello%2",
            "/%zz",
            "/%G0",
            "/%0g",
            "/%%34",
            "/%FF",
            "/caf%C3.txt",
            "/caf%C3%28.txt",
            "/%C0%AF",
        ],
    );
}

#[test]
fn dot_segments_and_hidden_names_are_403() {
    let server = server();
    expect_errors(
        &server,
        403,
        &[
            "/..",
            "/../",
            "/../secret.txt",
            "/../../secret.txt",
            "/../../../../../../../../secret.txt",
            "/docs/../../secret.txt",
            "/docs/guide/../../../secret.txt",
            // Would stay inside the root, but `..` is never interpreted.
            "/docs/../hello.txt",
            "/.",
            "/./hello.txt",
            "/docs/./index.html",
            "/...",
            "/..hidden",
            "/.hidden",
            "/.git/config",
            "/.git",
            "/sub/.env",
            // The same, with the dots escaped.
            "/%2e%2e/secret.txt",
            "/%2E%2E/secret.txt",
            "/.%2e/secret.txt",
            "/%2e./secret.txt",
            "/docs/%2e%2e/%2e%2e/secret.txt",
            "/%2e/hello.txt",
            "/%2ehidden",
            "/sub/%2Eenv",
        ],
    );
}

#[test]
fn an_escape_cannot_smuggle_a_separator_or_a_drive() {
    let server = server();
    expect_errors(
        &server,
        403,
        &[
            // %2F is `/`. Decoding before splitting would turn these into
            // `../secret.txt` and `docs/guide/intro.txt`.
            "/..%2Fsecret.txt",
            "/..%2fsecret.txt",
            "/%2e%2e%2fsecret.txt",
            "/docs%2Fguide%2Fintro.txt",
            "/%2Fhello.txt",
            // `\` separates path components on Windows.
            "/..%5Csecret.txt",
            "/..%5csecret.txt",
            "/..\\secret.txt",
            "/docs\\index.html",
            "/docs%5Cindex.html",
            // `:` names a drive or an alternate data stream on Windows.
            "/C:",
            "/c%3A/secret.txt",
            "/hello.txt::$DATA",
            "/hello.txt%3A%3A$DATA",
            // NUL ends a file name early in C.
            "/hello.txt%00",
            "/%00",
            "/hello%00.txt",
            "/hello.txt%00.jpg",
        ],
    );
}

#[test]
fn the_first_bad_segment_decides_between_400_and_403() {
    let server = server();
    expect_errors(&server, 403, &["/../%zz", "/.x/%"]);
    expect_errors(&server, 400, &["/%zz/..", "/%/.x"]);
}

#[test]
fn files_arrive_byte_for_byte() {
    let site = Site::new(FILES);
    // Every byte value, in an order that is neither sorted nor text.
    let binary = pattern_bytes(4096);
    assert!(
        (0..=255u8).all(|value| binary.contains(&value)),
        "test setup: every byte value occurs"
    );
    site.write("blob.bin", &binary);
    let large = pattern_bytes(3 * 1024 * 1024 + 123);
    site.write("large.bin", &large);
    let server = Server::start_site(site, &[]);

    let mut conn = server.connect();
    conn.send(get("/blob.bin"));
    let response = conn.response();
    response.expect_status(200, "GET /blob.bin");
    assert!(
        response.body == binary,
        "a binary file must arrive unchanged (4096 bytes)"
    );

    conn.send(head("/large.bin"));
    let response = conn.head_response();
    response.expect_status(200, "HEAD /large.bin");
    assert_eq!(
        response.content_length(),
        large.len() as u64,
        "HEAD reports the size of the file"
    );

    conn.send(get("/large.bin"));
    let response = conn.response();
    response.expect_status(200, "GET /large.bin");
    assert_eq!(
        response.body.len(),
        large.len(),
        "the length of a 3 MiB file"
    );
    assert!(response.body == large, "a 3 MiB file must arrive unchanged");

    conn.send(get("/empty.txt"));
    let response = conn.response();
    response.expect_status(200, "GET /empty.txt");
    assert_eq!(
        response.content_length(),
        0,
        "an empty file is 200 with Content-Length: 0"
    );
    assert_eq!(
        response.header("content-type"),
        Some("text/plain; charset=utf-8")
    );

    conn.send(get("/hello.txt"));
    conn.response()
        .expect_status(200, "the connection is still in step after four responses");
}

#[test]
fn files_are_read_when_they_are_requested() {
    let server = server();
    let mut conn = server.connect();
    conn.send(get("/hello.txt"));
    assert_eq!(conn.response().text(), "hello\n");

    // The file changes, grows, and a new one appears while the server runs.
    server
        .site()
        .write("hello.txt", "a longer greeting than before\n");
    server
        .site()
        .write("fresh/new.txt", "created after start\n");
    conn.send(get("/hello.txt"));
    let response = conn.response();
    assert_eq!(
        response.text(),
        "a longer greeting than before\n",
        "the server must read the file for each request, not once at start-up (SPEC 8.3)"
    );
    conn.send(get("/fresh/new.txt"));
    let response = conn.response();
    response.expect_status(200, "a file created after the server started");
    assert_eq!(response.text(), "created after start\n");
}

/// SPEC 8.3 (4). The test exists on Unix only: there the standard library
/// can take the read permission away from a file.
#[cfg(unix)]
#[test]
fn a_file_the_system_refuses_to_open_is_403() {
    use std::os::unix::fs::PermissionsExt;

    let site = Site::new(FILES);
    site.write("locked.txt", "nobody may read this\n");
    let locked = site.root().join("locked.txt");
    std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o000))
        .expect("test setup: take every permission away from a file");
    let server = Server::start_site(site, &[]);
    // The administrator (`root`) may open every file, whatever its
    // permission bits say. Then the system refuses nothing, and there is
    // nothing to test.
    if std::fs::File::open(&locked).is_ok() {
        return;
    }

    // The file exists, so this is not 404: the system said "permission
    // denied", and the server passes that on as 403.
    expect_errors(&server, 403, &["/locked.txt", "/locked.txt?x=1"]);
    let mut conn = server.connect();
    conn.send(head("/locked.txt"));
    let response = conn.head_response();
    response.expect_status(403, "HEAD of a file that the system refuses to open");
    assert_eq!(
        response.content_length(),
        14,
        "the length of `403 Forbidden` and a line feed"
    );
}

#[test]
fn nothing_outside_the_root_is_ever_served() {
    let server = server();
    let attacks = [
        "/secret.txt",
        "/../secret.txt",
        "/%2e%2e/secret.txt",
        "/..%2fsecret.txt",
        "/..%5csecret.txt",
        "/..\\secret.txt",
        "/....//secret.txt",
        "/.../secret.txt",
        "/docs/../../secret.txt",
        "/docs/..%2f..%2fsecret.txt",
        "//../secret.txt",
        "/./../secret.txt",
        "/public/../../secret.txt",
        "/%c0%ae%c0%ae/secret.txt",
        "/..%c0%afsecret.txt",
        "/..%252fsecret.txt",
        "/%252e%252e/secret.txt",
        "/..;/secret.txt",
        "/..%00/secret.txt",
        "/?/../secret.txt",
        "/hello.txt/../../secret.txt",
    ];
    for target in attacks {
        // A new connection each time: some of these may close it.
        let mut conn = Conn::open(server.addr());
        conn.send(get(target));
        let response = conn.response();
        assert!(
            !response.text().contains(SECRET),
            "GET {target} returned the content of a file outside the root directory (SPEC 8.2)"
        );
        assert!(
            matches!(
                (target, response.status),
                ("/?/../secret.txt", 200) | (_, 400 | 403 | 404)
            ),
            "GET {target}: expected 400, 403 or 404, got {}",
            response.status
        );
    }
}

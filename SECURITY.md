# Security policy

Own It: Hands-On Mastery of Rust is a course, not a library or a service. Indicogni LLC maintains it. Only the latest commit on `main` is supported.

## What to report

- A problem in the course's own tooling that could harm a learner's machine or data: the coach (`cargo progress`, `cargo drill`, `cargo progress doctor`), the scripts in `scripts/`, the build scripts, the `.cargo/config.toml` aliases, or the dev container and Codespaces setup (`.devcontainer/`).
- A weakness in the GitHub Actions workflows (`.github/workflows/`).
- Course instructions that tell learners to do something unsafe, for example to expose a service to the network.
- A secret or personal data committed by mistake.
- A known-vulnerable dependency in a `Cargo.lock` that a lesson really uses.

Not in scope: bugs that a lesson contains on purpose; the model solutions, which are deliberately simple learning projects and not hardened servers (httpd, kvstore and others); and vulnerabilities in Rust, Cargo, QEMU, Docker or third-party crates themselves. Report those to their own projects.

## How to report

Please do not open a public issue. Use one of these:

1. GitHub private vulnerability reporting: on the repository's **Security** tab, click **Report a vulnerability**.
2. Email feedback@indicogni.com with "SECURITY" in the subject.

Include the file and line, or the command, what happens, and how to reproduce it. We aim to acknowledge a report within 5 business days and to fix it or answer within 30 days. With your permission, we credit reporters in the fix commit.

Course errors that are not security problems belong in a regular issue; see [CONTRIBUTING.md](CONTRIBUTING.md).

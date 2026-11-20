# Lesson 02 fixture: project setup files

Two files that travel with a repository and make every contributor's tools agree:

- `rust-toolchain.toml`: which toolchain rustup selects in this directory, and which
  components it installs with it.
- `editor/settings.json`: rust-analyzer settings. In a real project this content lives
  in `.vscode/settings.json`. Other editors (Neovim, Helix, Zed, Emacs) take the same
  keys in their own configuration file.

Do not run Cargo inside this directory unless you want rustup to install what
`rust-toolchain.toml` asks for. The lesson's tests only read the files.

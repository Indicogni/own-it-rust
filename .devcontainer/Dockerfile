# The image of the course's dev container. devcontainer.json builds it and
# then adds the desktop-lite feature (the browser desktop for the game).
#
# Pinned on purpose, so that every learner gets the same tools:
#   - the base image by its full version tag (move it forward on purpose;
#     the tags are listed at https://mcr.microsoft.com/v2/devcontainers/base/tags/list);
#   - rustup-init by version and SHA-256 checksum;
#   - Rust itself by rust-toolchain.toml (1.99.0). If you move the pin in
#     rust-toolchain.toml, change RUST_VERSION below too.
#     .github/workflows/devcontainer.yml fails when the two differ.
#
# The base image is multi-architecture (linux/amd64 and linux/arm64), so the
# same file builds on Intel and AMD machines and on Apple silicon.
FROM mcr.microsoft.com/devcontainers/base:2.2.7-trixie

# System packages. The base image already has git, curl and sudo.
#   build-essential  a C compiler and linker: Rust links with `cc`; chapter
#                    24 and M6 build C code; minifb compiles one C file.
#   python3          scripts/check_solutions.py and the other course scripts.
#   qemu-system-arm  contains qemu-system-aarch64 for the kernel capstone.
#   gdb              the M9 debugger lab (rust-gdb wraps gdb with Rust's
#                    pretty-printers; rustup installs rust-gdb).
#   pkg-config, libxkbcommon-dev, libwayland-dev, libwayland-cursor0
#                    what minifb (the game capstone's window crate) needs to
#                    build on Linux, the same list as the window-libs step in
#                    .github/workflows/ci.yml.
#   libx11-6, libxcursor1
#                    the X11 libraries minifb loads at run time to open its
#                    window on the desktop-lite display.
RUN apt-get update \
    && export DEBIAN_FRONTEND=noninteractive \
    && apt-get install -y --no-install-recommends \
        build-essential \
        python3 \
        qemu-system-arm \
        gdb \
        pkg-config \
        libxkbcommon-dev \
        libwayland-dev \
        libwayland-cursor0 \
        libx11-6 \
        libxcursor1 \
    && apt-get clean \
    && rm -rf /var/lib/apt/lists/*

# Rust is installed for the `vscode` user (the remoteUser in
# devcontainer.json), in its home directory, the same way a learner installs
# it on their own computer. The learner can then use `rustup` without sudo,
# for example to add the Miri nightly.
#
# rustup-init installs the toolchain rust-toolchain.toml names, with its
# components, and the bare-metal target aarch64-unknown-none of the kernel
# and nostd capstones, so that nothing downloads the first time the learner
# runs Cargo. The checksums are those published next to each rustup-init at
# static.rust-lang.org (rustup-init.sha256); update them with the version.
# PATH is set twice on purpose: ENV for commands the tools run without a
# shell, and rustup's lines in ~/.profile, ~/.bashrc and ~/.zshenv for login
# shells (Debian's /etc/profile resets PATH).
ARG RUSTUP_VERSION=1.29.3
ARG RUST_VERSION=1.99.0
USER vscode
ENV PATH=/home/vscode/.cargo/bin:${PATH}
RUN set -eu; \
    arch="$(dpkg --print-architecture)"; \
    case "$arch" in \
        amd64) triple=x86_64-unknown-linux-gnu; \
               sha=b1c8cb14e30889e7e2999e6b69e62d9e9b9ffc0f4df8180564d306d9680e6c55 ;; \
        arm64) triple=aarch64-unknown-linux-gnu; \
               sha=9f608c142e0e38a78957d0661ba1dfaf0b6fb37184cfaa4b9a0aa57af6184282 ;; \
        *) echo "unsupported architecture: $arch" >&2; exit 1 ;; \
    esac; \
    cd /tmp; \
    curl --proto '=https' --tlsv1.2 -sSf -o rustup-init \
        "https://static.rust-lang.org/rustup/archive/${RUSTUP_VERSION}/${triple}/rustup-init"; \
    echo "${sha}  rustup-init" | sha256sum -c -; \
    chmod +x rustup-init; \
    ./rustup-init -y --profile minimal \
        --default-toolchain "${RUST_VERSION}" \
        --component rustfmt,clippy,rust-src \
        --target aarch64-unknown-none; \
    rm rustup-init; \
    rustc --version; \
    cargo --version

# Back to root: the dev container tooling and features expect it here.
USER root

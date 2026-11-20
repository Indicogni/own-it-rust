# syntax=docker/dockerfile:1
#
# Lesson 08 lab: a multi-stage image for the capstone's `beacon-server`.
# Build from the module directory, which is the workspace root:
#
#   docker build -f lab/Dockerfile -t beacon-server:lab .
#
# The course's automatic checks do not build this file; building it is step 1 of
# the lab (lab/README.md). Both base images are chosen by build arguments, so
# that the build stage and the runtime stage always come from the same Debian
# release. A binary linked against a newer C library does not start on an older
# one.
ARG RUST_VERSION=1.99.0
ARG DEBIAN_RELEASE=trixie

# ---- Stage 1: build. Compilers, sources and caches stay in this stage. ----
FROM rust:${RUST_VERSION}-slim-${DEBIAN_RELEASE} AS build
WORKDIR /src
# rust-toolchain.toml is not copied on purpose: the image tag already pins the
# compiler, and the file would make rustup download extra components.
COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY migrations ./migrations
COPY wire ./wire
COPY capstone ./capstone
# The two cache mounts keep downloaded crates and compiled dependencies between
# builds without putting them into an image layer. The binary must be copied
# out of the cache mount inside the same RUN step.
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/src/target \
    cargo build --locked --release -p beacon --bin beacon-server \
    && cp target/release/beacon-server /beacon-server

# ---- Stage 2: runtime. Only the binary and what it needs to run. ----
FROM debian:${DEBIAN_RELEASE}-slim AS runtime
# A fixed numeric user id: no root inside the container, and file ownership on
# mounted volumes is predictable.
RUN useradd --system --uid 10001 --no-create-home --shell /usr/sbin/nologin beacon \
    && mkdir /data \
    && chown 10001:10001 /data
COPY --from=build /beacon-server /usr/local/bin/beacon-server
USER 10001:10001
# Inside a container, "127.0.0.1" is the container itself. A server that should
# be reachable through a published port must listen on 0.0.0.0.
ENV BEACON_INGEST_ADDR=0.0.0.0:7400 \
    BEACON_HTTP_ADDR=0.0.0.0:7401 \
    BEACON_DB=sqlite:///data/beacon.db
# Secrets (BEACON_ADMIN_USER, BEACON_ADMIN_PASSWORD, BEACON_TOKEN_KEY) are given
# when the container starts. They are never written into the image.
VOLUME /data
EXPOSE 7400 7401
STOPSIGNAL SIGTERM
# The exec form (a JSON array) starts the server as process 1 without a shell
# in between, so the server itself receives SIGTERM.
ENTRYPOINT ["/usr/local/bin/beacon-server"]

# Lesson 08 lab · The service in a container

[Module guide](../README.md) · [Concepts](../CONCEPTS.md#lesson-08--observability-and-operations) · [Lesson 08](../src/l08_operations.rs)

This lab is done by hand. No test in this module needs Docker, and the course's automatic checks do not build the image. You need a working `docker` (or `podman`, with the same commands) and a finished capstone, because the image contains `beacon-server`. Run this lab on your host (your own computer), not inside the course's dev container or a codespace: the dev container does not include Docker. If you did the module in the dev container, push your work and pull it on your computer first. If your capstone is not finished, copy the reference solution over a scratch copy of the module first (see "Where the solutions are" in the module guide).

Estimated time: 60 to 90 minutes.

## What you will have checked afterwards

A container image is a second build of your program, for a different machine. Things that work on your laptop can fail there for reasons that have nothing to do with Rust. The checklist below covers the failures that happen most often.

## Step 1 · Build

From the module directory `mastery/m11_services`:

```sh
docker build -f lab/Dockerfile -t beacon-server:lab .
```

Read [`Dockerfile`](Dockerfile) while it builds. Write down:

1. Which files end up in the final image, and which stay in the build stage?
2. Why are there two `FROM` lines with the same `DEBIAN_RELEASE`?
3. What would change if `ENTRYPOINT` were written as `ENTRYPOINT beacon-server` (shell form)?

If a base image tag is not available to you, choose another one with `--build-arg RUST_VERSION=...` or `--build-arg DEBIAN_RELEASE=...`. Keep both stages on the same Debian release.

## Step 2 · Run

```sh
cat > /tmp/beacon.env <<'ENV'
BEACON_ADMIN_USER=admin
BEACON_ADMIN_PASSWORD=change-me-for-the-lab
BEACON_TOKEN_KEY=0123456789abcdef0123456789abcdef-lab-only
ENV

docker volume create beacon-data
docker run -d --name beacon --env-file /tmp/beacon.env \
  -p 127.0.0.1:7400:7400 -p 127.0.0.1:7401:7401 \
  -v beacon-data:/data beacon-server:lab
docker logs beacon
curl -i http://127.0.0.1:7401/healthz
```

Publishing on `127.0.0.1:` keeps the lab service off your network.

## Step 3 · The checklist

Work through every line. For each, write what you ran and what you saw.

| # | Check | How |
| --- | --- | --- |
| 1 | The image is small and holds no compiler. | `docker images beacon-server:lab`; `docker run --rm --entrypoint sh beacon-server:lab -c 'command -v cargo rustc gcc'` prints nothing. |
| 2 | The process does not run as root. | `docker exec beacon id` shows uid 10001. |
| 3 | The server listens on all interfaces inside the container. | `docker logs beacon` shows `ingest=0.0.0.0:7400`. Change `BEACON_HTTP_ADDR` to `127.0.0.1:7401` in a second container and see `curl` fail. |
| 4 | The binary finds its shared libraries. | The container starts at all. To see what it needs: `docker run --rm --entrypoint sh beacon-server:lab -c 'ldd /usr/local/bin/beacon-server'`. |
| 5 | Stopping is graceful. | `time docker stop beacon` returns in well under 10 seconds, and `docker inspect -f '{{.State.ExitCode}}' beacon` is `0`, not `137`. Code 137 means the process ignored SIGTERM and was killed. |
| 6 | Data survives the container. | Send samples with your agent, `docker rm -f beacon`, start a new container on the same volume, query the API. |
| 7 | No secret is stored in the image. | `docker history --no-trunc beacon-server:lab` and `docker inspect beacon-server:lab` show no password and no key. |
| 8 | The root file system can be read-only. | Add `--read-only` to `docker run`. The service still works, because it writes only below `/data`. |
| 9 | Logs go to standard output and standard error. | `docker logs beacon` shows them. Nothing is written to a log file inside the container. |
| 10 | A memory limit leaves room for password hashing. | Run with `--memory 64m` and log in. Argon2 with the default parameters needs about 19 MiB per concurrent login. What happens with `--memory 16m`? |
| 11 | The build context is small. | The first lines of `docker build` show how many bytes were sent; `.dockerignore` keeps `target/` out. |
| 12 | Rebuilding after a source change is fast. | Change one line in `capstone/src`, build again, and compare the time. The cache mounts keep the compiled dependencies. |

## Step 4 · Clean up

```sh
docker rm -f beacon
docker volume rm beacon-data
docker rmi beacon-server:lab
rm /tmp/beacon.env
```

## Questions for your notes

1. Process 1 in a container gets no default signal handlers from the kernel. What does that mean for a server that does not handle SIGTERM itself?
2. The image has no `HEALTHCHECK`. Who checks `/healthz` and `/readyz` in a real deployment, and from where?
3. This image uses a Debian base with glibc. What would you have to verify before switching the runtime stage to a "distroless" or `scratch` image? Think about the C library, the bundled SQLite, the user database, and time zone and certificate files.

Model answers are in [HINTS.md](../HINTS.md#lesson-08-lab).

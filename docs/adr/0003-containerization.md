# ADR 0003 — Containerization: multi-stage Docker image

- Status: Accepted
- Date: 2026-06-27

## Context

The app needs a reproducible, self-hostable deployment artifact. The build is
non-trivial: a Dioxus fullstack app compiles a wasm client and a native server
binary via the Dioxus CLI (`dx bundle`), and the styling pipeline shells out to
the Tailwind v4 CLI (Node). None of that toolchain should leak into the thing
that actually runs in production.

## Decision

Ship a **multi-stage Dockerfile** plus a `compose.yaml` for the common
single-host run.

- **Builder** (`rust:1.96-bookworm`): adds the `wasm32-unknown-unknown` target,
  the standalone Tailwind binary, and `dioxus-cli@0.7.9` (fetched prebuilt via
  cargo-binstall to avoid a multi-minute source build). It generates
  `assets/tailwind.css` explicitly before `dx bundle`, because the `asset!`
  macro needs the file at compile time — the same ordering mise's lint/test
  tasks rely on. No Node: dx is self-contained, and the standalone Tailwind CLI
  bundles the `tailwindcss` library (so `@import "tailwindcss"` resolves with no
  `node_modules`, which `npx @tailwindcss/cli` cannot do).
- **Runtime** (`debian:bookworm-slim`): carries only the bundle
  (`target/dx/invites/release/web/`) and `ca-certificates`. Runs as a non-root
  user (`appuser`, uid 10001).
- The bundle's `server` binary and its `public/` directory are copied together
  and kept as siblings: the server resolves static assets as
  `<dir-of-executable>/public` (overridable with `DIOXUS_ASSET_ROOT`).
- The server binds `IP`/`PORT` (read by `dioxus::serve`); the image defaults
  these to `0.0.0.0:8080` so it is reachable from outside the container.
- The container `HEALTHCHECK` runs `server healthcheck`: the same binary probes
  its own `/healthz` route over HTTP and exits 0/1. Reusing the binary (which
  already has reqwest) keeps the runtime image free of curl/wget.
- SQLite lives on a `/data` volume with `DATABASE_URL` defaulted to
  `sqlite:///data/invites.db?mode=rwc`, so the database survives container
  replacement. `/data` is created owned by `appuser` in the image, so a fresh
  named volume inherits that ownership and writes work without root.

## Alternatives considered

- **Single-stage image** — rejected. Would bake the full Rust + Node + CLI
  toolchain (gigabytes) into the runtime image for no benefit.
- **`cargo install dioxus-cli` from source** — rejected as the default. Adds
  several minutes to every cold build; binstall pulls the pinned prebuilt
  binary. (Source install remains the fallback if a binstall artifact is ever
  unavailable.)
- **Distroless / `scratch` runtime** — deferred. The reqwest/rustls path uses
  bundled webpki roots, but `ca-certificates` on slim keeps TLS robust and the
  image is still small; revisit if image size becomes a constraint.

## Consequences

- `docker build` (or `docker compose up --build`) is the one-command path from a
  clean checkout to a running server.
- The runtime image contains no compiler or Node — smaller surface, smaller
  image.
- Required secrets (`APP_BASE_URL`, OIDC client id/secret, service token) are
  supplied at run time via `--env-file`/compose `env_file`, never baked into a
  layer (`.dockerignore` excludes `.env` and `*.db`).

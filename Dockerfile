# syntax=docker/dockerfile:1

# ── Builder ────────────────────────────────────────────────────────────────
# Compiles the wasm client + server bundle with the Dioxus CLI. dx is
# self-contained (no Node needed); the only external tool is the standalone
# Tailwind binary, which generates the stylesheet the `asset!` macro embeds.
#
# trixie (glibc 2.41), not bookworm (glibc 2.36): the prebuilt dioxus-cli is
# linked against glibc 2.39+, so it won't run on bookworm. The runtime stage
# matches (trixie) so the server binary built here runs there.
FROM rust:1.96-trixie AS builder

RUN apt-get update \
    && apt-get install -y --no-install-recommends curl ca-certificates \
    && rm -rf /var/lib/apt/lists/*

RUN rustup target add wasm32-unknown-unknown

# Prebuilt dioxus-cli via cargo-binstall — avoids a multi-minute source build.
# Pinned to the version this project is developed against (mise.toml).
RUN curl -L --proto '=https' --tlsv1.2 -sSf \
        https://raw.githubusercontent.com/cargo-bins/cargo-binstall/main/install-from-binstall-release.sh | bash \
    && cargo binstall -y dioxus-cli@0.7.9

# Standalone Tailwind CLI (per ADR 0002). It bundles the `tailwindcss` library,
# so it resolves the `@import "tailwindcss"` in tailwind.css with no node_modules
# — unlike `npx @tailwindcss/cli`, which only fetches the CLI and then fails to
# resolve that import. The linux-x64 build is glibc-linked (matches the base).
RUN curl -fsSL -o /usr/local/bin/tailwindcss \
        https://github.com/tailwindlabs/tailwindcss/releases/download/v4.3.1/tailwindcss-linux-x64 \
    && chmod +x /usr/local/bin/tailwindcss

WORKDIR /src
COPY . .

# Generate assets/tailwind.css before compiling: the `asset!` macro needs it at
# compile time (same step mise's lint/test tasks run).
RUN tailwindcss -i ./tailwind.css -o ./assets/tailwind.css --minify

# Produces target/dx/invites/release/web/{server, public/}.
RUN dx bundle --platform web --release

# ── Runtime ────────────────────────────────────────────────────────────────
# trixie to match the builder's glibc (the server binary is glibc-linked).
FROM debian:trixie-slim AS runtime

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/* \
    && useradd -r -u 10001 appuser

WORKDIR /app

# The server resolves static assets as `<dir of executable>/public`, so the
# binary and the public/ dir must stay siblings (they are in the bundle).
COPY --from=builder /src/target/dx/invites/release/web/ ./

# SQLite lives on a volume so the database survives container replacement. A
# fresh named volume inherits this directory's ownership (appuser), so writes
# work without running as root. Bind mounts must be writable by uid 10001.
RUN install -d -o appuser -g appuser /data

USER appuser

ENV IP=0.0.0.0 \
    PORT=8080 \
    DATABASE_URL="sqlite:///data/invites.db?mode=rwc"

EXPOSE 8080
VOLUME ["/data"]

# The binary probes its own /healthz (reads PORT from the env) and exits 0/1,
# so no extra tooling is needed in the runtime image.
HEALTHCHECK --interval=30s --timeout=3s --start-period=10s --retries=3 \
    CMD ["./server", "healthcheck"]

# Remaining required config (APP_BASE_URL, OIDC_CLIENT_ID, OIDC_CLIENT_SECRET,
# KANIDM_SERVICE_TOKEN) must be supplied at run time — see .env.example.
CMD ["./server"]

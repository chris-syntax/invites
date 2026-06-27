# syntax=docker/dockerfile:1

# ── Builder ────────────────────────────────────────────────────────────────
# Compiles the wasm client + server bundle with the Dioxus CLI. Needs the wasm
# target (client) and Node (Tailwind v4 CLI, which `dx bundle` shells out to).
FROM rust:1.96-bookworm AS builder

# Node 24 for the Tailwind CLI; curl/ca-certificates for the binstall fetch.
RUN apt-get update \
    && apt-get install -y --no-install-recommends curl ca-certificates \
    && curl -fsSL https://deb.nodesource.com/setup_24.x | bash - \
    && apt-get install -y --no-install-recommends nodejs \
    && rm -rf /var/lib/apt/lists/*

RUN rustup target add wasm32-unknown-unknown

# Prebuilt dioxus-cli via cargo-binstall — avoids a multi-minute source build.
# Pinned to the version this project is developed against (mise.toml).
RUN curl -L --proto '=https' --tlsv1.2 -sSf \
        https://raw.githubusercontent.com/cargo-bins/cargo-binstall/main/install-from-binstall-release.sh | bash \
    && cargo binstall -y dioxus-cli@0.7.9

WORKDIR /src
COPY . .

# Generate assets/tailwind.css before compiling: the `asset!` macro needs it at
# compile time (same step mise's lint/test tasks run).
RUN npx --yes @tailwindcss/cli -i ./tailwind.css -o ./assets/tailwind.css --minify

# Produces target/dx/invites/release/web/{server, public/}.
RUN dx bundle --platform web --release

# ── Runtime ────────────────────────────────────────────────────────────────
FROM debian:bookworm-slim AS runtime

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

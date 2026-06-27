//! Server-only backend: configuration, persistence, the kanidm client, session
//! handling, and the OIDC login routes. Compiled only with the `server` feature.

pub mod config;
pub mod db;
pub mod kanidm;
pub mod migration;
pub mod models;
pub mod oidc;
pub mod session;

use dioxus::prelude::*;

/// Current unix time in seconds.
pub fn now() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// A 256-bit URL-safe random token (invitation links and session ids).
pub fn gen_token() -> String {
    use base64::Engine as _;
    use rand::RngCore;
    let mut bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut bytes);
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
}

/// Probe the locally-running server's `/healthz` and exit 0 on success, 1
/// otherwise. Binds to the same `PORT` the server reads (default 8080).
fn health_check() -> ! {
    let port = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse::<u16>().ok())
        .unwrap_or(8080);
    let url = format!("http://127.0.0.1:{port}/healthz");
    let healthy = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("build healthcheck runtime")
        .block_on(async move {
            matches!(reqwest::get(&url).await, Ok(r) if r.status().is_success())
        });
    std::process::exit(if healthy { 0 } else { 1 });
}

/// Build the axum router (Dioxus app + custom OIDC routes) and run the server.
pub fn serve(app: fn() -> Element) {
    // `server healthcheck` probes the running server over HTTP and exits — used
    // as the container HEALTHCHECK so no extra tooling (curl) is needed in the
    // image. Handled before config::load so the probe doesn't need full config.
    if std::env::args().nth(1).as_deref() == Some("healthcheck") {
        health_check();
    }

    // Load .env and validate config before serving, so a misconfiguration fails
    // fast at startup rather than poisoning the config lock mid-request.
    config::load();
    dioxus::serve(move || async move {
        use dioxus::server::axum::routing::get;
        let router = dioxus::server::router(app)
            .route("/healthz", get(|| async { "ok" }))
            .route("/login", get(oidc::login))
            .route("/callback", get(oidc::callback))
            .route("/logout", get(oidc::logout));
        Ok(router)
    });
}

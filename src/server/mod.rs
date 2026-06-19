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

/// Build the axum router (Dioxus app + custom OIDC routes) and run the server.
pub fn serve(app: fn() -> Element) {
    dioxus::serve(move || async move {
        use dioxus::server::axum::routing::get;
        let router = dioxus::server::router(app)
            .route("/login", get(oidc::login))
            .route("/callback", get(oidc::callback))
            .route("/logout", get(oidc::logout));
        Ok(router)
    });
}

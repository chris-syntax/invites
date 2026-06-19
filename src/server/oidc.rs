//! OIDC authorization-code login against kanidm, implemented as plain axum
//! routes (redirects + cookies, not reactive UI).

use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};

use dioxus::fullstack::HeaderMap;
use dioxus::server::axum::{
    extract::Query,
    http::{header, StatusCode},
    response::{IntoResponse, Redirect, Response},
};
use openidconnect::core::{
    CoreAuthenticationFlow, CoreClient, CoreIdToken, CoreProviderMetadata,
};
use openidconnect::{
    reqwest, AuthorizationCode, ClientId, ClientSecret, CsrfToken, IssuerUrl, Nonce,
    PkceCodeChallenge, PkceCodeVerifier, RedirectUrl, Scope, TokenResponse,
};
use serde::Deserialize;
use tokio::sync::OnceCell;

use crate::server::config::CONFIG;
use crate::server::session;
use crate::shared::{CurrentUser, Role};

/// Per-login transient state, keyed by the OAuth2 `state` parameter.
struct LoginState {
    pkce: String,
    nonce: String,
}

static LOGIN_STATES: LazyLock<Mutex<HashMap<String, LoginState>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// HTTP client for OIDC calls. Must not follow redirects (SSRF hardening).
static HTTP: LazyLock<reqwest::Client> = LazyLock::new(|| {
    reqwest::ClientBuilder::new()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .expect("failed to build OIDC http client")
});

/// Discovered provider metadata, fetched once.
static META: OnceCell<CoreProviderMetadata> = OnceCell::const_new();

async fn provider_metadata() -> anyhow::Result<&'static CoreProviderMetadata> {
    META.get_or_try_init(|| async {
        let issuer = IssuerUrl::new(CONFIG.kanidm_url.clone())?;
        let meta = CoreProviderMetadata::discover_async(issuer, &*HTTP).await?;
        anyhow::Ok(meta)
    })
    .await
}

/// Build a client from cached metadata (cheap; no network).
async fn oidc_client() -> anyhow::Result<
    CoreClient<
        openidconnect::EndpointSet,
        openidconnect::EndpointNotSet,
        openidconnect::EndpointNotSet,
        openidconnect::EndpointNotSet,
        openidconnect::EndpointMaybeSet,
        openidconnect::EndpointMaybeSet,
    >,
> {
    let meta = provider_metadata().await?.clone();
    Ok(CoreClient::from_provider_metadata(
        meta,
        ClientId::new(CONFIG.oidc_client_id.clone()),
        Some(ClientSecret::new(CONFIG.oidc_client_secret.clone())),
    )
    .set_redirect_uri(RedirectUrl::new(format!(
        "{}/callback",
        CONFIG.app_base_url
    ))?))
}

fn error_response(e: anyhow::Error) -> Response {
    tracing::error!("auth error: {e:#}");
    (StatusCode::INTERNAL_SERVER_ERROR, "Authentication error").into_response()
}

pub async fn login() -> Response {
    match login_inner().await {
        Ok(r) => r,
        Err(e) => error_response(e),
    }
}

async fn login_inner() -> anyhow::Result<Response> {
    let client = oidc_client().await?;
    let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();
    let (auth_url, csrf, nonce) = client
        .authorize_url(
            CoreAuthenticationFlow::AuthorizationCode,
            CsrfToken::new_random,
            Nonce::new_random,
        )
        .add_scope(Scope::new("openid".into()))
        .add_scope(Scope::new("email".into()))
        .add_scope(Scope::new("profile".into()))
        .add_scope(Scope::new("groups".into()))
        .set_pkce_challenge(pkce_challenge)
        .url();

    if let Ok(mut states) = LOGIN_STATES.lock() {
        states.insert(
            csrf.secret().clone(),
            LoginState {
                pkce: pkce_verifier.secret().clone(),
                nonce: nonce.secret().clone(),
            },
        );
    }
    Ok(Redirect::to(auth_url.as_str()).into_response())
}

#[derive(Deserialize)]
pub struct CallbackQuery {
    code: Option<String>,
    state: Option<String>,
    error: Option<String>,
}

pub async fn callback(Query(q): Query<CallbackQuery>) -> Response {
    match callback_inner(q).await {
        Ok(r) => r,
        Err(e) => error_response(e),
    }
}

async fn callback_inner(q: CallbackQuery) -> anyhow::Result<Response> {
    if let Some(err) = q.error {
        anyhow::bail!("oidc provider returned error: {err}");
    }
    let code = q.code.ok_or_else(|| anyhow::anyhow!("missing authorization code"))?;
    let state = q.state.ok_or_else(|| anyhow::anyhow!("missing state"))?;
    let login_state = LOGIN_STATES
        .lock()
        .ok()
        .and_then(|mut s| s.remove(&state))
        .ok_or_else(|| anyhow::anyhow!("unknown or expired login state"))?;

    let client = oidc_client().await?;
    let token_response = client
        .exchange_code(AuthorizationCode::new(code))?
        .set_pkce_verifier(PkceCodeVerifier::new(login_state.pkce))
        .request_async(&*HTTP)
        .await?;

    let id_token = token_response
        .id_token()
        .ok_or_else(|| anyhow::anyhow!("no id token returned"))?;
    let verifier = client.id_token_verifier();
    let nonce = Nonce::new(login_state.nonce);
    let claims = id_token.claims(&verifier, &nonce)?;

    let sub = claims.subject().as_str().to_string();
    let username = claims
        .preferred_username()
        .map(|u| u.as_str().to_string())
        .unwrap_or_else(|| sub.clone());
    let display_name = claims
        .name()
        .and_then(|n| n.get(None))
        .map(|n| n.as_str().to_string())
        .unwrap_or_else(|| username.clone());
    let role = if is_admin(id_token) {
        Role::Admin
    } else {
        Role::Inviter
    };

    let sid = session::create(CurrentUser {
        sub,
        username,
        display_name,
        role,
    });
    let cookie = format!(
        "{}={}; Path=/; HttpOnly; SameSite=Lax; Max-Age=86400",
        session::COOKIE,
        sid
    );
    let mut resp = Redirect::to("/").into_response();
    resp.headers_mut().insert(header::SET_COOKIE, cookie.parse()?);
    Ok(resp)
}

pub async fn logout(headers: HeaderMap) -> Response {
    if let Some(sid) = session::session_id(&headers) {
        session::destroy(&sid);
    }
    let cookie = format!("{}=; Path=/; HttpOnly; Max-Age=0", session::COOKIE);
    let mut resp = Redirect::to("/").into_response();
    if let Ok(value) = cookie.parse() {
        resp.headers_mut().insert(header::SET_COOKIE, value);
    }
    resp
}

/// kanidm exposes group membership in the `groups` claim (via a scope map). The
/// standard OIDC claim set doesn't include it, so read it from the verified
/// id token's payload directly.
fn is_admin(id_token: &CoreIdToken) -> bool {
    let prefixed = format!("{}@", CONFIG.admin_group);
    groups_from_id_token(id_token)
        .iter()
        .any(|g| g == &CONFIG.admin_group || g.starts_with(&prefixed))
}

fn groups_from_id_token(id_token: &CoreIdToken) -> Vec<String> {
    use base64::Engine as _;
    let jwt = id_token.to_string();
    let Some(payload) = jwt.split('.').nth(1) else {
        return Vec::new();
    };
    let Ok(bytes) = base64::engine::general_purpose::URL_SAFE_NO_PAD.decode(payload) else {
        return Vec::new();
    };
    let Ok(value) = serde_json::from_slice::<serde_json::Value>(&bytes) else {
        return Vec::new();
    };
    value
        .get("groups")
        .and_then(|g| g.as_array())
        .map(|arr| arr.iter().filter_map(|x| x.as_str().map(str::to_string)).collect())
        .unwrap_or_default()
}

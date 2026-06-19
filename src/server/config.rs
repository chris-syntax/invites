use std::sync::LazyLock;

/// Runtime configuration, read once from the environment. Required values panic
/// at first access if unset (fail fast on a misconfigured deployment).
pub struct Config {
    pub kanidm_url: String,
    pub app_base_url: String,
    pub oidc_client_id: String,
    pub oidc_client_secret: String,
    pub service_account_token: String,
    pub database_url: String,
    pub admin_group: String,
    pub reset_token_ttl_secs: u32,
}

fn required(key: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| panic!("missing required env var {key}"))
}

fn optional(key: &str, default: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| default.to_string())
}

pub static CONFIG: LazyLock<Config> = LazyLock::new(|| Config {
    kanidm_url: optional("KANIDM_URL", "https://account.loaf.moe"),
    app_base_url: required("APP_BASE_URL"),
    oidc_client_id: required("OIDC_CLIENT_ID"),
    oidc_client_secret: required("OIDC_CLIENT_SECRET"),
    service_account_token: required("KANIDM_SERVICE_TOKEN"),
    database_url: optional("DATABASE_URL", "sqlite://invites.db?mode=rwc"),
    admin_group: optional("KANIDM_ADMIN_GROUP", "idm_admins"),
    reset_token_ttl_secs: optional("RESET_TOKEN_TTL_SECS", "3600")
        .parse::<u32>()
        .unwrap_or(3600)
        .clamp(60, 86_400),
});

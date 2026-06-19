use std::time::Duration;

use serde::Deserialize;

use crate::server::config::CONFIG;

/// Thin client over the kanidm REST API, authenticated with the service
/// account API token (member of `idm_people_admins`).
pub struct Kanidm {
    http: reqwest::Client,
}

#[derive(Deserialize)]
struct CuIntentToken {
    token: String,
}

impl Kanidm {
    pub fn new() -> anyhow::Result<Self> {
        // A request timeout matters: in the redeem path this call happens while
        // holding the invitation's write lock, so it must not hang forever.
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(15))
            .build()?;
        Ok(Self { http })
    }

    /// Create a person with name, displayname and mail in a single `Entry`.
    pub async fn create_person(
        &self,
        name: &str,
        displayname: &str,
        mail: &str,
    ) -> anyhow::Result<()> {
        let body = serde_json::json!({
            "attrs": {
                "name": [name],
                "displayname": [displayname],
                "mail": [mail],
            }
        });
        let resp = self
            .http
            .post(format!("{}/v1/person", CONFIG.kanidm_url))
            .bearer_auth(&CONFIG.service_account_token)
            .json(&body)
            .send()
            .await?;
        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            anyhow::bail!("kanidm create_person failed ({status}): {text}");
        }
        Ok(())
    }

    /// Mint a credential update intent token for the given person (referenced by
    /// name). The returned token is embedded in the reset URL.
    pub async fn credential_update_intent(&self, id: &str) -> anyhow::Result<String> {
        let ttl = CONFIG.reset_token_ttl_secs.min(86_400);
        let resp = self
            .http
            .get(format!(
                "{}/v1/person/{}/_credential/_update_intent/{}",
                CONFIG.kanidm_url, id, ttl
            ))
            .bearer_auth(&CONFIG.service_account_token)
            .send()
            .await?;
        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            anyhow::bail!("kanidm credential intent failed ({status}): {text}");
        }
        let parsed: CuIntentToken = resp.json().await?;
        Ok(parsed.token)
    }

    /// The kanidm credential reset page the invitee is redirected to.
    pub fn reset_url(&self, intent_token: &str) -> String {
        format!("{}/ui/reset?token={}", CONFIG.kanidm_url, intent_token)
    }
}

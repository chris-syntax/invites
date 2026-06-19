use dioxus::prelude::*;

use crate::shared::{InviteePrompt, SignupForm, SignupOutcome};

/// Public: what an invitee sees when opening an invitation link.
#[get("/api/invite/{token}")]
pub async fn get_invite(token: String) -> Result<InviteePrompt> {
    Ok(crate::server::db::prompt(&token).await?)
}

/// Provision an account from an invitation. Returns a typed `SignupOutcome` so
/// the form can show a precise message (notably "username taken") instead of a
/// generic failure; only network/RPC errors surface as `Err`.
#[post("/api/invite/{token}/signup")]
pub async fn signup(token: String, form: SignupForm) -> Result<SignupOutcome> {
    use crate::server::db::RedeemError;
    Ok(match crate::server::db::redeem(&token, form).await {
        Ok(reset_url) => SignupOutcome::Success { reset_url },
        Err(RedeemError::UsernameTaken) => SignupOutcome::UsernameTaken,
        Err(RedeemError::Invalid(msg)) => SignupOutcome::Invalid(msg),
        Err(RedeemError::Unavailable) => SignupOutcome::Unavailable,
        Err(RedeemError::Internal(e)) => {
            tracing::error!("signup failed: {e:#}");
            SignupOutcome::Internal
        }
    })
}

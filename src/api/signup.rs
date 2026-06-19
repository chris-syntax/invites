use dioxus::fullstack::{Form, Redirect};
use dioxus::prelude::*;

use crate::shared::{InviteePrompt, SignupForm};

/// Public: what an invitee sees when opening an invitation link.
#[get("/api/invite/{token}")]
pub async fn get_invite(token: String) -> Result<InviteePrompt> {
    Ok(crate::server::db::prompt(&token).await?)
}

/// Public native form post. On success the browser follows the 303 to kanidm's
/// credential reset page; on failure it returns to the invite form.
#[post("/api/invite/{token}/signup")]
pub async fn signup(token: String, form: Form<SignupForm>) -> Result<Redirect> {
    match crate::server::db::redeem(&token, form.0).await {
        Ok(reset_url) => Ok(Redirect::to(&reset_url)),
        Err(_) => Ok(Redirect::to(&format!("/invite/{token}"))),
    }
}

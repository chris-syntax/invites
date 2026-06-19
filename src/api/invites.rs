#[cfg(feature = "server")]
use dioxus::fullstack::HeaderMap;
use dioxus::prelude::*;

use crate::shared::{CreateInviteReq, InvitationView};

#[post("/api/invites", headers: HeaderMap)]
pub async fn create_invite(req: CreateInviteReq) -> Result<InvitationView> {
    let user = crate::server::session::current_user(&headers)
        .ok_or_else(|| anyhow::anyhow!("not authenticated"))?;
    Ok(crate::server::db::create_invitation(&user, req).await?)
}

#[post("/api/invites/revoke", headers: HeaderMap)]
pub async fn revoke_invite(token: String) -> Result<()> {
    let user = crate::server::session::current_user(&headers)
        .ok_or_else(|| anyhow::anyhow!("not authenticated"))?;
    Ok(crate::server::db::revoke(&token, &user).await?)
}

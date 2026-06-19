#[cfg(feature = "server")]
use dioxus::fullstack::HeaderMap;
use dioxus::prelude::*;

use crate::shared::Dashboard;

/// Current session + the invitations the signed-in inviter may see. Returns an
/// empty dashboard with `user: None` when not signed in.
#[get("/api/dashboard", headers: HeaderMap)]
pub async fn get_dashboard() -> Result<Dashboard> {
    let user = crate::server::session::current_user(&headers);
    let invites = match &user {
        Some(u) => crate::server::db::list_invitations(u).await?,
        None => Vec::new(),
    };
    Ok(Dashboard { user, invites })
}

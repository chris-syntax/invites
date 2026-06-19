use dioxus::prelude::*;

use crate::api::revoke_invite;
use crate::shared::{InvitationView, InviteStatus};

#[component]
pub fn InviteRow(inv: InvitationView, on_revoked: EventHandler<()>) -> Element {
    let token = inv.token.clone();
    let link = format!("/invite/{token}");
    let used = match inv.max_uses {
        Some(max) => format!("{} / {}", inv.accounts_created, max),
        None => inv.accounts_created.to_string(),
    };
    let can_revoke = inv.owned && matches!(inv.status, InviteStatus::Active);

    rsx! {
        tr {
            td { "{inv.label}" }
            td { a { href: "{link}", "{link}" } }
            td { "{inv.status.label()}" }
            td { "{used}" }
            td {
                if can_revoke {
                    button {
                        class: "secondary outline",
                        onclick: move |_| {
                            let token = token.clone();
                            async move {
                                if revoke_invite(token).await.is_ok() {
                                    on_revoked.call(());
                                }
                            }
                        },
                        "Revoke"
                    }
                }
            }
        }
    }
}

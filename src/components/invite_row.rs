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
    let badge_tone = match inv.status {
        InviteStatus::Active => "badge-success",
        InviteStatus::Expired => "badge-neutral",
        InviteStatus::Revoked => "badge-accent",
        InviteStatus::Exhausted => "badge-warning",
    };
    let label = if inv.label.is_empty() { "untitled".to_string() } else { inv.label.clone() };

    rsx! {
        tr { class: "border-t border-line",
            td { class: "px-4 py-3.5 font-medium text-ink", "{label}" }
            td { class: "px-4 py-3.5",
                a { class: "font-mono text-[0.6875rem] text-ink-500", href: "{link}", "{link}" }
            }
            td { class: "px-4 py-3.5",
                span { class: "badge {badge_tone}", "{inv.status.label()}" }
            }
            td { class: "px-4 py-3.5 text-ink-500 tabular-nums", "{used}" }
            td { class: "px-4 py-3.5 text-right",
                if can_revoke {
                    button {
                        class: "btn btn-outline btn-sm",
                        onclick: move |_| {
                            let token = token.clone();
                            async move {
                                if revoke_invite(token).await.is_ok() {
                                    on_revoked.call(());
                                }
                            }
                        },
                        "revoke"
                    }
                }
            }
        }
    }
}

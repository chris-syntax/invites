use dioxus::prelude::*;

use crate::components::InviteRow;
use crate::shared::InvitationView;

#[component]
pub fn InviteList(invites: Vec<InvitationView>, on_revoked: EventHandler<()>) -> Element {
    if invites.is_empty() {
        return rsx! {
            div { class: "card flex flex-col items-center gap-1 text-center py-10",
                p { class: "text-ink font-medium", "no invites yet" }
                p { class: "text-[0.8125rem] text-muted", "bake one above to start welcoming people in." }
            }
        };
    }
    rsx! {
        div { class: "card p-0 overflow-x-auto",
            table { class: "w-full border-collapse text-[0.8125rem]",
                thead {
                    tr {
                        th { class: "eyebrow text-left px-4 pt-4 pb-2.5", "label" }
                        th { class: "eyebrow text-left px-4 pt-4 pb-2.5", "link" }
                        th { class: "eyebrow text-left px-4 pt-4 pb-2.5", "status" }
                        th { class: "eyebrow text-left px-4 pt-4 pb-2.5", "accounts" }
                        th { class: "px-4 pt-4 pb-2.5" }
                    }
                }
                tbody {
                    for inv in invites.into_iter() {
                        InviteRow { key: "{inv.token}", inv, on_revoked }
                    }
                }
            }
        }
    }
}

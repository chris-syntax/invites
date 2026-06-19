use dioxus::prelude::*;

use crate::components::InviteRow;
use crate::shared::InvitationView;

#[component]
pub fn InviteList(invites: Vec<InvitationView>, on_revoked: EventHandler<()>) -> Element {
    if invites.is_empty() {
        return rsx! { p { "No invitations yet." } };
    }
    rsx! {
        table {
            thead {
                tr {
                    th { "Label" }
                    th { "Link" }
                    th { "Status" }
                    th { "Accounts created" }
                    th {}
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

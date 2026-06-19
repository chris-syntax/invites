use dioxus::prelude::*;

use crate::api::get_dashboard;
use crate::components::{CreateInviteForm, InviteList};
use crate::shared::Dashboard;

#[component]
pub fn Home() -> Element {
    let mut dash = use_resource(get_dashboard);
    let view = dash.read();

    let body = match &*view {
        None => rsx! { p { "Loading…" } },
        Some(Err(e)) => rsx! { p { "Failed to load: {e}" } },
        Some(Ok(Dashboard { user: None, .. })) => rsx! {
            hgroup {
                h1 { "Invites" }
                p { "Self-service kanidm account invitations." }
            }
            p { "Sign in with your kanidm account to create invitation links." }
            a { href: "/login", role: "button", "Sign in with kanidm" }
        },
        Some(Ok(Dashboard { user: Some(user), invites })) => {
            let invites = invites.clone();
            rsx! {
                nav {
                    ul { li { strong { "Invites" } } }
                    ul {
                        li { "Signed in as {user.display_name}" }
                        li { a { href: "/logout", "Sign out" } }
                    }
                }
                CreateInviteForm { on_created: move |_| dash.restart() }
                InviteList { invites, on_revoked: move |_| dash.restart() }
            }
        }
    };

    rsx! { main { class: "container", {body} } }
}

use dioxus::prelude::*;

use crate::api::get_dashboard;
use crate::components::{CreateInviteForm, InviteList};
use crate::shared::Dashboard;

#[component]
pub fn Home() -> Element {
    let mut dash = use_resource(get_dashboard);

    // No landing page: unauthenticated visitors go straight into the OAuth flow.
    // `/login` is a server route that 302s to kanidm. Runs on the client only
    // (effects don't run during SSR), once the dashboard reports no user.
    use_effect(move || {
        if let Some(Ok(Dashboard { user: None, .. })) = &*dash.read() {
            spawn(async move {
                let _ = document::eval("window.location.href = '/login';").await;
            });
        }
    });

    let view = dash.read();
    let body = match &*view {
        None => rsx! { p { aria_busy: "true", "Loading…" } },
        Some(Err(e)) => rsx! { p { "Failed to load: {e}" } },
        Some(Ok(Dashboard { user: None, .. })) => rsx! {
            p { aria_busy: "true", "Redirecting to sign in…" }
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

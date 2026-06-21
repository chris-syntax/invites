use dioxus::prelude::*;

use crate::api::get_dashboard;
use crate::components::{CreateInviteForm, InviteList, Logo};
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
    match &*view {
        None => rsx! {
            div { class: "min-h-screen flex items-center justify-center text-muted",
                "loading your loaf…"
            }
        },
        Some(Err(e)) => rsx! {
            div { class: "min-h-screen flex items-center justify-center text-muted",
                "couldn't load your invites: {e}"
            }
        },
        Some(Ok(Dashboard { user: None, .. })) => rsx! {
            div { class: "min-h-screen flex items-center justify-center text-muted",
                "redirecting you to sign in…"
            }
        },
        Some(Ok(Dashboard { user: Some(user), invites })) => {
            let invites = invites.clone();
            let name = user.display_name.clone();
            rsx! {
                header { class: "flex items-center justify-between gap-4 px-6 py-4 bg-cream border-b border-line",
                    Logo {}
                    div { class: "flex items-center gap-4 text-[0.8125rem] text-muted",
                        span { "signed in as " span { class: "text-ink font-medium", "{name}" } }
                        a { class: "btn btn-outline btn-sm", href: "/logout", "sign out" }
                    }
                }
                main { class: "max-w-[880px] mx-auto px-6 pt-12 pb-20 flex flex-col gap-8",
                    CreateInviteForm { on_created: move |_| dash.restart() }
                    section { class: "flex flex-col gap-4",
                        div { class: "flex flex-col gap-1",
                            span { class: "eyebrow", "your invites" }
                            h2 { class: "text-xl text-ink", "everyone you've welcomed in" }
                        }
                        InviteList { invites, on_revoked: move |_| dash.restart() }
                    }
                }
            }
        }
    }
}

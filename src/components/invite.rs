use dioxus::prelude::*;

use crate::api::get_invite;
use crate::shared::InviteePrompt;

#[component]
pub fn Invite(token: String) -> Element {
    let prompt = use_resource({
        let token = token.clone();
        move || {
            let token = token.clone();
            async move { get_invite(token).await }
        }
    });
    let action = format!("/api/invite/{token}/signup");
    let view = prompt.read();

    let body = match &*view {
        None => rsx! { p { "Loading…" } },
        Some(Err(e)) => rsx! { p { "Failed to load: {e}" } },
        Some(Ok(InviteePrompt::Unavailable(reason))) => rsx! {
            hgroup {
                h1 { "Invitation unavailable" }
                p { "{reason.message()}" }
            }
        },
        Some(Ok(InviteePrompt::Open { label })) => rsx! {
            hgroup {
                h1 { "You're invited" }
                if !label.is_empty() {
                    p { "{label}" }
                }
            }
            form { method: "post", action: "{action}",
                label { "Username"
                    input {
                        name: "username",
                        required: true,
                        autocapitalize: "off",
                        autocomplete: "off",
                        pattern: "[a-z][a-z0-9._-]*",
                    }
                    small { "Lowercase letters, digits, '.', '-' and '_'. Must start with a letter." }
                }
                label { "Display name"
                    input { name: "displayname", required: true }
                }
                label { "Email"
                    input { name: "email", r#type: "email", required: true }
                }
                button { r#type: "submit", "Create account" }
            }
            p { small { "You'll set your password on the kanidm site after this step." } }
        },
    };

    rsx! { main { class: "container", {body} } }
}

use dioxus::prelude::*;

use crate::api::{get_invite, signup};
use crate::shared::{InviteePrompt, SignupForm, SignupOutcome};

#[component]
pub fn Invite(token: String) -> Element {
    let mut prompt = use_resource({
        let token = token.clone();
        move || {
            let token = token.clone();
            async move { get_invite(token).await }
        }
    });

    // Controlled inputs so the invitee's entries survive a failed submit, and an
    // error message held in component state (not the URL).
    let mut username = use_signal(String::new);
    let mut displayname = use_signal(String::new);
    let mut email = use_signal(String::new);
    let mut error = use_signal(|| Option::<String>::None);
    let mut submitting = use_signal(|| false);

    let view = prompt.read();
    let body = match &*view {
        None => rsx! { p { aria_busy: "true", "Loading…" } },
        Some(Err(e)) => rsx! { p { "Failed to load: {e}" } },
        Some(Ok(InviteePrompt::Unavailable(reason))) => rsx! {
            hgroup {
                h1 { "Invitation unavailable" }
                p { "{reason.message()}" }
            }
        },
        Some(Ok(InviteePrompt::Open { label })) => {
            let token = token.clone();
            let on_submit = move |evt: FormEvent| {
                // Stop the browser's native navigation; we submit over RPC.
                evt.prevent_default();
                let token = token.clone();
                async move {
                    error.set(None);
                    submitting.set(true);
                    let form = SignupForm {
                        username: username(),
                        displayname: displayname(),
                        email: email(),
                    };
                    match signup(token, form).await {
                        Ok(SignupOutcome::Success { reset_url }) => {
                            // Hand off to kanidm's credential-reset page.
                            let _ = document::eval(&format!(
                                "window.location.href = {reset_url:?};"
                            ))
                            .await;
                        }
                        Ok(SignupOutcome::UsernameTaken) => {
                            error.set(Some(
                                "That username is already taken — please choose another.".into(),
                            ));
                        }
                        Ok(SignupOutcome::Invalid(msg)) => error.set(Some(msg)),
                        Ok(SignupOutcome::Unavailable) => prompt.restart(),
                        Ok(SignupOutcome::Internal) => error.set(Some(
                            "Something went wrong creating your account. Please try again.".into(),
                        )),
                        Err(e) => error.set(Some(format!("Request failed: {e}"))),
                    }
                    submitting.set(false);
                }
            };
            rsx! {
                hgroup {
                    h1 { "You're invited" }
                    if !label.is_empty() {
                        p { "{label}" }
                    }
                }
                if let Some(msg) = error() {
                    p { role: "alert", style: "color: var(--pico-del-color)", "{msg}" }
                }
                form { onsubmit: on_submit,
                    label { "Username"
                        input {
                            name: "username",
                            required: true,
                            autocapitalize: "off",
                            autocomplete: "off",
                            pattern: "[a-z][a-z0-9._-]*",
                            value: "{username}",
                            oninput: move |e| username.set(e.value()),
                        }
                        small { "Lowercase letters, digits, '.', '-' and '_'. Must start with a letter." }
                    }
                    label { "Display name"
                        input {
                            name: "displayname",
                            required: true,
                            value: "{displayname}",
                            oninput: move |e| displayname.set(e.value()),
                        }
                    }
                    label { "Email"
                        input {
                            name: "email",
                            r#type: "email",
                            required: true,
                            value: "{email}",
                            oninput: move |e| email.set(e.value()),
                        }
                    }
                    button { r#type: "submit", disabled: submitting(),
                        if submitting() { "Creating…" } else { "Create account" }
                    }
                }
                p { small { "You'll set your password on the kanidm site after this step." } }
            }
        }
    };

    rsx! { main { class: "container", {body} } }
}

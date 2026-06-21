use dioxus::prelude::*;

use crate::api::{get_invite, signup};
use crate::components::Logo;
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
        None => rsx! { p { class: "text-center text-muted", "loading…" } },
        Some(Err(e)) => rsx! { p { class: "text-center text-muted", "couldn't load this invite: {e}" } },
        Some(Ok(InviteePrompt::Unavailable(reason))) => rsx! {
            div { class: "flex flex-col gap-2 text-center",
                h1 { class: "text-2xl text-ink", "this invite isn't available" }
                p { class: "text-[0.8125rem] text-muted", "{reason.message()}" }
            }
        },
        Some(Ok(InviteePrompt::Open)) => {
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
                div { class: "flex flex-col gap-2 text-center",
                    h1 { class: "text-3xl text-ink",
                        "you're "
                        span { class: "italic text-accent", "invited" }
                        " 🍞"
                    }
                }
                if let Some(msg) = error() {
                    p { class: "alert-error", role: "alert", "{msg}" }
                }
                form { class: "flex flex-col gap-[18px]", onsubmit: on_submit,
                    label { class: "flex flex-col gap-1.5",
                        span { class: "text-[0.8125rem] font-semibold text-ink", "username" }
                        input {
                            class: "field-input",
                            name: "username",
                            required: true,
                            autocapitalize: "off",
                            autocomplete: "off",
                            pattern: "[a-z][a-z0-9._-]*",
                            value: "{username}",
                            oninput: move |e| username.set(e.value()),
                        }
                        span { class: "text-[0.6875rem] text-muted",
                            "lowercase letters, digits, '.', '-' and '_'. must start with a letter."
                        }
                    }
                    label { class: "flex flex-col gap-1.5",
                        span { class: "text-[0.8125rem] font-semibold text-ink", "display name" }
                        input {
                            class: "field-input",
                            name: "displayname",
                            required: true,
                            value: "{displayname}",
                            oninput: move |e| displayname.set(e.value()),
                        }
                    }
                    label { class: "flex flex-col gap-1.5",
                        span { class: "text-[0.8125rem] font-semibold text-ink", "email" }
                        input {
                            class: "field-input",
                            name: "email",
                            r#type: "email",
                            required: true,
                            value: "{email}",
                            oninput: move |e| email.set(e.value()),
                        }
                    }
                    button { class: "btn btn-primary w-full mt-1", r#type: "submit", disabled: submitting(),
                        if submitting() { "baking your account…" } else { "create account" }
                    }
                }
                p { class: "text-[0.6875rem] text-muted text-center",
                    "you'll set your password on the kanidm site after this step."
                }
            }
        }
    };

    rsx! {
        main { class: "min-h-screen flex items-center justify-center px-5 py-10",
            div { class: "w-full max-w-[440px] card rounded-[22px] shadow-lift p-9 flex flex-col gap-5",
                div { class: "flex justify-center", Logo {} }
                {body}
            }
        }
    }
}

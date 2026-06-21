use std::num::NonZeroU32;

use dioxus::prelude::*;

use crate::api::create_invite;
use crate::shared::{CreateInviteReq, Ttl};

#[component]
pub fn CreateInviteForm(on_created: EventHandler<()>) -> Element {
    let mut label = use_signal(String::new);
    let mut ttl = use_signal(|| 86_400u32);
    let mut max_uses = use_signal(String::new);

    rsx! {
        article { class: "card card-accent flex flex-col gap-5",
            div { class: "flex flex-col gap-1",
                span { class: "eyebrow", "new invite" }
                h2 { class: "text-2xl text-ink", "gather your crew" }
                p { class: "text-[0.8125rem] text-muted",
                    "make a link, share it with your people — they'll set up their own account."
                }
            }
            form {
                class: "flex flex-col gap-[18px]",
                onsubmit: move |evt: FormEvent| async move {
                    evt.prevent_default();
                    // 0 is the "never" sentinel (Ttl::new rejects anything < 60).
                    let ttl = match ttl() {
                        0 => None,
                        secs => match Ttl::new(secs) {
                            Some(t) => Some(t),
                            None => return,
                        },
                    };
                    let parsed_max = max_uses().trim().parse::<u32>().ok().and_then(NonZeroU32::new);
                    let req = CreateInviteReq { label: label(), ttl, max_uses: parsed_max };
                    if create_invite(req).await.is_ok() {
                        label.set(String::new());
                        max_uses.set(String::new());
                        on_created.call(());
                    }
                },
                label { class: "flex flex-col gap-1.5",
                    span { class: "text-[0.8125rem] font-semibold text-ink", "label" }
                    input {
                        class: "field-input",
                        value: "{label}",
                        oninput: move |e| label.set(e.value()),
                        placeholder: "e.g. friends of the homelab",
                    }
                }
                div { class: "grid grid-cols-1 sm:grid-cols-2 gap-[18px]",
                    label { class: "flex flex-col gap-1.5",
                        span { class: "text-[0.8125rem] font-semibold text-ink", "expires after" }
                        select {
                            class: "field-input",
                            value: "{ttl}",
                            oninput: move |e| {
                                if let Ok(v) = e.value().parse::<u32>() {
                                    ttl.set(v);
                                }
                            },
                            option { value: "3600", "1 hour" }
                            option { value: "86400", "1 day" }
                            option { value: "604800", "7 days" }
                            option { value: "2592000", "30 days" }
                            option { value: "0", "never" }
                        }
                    }
                    label { class: "flex flex-col gap-1.5",
                        span { class: "text-[0.8125rem] font-semibold text-ink", "max uses" }
                        input {
                            class: "field-input",
                            r#type: "number",
                            min: "1",
                            value: "{max_uses}",
                            oninput: move |e| max_uses.set(e.value()),
                            placeholder: "unlimited",
                        }
                    }
                }
                div {
                    button { class: "btn btn-primary", r#type: "submit", "create invite" }
                }
            }
        }
    }
}

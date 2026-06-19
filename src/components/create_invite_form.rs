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
        article {
            h2 { "Create an invitation" }
            form {
                onsubmit: move |evt: FormEvent| async move {
                    evt.prevent_default();
                    let Some(ttl) = Ttl::new(ttl()) else { return };
                    let parsed_max = max_uses().trim().parse::<u32>().ok().and_then(NonZeroU32::new);
                    let req = CreateInviteReq { label: label(), ttl, max_uses: parsed_max };
                    if create_invite(req).await.is_ok() {
                        label.set(String::new());
                        max_uses.set(String::new());
                        on_created.call(());
                    }
                },
                label { "Label"
                    input {
                        value: "{label}",
                        oninput: move |e| label.set(e.value()),
                        placeholder: "e.g. Friends of the homelab",
                    }
                }
                label { "Expires after"
                    select {
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
                    }
                }
                label { "Max uses (blank = unlimited)"
                    input {
                        r#type: "number",
                        min: "1",
                        value: "{max_uses}",
                        oninput: move |e| max_uses.set(e.value()),
                        placeholder: "unlimited",
                    }
                }
                button { r#type: "submit", "Create invitation" }
            }
        }
    }
}

use dioxus::prelude::*;

/// The loaf.moe mark: a rounded navy "toast tile" with a cream serif `l` and a
/// red dot, beside the lowercase serif wordmark. The red dot is the one playful
/// flourish — keep it the only accent in the lockup.
#[component]
pub fn Logo() -> Element {
    rsx! {
        span { class: "inline-flex items-center gap-2 font-display select-none",
            span {
                class: "inline-flex items-center justify-center w-8 h-8 rounded-[0.32em] \
                        bg-ink text-cream font-semibold text-xl leading-none shadow-soft \
                        tracking-[-0.02em] shrink-0",
                "aria-hidden": "true",
                "l"
                span { class: "text-accent", "." }
            }
            span { class: "text-xl font-semibold tracking-[-0.02em] text-ink",
                "loaf"
                span { class: "text-muted font-medium",
                    span { class: "text-accent", "." }
                    "moe"
                }
            }
        }
    }
}

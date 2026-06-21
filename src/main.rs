#![allow(non_snake_case)]

use dioxus::prelude::*;

mod api;
mod components;
mod shared;

#[cfg(feature = "server")]
mod server;

use components::{Home, Invite};

fn main() {
    #[cfg(not(feature = "server"))]
    dioxus::launch(app);

    #[cfg(feature = "server")]
    server::serve(app);
}

// Compiled from `tailwind.css` at the project root; dx runs the Tailwind
// watcher automatically during `dx serve` / `dx bundle`.
static TAILWIND: Asset = asset!("/assets/tailwind.css");

fn app() -> Element {
    rsx! {
        document::Stylesheet { href: TAILWIND }
        Router::<Route> {}
    }
}

#[derive(Clone, Routable, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum Route {
    #[route("/")]
    Home {},
    #[route("/invite/:token")]
    Invite { token: String },
}

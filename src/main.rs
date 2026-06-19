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

fn app() -> Element {
    rsx! {
        document::Stylesheet { href: "https://cdn.jsdelivr.net/npm/@picocss/pico@2/css/pico.min.css" }
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

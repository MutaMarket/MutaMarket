// Statically-typed Leptos views nest deeply; the filter sidebar alone
// overflows the default type-layout recursion limit.
#![recursion_limit = "256"]

pub mod app;
#[cfg(feature = "ssr")]
pub mod auth;
#[cfg(feature = "ssr")]
pub mod characters;
#[cfg(feature = "ssr")]
pub mod contracts;
#[cfg(feature = "ssr")]
pub mod db;
#[cfg(feature = "ssr")]
pub mod docs;
#[cfg(feature = "ssr")]
pub mod scheduler;
#[cfg(feature = "ssr")]
pub mod esi;
pub mod modules;
pub mod mutation;
pub mod pages;
#[cfg(feature = "ssr")]
pub mod sde;
#[cfg(feature = "ssr")]
pub mod server;

#[cfg(feature = "hydrate")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn hydrate() {
    use crate::app::App;

    console_error_panic_hook::set_once();
    leptos::mount::hydrate_body(App);
}

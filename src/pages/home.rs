//! The home page: the module browser.

use leptos::prelude::*;
use leptos_meta::Title;

use super::modules_page::ModuleBrowser;

#[component]
pub fn HomePage() -> impl IntoView {
    view! {
        <Title text="MutaMarket - Abyssal Modules"/>
        <ModuleBrowser/>
    }
}

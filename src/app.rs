use leptos::prelude::*;
use leptos_meta::{MetaTags, Stylesheet, Title, provide_meta_context};
use leptos_router::components::{ParentRoute, Route, Router, Routes};
use leptos_router::{SsrMode, path};

use crate::pages::{
    PersonalModulesPage,
    AllModulesPage, CharacterPage, CharactersPage, CollectionPage, CollectionsPage,
    DocumentationPage, HomePage, Layout, LoginPage, ModulesPage, PlaceholderPage,
};

pub fn shell(options: LeptosOptions) -> impl IntoView {
    view! {
        <!DOCTYPE html>
        <html lang="en" class="dark">
            <head>
                <meta charset="utf-8"/>
                <meta name="viewport" content="width=device-width, initial-scale=1"/>
                <AutoReload options=options.clone()/>
                <HydrationScripts options/>
                <MetaTags/>
            </head>
            <body>
                <App/>
            </body>
        </html>
    }
}

#[component]
pub fn App() -> impl IntoView {
    provide_meta_context();

    // With a `set_is_routing` setter the router keeps the current page mounted
    // until the incoming route's async resources resolve, then swaps (via the
    // View Transition API), instead of showing the new page's loading state
    // first. The signal also feeds a slim top bar so the wait has feedback.
    let (is_routing, set_is_routing) = signal(false);

    view! {
        <Title text="MutaMarket"/>
        <Stylesheet id="app" href="/pkg/mutamarket.css"/>
        <Router set_is_routing>
            <RoutingIndicator is_routing/>
            <Routes transition=true fallback=|| view! { <NotFoundPage/> }>
                <ParentRoute path=path!("") view=Layout ssr=SsrMode::Async>
                    <Route path=path!("") view=HomePage/>
                    <Route path=path!("login") view=LoginPage/>
                    <Route path=path!("modules") view=ModulesPage/>
                    <Route path=path!("modules/add") view=|| view! { <PlaceholderPage title="Add Module"/> }/>
                    <Route path=path!("modules/*query") view=ModulesPage/>
                    <Route path=path!("all-modules") view=AllModulesPage/>
                    <Route path=path!("all-modules/*query") view=AllModulesPage/>
                    <Route path=path!("characters") view=CharactersPage/>
                    <Route path=path!("characters/:character") view=CharacterPage/>
                    <Route path=path!("characters/:character/*query") view=CharacterPage/>
                    <Route path=path!("collections") view=CollectionsPage/>
                    <Route path=path!("collections/:collection") view=CollectionPage/>
                    <Route path=path!("collections/:collection/*query") view=CollectionPage/>
                    <Route path=path!("calculator") view=|| view! { <PlaceholderPage title="Roll Calculator"/> }/>
                    <Route path=path!("statistics") view=|| view! { <PlaceholderPage title="Statistics"/> }/>
                    <Route path=path!("personal/modules") view=PersonalModulesPage/>
                    <Route path=path!("personal/modules/*query") view=PersonalModulesPage/>
                    <Route path=path!("premium") view=|| view! { <PlaceholderPage title="Premium"/> }/>
                    <Route path=path!("omega-calculator") view=|| view! { <PlaceholderPage title="Omega Calculator"/> }/>
                    <Route path=path!("documentation") view=DocumentationPage/>
                    <Route path=path!("documentation/:page") view=DocumentationPage/>
                    <Route path=path!("donations") view=|| view! { <PlaceholderPage title="Donations"/> }/>
                    <Route path=path!("moderator/contracts") view=|| view! { <PlaceholderPage title="Contract Review"/> }/>
                    <Route path=path!("workbench/*modules") view=|| view! { <PlaceholderPage title="Workbench"/> }/>
                </ParentRoute>
            </Routes>
        </Router>
    }
}

/// A slim top bar shown while the router awaits the next route's data, so a
/// navigation that holds the current page for a moment still feels responsive.
#[component]
fn RoutingIndicator(#[prop(into)] is_routing: Signal<bool>) -> impl IntoView {
    view! {
        <div
            class="pointer-events-none fixed inset-x-0 top-0 z-[100] h-0.5 bg-primary transition-opacity duration-200"
            style:opacity=move || if is_routing.get() { "1" } else { "0" }
        ></div>
    }
}

#[component]
fn NotFoundPage() -> impl IntoView {
    #[cfg(feature = "ssr")]
    if let Some(response) = use_context::<leptos_axum::ResponseOptions>() {
        response.set_status(axum::http::StatusCode::NOT_FOUND);
    }

    view! { <h1>"Page not found"</h1> }
}

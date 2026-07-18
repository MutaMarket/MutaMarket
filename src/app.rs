use leptos::prelude::*;
use leptos_meta::{MetaTags, Stylesheet, Title, provide_meta_context};
use leptos_router::components::{ParentRoute, Route, Router, Routes};
use leptos_router::{SsrMode, path};

use crate::pages::{HomePage, Layout, LoginPage, ModulesPage, PlaceholderPage};

pub fn shell(options: LeptosOptions) -> impl IntoView {
    view! {
        <!DOCTYPE html>
        <html lang="en">
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

    view! {
        <Title text="MutaMarket"/>
        <Stylesheet id="app" href="/app.css"/>
        <Router>
            <Routes fallback=|| view! { <NotFoundPage/> }>
                <ParentRoute path=path!("") view=Layout ssr=SsrMode::Async>
                    <Route path=path!("") view=HomePage/>
                    <Route path=path!("login") view=LoginPage/>
                    <Route path=path!("modules") view=ModulesPage/>
                    <Route path=path!("modules/add") view=|| view! { <PlaceholderPage title="Add Module"/> }/>
                    <Route path=path!("modules/*query") view=ModulesPage/>
                    <Route path=path!("all-modules") view=|| view! { <PlaceholderPage title="All Modules"/> }/>
                    <Route path=path!("characters") view=|| view! { <PlaceholderPage title="Characters"/> }/>
                    <Route path=path!("collections") view=|| view! { <PlaceholderPage title="Collections"/> }/>
                    <Route path=path!("calculator") view=|| view! { <PlaceholderPage title="Roll Calculator"/> }/>
                    <Route path=path!("statistics") view=|| view! { <PlaceholderPage title="Statistics"/> }/>
                    <Route path=path!("premium") view=|| view! { <PlaceholderPage title="Premium"/> }/>
                    <Route path=path!("omega-calculator") view=|| view! { <PlaceholderPage title="Omega Calculator"/> }/>
                    <Route path=path!("documentation") view=|| view! { <PlaceholderPage title="Documentation"/> }/>
                    <Route path=path!("documentation/:page") view=|| view! { <PlaceholderPage title="Documentation"/> }/>
                    <Route path=path!("donations") view=|| view! { <PlaceholderPage title="Donations"/> }/>
                    <Route path=path!("moderator/contracts") view=|| view! { <PlaceholderPage title="Contract Review"/> }/>
                    <Route path=path!("workbench/*modules") view=|| view! { <PlaceholderPage title="Workbench"/> }/>
                </ParentRoute>
            </Routes>
        </Router>
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

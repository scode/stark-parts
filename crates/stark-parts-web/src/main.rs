//! Static browser entrypoint and catalog-loading boundary.
//!
//! This is the only web-app layer allowed to fetch catalog data. It loads and validates the complete deployed snapshot
//! once, then hands ownership to the browser-local UI in the library crate. Search interactions must never return to
//! this transport layer.

#[cfg(target_arch = "wasm32")]
use gloo_net::http::Request;
#[cfg(target_arch = "wasm32")]
use leptos::prelude::*;
#[cfg(target_arch = "wasm32")]
use stark_parts_catalog::{Catalog, parse_catalog_json5};

#[cfg(target_arch = "wasm32")]
const CATALOG_PATH: &str = "/stark-parts.json5";

/// Mount the app around the one static catalog request required at startup.
///
/// The resource owns the loading and failure states so a missing or malformed deployment asset cannot be mistaken for
/// an empty search result. Once parsing succeeds, the complete catalog moves into the browser-local search app.
#[cfg(target_arch = "wasm32")]
#[component]
fn Root() -> impl IntoView {
    let catalog = LocalResource::new(fetch_catalog);

    view! {
        <Suspense fallback=move || view! {
            <stark_parts_web::CatalogStartupState
                message="Loading parts catalog…"
                error=None
            />
        }>
            {move || Suspend::new(async move {
                match catalog.await {
                    Ok(catalog) => view! { <stark_parts_web::App catalog=catalog /> }.into_any(),
                    Err(error) => view! {
                        <stark_parts_web::CatalogStartupState
                            message="The parts catalog could not be loaded. Reload the page to try again."
                            error=Some(error)
                        />
                    }.into_any(),
                }
            })}
        </Suspense>
    }
}

/// Fetch and validate the complete committed catalog snapshot.
#[cfg(target_arch = "wasm32")]
async fn fetch_catalog() -> Result<Catalog, String> {
    let response = Request::get(CATALOG_PATH)
        .send()
        .await
        .map_err(|error| format!("catalog request failed: {error}"))?;
    if !response.ok() {
        return Err(format!(
            "catalog request returned HTTP {}",
            response.status()
        ));
    }
    let body = response
        .text()
        .await
        .map_err(|error| format!("catalog response could not be read: {error}"))?;
    parse_catalog_json5(&body).map_err(|error| format!("catalog response was invalid: {error}"))
}

#[cfg(target_arch = "wasm32")]
fn main() {
    console_error_panic_hook::set_once();
    leptos::mount::mount_to_body(Root);
}

#[cfg(not(target_arch = "wasm32"))]
fn main() {}

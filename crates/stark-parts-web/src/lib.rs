pub mod search;

use leptos::prelude::*;

const APP_TITLE: &str = "Stark Parts";
const UNOFFICIAL_NOTICE: &str = "Unofficial catalog helper. Not endorsed by Stark. May contain errors. Stark's website remains the authoritative source.";

/// Minimal Leptos shell for the future static catalog search app.
///
/// The component only establishes the application frame for now. Search,
/// catalog loading, and result rendering belong to later plan steps after the
/// committed catalog schema and search model exist.
#[component]
pub fn App() -> impl IntoView {
    view! {
        <main>
            <header>
                <p>{UNOFFICIAL_NOTICE}</p>
                <h1>{APP_TITLE}</h1>
                <label for="search">"Search"</label>
                <input id="search" type="search" disabled=true placeholder="Catalog search lands in a later plan step" />
                <p role="status">"Catalog data and search behavior are not implemented yet."</p>
            </header>
        </main>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constants_identify_unofficial_app() {
        assert_eq!(APP_TITLE, "Stark Parts");
        assert!(UNOFFICIAL_NOTICE.contains("Unofficial"));
        assert!(UNOFFICIAL_NOTICE.contains("Not endorsed by Stark"));
        assert!(UNOFFICIAL_NOTICE.contains("May contain errors"));
        assert!(UNOFFICIAL_NOTICE.contains("authoritative"));
    }

    #[test]
    fn app_component_renders_unofficial_search_shell() {
        let html = App().to_html();

        assert!(html.contains(APP_TITLE));
        assert!(html.contains("Not endorsed by Stark"));
        assert!(html.contains("May contain errors"));
        assert!(html.contains("type=\"search\""));
    }
}

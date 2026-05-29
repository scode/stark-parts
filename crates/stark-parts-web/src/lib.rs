pub mod search;

use leptos::prelude::*;
use search::{
    ArticleVariantSummary, BikeVariantSummary, ProjectedCatalogTree, ProjectedCategory,
    ProjectedProductGroup, SearchIndex, SearchRequest, SearchResultRow, SearchResults,
};
use stark_parts_catalog::{Catalog, CatalogMetadata, parse_catalog_json5};
use std::sync::Arc;

const APP_TITLE: &str = "Stark Parts";
const UNOFFICIAL_NOTICE: &str = "Unofficial catalog helper. Not endorsed by Stark. May contain errors. Stark's website remains the authoritative source.";
const CATALOG_JSON5: &str = include_str!("../../../catalog/stark-parts.json5");
const DETAIL_RENDER_LIMIT: usize = 50;
// Tree virtualization depends on fixed-height rows. Keep this in sync with `.tree-node`.
const TREE_ROW_HEIGHT_PX: usize = 36;
const TREE_VIEWPORT_HEIGHT_PX: usize = 512;
const TREE_OVERSCAN_ROWS: usize = 8;

/// Static Leptos app for searching the committed Stark catalog.
#[component]
pub fn App() -> impl IntoView {
    view! { <AppWithInitialState initial_request=initial_search_request() /> }
}

#[component]
fn AppWithInitialState(initial_request: SearchRequest) -> impl IntoView {
    let catalog = load_catalog();
    let metadata = catalog.metadata.clone();
    let index = Arc::new(SearchIndex::from_catalog(&catalog));
    let (query, set_query) = signal(initial_request.query);
    let (selected_bikes, set_selected_bikes) = signal(initial_request.selected_bike_variant_ids);
    let search_index = Arc::clone(&index);
    #[cfg(target_arch = "wasm32")]
    let current_results = move || {
        search_index.search(&SearchRequest {
            query: query.get(),
            selected_bike_variant_ids: selected_bikes.get(),
        })
    };
    #[cfg(not(target_arch = "wasm32"))]
    let current_results = move || {
        search_index.search(&SearchRequest {
            query: query.get_untracked(),
            selected_bike_variant_ids: selected_bikes.get_untracked(),
        })
    };

    #[cfg(target_arch = "wasm32")]
    {
        Effect::new(move |_| {
            write_current_url_state(&SearchRequest {
                query: query.get(),
                selected_bike_variant_ids: selected_bikes.get(),
            });
        });
    }

    view! {
        <main class="app-shell">
            <style>{APP_CSS}</style>
            <section class="notice" aria-label="Unofficial catalog notice">
                <p>{UNOFFICIAL_NOTICE}</p>
            </section>
            <header class="toolbar">
                <div>
                    <h1>{APP_TITLE}</h1>
                    <CatalogMetadataView metadata=metadata />
                </div>
                <label class="search-control" for="catalog-search">
                    <span>"Search"</span>
                    <input
                        id="catalog-search"
                        type="search"
                        autocomplete="off"
                        placeholder="part, SKU, assembly, subsystem"
                        prop:value=move || query.get()
                        on:input=move |event| set_query.set(event_target_value(&event))
                    />
                </label>
            </header>
            <section class="layout">
                <aside class="filters" aria-label="Bike filters">
                    <h2>"Bikes"</h2>
                    <BikeFilters
                        variants=index.bike_variants().to_vec()
                        selected_bikes=selected_bikes
                        set_selected_bikes=set_selected_bikes
                    />
                </aside>
                <section class="results" aria-live="polite">
                    {move || {
                        let results = current_results();
                        view! {
                            <ResultSummary results=results.clone() />
                            <CatalogTreeView trees=results.trees.clone() />
                            <ResultDetails rows=results.rows.clone() />
                        }
                    }}
                </section>
            </section>
        </main>
    }
}

#[component]
fn CatalogMetadataView(metadata: CatalogMetadata) -> impl IntoView {
    view! {
        <dl class="metadata" aria-label="Catalog metadata">
            <div>
                <dt>"Generated"</dt>
                <dd>{metadata.generated_at}</dd>
            </div>
            <div>
                <dt>"Source"</dt>
                <dd>{metadata.source.country} " storefront, " {metadata.source.language}</dd>
            </div>
            <div>
                <dt>"API"</dt>
                <dd>{metadata.source.api_base_url}</dd>
            </div>
        </dl>
    }
}

#[component]
fn BikeFilters(
    variants: Vec<BikeVariantSummary>,
    selected_bikes: ReadSignal<Vec<String>>,
    set_selected_bikes: WriteSignal<Vec<String>>,
) -> impl IntoView {
    view! {
        <div class="bike-options">
            <For
                each=move || variants.clone()
                key=|variant| variant.id.clone()
                let:variant
            >
                {
                    let id = variant.id.clone();
                    let checked_id = id.clone();
                    let label = variant.display_name.clone().unwrap_or_else(|| variant.code.clone());
                    view! {
                        <label class="bike-option">
                            <input
                                type="checkbox"
                                value=id.clone()
                                prop:checked=move || selected_bikes.get().contains(&checked_id)
                                on:change=move |event| {
                                    let checked = event_target_checked(&event);
                                    let id = id.clone();
                                    set_selected_bikes.update(move |selected| {
                                        if checked {
                                            if !selected.contains(&id) {
                                                selected.push(id);
                                            }
                                        } else {
                                            selected.retain(|selected_id| selected_id != &id);
                                        }
                                    });
                                }
                            />
                            <span>{label}</span>
                        </label>
                    }
                }
            </For>
        </div>
    }
}

#[component]
fn ResultSummary(results: SearchResults) -> impl IntoView {
    let text = if results.has_matches() {
        if results.is_empty_query {
            format!("{} catalog entries", results.rows.len())
        } else {
            format!("{} matches", results.rows.len())
        }
    } else {
        "No matching catalog entries".to_owned()
    };

    view! {
        <div class="result-summary" role="status">
            <strong>{text}</strong>
        </div>
    }
}

#[component]
fn CatalogTreeView(trees: Vec<ProjectedCatalogTree>) -> impl IntoView {
    if trees.is_empty() {
        return view! { <p class="empty-state">"No matching catalog entries."</p> }.into_any();
    }

    let nodes = Arc::new(flatten_trees(&trees));
    let total_nodes = nodes.len();
    let (scroll_top, set_scroll_top) = signal(0usize);
    view! {
        <ol
            class="catalog-tree"
            aria-label="Catalog tree"
            style=format!("max-height: {TREE_VIEWPORT_HEIGHT_PX}px")
            on:scroll=move |event| set_scroll_top.set(scroll_top_from_event(&event))
        >
            {move || {
                let window = virtual_tree_window(total_nodes, scroll_top.get());
                let visible_nodes = Arc::clone(&nodes);
                view! {
                    {(window.before_px > 0).then(|| tree_spacer_view(window.before_px))}
                    {visible_nodes[window.start..window.end].iter().cloned().map(tree_node_view).collect_view()}
                    {(window.after_px > 0).then(|| tree_spacer_view(window.after_px))}
                }
            }}
        </ol>
    }
    .into_any()
}

fn tree_node_view(node: FlatTreeNode) -> impl IntoView {
    view! {
        <li
            class=format!("tree-node tree-node-{}", node.kind)
            style=format!("--depth: {}", node.depth)
        >
            <span class="tree-label">{node.label}</span>
            {node.meta.map(|meta| view! { <span class="tree-meta">{meta}</span> })}
        </li>
    }
}

fn tree_spacer_view(height_px: usize) -> impl IntoView {
    view! {
        <li
            class="tree-spacer"
            aria-hidden="true"
            style=format!("height: {height_px}px")
        ></li>
    }
}

fn virtual_tree_window(total_nodes: usize, scroll_top_px: usize) -> VirtualTreeWindow {
    if total_nodes == 0 {
        return VirtualTreeWindow {
            start: 0,
            end: 0,
            before_px: 0,
            after_px: 0,
        };
    }

    let first_visible_row = (scroll_top_px / TREE_ROW_HEIGHT_PX).min(total_nodes - 1);
    let visible_rows = TREE_VIEWPORT_HEIGHT_PX.div_ceil(TREE_ROW_HEIGHT_PX);
    let start = first_visible_row.saturating_sub(TREE_OVERSCAN_ROWS);
    let end = (first_visible_row + visible_rows + TREE_OVERSCAN_ROWS).min(total_nodes);

    VirtualTreeWindow {
        start,
        end,
        before_px: start * TREE_ROW_HEIGHT_PX,
        after_px: (total_nodes - end) * TREE_ROW_HEIGHT_PX,
    }
}

#[cfg(target_arch = "wasm32")]
fn scroll_top_from_event(event: &leptos::ev::Event) -> usize {
    use leptos::wasm_bindgen::JsCast;

    event
        .target()
        .and_then(|target| target.dyn_into::<leptos::web_sys::HtmlElement>().ok())
        .map(|element| element.scroll_top().max(0) as usize)
        .unwrap_or_default()
}

#[cfg(not(target_arch = "wasm32"))]
fn scroll_top_from_event(_event: &leptos::ev::Event) -> usize {
    0
}

#[derive(Debug, Eq, PartialEq)]
struct VirtualTreeWindow {
    start: usize,
    end: usize,
    before_px: usize,
    after_px: usize,
}

#[component]
fn ResultDetails(rows: Vec<SearchResultRow>) -> impl IntoView {
    let total_rows = rows.len();
    view! {
        <section class="details" aria-label="Result details">
            <h2>"Details"</h2>
            {(total_rows > DETAIL_RENDER_LIMIT).then(|| view! {
                <p class="detail-limit" role="status">
                    "Showing the first " {DETAIL_RENDER_LIMIT} " of " {total_rows} " details. Narrow the search to inspect the rest."
                </p>
            })}
            <div class="detail-grid">
                {rows.into_iter().take(DETAIL_RENDER_LIMIT).map(result_card).collect_view()}
            </div>
        </section>
    }
}

fn result_card(row: SearchResultRow) -> impl IntoView {
    let title = row
        .article
        .display_name
        .clone()
        .unwrap_or_else(|| row.article.code.clone());
    let group_name = row
        .product_group
        .display_name
        .clone()
        .unwrap_or_else(|| row.product_group.code.clone());
    let category_path = if row.category_display_path.is_empty() {
        row.category_path.join(" / ")
    } else {
        row.category_display_path.join(" / ")
    };
    let variant = row.variant.clone();
    let sku = variant.as_ref().and_then(|variant| variant.sku.clone());
    let price = variant
        .as_ref()
        .and_then(|variant| variant.price.as_ref())
        .map(format_price);
    let availability = variant
        .as_ref()
        .and_then(|variant| variant.availability.as_ref())
        .map(|availability| availability.status.clone());
    let link = first_stark_link(&row);
    let image = first_image_url(&row);

    view! {
        <article class="result-card">
            <div>
                <h3>{title}</h3>
                <p class="muted">{row.bike_display_name.clone().unwrap_or(row.bike_variant_id.clone())} " / " {group_name}</p>
            </div>
            <dl class="detail-list">
                <DetailItem label="Code" value=row.article.code.clone() />
                <DetailItem label="Category path" value=category_path />
                {sku.map(|value| view! { <DetailItem label="SKU" value=value /> })}
                {variant.as_ref().map(|variant| view! { <DetailItem label="Variant" value=variant.code.clone() /> })}
                {price.map(|value| view! { <DetailItem label="Price" value=value /> })}
                {availability.map(|value| view! { <DetailItem label="Availability" value=value /> })}
                {(!row.article.kit_memberships.is_empty()).then(|| view! {
                    <DetailItem label="Kit membership" value=row.article.kit_memberships.join(", ") />
                })}
                {(!row.article.kit_contents.is_empty()).then(|| view! {
                    <DetailItem label="Kit contents" value=row.article.kit_contents.join(", ") />
                })}
            </dl>
            <p class="stale-warning">"Price and availability are from the committed catalog snapshot."</p>
            {variant_attributes(variant.clone())}
            {image.map(|url| view! {
                <img class="part-image" src=url alt="" loading="lazy" referrerpolicy="no-referrer" />
            })}
            {link.map(|url| view! {
                <a class="stark-link" href=url target="_blank" rel="noopener noreferrer">"View on Stark"</a>
            })}
        </article>
    }
}

#[component]
fn DetailItem(label: &'static str, value: String) -> impl IntoView {
    view! {
        <div>
            <dt>{label}</dt>
            <dd>{value}</dd>
        </div>
    }
}

fn variant_attributes(variant: Option<ArticleVariantSummary>) -> impl IntoView {
    let attributes = variant
        .map(|variant| variant.attributes)
        .unwrap_or_default();
    if attributes.is_empty() {
        return ().into_any();
    }

    view! {
        <ul class="attributes" aria-label="Variant attributes">
            {attributes.into_iter().map(|attribute| {
                let option = attribute
                    .option_display_name
                    .unwrap_or_else(|| attribute.option_code.clone());
                view! { <li>{attribute.code} ": " {option}</li> }
            }).collect_view()}
        </ul>
    }
    .into_any()
}

fn load_catalog() -> Catalog {
    parse_catalog_json5(CATALOG_JSON5).expect("committed catalog must parse")
}

fn first_stark_link(row: &SearchResultRow) -> Option<String> {
    row.variant
        .as_ref()
        .and_then(|variant| variant.stark_url.clone())
        .or_else(|| row.article.stark_url.clone())
        .or_else(|| row.product_group.stark_url.clone())
        .or_else(|| bike_spare_parts_url(&row.bike_variant_id))
}

fn first_image_url(row: &SearchResultRow) -> Option<String> {
    row.variant
        .as_ref()
        .and_then(|variant| variant.image_urls.first().cloned())
        .or_else(|| row.article.image_urls.first().cloned())
        .or_else(|| row.product_group.image_urls.first().cloned())
}

fn bike_spare_parts_url(bike_variant_id: &str) -> Option<String> {
    bike_variant_id
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
        .then(|| {
            format!("https://starkfuture.com/parts-and-accessories/spare-parts/{bike_variant_id}")
        })
}

fn format_price(price: &stark_parts_catalog::Price) -> String {
    format!(
        "{} {:.2}",
        price.currency,
        price.amount_minor as f64 / 100.0
    )
}

fn flatten_trees(trees: &[ProjectedCatalogTree]) -> Vec<FlatTreeNode> {
    let mut nodes = Vec::new();
    for tree in trees {
        nodes.push(FlatTreeNode {
            depth: 0,
            kind: "bike",
            label: tree
                .bike_display_name
                .clone()
                .unwrap_or_else(|| tree.bike_variant_id.clone()),
            meta: Some(tree.bike_variant_id.clone()),
        });
        for category in &tree.categories {
            flatten_category(category, 1, &mut nodes);
        }
    }
    nodes
}

fn flatten_category(category: &ProjectedCategory, depth: usize, nodes: &mut Vec<FlatTreeNode>) {
    nodes.push(FlatTreeNode {
        depth,
        kind: "category",
        label: category
            .display_name
            .clone()
            .unwrap_or_else(|| category.code.clone()),
        meta: Some(category.code.clone()),
    });
    for child in &category.categories {
        flatten_category(child, depth + 1, nodes);
    }
    for group in &category.product_groups {
        flatten_group(group, depth + 1, nodes);
    }
}

fn flatten_group(group: &ProjectedProductGroup, depth: usize, nodes: &mut Vec<FlatTreeNode>) {
    nodes.push(FlatTreeNode {
        depth,
        kind: "group",
        label: group
            .display_name
            .clone()
            .unwrap_or_else(|| group.code.clone()),
        meta: Some(group.code.clone()),
    });
    for article in &group.articles {
        nodes.push(FlatTreeNode {
            depth: depth + 1,
            kind: "article",
            label: article
                .display_name
                .clone()
                .unwrap_or_else(|| article.code.clone()),
            meta: Some(article.code.clone()),
        });
        for variant in &article.variants {
            nodes.push(FlatTreeNode {
                depth: depth + 2,
                kind: "variant",
                label: variant.sku.clone().unwrap_or_else(|| variant.code.clone()),
                meta: Some(variant.code.clone()),
            });
        }
    }
}

#[derive(Clone)]
struct FlatTreeNode {
    depth: usize,
    kind: &'static str,
    label: String,
    meta: Option<String>,
}

#[cfg(target_arch = "wasm32")]
fn initial_search_request() -> SearchRequest {
    leptos::web_sys::window()
        .and_then(|window| window.location().search().ok())
        .map(|search| SearchRequest::from_query_string(&search))
        .unwrap_or_default()
}

#[cfg(not(target_arch = "wasm32"))]
fn initial_search_request() -> SearchRequest {
    SearchRequest::default()
}

#[cfg(target_arch = "wasm32")]
fn write_current_url_state(request: &SearchRequest) {
    let Some(window) = leptos::web_sys::window() else {
        return;
    };
    let Ok(location) = window.location().pathname() else {
        return;
    };
    let url = url_for_search_state(&location, request);
    if let Ok(history) = window.history() {
        let _ =
            history.replace_state_with_url(&leptos::wasm_bindgen::JsValue::NULL, "", Some(&url));
    }
}

#[cfg(any(target_arch = "wasm32", test))]
fn url_for_search_state(pathname: &str, request: &SearchRequest) -> String {
    let query = request.to_query_string();
    if query.is_empty() {
        pathname.to_owned()
    } else {
        format!("{pathname}?{query}")
    }
}

const APP_CSS: &str = r#"
:root {
  color: #172026;
  background: #f7f8f5;
  font-family: Inter, ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
}

body {
  margin: 0;
}

.app-shell {
  min-height: 100vh;
}

.notice {
  background: #ffe8a3;
  border-bottom: 1px solid #d7a820;
  color: #281d00;
  padding: 0.75rem clamp(1rem, 3vw, 2rem);
}

.notice p {
  margin: 0;
  font-weight: 650;
}

.toolbar {
  align-items: end;
  background: #fffdf7;
  border-bottom: 1px solid #dfe2d6;
  display: grid;
  gap: 1rem;
  grid-template-columns: minmax(0, 1fr) minmax(18rem, 34rem);
  padding: 1rem clamp(1rem, 3vw, 2rem);
}

h1, h2, h3, p {
  margin-top: 0;
}

h1 {
  font-size: 1.8rem;
  margin-bottom: 0.5rem;
}

h2 {
  font-size: 1rem;
}

h3 {
  font-size: 1rem;
  margin-bottom: 0.25rem;
}

.metadata {
  display: flex;
  flex-wrap: wrap;
  gap: 0.75rem 1.25rem;
  margin: 0;
}

.metadata div, .detail-list div {
  display: grid;
  gap: 0.1rem;
}

dt {
  color: #68736c;
  font-size: 0.75rem;
  font-weight: 700;
  text-transform: uppercase;
}

dd {
  margin: 0;
}

.search-control {
  display: grid;
  gap: 0.4rem;
  font-weight: 700;
}

input[type="search"] {
  border: 1px solid #a9b2aa;
  border-radius: 6px;
  font: inherit;
  min-height: 2.75rem;
  padding: 0.6rem 0.75rem;
}

.layout {
  display: grid;
  grid-template-columns: minmax(12rem, 16rem) minmax(0, 1fr);
  gap: 1.25rem;
  padding: 1.25rem clamp(1rem, 3vw, 2rem) 2rem;
}

.filters {
  border-right: 1px solid #dfe2d6;
  padding-right: 1rem;
}

.bike-options {
  display: grid;
  gap: 0.5rem;
}

.bike-option {
  align-items: center;
  display: flex;
  gap: 0.5rem;
}

.results {
  min-width: 0;
}

.result-summary {
  margin-bottom: 0.75rem;
}

.catalog-tree {
  background: #ffffff;
  border: 1px solid #dfe2d6;
  border-radius: 6px;
  list-style: none;
  margin: 0 0 1.25rem;
  overflow: auto;
  padding: 0.5rem 0;
}

.tree-node {
  align-items: baseline;
  border-bottom: 1px solid #eef0e8;
  box-sizing: border-box;
  display: flex;
  gap: 0.6rem;
  height: 36px;
  overflow: hidden;
  padding: 0.35rem 0.75rem 0.35rem calc(0.75rem + var(--depth) * 1.1rem);
}

.tree-node:last-child {
  border-bottom: 0;
}

.tree-spacer {
  display: block;
  pointer-events: none;
}

.tree-node-bike .tree-label {
  font-weight: 800;
}

.tree-node .tree-label, .tree-node .tree-meta {
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.tree-node-article .tree-label, .tree-node-variant .tree-label {
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
}

.tree-meta, .muted {
  color: #68736c;
  font-size: 0.85rem;
}

.detail-grid {
  display: grid;
  gap: 0.9rem;
  grid-template-columns: repeat(auto-fit, minmax(18rem, 1fr));
}

.detail-limit {
  color: #4e5b53;
  margin-bottom: 0.75rem;
}

.result-card {
  background: #ffffff;
  border: 1px solid #dfe2d6;
  border-radius: 6px;
  display: grid;
  gap: 0.75rem;
  padding: 0.9rem;
}

.detail-list {
  display: grid;
  gap: 0.45rem;
  margin: 0;
}

.stale-warning {
  color: #725400;
  font-size: 0.9rem;
  margin-bottom: 0;
}

.attributes {
  margin: 0;
  padding-left: 1.1rem;
}

.part-image {
  border: 1px solid #dfe2d6;
  border-radius: 4px;
  max-height: 12rem;
  max-width: 100%;
  object-fit: contain;
}

.stark-link {
  color: #005f73;
  font-weight: 700;
}

.empty-state {
  background: #ffffff;
  border: 1px solid #dfe2d6;
  border-radius: 6px;
  padding: 1rem;
}

@media (max-width: 760px) {
  .toolbar, .layout {
    grid-template-columns: 1fr;
  }

  .filters {
    border-right: 0;
    border-bottom: 1px solid #dfe2d6;
    padding-bottom: 1rem;
    padding-right: 0;
  }
}
"#;

#[cfg(test)]
mod tests {
    use super::*;
    use leptos::prelude::Owner;

    #[test]
    fn app_component_renders_search_experience() {
        let html = Owner::new().with(|| {
            AppWithInitialState(AppWithInitialStateProps {
                initial_request: SearchRequest::default(),
            })
            .to_html()
        });

        assert!(html.contains(APP_TITLE));
        assert!(html.contains("Not endorsed by Stark"));
        assert!(html.contains("type=\"search\""));
        assert!(html.contains("Catalog metadata"));
        assert!(html.contains("Bike filters"));
        assert!(html.contains("Catalog tree"));
        assert!(html.contains("Details"));
        assert!(html.contains("SMX1-TOOLBOX"));
        assert!(!html.contains("disabled"));
    }

    #[test]
    fn app_restores_query_state_into_initial_results() {
        let html = Owner::new().with(|| {
            AppWithInitialState(AppWithInitialStateProps {
                initial_request: SearchRequest {
                    query: "SMX1-TOOLBOX".to_owned(),
                    selected_bike_variant_ids: vec!["varg-sm".to_owned()],
                },
            })
            .to_html()
        });

        assert!(html.contains("SMX1-TOOLBOX"));
        assert!(html.contains("matches"));
        assert!(html.contains("VARG SM"));
    }

    #[test]
    fn app_uses_safe_static_catalog_data_path() {
        let source = include_str!("lib.rs");

        assert!(source.contains("include_str!(\"../../../catalog/stark-parts.json5\")"));
        assert!(!source.contains(concat!(".", "fetch")));
        assert!(!source.contains(concat!("req", "west")));
    }

    #[test]
    fn web_app_source_has_no_runtime_catalog_network_client() {
        let lib_source = include_str!("lib.rs");
        let main_source = include_str!("main.rs");
        let manifest = include_str!("../Cargo.toml");
        let app_source = lib_source
            .split("#[cfg(test)]")
            .next()
            .expect("library source should contain app code before tests");

        assert!(app_source.contains("include_str!(\"../../../catalog/stark-parts.json5\")"));
        for source in [app_source, main_source, manifest] {
            assert!(!source.contains(concat!("gloo_", "net")));
            assert!(!source.contains(concat!("req", "west")));
            assert!(!source.contains(concat!("web_sys::", "Request")));
            assert!(!source.contains(concat!(".", "fetch")));
        }
    }

    #[test]
    fn static_entrypoint_builds_the_web_binary() {
        let index_html = include_str!("../../../index.html");
        let trunk_config = include_str!("../../../Trunk.toml");
        let web_main = include_str!("main.rs");

        assert!(index_html.contains("data-trunk"));
        assert!(index_html.contains("crates/stark-parts-web/Cargo.toml"));
        assert!(index_html.contains("data-bin=\"stark-parts-web\""));
        assert!(trunk_config.contains("target = \"index.html\""));
        assert!(web_main.contains("mount_to_body(stark_parts_web::App)"));
    }

    #[test]
    fn result_details_render_fallbacks_and_stale_warning() {
        let catalog = load_catalog();
        let index = SearchIndex::from_catalog(&catalog);
        let results = index.search(&SearchRequest {
            query: "SMX1-TOOLBOX".to_owned(),
            selected_bike_variant_ids: Vec::new(),
        });
        let html = result_card(results.rows[0].clone()).to_html();

        assert!(html.contains("Price and availability are from the committed catalog snapshot"));
        assert!(html.contains("SKU"));
        assert!(html.contains("Category path"));
        assert!(html.contains("Accessories"));
        assert!(html.contains("loading=\"lazy\""));
        assert!(html.contains("referrerpolicy=\"no-referrer\""));
        assert!(html.contains("https://starkfuture.com/parts-and-accessories/spare-parts/"));
        assert!(html.contains("View on Stark"));
    }

    #[test]
    fn stark_link_fallback_rejects_unsafe_bike_ids() {
        assert_eq!(
            bike_spare_parts_url("varg-sm").as_deref(),
            Some("https://starkfuture.com/parts-and-accessories/spare-parts/varg-sm")
        );
        assert_eq!(bike_spare_parts_url("javascript:alert(1)"), None);
        assert_eq!(bike_spare_parts_url("../varg-sm"), None);
    }

    #[test]
    fn no_result_state_renders_clear_empty_message() {
        let html = Owner::new().with(|| {
            AppWithInitialState(AppWithInitialStateProps {
                initial_request: SearchRequest {
                    query: "definitely-not-a-real-part".to_owned(),
                    selected_bike_variant_ids: Vec::new(),
                },
            })
            .to_html()
        });

        assert!(html.contains("No matching catalog entries"));
        assert!(!html.contains("SMX1-TOOLBOX"));
    }

    #[test]
    fn bike_filter_state_changes_visible_results() {
        let sm_html = Owner::new().with(|| {
            AppWithInitialState(AppWithInitialStateProps {
                initial_request: SearchRequest {
                    query: "SSM1-P-FF-01-G".to_owned(),
                    selected_bike_variant_ids: vec!["varg-sm".to_owned()],
                },
            })
            .to_html()
        });
        let ex_html = Owner::new().with(|| {
            AppWithInitialState(AppWithInitialStateProps {
                initial_request: SearchRequest {
                    query: "SSM1-P-FF-01-G".to_owned(),
                    selected_bike_variant_ids: vec!["varg-ex".to_owned()],
                },
            })
            .to_html()
        });

        assert!(sm_html.contains("matches"));
        assert!(!sm_html.contains("No matching catalog entries"));
        assert!(ex_html.contains("No matching catalog entries"));
    }

    #[test]
    fn url_state_is_encoded_for_sharing() {
        let request = SearchRequest {
            query: "front disc".to_owned(),
            selected_bike_variant_ids: vec!["varg-ex".to_owned(), "varg-sm".to_owned()],
        };

        assert_eq!(
            url_for_search_state("/parts", &request),
            "/parts?q=front+disc&bike=varg-ex&bike=varg-sm"
        );
        assert_eq!(
            url_for_search_state("/parts", &SearchRequest::default()),
            "/parts"
        );
    }

    #[test]
    fn result_details_render_all_rows_under_the_limit() {
        let catalog = load_catalog();
        let index = SearchIndex::from_catalog(&catalog);
        let results = index.search(&SearchRequest {
            query: "SMX1-TOOLBOX".to_owned(),
            selected_bike_variant_ids: Vec::new(),
        });
        let expected_cards = results.rows.len();
        let html = ResultDetails(ResultDetailsProps { rows: results.rows }).to_html();

        assert!(expected_cards > 0);
        assert!(expected_cards < DETAIL_RENDER_LIMIT);
        assert_eq!(
            html.matches("class=\"result-card\"").count(),
            expected_cards
        );
        assert!(!html.contains("Narrow the search to inspect the rest"));
    }

    #[test]
    fn result_details_cap_broad_rows_with_narrow_prompt() {
        let catalog = load_catalog();
        let index = SearchIndex::from_catalog(&catalog);
        let results = index.search(&SearchRequest {
            query: "SMX1".to_owned(),
            selected_bike_variant_ids: Vec::new(),
        });
        let expected_cards = results.rows.len();
        let html = ResultDetails(ResultDetailsProps { rows: results.rows }).to_html();

        assert!(expected_cards > DETAIL_RENDER_LIMIT);
        assert_eq!(
            html.matches("class=\"result-card\"").count(),
            DETAIL_RENDER_LIMIT
        );
        assert!(html.contains("Showing the first"));
        assert!(html.contains(&DETAIL_RENDER_LIMIT.to_string()));
        assert!(html.contains(&expected_cards.to_string()));
        assert!(html.contains("details"));
        assert!(html.contains("Narrow the search to inspect the rest"));
    }

    #[test]
    fn catalog_tree_mounts_only_the_initial_virtual_window() {
        let catalog = load_catalog();
        let index = SearchIndex::from_catalog(&catalog);
        let results = index.search(&SearchRequest {
            query: "S".to_owned(),
            selected_bike_variant_ids: Vec::new(),
        });
        let total_nodes = flatten_trees(&results.trees).len();
        let html = CatalogTreeView(CatalogTreeViewProps {
            trees: results.trees,
        })
        .to_html();

        assert!(total_nodes > TREE_VIEWPORT_HEIGHT_PX / TREE_ROW_HEIGHT_PX);
        assert!(
            html.matches("class=\"tree-node ").count() < total_nodes,
            "virtualized tree should not mount every flattened node"
        );
        assert!(html.contains("class=\"tree-spacer\""));
    }

    #[test]
    fn virtual_tree_window_overscans_around_the_scroll_position() {
        let window = virtual_tree_window(100, TREE_ROW_HEIGHT_PX * 40);

        assert_eq!(window.start, 40 - TREE_OVERSCAN_ROWS);
        assert_eq!(
            window.end,
            40 + TREE_VIEWPORT_HEIGHT_PX.div_ceil(TREE_ROW_HEIGHT_PX) + TREE_OVERSCAN_ROWS
        );
        assert_eq!(window.before_px, window.start * TREE_ROW_HEIGHT_PX);
        assert_eq!(window.after_px, (100 - window.end) * TREE_ROW_HEIGHT_PX);
    }

    #[test]
    fn virtual_tree_window_clamps_stale_scroll_positions() {
        let window = virtual_tree_window(10, TREE_ROW_HEIGHT_PX * 1_000);

        assert!(window.start <= window.end);
        assert!(window.end <= 10);
        assert_eq!(window.after_px, 0);
    }

    #[test]
    fn result_details_render_all_rows_at_the_limit() {
        let catalog = load_catalog();
        let index = SearchIndex::from_catalog(&catalog);
        let results = index.search(&SearchRequest {
            query: "SMX1".to_owned(),
            selected_bike_variant_ids: Vec::new(),
        });
        let rows = results
            .rows
            .into_iter()
            .take(DETAIL_RENDER_LIMIT)
            .collect::<Vec<_>>();
        let html = ResultDetails(ResultDetailsProps { rows }).to_html();

        assert_eq!(
            html.matches("class=\"result-card\"").count(),
            DETAIL_RENDER_LIMIT
        );
        assert!(!html.contains("Narrow the search to inspect the rest"));
    }
}

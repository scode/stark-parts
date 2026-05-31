#![recursion_limit = "256"]

pub mod search;

use leptos::prelude::*;
use search::{
    ArticleVariantSummary, BikeVariantSummary, SearchIndex, SearchRequest, SearchResultRow,
    SearchResults,
};
use stark_parts_catalog::{Catalog, CatalogMetadata, parse_catalog_json5};
use std::sync::Arc;

const APP_TITLE: &str = "Stark Parts";
const UNOFFICIAL_NOTICE: &str = "Unofficial catalog helper. Not endorsed by Stark. May contain errors. Stark's website remains the authoritative source.";
const CATALOG_JSON5: &str = include_str!("../../../catalog/stark-parts.json5");
// Result virtualization depends on fixed-height rows. Keep this in sync with `.result-row`.
const RESULT_ROW_HEIGHT_PX: usize = 88;
const RESULT_VIEWPORT_HEIGHT_PX: usize = 512;
const RESULT_OVERSCAN_ROWS: usize = 8;

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
    let search_input = NodeRef::<leptos::html::Input>::new();
    let results = Memo::new(move |_| {
        search_index.search(&SearchRequest {
            query: query.get(),
            selected_bike_variant_ids: selected_bikes.get(),
        })
    });
    let (selected_detail, set_selected_detail) = signal(None::<Arc<SearchResultRow>>);

    #[cfg(target_arch = "wasm32")]
    {
        Effect::new(move |_| {
            write_current_url_state(&SearchRequest {
                query: query.get(),
                selected_bike_variant_ids: selected_bikes.get(),
            });
        });
    }

    #[cfg(target_arch = "wasm32")]
    search_input.on_load(|input| {
        // Browsers may refuse programmatic focus for page-level reasons; the input remains usable either way.
        let _ = input.focus();
    });

    Effect::new(move |_| {
        let results = results.get();
        if let Some(detail) = selected_detail.get()
            && !results.rows.iter().any(|row| row == detail.as_ref())
        {
            set_selected_detail.set(None);
        }
    });

    view! {
        <main class="app-shell">
            <style>{APP_CSS}</style>
            <section class="notice" aria-label="Unofficial catalog notice">
                <p>{UNOFFICIAL_NOTICE}</p>
            </section>
            <header class="toolbar">
                <div class="primary-search">
                    <h1>{APP_TITLE}</h1>
                    <label class="search-control" for="catalog-search">
                        <span>"Search"</span>
                        <input
                            id="catalog-search"
                            type="search"
                            autofocus
                            node_ref=search_input
                            autocomplete="off"
                            placeholder="part, SKU, assembly, subsystem"
                            prop:value=move || query.get()
                            on:input=move |event| set_query.set(event_target_value(&event))
                        />
                    </label>
                    <BikeFilters
                        variants=index.bike_variants().to_vec()
                        selected_bikes=selected_bikes
                        set_selected_bikes=set_selected_bikes
                    />
                </div>
                <CatalogMetadataView metadata=metadata />
            </header>
            <section class="layout">
                <section class="results" aria-live="polite">
                    {move || {
                        let results = results.get();
                        view! {
                            <ResultSummary results=results.clone() />
                            <SearchResultList
                                rows=results.rows.clone()
                                selected_detail=selected_detail
                                set_selected_detail=set_selected_detail
                            />
                        }
                    }}
                </section>
            </section>
        </main>
    }
}

#[component]
fn CatalogMetadataView(metadata: CatalogMetadata) -> impl IntoView {
    let last_updated = catalog_last_updated_date(&metadata.generated_at);
    view! {
        <dl class="metadata" aria-label="Catalog metadata">
            <div>
                <dt>"Parts data last updated"</dt>
                <dd>{last_updated}</dd>
            </div>
            <div>
                <dt>"Source"</dt>
                <dd>{metadata.source.country} " storefront, " {metadata.source.language}</dd>
            </div>
        </dl>
    }
}

fn catalog_last_updated_date(generated_at: &str) -> String {
    generated_at
        .split_once('T')
        .map(|(date, _)| date)
        .unwrap_or(generated_at)
        .to_owned()
}

#[component]
fn BikeFilters(
    variants: Vec<BikeVariantSummary>,
    selected_bikes: ReadSignal<Vec<String>>,
    set_selected_bikes: WriteSignal<Vec<String>>,
) -> impl IntoView {
    view! {
        <section class="bike-filter-bar" aria-label="Bike filters">
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
            {move || selected_bikes.get().is_empty().then(|| view! {
                <p class="bike-filter-default">"default: all bikes"</p>
            })}
        </section>
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
fn SearchResultList(
    rows: Vec<SearchResultRow>,
    selected_detail: ReadSignal<Option<Arc<SearchResultRow>>>,
    set_selected_detail: WriteSignal<Option<Arc<SearchResultRow>>>,
) -> impl IntoView {
    if rows.is_empty() {
        return view! { <p class="empty-state">"No matching catalog entries."</p> }.into_any();
    }

    let rows = Arc::new(rows.into_iter().map(Arc::new).collect::<Vec<_>>());
    let total_nodes = rows.len();
    let (scroll_top, set_scroll_top) = signal(0usize);
    view! {
        <div class="result-list-with-detail">
            <ol
                class="result-list"
                aria-label="Result list"
                style=format!("max-height: {RESULT_VIEWPORT_HEIGHT_PX}px")
                on:scroll=move |event| set_scroll_top.set(scroll_top_from_event(&event))
            >
                {move || {
                    let window = virtual_result_window(total_nodes, scroll_top.get());
                    let visible_rows = Arc::clone(&rows);
                    view! {
                        {(window.before_px > 0).then(|| result_spacer_view(window.before_px))}
                        {visible_rows[window.start..window.end].iter().cloned().map(|row| {
                            result_row_view(row, selected_detail, set_selected_detail)
                        }).collect_view()}
                        {(window.after_px > 0).then(|| result_spacer_view(window.after_px))}
                    }
                }}
            </ol>
            <aside class="result-detail-popover" aria-label="Hovered part detail">
                {move || selected_detail.get().map(|row| result_card((*row).clone()))}
            </aside>
        </div>
    }
    .into_any()
}

fn result_row_view(
    row: Arc<SearchResultRow>,
    selected_detail: ReadSignal<Option<Arc<SearchResultRow>>>,
    set_selected_detail: WriteSignal<Option<Arc<SearchResultRow>>>,
) -> impl IntoView {
    let label = row
        .article
        .display_name
        .clone()
        .unwrap_or_else(|| row.article.code.clone());
    let meta = row.variant.as_ref().and_then(|variant| variant.sku.clone());
    let image_url = first_image_url(&row);
    let detail = Some(Arc::clone(&row));
    let active_row = Arc::clone(&row);
    view! {
        <li
            class=move || {
                if selected_detail.get().as_deref() == Some(active_row.as_ref()) {
                    "result-row result-list-row result-row-active"
                } else {
                    "result-row result-list-row"
                }
            }
            style="--depth: 0"
            on:mouseenter=move |_| set_selected_detail.set(detail.clone())
        >
            {image_url.map(|url| view! {
                <span class="result-thumb-frame" aria-hidden="true">
                    <img
                        class="result-thumb"
                        src=url
                        alt=""
                        loading="lazy"
                        referrerpolicy="no-referrer"
                        on:error=mark_image_failed
                    />
                </span>
            })}
            <span class="result-label">{label}</span>
            {meta.map(|meta| view! { <span class="result-meta">{meta}</span> })}
        </li>
    }
}

fn result_spacer_view(height_px: usize) -> impl IntoView {
    view! {
        <li
            class="result-spacer"
            aria-hidden="true"
            style=format!("height: {height_px}px")
        ></li>
    }
}

fn virtual_result_window(total_nodes: usize, scroll_top_px: usize) -> VirtualResultWindow {
    if total_nodes == 0 {
        return VirtualResultWindow {
            start: 0,
            end: 0,
            before_px: 0,
            after_px: 0,
        };
    }

    let first_visible_row = (scroll_top_px / RESULT_ROW_HEIGHT_PX).min(total_nodes - 1);
    let visible_rows = RESULT_VIEWPORT_HEIGHT_PX.div_ceil(RESULT_ROW_HEIGHT_PX);
    let start = first_visible_row.saturating_sub(RESULT_OVERSCAN_ROWS);
    let end = (first_visible_row + visible_rows + RESULT_OVERSCAN_ROWS).min(total_nodes);

    VirtualResultWindow {
        start,
        end,
        before_px: start * RESULT_ROW_HEIGHT_PX,
        after_px: (total_nodes - end) * RESULT_ROW_HEIGHT_PX,
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
struct VirtualResultWindow {
    start: usize,
    end: usize,
    before_px: usize,
    after_px: usize,
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
    let compatible_bikes = compatible_bikes_text(&row);

    view! {
        <article class="result-card">
            {image.map(|url| view! {
                <figure class="part-image-frame">
                    <img
                        class="part-image"
                        src=url
                        alt=""
                        loading="lazy"
                        referrerpolicy="no-referrer"
                        on:error=mark_image_failed
                    />
                </figure>
            })}
            <div class="result-card-heading">
                <h3>{title}</h3>
                <p class="muted">{compatible_bikes} " / " {group_name}</p>
                {link.map(|url| view! {
                    <a class="stark-link" href=url target="_blank" rel="noopener noreferrer">"View on Stark"</a>
                })}
            </div>
            <dl class="detail-list detail-list-primary">
                {sku.map(|value| view! { <DetailItem label="SKU" value=value /> })}
                {price.map(|value| view! { <DetailItem label="Price" value=value /> })}
                {availability.map(|value| view! { <DetailItem label="Availability" value=value /> })}
            </dl>
            <dl class="detail-list">
                <DetailItem label="Code" value=row.article.code.clone() />
                <DetailItem label="Category path" value=category_path />
                {variant.as_ref().map(|variant| view! { <DetailItem label="Variant" value=variant.code.clone() /> })}
                {(!row.article.kit_memberships.is_empty()).then(|| view! {
                    <DetailItem label="Kit membership" value=row.article.kit_memberships.join(", ") />
                })}
                {(!row.article.kit_contents.is_empty()).then(|| view! {
                    <DetailItem label="Kit contents" value=row.article.kit_contents.join(", ") />
                })}
            </dl>
            <p class="stale-warning">"Price and availability are from the committed catalog snapshot."</p>
            {variant_attributes(variant.clone())}
        </article>
    }
}

fn compatible_bikes_text(row: &SearchResultRow) -> String {
    if row.compatible_bikes.is_empty() {
        return row
            .bike_display_name
            .clone()
            .or_else(|| row.bike_code.clone())
            .unwrap_or_else(|| row.bike_variant_id.clone());
    }

    row.compatible_bikes
        .iter()
        .map(|bike| {
            bike.display_name
                .clone()
                .unwrap_or_else(|| bike.code.clone())
        })
        .collect::<Vec<_>>()
        .join(", ")
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
}

fn first_image_url(row: &SearchResultRow) -> Option<String> {
    row.variant
        .as_ref()
        .and_then(|variant| variant.image_urls.first().cloned())
        .or_else(|| row.article.image_urls.first().cloned())
        .or_else(|| row.product_group.image_urls.first().cloned())
}

#[cfg(target_arch = "wasm32")]
fn mark_image_failed(event: leptos::ev::ErrorEvent) {
    use leptos::wasm_bindgen::JsCast;

    let Some(target) = event.target() else {
        return;
    };
    let Ok(image) = target.dyn_into::<leptos::web_sys::HtmlElement>() else {
        return;
    };
    let Some(parent) = image.parent_element() else {
        return;
    };

    let parent_classes = parent.class_name();
    if !parent_classes
        .split_ascii_whitespace()
        .any(|class| class == "image-frame-missing")
    {
        parent.set_class_name(&format!("{parent_classes} image-frame-missing"));
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn mark_image_failed(_event: leptos::ev::ErrorEvent) {}

fn format_price(price: &stark_parts_catalog::Price) -> String {
    format!(
        "{} {:.2}",
        price.currency,
        price.amount_minor as f64 / 100.0
    )
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
  background: #ecefea;
  margin: 0;
}

.app-shell {
  background: #f7f8f5;
  box-shadow: 0 0 0 1px rgba(23, 32, 38, 0.04);
  margin: 0 auto;
  max-width: 1500px;
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
  align-items: start;
  background: #fffdf7;
  border-bottom: 1px solid #dfe2d6;
  display: grid;
  gap: 1rem;
  grid-template-columns: minmax(18rem, 34rem) minmax(0, 1fr);
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
  font-size: 1.1rem;
  margin-bottom: 0.25rem;
}

.primary-search {
  display: grid;
  gap: 0.75rem;
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
  padding: 1.25rem clamp(1rem, 3vw, 2rem) 2rem;
}

.bike-filter-bar {
  align-items: center;
  display: flex;
  flex-wrap: wrap;
  gap: 0.5rem 0.75rem;
}

.bike-options {
  display: flex;
  flex-wrap: wrap;
  gap: 0.45rem;
}

.bike-option {
  align-items: center;
  background: #ffffff;
  border: 1px solid #cfd6cc;
  border-radius: 999px;
  color: #2f3a33;
  display: flex;
  gap: 0.4rem;
  min-height: 2rem;
  padding: 0.25rem 0.7rem 0.25rem 0.55rem;
}

.bike-option:hover {
  border-color: #9cac9b;
}

.bike-option:has(input:checked) {
  background: #eef5ec;
  border-color: #3f7f57;
  color: #172026;
}

.bike-option input {
  accent-color: #3f7f57;
  margin: 0;
}

.bike-filter-default {
  color: #68736c;
  font-size: 0.85rem;
  margin: 0;
}

.results {
  min-width: 0;
}

.result-summary {
  margin-bottom: 0.75rem;
}

.result-list-with-detail {
  align-items: start;
  display: grid;
  gap: 1.25rem;
  grid-template-columns: minmax(0, 1fr) minmax(22rem, 28rem);
  margin-bottom: 1.25rem;
}

.result-list {
  background: #ffffff;
  border: 1px solid #dfe2d6;
  border-radius: 6px;
  list-style: none;
  margin: 0;
  overflow: auto;
  padding: 0.5rem 0;
}

.result-row {
  align-items: center;
  border-bottom: 1px solid #eef0e8;
  border-left: 4px solid transparent;
  box-sizing: border-box;
  cursor: default;
  display: flex;
  gap: 0.75rem;
  height: 88px;
  overflow: hidden;
  padding: 0.4rem 0.85rem 0.4rem calc(0.65rem + var(--depth) * 1.1rem);
}

.result-row:last-child {
  border-bottom: 0;
}

.result-list-row {
  cursor: pointer;
}

.result-row:hover {
  background: #f6f8f2;
}

.result-row-active {
  background: #eef5ec;
  border-left-color: #3f7f57;
}

.result-spacer {
  display: block;
  pointer-events: none;
}

.result-thumb-frame {
  align-items: center;
  background: #f3f5f0;
  border: 1px solid #dfe2d6;
  border-radius: 5px;
  box-sizing: border-box;
  color: #68736c;
  display: flex;
  flex: 0 0 72px;
  height: 72px;
  justify-content: center;
  overflow: hidden;
  position: relative;
  width: 72px;
}

.result-thumb-frame::after, .part-image-frame::after {
  content: "No image";
  display: none;
  font-size: 0.72rem;
}

.image-frame-missing::after {
  display: block;
}

.result-thumb {
  background: #f3f5f0;
  height: 100%;
  object-fit: contain;
  padding: 0.15rem;
  position: absolute;
  width: 100%;
}

.image-frame-missing .result-thumb, .image-frame-missing .part-image {
  display: none;
}

.result-row .result-label, .result-row .result-meta {
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.result-meta, .muted {
  color: #68736c;
  font-size: 0.85rem;
}

.result-detail-popover {
  align-self: start;
  max-height: calc(100vh - 2.5rem);
  min-width: 0;
  overflow: auto;
  position: sticky;
  top: 1.25rem;
}

.result-card {
  background: #ffffff;
  border: 1px solid #cfd6cc;
  border-radius: 7px;
  box-shadow: 0 12px 30px rgba(23, 32, 38, 0.08);
  display: grid;
  gap: 0.75rem;
  padding: 1rem;
}

.result-card-heading {
  border-bottom: 1px solid #eef0e8;
  display: grid;
  gap: 0.35rem;
  padding-bottom: 0.75rem;
}

.result-card-heading p {
  margin-bottom: 0;
}

.detail-list {
  display: grid;
  gap: 0.45rem;
  margin: 0;
}

.detail-list-primary {
  background: #f7f8f5;
  border: 1px solid #e3e7dd;
  border-radius: 6px;
  gap: 0.6rem;
  grid-template-columns: repeat(auto-fit, minmax(7rem, 1fr));
  padding: 0.75rem;
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

.part-image-frame {
  align-items: center;
  aspect-ratio: 4 / 3;
  background: #f3f5f0;
  border: 1px solid #dfe2d6;
  border-radius: 4px;
  box-sizing: border-box;
  color: #68736c;
  display: flex;
  justify-content: center;
  margin: 0;
  max-height: 12rem;
  overflow: hidden;
  position: relative;
}

.part-image {
  background: #f3f5f0;
  height: 100%;
  max-width: 100%;
  object-fit: contain;
  position: relative;
  width: 100%;
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
  .toolbar, .layout, .result-list-with-detail {
    grid-template-columns: 1fr;
  }

  .result-detail-popover {
    position: static;
  }
}
"#;

#[cfg(test)]
mod tests {
    use super::*;
    use leptos::prelude::Owner;

    fn minimal_result_row(
        article_display_name: Option<&str>,
        sku: Option<&str>,
    ) -> SearchResultRow {
        SearchResultRow {
            bike_variant_id: "bike".to_owned(),
            bike_code: Some("bike".to_owned()),
            bike_display_name: Some("Bike".to_owned()),
            compatible_bikes: vec![BikeVariantSummary {
                id: "bike".to_owned(),
                code: "bike".to_owned(),
                display_name: Some("Bike".to_owned()),
            }],
            category_path: vec!["category".to_owned()],
            category_display_path: vec!["Category".to_owned()],
            product_group: search::ProductGroupSummary {
                code: "group".to_owned(),
                display_name: Some("Group".to_owned()),
                description: None,
                stark_url: None,
                image_urls: Vec::new(),
            },
            article: search::ArticleSummary {
                code: "article_code".to_owned(),
                display_name: article_display_name.map(str::to_owned),
                description: None,
                stark_url: None,
                image_urls: Vec::new(),
                kit_memberships: Vec::new(),
                kit_contents: Vec::new(),
            },
            variant: Some(ArticleVariantSummary {
                code: "variant_code".to_owned(),
                sku: sku.map(str::to_owned),
                stark_url: None,
                image_urls: Vec::new(),
                attributes: Vec::new(),
                price: None,
                availability: None,
            }),
        }
    }

    fn search_result_list_html(rows: Vec<SearchResultRow>) -> String {
        Owner::new().with(|| {
            let (selected_detail, set_selected_detail) = signal(None::<Arc<SearchResultRow>>);
            SearchResultList(SearchResultListProps {
                rows,
                selected_detail,
                set_selected_detail,
            })
            .to_html()
        })
    }

    fn search_result_list_html_with_selection(
        rows: Vec<SearchResultRow>,
        selected: SearchResultRow,
    ) -> String {
        Owner::new().with(|| {
            let (selected_detail, set_selected_detail) = signal(Some(Arc::new(selected)));
            SearchResultList(SearchResultListProps {
                rows,
                selected_detail,
                set_selected_detail,
            })
            .to_html()
        })
    }

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
        assert!(html.contains("autofocus"));
        let title_position = html
            .find("<h1>Stark Parts</h1>")
            .expect("page title should render");
        let search_position = html
            .find("id=\"catalog-search\"")
            .expect("search input should render");
        let metadata_position = html
            .find("Catalog metadata")
            .expect("catalog metadata should render");
        assert!(title_position < search_position);
        assert!(search_position < metadata_position);
        assert!(html.contains("Catalog metadata"));
        assert!(!html.contains("api.starkfuture.com"));
        assert!(!html.contains("<dt>API</dt>"));
        assert!(html.contains("Bike filters"));
        assert!(html.contains("default: all bikes"));
        assert!(html.contains("Result list"));
        assert!(html.contains("Hovered part detail"));
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
        assert!(!html.contains("default: all bikes"));
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
    fn catalog_metadata_shows_last_updated_date_without_time() {
        let html = CatalogMetadataView(CatalogMetadataViewProps {
            metadata: CatalogMetadata {
                schema_version: 1,
                generated_at: "2026-05-26T12:34:56Z".to_owned(),
                source: stark_parts_catalog::SourceMetadata {
                    api_base_url: "https://api.starkfuture.com/v2".to_owned(),
                    country: "US".to_owned(),
                    language: "en".to_owned(),
                    endpoints: Vec::new(),
                },
            },
        })
        .to_html();

        assert!(html.contains("<dt>Parts data last updated</dt>"));
        assert!(html.contains("<dd>2026-05-26</dd>"));
        assert!(!html.contains("12:34:56"));
        assert!(!html.contains("<dt>Generated</dt>"));
    }

    #[test]
    fn static_entrypoint_builds_the_web_binary() {
        let index_html = include_str!("../../../index.html");
        let trunk_config = include_str!("../../../Trunk.toml");
        let web_main = include_str!("main.rs");

        assert!(index_html.contains("data-trunk"));
        assert!(index_html.contains("crates/stark-parts-web/Cargo.toml"));
        assert!(index_html.contains("data-bin=\"stark-parts-web\""));
        assert!(index_html.contains("/_vercel/insights/script.js"));
        assert!(trunk_config.contains("target = \"index.html\""));
        assert!(web_main.contains("mount_to_body(stark_parts_web::App)"));
    }

    #[test]
    fn virtualized_row_height_matches_css() {
        assert!(APP_CSS.contains(&format!("height: {RESULT_ROW_HEIGHT_PX}px;")));
    }

    #[test]
    fn result_details_render_fallbacks_and_stale_warning() {
        let catalog = load_catalog();
        let index = SearchIndex::from_catalog(&catalog);
        let results = index.search(&SearchRequest {
            query: "SMX1-TOOLBOX".to_owned(),
            selected_bike_variant_ids: vec!["varg-sm".to_owned()],
        });
        let html = result_card(results.rows[0].clone()).to_html();

        assert!(html.contains("Price and availability are from the committed catalog snapshot"));
        assert!(html.contains("SKU"));
        assert!(html.contains("Category path"));
        assert!(html.contains("Accessories"));
        assert!(html.contains("class=\"part-image-frame\""));
        assert!(html.contains("loading=\"lazy\""));
        assert!(html.contains("referrerpolicy=\"no-referrer\""));
        assert!(html.contains(
            "https://starkfuture.com/parts-and-accessories/spare-parts/varg-sm/accessories/1_toolbox"
        ));
        assert!(html.contains("View on Stark"));
    }

    #[test]
    fn result_card_renders_image_before_text_details() {
        let catalog = load_catalog();
        let index = SearchIndex::from_catalog(&catalog);
        let results = index.search(&SearchRequest {
            query: "SMX1-TOOLBOX".to_owned(),
            selected_bike_variant_ids: vec!["varg-sm".to_owned()],
        });
        let html = result_card(results.rows[0].clone()).to_html();
        let image_position = html
            .find("class=\"part-image\"")
            .expect("part image should render");
        let heading_position = html.find("<h3>").expect("heading should render");

        assert!(image_position < heading_position);
    }

    #[test]
    fn result_card_renders_stark_link_after_title_before_fields() {
        let catalog = load_catalog();
        let index = SearchIndex::from_catalog(&catalog);
        let results = index.search(&SearchRequest {
            query: "SMX1-TOOLBOX".to_owned(),
            selected_bike_variant_ids: Vec::new(),
        });
        let html = result_card(results.rows[0].clone()).to_html();
        let subtitle_position = html
            .find("class=\"muted\"")
            .expect("subtitle should render");
        let link_position = html.find("View on Stark").expect("link should render");
        let fields_position = html.find("<dl").expect("detail fields should render");

        assert!(subtitle_position < link_position);
        assert!(link_position < fields_position);
    }

    #[test]
    fn result_card_groups_primary_facts_before_catalog_metadata() {
        let catalog = load_catalog();
        let index = SearchIndex::from_catalog(&catalog);
        let results = index.search(&SearchRequest {
            query: "SMX1-TOOLBOX".to_owned(),
            selected_bike_variant_ids: Vec::new(),
        });
        let html = result_card(results.rows[0].clone()).to_html();
        let sku_position = html.find("<dt>SKU</dt>").expect("SKU should render");
        let code_position = html.find("<dt>Code</dt>").expect("code should render");

        assert!(html.contains("detail-list detail-list-primary"));
        assert!(sku_position < code_position);
    }

    #[test]
    fn search_result_list_renders_article_names_sku_and_lazy_thumbnails() {
        let catalog = load_catalog();
        let index = SearchIndex::from_catalog(&catalog);
        let results = index.search(&SearchRequest {
            query: "SMX1-TOOLBOX".to_owned(),
            selected_bike_variant_ids: vec!["varg-sm".to_owned()],
        });
        assert_eq!(results.rows.len(), 1);
        let html = search_result_list_html(results.rows);

        assert!(html.contains("<span class=\"result-label\">Stark VARG toolbox</span>"));
        assert!(html.contains("<span class=\"result-meta\">SMX1-TOOLBOX</span>"));
        assert!(!html.contains("1_toolbox_kamasa_stark_varg</span>"));
        assert!(html.contains("class=\"result-thumb-frame\""));
        assert!(html.contains("class=\"result-thumb\""));
        assert!(html.contains("loading=\"lazy\""));
        assert!(html.contains("referrerpolicy=\"no-referrer\""));
        assert!(html.contains("260327_SpareParts"));
    }

    #[test]
    fn search_result_list_falls_back_to_article_code_without_display_name() {
        let html = search_result_list_html(vec![minimal_result_row(None, Some("SKU-1"))]);

        assert!(html.contains("<span class=\"result-label\">article_code</span>"));
        assert!(html.contains("<span class=\"result-meta\">SKU-1</span>"));
    }

    #[test]
    fn search_result_list_omits_secondary_text_without_sku() {
        let html =
            search_result_list_html(vec![minimal_result_row(Some("Readable article"), None)]);

        assert!(html.contains("<span class=\"result-label\">Readable article</span>"));
        assert!(!html.contains("class=\"result-meta\""));
        assert!(!html.contains("variant_code"));
    }

    #[test]
    fn search_result_list_marks_selected_row_active() {
        let selected = minimal_result_row(Some("Readable article"), Some("SKU-1"));
        let html = search_result_list_html_with_selection(vec![selected.clone()], selected);

        assert!(html.contains("class=\"result-row result-list-row result-row-active\""));
    }

    #[test]
    fn stark_link_does_not_fall_back_to_bike_overview() {
        let catalog = load_catalog();
        let index = SearchIndex::from_catalog(&catalog);
        let results = index.search(&SearchRequest {
            query: "SMX1-TOOLBOX".to_owned(),
            selected_bike_variant_ids: Vec::new(),
        });
        let mut row = results.rows[0].clone();
        row.product_group.stark_url = None;
        row.article.stark_url = None;
        row.variant = None;

        assert_eq!(first_stark_link(&row), None);
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
    fn search_result_list_mounts_only_the_initial_virtual_window() {
        let catalog = load_catalog();
        let index = SearchIndex::from_catalog(&catalog);
        let results = index.search(&SearchRequest {
            query: "S".to_owned(),
            selected_bike_variant_ids: Vec::new(),
        });
        let total_nodes = results.rows.len();
        let html = search_result_list_html(results.rows);

        assert!(total_nodes > RESULT_VIEWPORT_HEIGHT_PX / RESULT_ROW_HEIGHT_PX);
        assert!(
            html.matches("class=\"result-row ").count() < total_nodes,
            "virtualized results should not mount every result row"
        );
        assert!(html.contains("class=\"result-spacer\""));
    }

    #[test]
    fn virtual_result_window_overscans_around_the_scroll_position() {
        let window = virtual_result_window(100, RESULT_ROW_HEIGHT_PX * 40);

        assert_eq!(window.start, 40 - RESULT_OVERSCAN_ROWS);
        assert_eq!(
            window.end,
            40 + RESULT_VIEWPORT_HEIGHT_PX.div_ceil(RESULT_ROW_HEIGHT_PX) + RESULT_OVERSCAN_ROWS
        );
        assert_eq!(window.before_px, window.start * RESULT_ROW_HEIGHT_PX);
        assert_eq!(window.after_px, (100 - window.end) * RESULT_ROW_HEIGHT_PX);
    }

    #[test]
    fn virtual_result_window_clamps_stale_scroll_positions() {
        let window = virtual_result_window(10, RESULT_ROW_HEIGHT_PX * 1_000);

        assert!(window.start <= window.end);
        assert!(window.end <= 10);
        assert_eq!(window.after_px, 0);
    }
}

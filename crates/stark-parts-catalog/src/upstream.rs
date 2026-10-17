//! Current storefront ingestion, independent of HTML and browser execution.
//!
//! The v3 endpoints are used by Stark's storefront without authentication.
//! That is an observed access pattern, not a published API stability promise.
//! Listings define the snapshot membership; details enrich those products but
//! never resurrect entries from an earlier committed catalog.

use super::*;
use serde::de::DeserializeOwned;
use std::collections::{HashMap, HashSet};

const PAGE_SIZE: &str = "100";
const MAX_PAGES: usize = 10_000;

/// Snapshot boundary used by the CLI; tests supply complete canned snapshots.
pub trait UpstreamCatalog {
    fn fetch_catalog(&self, config: &CrawlConfig) -> UpstreamResult<Catalog>;
}

/// Validate a complete source snapshot before the CLI can replace committed bytes.
pub fn crawl_catalog(
    client: &impl UpstreamCatalog,
    config: &CrawlConfig,
) -> Result<Catalog, CrawlError> {
    let catalog = client.fetch_catalog(config)?;
    validate_catalog(&catalog)?;
    Ok(catalog)
}

/// Adapter for the storefront's category, listing, detail, and kit endpoints.
pub struct StarkHttpClient<T> {
    transport: T,
}

#[cfg(feature = "http")]
impl StarkHttpClient<ReqwestTransport> {
    /// Configure transport only; repository preflight runs before any network work.
    pub fn new(config: &CrawlConfig) -> UpstreamResult<Self> {
        Ok(Self::with_transport(ReqwestTransport::new(
            &config.api_base_url,
        )?))
    }
}

impl<T: HttpTransport> StarkHttpClient<T> {
    /// Inject transport so pagination and source-shape failures are tested offline.
    pub fn with_transport(transport: T) -> Self {
        Self { transport }
    }

    /// Decode the success envelope, rejecting API-level errors even on HTTP 200.
    fn get<R: DeserializeOwned>(&self, path: &str, params: &[(&str, &str)]) -> UpstreamResult<R> {
        let value = self.transport.get_json(path, params)?;
        let data = if value.get("success").is_some() {
            if value.get("success").and_then(serde_json::Value::as_bool) != Some(true) {
                return Err(UpstreamError::new(format!(
                    "Stark API rejected {path}: {value}"
                )));
            }
            value
                .get("data")
                .cloned()
                .ok_or_else(|| UpstreamError::new(format!("missing data for {path}")))?
        } else {
            // Pack composition is returned as a bare array by the storefront API.
            value
        };
        serde_json::from_value(data).map_err(|error| {
            UpstreamError::new(format!("response shape changed for {path}: {error}"))
        })
    }

    /// Follow every advertised page and reject incomplete or overlapping results.
    /// A successful but truncated crawl must never replace the committed catalog.
    fn listings(&self, config: &CrawlConfig) -> UpstreamResult<Vec<Listing>> {
        let mut products = Vec::new();
        let mut ids = HashSet::new();
        let mut expected = None;
        for page in 1..=MAX_PAGES {
            let page_number = page.to_string();
            let response: ProductPage = self.get(
                "categories/by-slug/parts/products",
                &[
                    ("country", &config.country),
                    ("language", &config.language),
                    ("limit", PAGE_SIZE),
                    ("page", &page_number),
                ],
            )?;
            if response.page != page
                || response.limit == 0
                || response.total_pages == 0
                || expected.is_some_and(|total| total != response.total)
                || response.data.is_empty()
            {
                return Err(UpstreamError::new("inconsistent or empty Parts pagination"));
            }
            expected = Some(response.total);
            for product in response.data {
                if !ids.insert(product.id.clone()) {
                    return Err(UpstreamError::new(format!(
                        "duplicate product {} across listing pages",
                        product.id
                    )));
                }
                products.push(product);
            }
            tracing::info!(
                page,
                products = products.len(),
                total = response.total,
                "catalog listing page fetched"
            );
            if page == response.total_pages {
                if products.len() != response.total {
                    return Err(UpstreamError::new(
                        "Parts listing count did not match total",
                    ));
                }
                return Ok(products);
            }
        }
        Err(UpstreamError::new("Parts listing exceeded page limit"))
    }
}

impl<T: HttpTransport> UpstreamCatalog for StarkHttpClient<T> {
    fn fetch_catalog(&self, config: &CrawlConfig) -> UpstreamResult<Catalog> {
        let tree: Vec<SourceCategory> =
            self.get("categories/tree", &[("language", &config.language)])?;
        let parts = tree
            .iter()
            .find(|node| node.slug == "parts")
            .ok_or_else(|| UpstreamError::new("category tree has no Parts root"))?;
        let mut category_paths = HashMap::<String, Vec<Vec<String>>>::new();
        let categories = convert_categories(&parts.children, &[], &mut category_paths, 0)?;
        let mut bikes = Vec::new();
        for node in &tree {
            if let Some(slug) = &node.bike_slug {
                let id = bike_id(slug);
                if !bikes.iter().any(|bike: &BikeVariant| bike.id == id) {
                    bikes.push(BikeVariant {
                        id: id.clone(),
                        code: id,
                        display_name: bike_display_name(slug, node.name.as_deref()),
                    });
                }
            }
        }
        if bikes.is_empty() {
            return Err(UpstreamError::new("category tree has no bike models"));
        }
        let listings = self.listings(config)?;
        let mut products = Vec::with_capacity(listings.len());
        let mut memberships = HashMap::<String, Vec<String>>::new();
        let params = [
            ("country", config.country.as_str()),
            ("language", config.language.as_str()),
        ];
        for (index, listing) in listings.iter().enumerate() {
            safe_segment(&listing.slug)?;
            let detail: SourceProduct = self.get(&format!("products/{}", listing.slug), &params)?;
            if detail.id != listing.id || detail.slug != listing.slug {
                return Err(UpstreamError::new(format!(
                    "product identity changed for {}",
                    listing.slug
                )));
            }
            let mut paths = Vec::new();
            for category in &detail.categories {
                let source_paths = category_paths.get(&category.slug).ok_or_else(|| {
                    UpstreamError::new(format!(
                        "unknown category {} on {}",
                        category.slug, detail.slug
                    ))
                })?;
                for path in source_paths {
                    if !paths.contains(path) {
                        paths.push(path.clone());
                    }
                }
            }
            if paths.is_empty() {
                return Err(UpstreamError::new(format!(
                    "product {} has no Parts category",
                    detail.slug
                )));
            }
            // The storefront can resolve the product description from its default
            // variant. That wording is not a safe fallback for an untranslated
            // sibling SKU. Keep it on the matching variant instead.
            let description_is_variant_default = detail.description.as_ref().is_some_and(|text| {
                detail
                    .variants
                    .iter()
                    .any(|variant| variant.description.as_ref() == Some(text))
            });
            let mut variants = convert_variants(&detail)?;
            if matches!(detail.kind.as_str(), "KIT" | "BUNDLE" | "PACK") {
                // Bind each response to its concrete SKU. Product-wide unioning
                // makes other colours' components match as direct part wording.
                for source in &detail.variants {
                    let Some(variant) = variants
                        .iter_mut()
                        .find(|variant| variant.code == source.id)
                    else {
                        continue;
                    };
                    let mut composition_params = params.to_vec();
                    if let Some(value) = source.attribute_value_id.as_deref() {
                        composition_params.push(("variantValueId", value));
                    }
                    let composition: Vec<PackItem> = self.get(
                        &format!("products/{}/pack-composition", detail.slug),
                        &composition_params,
                    )?;
                    let mut contents = Vec::new();
                    for item in composition {
                        let name = item.name.as_deref().unwrap_or(&item.slug);
                        let options = item
                            .pre_assigned_values
                            .iter()
                            // This placeholder carries pricing, not a component option.
                            .filter(|value| value.attribute_code != "price")
                            .map(|value| value.value_name.as_deref().unwrap_or(&value.value_code))
                            .collect::<Vec<_>>()
                            .join(", ");
                        let text = format!(
                            "{} × {}{}",
                            item.quantity,
                            name,
                            if options.is_empty() {
                                String::new()
                            } else {
                                format!(" ({options})")
                            }
                        );
                        if !contents.contains(&text) {
                            contents.push(text);
                        }
                        let kits = memberships.entry(item.slug).or_default();
                        let kit = detail.name.as_ref().unwrap_or(&detail.slug).clone();
                        if !kits.contains(&kit) {
                            kits.push(kit);
                        }
                    }
                    variant.kit_contents = Some(contents);
                }
            }
            let fitments = detail
                .fitments
                .iter()
                .map(|fitment| {
                    let id = bike_id(&fitment.bike_slug);
                    if !bikes.iter().any(|bike| bike.id == id) {
                        bikes.push(BikeVariant {
                            id: id.clone(),
                            code: id.clone(),
                            display_name: None,
                        });
                    }
                    Fitment {
                        bike_variant_id: id,
                        conditions: fitment
                            .conditions
                            .clone()
                            .unwrap_or_default()
                            .into_iter()
                            .map(|(key, value)| {
                                (
                                    key,
                                    value
                                        .as_str()
                                        .map(str::to_owned)
                                        .unwrap_or_else(|| value.to_string()),
                                )
                            })
                            .collect(),
                    }
                })
                .collect();
            let url = product_url(&detail.slug, None)?;
            let mut images: Vec<_> = detail.image_url.iter().cloned().collect();
            for media in &detail.media {
                if media.kind == "image" && !images.contains(&media.url) {
                    images.push(media.url.clone());
                }
            }
            products.push(ProductGroup {
                code: detail.slug.clone(),
                display_name: detail.name.clone(),
                description: None,
                localization_key: detail.name_key.clone(),
                description_localization_key: None,
                stark_url: Some(url.clone()),
                image_urls: images.clone(),
                category_paths: paths,
                articles: vec![Article {
                    code: detail.code,
                    display_name: detail.name,
                    description: detail
                        .description
                        .filter(|_| !description_is_variant_default),
                    localization_key: detail.name_key,
                    description_localization_key: detail.description_key,
                    stark_url: Some(url),
                    image_urls: images,
                    kit_memberships: Vec::new(),
                    kit_contents: Vec::new(),
                    variants,
                    fitments,
                }],
            });
            tracing::info!(
                products = index + 1,
                total = listings.len(),
                slug = listing.slug,
                "catalog product fetched"
            );
        }
        for product in &mut products {
            product.articles[0].kit_memberships =
                memberships.remove(&product.code).unwrap_or_default();
        }
        Ok(Catalog {
            metadata: CatalogMetadata {
                schema_version: SCHEMA_VERSION,
                generated_at: config.generated_at.clone(),
                source: SourceMetadata {
                    api_base_url: config.api_base_url.clone(),
                    country: config.country.clone(),
                    language: config.language.clone(),
                    endpoints: [
                        "/categories/tree",
                        "/categories/by-slug/parts/products",
                        "/products/{slug}",
                        "/products/{slug}/pack-composition",
                    ]
                    .into_iter()
                    .map(|path| SourceEndpoint {
                        method: "GET".to_owned(),
                        path: path.to_owned(),
                    })
                    .collect(),
                },
            },
            bike_variants: bikes,
            catalog_trees: Vec::new(),
            categories,
            products,
        })
    }
}

/// Retain existing bookmarked filter IDs while accepting new source model slugs.
fn bike_id(slug: &str) -> String {
    match slug {
        "stark-varg" => "varg",
        "stark-varg-1.2" => "varg-1.2",
        "stark-varg-ex" => "varg-ex",
        "stark-varg-sm" => "varg-sm",
        other => other,
    }
    .to_owned()
}

/// Disambiguate the original MX when the source calls it only "Stark VARG MX".
/// Other model names continue to follow the current source without a fixed list.
fn bike_display_name(slug: &str, source_name: Option<&str>) -> Option<String> {
    if slug == "stark-varg" {
        Some("Stark VARG MX 1.0".to_owned())
    } else {
        source_name.map(str::to_owned)
    }
}

/// Treat source slugs as path segments, never as URLs or traversal instructions.
fn safe_segment(value: &str) -> UpstreamResult<()> {
    if value.is_empty()
        || matches!(value, "." | "..")
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
    {
        return Err(UpstreamError::new(format!("unsafe source slug {value}")));
    }
    Ok(())
}

/// Build canonical product links; query encoding preserves variant identity safely.
fn product_url(slug: &str, variant: Option<&str>) -> UpstreamResult<String> {
    safe_segment(slug)?;
    let mut url = Url::parse(&format!(
        "https://starkfuture.com/parts-and-accessories/product/{slug}"
    ))
    .unwrap();
    if let Some(variant) = variant {
        url.query_pairs_mut().append_pair("variant", variant);
    }
    Ok(url.into())
}

/// Keep the global category order and every path through overlapping collections.
fn convert_categories(
    nodes: &[SourceCategory],
    parent: &[String],
    paths: &mut HashMap<String, Vec<Vec<String>>>,
    depth: usize,
) -> UpstreamResult<Vec<CategoryNode>> {
    if depth >= 32 {
        return Err(UpstreamError::new("category tree exceeds depth limit"));
    }
    nodes
        .iter()
        .map(|node| {
            safe_segment(&node.slug)?;
            let mut path = parent.to_vec();
            path.push(node.slug.clone());
            paths
                .entry(node.slug.clone())
                .or_default()
                .push(path.clone());
            Ok(CategoryNode {
                code: node.slug.clone(),
                path: path.clone(),
                display_name: node.name.clone(),
                localization_key: node.name_key.clone(),
                categories: convert_categories(&node.children, &path, paths, depth + 1)?,
                product_groups: Vec::new(),
            })
        })
        .collect()
}

/// Prefer stock selection records for SKU/attribute combinations over product totals.
/// Product-level availability must not be assigned to an individual colour or size.
/// Variant descriptions likewise take precedence over default product wording.
fn convert_variants(product: &SourceProduct) -> UpstreamResult<Vec<ArticleVariant>> {
    let mut variants = Vec::new();
    let mut seen = HashSet::new();
    for source in &product.variants {
        let Some(sku) = source.erp_code.as_ref().filter(|value| !value.is_empty()) else {
            continue;
        };
        if !seen.insert(sku.clone()) {
            continue;
        }
        let selection = product
            .stock_by_selection
            .iter()
            .find(|selection| selection.erp_code.as_ref() == Some(sku));
        let attributes = product
            .attributes
            .iter()
            .flat_map(|attribute| {
                attribute.values.iter().filter_map(|value| {
                    let selected = selection
                        .map(|selection| {
                            selection
                                .selected_attribute_value_ids
                                .contains(&value.attribute_value_id)
                        })
                        .unwrap_or_else(|| {
                            source.attribute_value_id.as_ref() == Some(&value.attribute_value_id)
                        });
                    (selected && attribute.code != "price").then(|| AttributeSelection {
                        code: attribute.code.clone(),
                        option_code: value.code.clone(),
                        option_display_name: value.name.clone(),
                        option_localization_key: value.name_key.clone(),
                    })
                })
            })
            .collect();
        let price = source
            .sale_price
            .as_ref()
            .map(|price| {
                let amount_minor = price.amount_minor.parse::<i64>().map_err(|error| {
                    UpstreamError::new(format!("invalid price on {}: {error}", product.slug))
                })?;
                Ok::<_, UpstreamError>(Price {
                    amount_minor,
                    currency: price.currency_code.clone(),
                })
            })
            .transpose()?;
        variants.push(ArticleVariant {
            code: source.id.clone(),
            sku: Some(sku.clone()),
            stark_url: Some(product_url(&product.slug, Some(&source.id))?),
            image_urls: source.image_url.iter().cloned().collect(),
            attributes,
            price,
            availability: selection
                .and_then(|stock| stock.availability.clone())
                .or_else(|| source.availability.clone())
                .map(|status| Availability {
                    status,
                    quantity: selection.and_then(|stock| stock.quantity_available),
                }),
            description: source.description.clone(),
            description_localization_key: source.description_key.clone(),
            kit_contents: None,
        });
    }
    if variants.is_empty() {
        return Err(UpstreamError::new(format!(
            "product {} has no concrete SKU variants",
            product.slug
        )));
    }
    Ok(variants)
}

// Only fields consumed by the adapter are modeled. Required collection fields
// intentionally lack defaults so a source schema change cannot silently empty them.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SourceCategory {
    slug: String,
    name: Option<String>,
    name_key: Option<String>,
    bike_slug: Option<String>,
    children: Vec<SourceCategory>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ProductPage {
    data: Vec<Listing>,
    total: usize,
    page: usize,
    limit: usize,
    total_pages: usize,
}
#[derive(Deserialize)]
struct Listing {
    id: String,
    slug: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SourceProduct {
    id: String,
    code: String,
    slug: String,
    #[serde(rename = "type")]
    kind: String,
    name: Option<String>,
    name_key: Option<String>,
    description: Option<String>,
    description_key: Option<String>,
    image_url: Option<String>,
    categories: Vec<ProductCategory>,
    variants: Vec<SourceVariant>,
    attributes: Vec<SourceAttribute>,
    fitments: Vec<SourceFitment>,
    #[serde(default)]
    media: Vec<Media>,
    #[serde(default)]
    stock_by_selection: Vec<StockSelection>,
}
#[derive(Deserialize)]
struct ProductCategory {
    slug: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SourceVariant {
    id: String,
    erp_code: Option<String>,
    attribute_value_id: Option<String>,
    image_url: Option<String>,
    sale_price: Option<SourcePrice>,
    availability: Option<String>,
    description: Option<String>,
    description_key: Option<String>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SourcePrice {
    amount_minor: String,
    currency_code: String,
}
#[derive(Deserialize)]
struct SourceAttribute {
    code: String,
    values: Vec<AttributeValue>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct AttributeValue {
    attribute_value_id: String,
    code: String,
    name: Option<String>,
    name_key: Option<String>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SourceFitment {
    bike_slug: String,
    conditions: Option<BTreeMap<String, serde_json::Value>>,
}
#[derive(Deserialize)]
struct Media {
    url: String,
    kind: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct StockSelection {
    erp_code: Option<String>,
    selected_attribute_value_ids: Vec<String>,
    availability: Option<String>,
    quantity_available: Option<u32>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PackItem {
    slug: String,
    name: Option<String>,
    quantity: u32,
    #[serde(default)]
    pre_assigned_values: Vec<PackValue>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PackValue {
    attribute_code: String,
    value_code: String,
    value_name: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, json};
    use std::cell::RefCell;

    /// Two pages and overlapping categories exercise completeness and deduplication.
    struct FixtureTransport {
        responses: HashMap<String, Value>,
        calls: RefCell<Vec<String>>,
    }

    impl FixtureTransport {
        fn new() -> Self {
            let category = |slug: &str| json!({"slug": slug, "name": slug, "children": []});
            let tree = json!([
                {"slug":"varg", "name":"VARG MX 1.0", "bikeSlug":"stark-varg", "children":[]},
                {"slug":"parts", "children":[category("frame"), category("race-parts")]}
            ]);
            let page = |page, slug: &str| json!({"data":[{"id":slug,"slug":slug}], "total":2, "page":page, "limit":100, "totalPages":2});
            Self {
                responses: HashMap::from([
                    ("categories/tree".to_owned(), tree),
                    (
                        "categories/by-slug/parts/products#1".to_owned(),
                        page(1, "kit"),
                    ),
                    (
                        "categories/by-slug/parts/products#2".to_owned(),
                        page(2, "bolt"),
                    ),
                    ("products/kit".to_owned(), product("kit", "KIT")),
                    ("products/bolt".to_owned(), product("bolt", "PART")),
                    (
                        "products/kit/pack-composition".to_owned(),
                        json!([{"slug":"bolt","name":"Bolt","quantity":2,"preAssignedValues":[]}]),
                    ),
                ]),
                calls: RefCell::new(Vec::new()),
            }
        }
    }

    impl HttpTransport for FixtureTransport {
        /// Distinguish pages and selected kit values while allowing simple fixtures
        /// to share a response for every value. Selection-specific tests override it.
        fn get_json(&self, path: &str, params: &[(&str, &str)]) -> UpstreamResult<Value> {
            let selector = params
                .iter()
                .find(|(key, _)| matches!(*key, "page" | "variantValueId"))
                .map(|(_, value)| *value);
            let selected_key = selector.map(|value| format!("{path}#{value}"));
            let key = selected_key
                .filter(|key| self.responses.contains_key(key))
                .unwrap_or_else(|| path.to_owned());
            self.calls.borrow_mut().push(key.clone());
            self.responses
                .get(&key)
                .cloned()
                .map(|data| {
                    if path.ends_with("pack-composition") {
                        data
                    } else {
                        json!({"success":true,"data":data})
                    }
                })
                .ok_or_else(|| UpstreamError::new(format!("unexpected fixture request {key}")))
        }
    }

    /// Model a priced SKU whose stock differs from the product-level total.
    fn product(slug: &str, kind: &str) -> Value {
        json!({
            "id":slug,"slug":slug,"code":slug,"type":kind,"name":slug,
            "categories":[{"slug":"frame"},{"slug":"race-parts"}],
            "fitments":[{"bikeSlug":"stark-varg","conditions":{"inverter":"gen2"}}],
            "variants":[{"id":format!("{slug}-grey"),"erpCode":format!("{slug}-SKU"),"attributeValueId":"grey",
                "salePrice":{"amountMinor":"14900","currencyCode":"USD"}, "availability":"NOT_AVAILABLE"}],
            "attributes":[{"code":"color","values":[{"attributeValueId":"grey","code":"grey","name":"Grey"}]}],
            "stockBySelection":[{"erpCode":format!("{slug}-SKU"),"selectedAttributeValueIds":["grey"],"availability":"AVAILABLE","quantityAvailable":3}],
            "availability":"NOT_AVAILABLE","quantityAvailable":0
        })
    }

    #[test]
    /// A refresh includes every page and stores each product once across model/category contexts.
    fn crawls_shared_products_with_fitment_stock_and_kit_relationships() {
        let client = StarkHttpClient::with_transport(FixtureTransport::new());
        let catalog =
            crawl_catalog(&client, &CrawlConfig::us_storefront("2026-10-17T00:00:00Z")).unwrap();
        assert_eq!(catalog.products.len(), 2);
        assert!(catalog.catalog_trees.is_empty());
        assert_eq!(catalog.bike_variants[0].id, "varg");
        assert_eq!(
            catalog.bike_variants[0].display_name.as_deref(),
            Some("Stark VARG MX 1.0")
        );
        let kit = &catalog.products[0];
        assert_eq!(kit.category_paths, [vec!["frame"], vec!["race-parts"]]);
        let article = &kit.articles[0];
        assert_eq!(article.fitments[0].conditions["inverter"], "gen2");
        assert!(article.kit_contents.is_empty());
        assert_eq!(catalog.products[1].articles[0].kit_memberships, ["kit"]);
        let variant = &article.variants[0];
        assert_eq!(variant.kit_contents.as_ref().unwrap(), &["2 × Bolt"]);
        assert_eq!(variant.price.as_ref().unwrap().amount_minor, 14900);
        assert_eq!(variant.availability.as_ref().unwrap().quantity, Some(3));
        assert_eq!(variant.availability.as_ref().unwrap().status, "AVAILABLE");
        assert_eq!(
            variant.attributes[0].option_display_name.as_deref(),
            Some("Grey")
        );
        assert!(
            variant
                .stark_url
                .as_ref()
                .unwrap()
                .ends_with("/kit?variant=kit-grey")
        );
        let bytes = format_catalog_json5(&catalog).unwrap();
        assert_eq!(parse_catalog_json5(&bytes).unwrap(), catalog);
        assert_eq!(format_catalog_json5(&catalog).unwrap(), bytes);
        assert_eq!(
            client
                .transport
                .calls
                .borrow()
                .iter()
                .filter(|path| path.as_str() == "products/kit")
                .count(),
            1
        );
    }

    #[test]
    /// Each kit SKU owns its composition and wording; internal pricing selections
    /// must not become searchable component options, while real colours survive.
    fn preserves_selected_kit_contents_and_ignores_price_options() {
        let mut transport = FixtureTransport::new();
        let kit = transport.responses.get_mut("products/kit").unwrap();
        kit["description"] = json!("Grey panels");
        kit["variants"] = json!([
            {"id":"kit-grey", "erpCode":"KIT-G", "attributeValueId":"grey", "description":"Grey panels"},
            {"id":"kit-white", "erpCode":"KIT-W", "attributeValueId":"white", "description":"White panels", "descriptionKey":"white.description"},
            {"id":"kit-empty", "erpCode":"KIT-E", "attributeValueId":"empty"}
        ]);
        for (selection, values) in [
            (
                "grey",
                json!([{"attributeCode":"price", "valueCode":"price"}]),
            ),
            (
                "white",
                json!([{"attributeCode":"price", "valueCode":"price"}, {"attributeCode":"color", "valueCode":"white", "valueName":"Snow White"}]),
            ),
        ] {
            transport.responses.insert(
                format!("products/kit/pack-composition#{selection}"),
                json!([{"slug":"bolt", "name":"Bolt", "quantity":2, "preAssignedValues":values}]),
            );
        }
        transport
            .responses
            .insert("products/kit/pack-composition#empty".to_owned(), json!([]));
        let catalog = crawl_catalog(
            &StarkHttpClient::with_transport(transport),
            &CrawlConfig::us_storefront("2026-10-17T00:00:00Z"),
        )
        .unwrap();
        let variants = &catalog.products[0].articles[0].variants;
        assert!(catalog.products[0].articles[0].description.is_none());
        assert!(variants[2].description.is_none());
        assert_eq!(variants[0].kit_contents, Some(vec!["2 × Bolt".to_owned()]));
        assert_eq!(
            variants[1].kit_contents,
            Some(vec!["2 × Bolt (Snow White)".to_owned()])
        );
        assert_eq!(variants[1].description.as_deref(), Some("White panels"));
        assert_eq!(
            variants[1].description_localization_key.as_deref(),
            Some("white.description")
        );
        assert_eq!(variants[2].kit_contents, Some(Vec::new()));
        let bytes = format_catalog_json5(&catalog).unwrap();
        assert_eq!(parse_catalog_json5(&bytes).unwrap(), catalog);
    }

    #[test]
    /// Repeated pages must fail rather than appear to be a successful partial snapshot.
    fn rejects_duplicate_or_incomplete_pagination() {
        for incomplete in [false, true] {
            let mut transport = FixtureTransport::new();
            let page = transport
                .responses
                .get_mut("categories/by-slug/parts/products#2")
                .unwrap();
            if incomplete {
                page["data"] = json!([]);
            } else {
                page["data"] = json!([{"id":"kit","slug":"kit"}]);
            }
            let error = crawl_catalog(
                &StarkHttpClient::with_transport(transport),
                &CrawlConfig::us_storefront("2026-10-17T00:00:00Z"),
            )
            .unwrap_err();
            assert!(error.to_string().contains(if incomplete {
                "pagination"
            } else {
                "duplicate product"
            }));
        }
    }

    #[test]
    /// Missing structured fitment is a schema error; a real empty array remains unspecified.
    fn distinguishes_missing_fitment_from_unspecified_compatibility() {
        let mut missing = product("bolt", "PART");
        missing.as_object_mut().unwrap().remove("fitments");
        assert!(serde_json::from_value::<SourceProduct>(missing).is_err());
        let mut empty = product("bolt", "PART");
        empty["fitments"] = json!([]);
        assert!(
            serde_json::from_value::<SourceProduct>(empty)
                .unwrap()
                .fitments
                .is_empty()
        );
    }

    #[test]
    /// Slugs never become arbitrary request URLs, even if upstream sends hostile data.
    fn rejects_path_traversal_and_encodes_variant_queries() {
        for slug in [
            "..",
            "../seat",
            "https://example.com",
            "seat?country=XX",
            "seat#bad",
        ] {
            assert!(product_url(slug, None).is_err());
        }
        let url = product_url("seat", Some("grey&country=XX")).unwrap();
        assert!(url.ends_with("variant=grey%26country%3DXX"));
    }
}

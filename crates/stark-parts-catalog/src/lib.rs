//! Committed catalog schema, validation, and deterministic JSON5 formatting.
//!
//! This crate owns the project schema rather than mirroring Stark's upstream
//! API. The crawler can change how it talks to Stark without making the
//! committed file noisy or unstable, while the web app can depend on this
//! smaller contract for search and rendering. Current products are shared across
//! categories and model fitments; legacy tree snapshots remain readable while
//! application code and generated data are deployed as separate changes.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt::Write as _;
#[cfg(feature = "http")]
use std::thread::sleep;
#[cfg(feature = "http")]
use std::time::Duration;
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;
use url::Url;

const SCHEMA_VERSION: u32 = 2;
#[cfg(test)]
const STARK_SPARE_PARTS_URL: &str = "https://starkfuture.com/parts-and-accessories/spare-parts";

mod upstream;
pub use upstream::{StarkHttpClient, UpstreamCatalog, crawl_catalog};
pub const DEFAULT_CATALOG_PATH: &str = "catalog/stark-parts.json5";
const ALLOWED_IMAGE_HOSTS: &[&str] = &[
    "assets.starkfuture.com",
    "s3-stark-prod.s3.eu-central-1.amazonaws.com",
    "s3-stark-production.s3.eu-west-1.amazonaws.com",
];
const ALLOWED_STARK_LINK_HOSTS: &[&str] = &["starkfuture.com", "www.starkfuture.com"];

/// A project-owned snapshot of the public Stark parts catalog.
///
/// The field order in this type is also the canonical formatting order. Keep
/// additions intentional: changing this type changes the review shape of the
/// committed catalog file and the browser-side data contract built on top of
/// it.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Catalog {
    pub metadata: CatalogMetadata,
    pub bike_variants: Vec<BikeVariant>,
    pub catalog_trees: Vec<BikeCatalogTree>,
    /// Shared category taxonomy and products used by the current storefront.
    /// Legacy snapshots can still be read while a code-only migration is reviewed.
    #[serde(default)]
    pub categories: Vec<CategoryNode>,
    #[serde(default)]
    pub products: Vec<ProductGroup>,
}

/// Crawl and source metadata a user can inspect to judge catalog freshness.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CatalogMetadata {
    pub schema_version: u32,
    pub generated_at: String,
    pub source: SourceMetadata,
}

/// Source assumptions that affect prices, availability, localization, and URLs.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SourceMetadata {
    pub api_base_url: String,
    pub country: String,
    pub language: String,
    pub endpoints: Vec<SourceEndpoint>,
}

/// An upstream endpoint that contributed data to the committed snapshot.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SourceEndpoint {
    pub method: String,
    pub path: String,
}

/// A Stark bike variant exposed as a stable browser filter.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BikeVariant {
    pub id: String,
    pub code: String,
    pub display_name: Option<String>,
}

/// The catalog hierarchy for one bike variant.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BikeCatalogTree {
    pub bike_variant_id: String,
    pub categories: Vec<CategoryNode>,
}

/// A category node whose children preserve Stark's user-visible tree order.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CategoryNode {
    pub code: String,
    pub path: Vec<String>,
    pub display_name: Option<String>,
    pub localization_key: Option<String>,
    pub categories: Vec<CategoryNode>,
    pub product_groups: Vec<ProductGroup>,
}

/// A source product group under a catalog category.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ProductGroup {
    pub code: String,
    pub display_name: Option<String>,
    pub description: Option<String>,
    pub localization_key: Option<String>,
    pub description_localization_key: Option<String>,
    pub stark_url: Option<String>,
    pub image_urls: Vec<String>,
    pub articles: Vec<Article>,
    /// A product can belong to several collections and subsystem paths.
    #[serde(default)]
    pub category_paths: Vec<Vec<String>>,
}

/// A part or article that may have several selectable variants and SKUs.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Article {
    pub code: String,
    pub display_name: Option<String>,
    pub description: Option<String>,
    pub localization_key: Option<String>,
    pub description_localization_key: Option<String>,
    pub stark_url: Option<String>,
    pub image_urls: Vec<String>,
    pub kit_memberships: Vec<String>,
    pub kit_contents: Vec<String>,
    pub variants: Vec<ArticleVariant>,
    /// Explicit source fitments; an empty list means compatibility is unspecified.
    #[serde(default)]
    pub fitments: Vec<Fitment>,
}

/// Model compatibility with source configuration restrictions kept verbatim.
///
/// Conditions narrow the model match. They must never be discarded or inferred
/// from product prose, which can disagree with the structured source data.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Fitment {
    pub bike_variant_id: String,
    pub conditions: BTreeMap<String, String>,
}

/// A concrete variant of an article, including SKU and storefront metadata.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ArticleVariant {
    pub code: String,
    pub sku: Option<String>,
    pub stark_url: Option<String>,
    pub image_urls: Vec<String>,
    pub attributes: Vec<AttributeSelection>,
    pub price: Option<Price>,
    pub availability: Option<Availability>,
    /// Source wording for this SKU, overriding the product's default description.
    pub description: Option<String>,
    pub description_localization_key: Option<String>,
    /// `None` inherits legacy article contents; `Some([])` explicitly has none.
    /// A selected kit must not inherit another option's component list.
    #[serde(default)]
    pub kit_contents: Option<Vec<String>>,
}

/// A selected article attribute, such as size or color.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AttributeSelection {
    pub code: String,
    pub option_code: String,
    pub option_display_name: Option<String>,
    pub option_localization_key: Option<String>,
}

/// Storefront price data captured during an offline catalog update.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Price {
    pub amount_minor: i64,
    pub currency: String,
}

/// Storefront availability data captured during an offline catalog update.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Availability {
    pub status: String,
    pub quantity: Option<u32>,
}

/// Errors from parsing, validation, or deterministic catalog formatting.
#[derive(Debug, thiserror::Error)]
pub enum CatalogError {
    #[error("catalog JSON5 did not parse: {0}")]
    Parse(#[from] json5::Error),
    #[error("catalog metadata uses unsupported schema_version {actual}; expected {expected}")]
    UnsupportedSchemaVersion { actual: u32, expected: u32 },
    #[error("catalog generated_at must be RFC3339: {0}")]
    InvalidGeneratedAt(#[source] time::error::Parse),
    #[error("source api_base_url is not a valid HTTPS URL: {0}")]
    InvalidApiBaseUrl(#[source] UrlValidationError),
    #[error("source endpoint at index {index} has unsupported method {method}")]
    InvalidEndpointMethod { index: usize, method: String },
    #[error("source endpoint at index {index} must start with '/': {path}")]
    InvalidEndpointPath { index: usize, path: String },
    #[error("bike variant id {id} has no matching catalog tree")]
    MissingCatalogTree { id: String },
    #[error("catalog tree references unknown bike_variant_id {id}")]
    UnknownTreeVariant { id: String },
    #[error("shared catalog contract violated: {0}")]
    InvalidSharedCatalog(String),
    #[error("image URL is not allowed: {url}")]
    InvalidImageUrl {
        url: String,
        #[source]
        source: UrlValidationError,
    },
    #[error("Stark URL is not allowed: {url}")]
    InvalidStarkUrl {
        url: String,
        #[source]
        source: UrlValidationError,
    },
    #[error("catalog serialization failed: {0}")]
    Serialize(#[from] serde_json::Error),
}

#[derive(Debug, thiserror::Error)]
pub enum UrlValidationError {
    #[error("URL did not parse")]
    Parse(#[from] url::ParseError),
    #[error("URL must use https")]
    NonHttps,
    #[error("URL must not include credentials")]
    Credentials,
    #[error("URL must not include a fragment")]
    Fragment,
    #[error("URL host is not allowed")]
    Host,
}

/// Configuration shared by the trait-backed crawler core and HTTP client.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CrawlConfig {
    pub api_base_url: String,
    pub country: String,
    pub language: String,
    pub generated_at: String,
}

impl CrawlConfig {
    /// Select the same country and English data path used for the committed Parts snapshot.
    pub fn us_storefront(generated_at: impl Into<String>) -> Self {
        Self {
            api_base_url: "https://api.starkfuture.com/v3/product-catalog".to_owned(),
            country: "US".to_owned(),
            language: "en".to_owned(),
            generated_at: generated_at.into(),
        }
    }
}

pub type UpstreamResult<T> = Result<T, UpstreamError>;

#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
#[error("{message}")]
pub struct UpstreamError {
    message: String,
}

impl UpstreamError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum CrawlError {
    #[error("upstream catalog request failed: {0}")]
    Upstream(#[from] UpstreamError),
    #[error("catalog validation failed after crawl: {0}")]
    Catalog(#[from] CatalogError),
}

/// Minimal HTTP boundary used by the real Stark client.
///
/// Tests can provide a fake transport that returns JSON values, while
/// production uses `ReqwestTransport`. Keeping transport below the
/// `UpstreamCatalog` trait prevents the crawler from knowing anything about
/// URLs, status codes, or HTTP client configuration.
pub trait HttpTransport {
    fn get_json(&self, path: &str, params: &[(&str, &str)]) -> UpstreamResult<serde_json::Value>;
}

#[cfg(feature = "http")]
#[derive(Debug)]
pub struct ReqwestTransport {
    client: reqwest::blocking::Client,
    api_base_url: Url,
}

#[cfg(feature = "http")]
impl ReqwestTransport {
    /// Bound each request so an upstream stall cannot hold a catalog refresh indefinitely.
    pub fn new(api_base_url: &str) -> UpstreamResult<Self> {
        let mut api_base_url = Url::parse(api_base_url)
            .map_err(|error| UpstreamError::new(format!("invalid Stark API base URL: {error}")))?;
        if !api_base_url.path().ends_with('/') {
            api_base_url.set_path(&format!("{}/", api_base_url.path()));
        }

        let client = reqwest::blocking::Client::builder()
            .user_agent("stark-parts/0.1")
            .timeout(Duration::from_secs(60))
            .build()
            .map_err(|error| {
                UpstreamError::new(format!("failed to create HTTP client: {error}"))
            })?;

        Ok(Self {
            client,
            api_base_url,
        })
    }

    fn build_url(&self, path: &str, params: &[(&str, &str)]) -> UpstreamResult<Url> {
        let mut url = self
            .api_base_url
            .join(path.trim_start_matches('/'))
            .map_err(|error| {
                UpstreamError::new(format!("invalid Stark API path {path}: {error}"))
            })?;

        {
            let mut query = url.query_pairs_mut();
            for (key, value) in params {
                query.append_pair(key, value);
            }
        }

        Ok(url)
    }

    fn send_with_retries(
        &self,
        url: Url,
        context: &str,
    ) -> UpstreamResult<reqwest::blocking::Response> {
        for attempt in 1..=3 {
            match self
                .client
                .get(url.clone())
                .send()
                .and_then(reqwest::blocking::Response::error_for_status)
            {
                Ok(response) => return Ok(response),
                Err(error) => {
                    tracing::warn!(%url, attempt, %error, "{context} failed");
                    let should_retry = should_retry_request_error(&error);
                    if attempt < 3 && should_retry {
                        sleep(Duration::from_millis(250 * attempt));
                    } else {
                        let label = if should_retry {
                            "failed after retries"
                        } else {
                            "failed"
                        };
                        return Err(UpstreamError::new(format!("{context} {label}: {error}")));
                    }
                }
            }
        }

        unreachable!("retry loop always returns on attempt 3")
    }
}

#[cfg(feature = "http")]
fn should_retry_request_error(error: &reqwest::Error) -> bool {
    if error.is_timeout() || error.is_connect() {
        return true;
    }

    matches!(
        error.status(),
        Some(reqwest::StatusCode::TOO_MANY_REQUESTS)
            | Some(reqwest::StatusCode::INTERNAL_SERVER_ERROR)
            | Some(reqwest::StatusCode::BAD_GATEWAY)
            | Some(reqwest::StatusCode::SERVICE_UNAVAILABLE)
            | Some(reqwest::StatusCode::GATEWAY_TIMEOUT)
    )
}

#[cfg(feature = "http")]
impl HttpTransport for ReqwestTransport {
    fn get_json(&self, path: &str, params: &[(&str, &str)]) -> UpstreamResult<serde_json::Value> {
        let url = self.build_url(path, params)?;

        tracing::debug!(%url, "fetching Stark catalog JSON");
        self.send_with_retries(url, "Stark API request")?
            .json()
            .map_err(|error| {
                UpstreamError::new(format!("Stark API response was not valid JSON: {error}"))
            })
    }
}

/// Parse committed JSON5 and enforce the catalog contract used by later steps.
pub fn parse_catalog_json5(input: &str) -> Result<Catalog, CatalogError> {
    let catalog = json5::from_str(input)?;
    validate_catalog(&catalog)?;
    Ok(catalog)
}

/// Format a catalog into the deterministic JSON5 shape committed to the repo.
pub fn format_catalog_json5(catalog: &Catalog) -> Result<String, CatalogError> {
    validate_catalog(catalog)?;

    let mut out = String::new();
    write_catalog(&mut out, catalog)?;
    Ok(out)
}

/// Validate fields whose safety or stability matters outside plain deserialization.
pub fn validate_catalog(catalog: &Catalog) -> Result<(), CatalogError> {
    if !matches!(catalog.metadata.schema_version, 1 | SCHEMA_VERSION) {
        return Err(CatalogError::UnsupportedSchemaVersion {
            actual: catalog.metadata.schema_version,
            expected: SCHEMA_VERSION,
        });
    }

    OffsetDateTime::parse(&catalog.metadata.generated_at, &Rfc3339)
        .map_err(CatalogError::InvalidGeneratedAt)?;
    validate_https_url(&catalog.metadata.source.api_base_url, None)
        .map_err(CatalogError::InvalidApiBaseUrl)?;

    for (index, endpoint) in catalog.metadata.source.endpoints.iter().enumerate() {
        if !matches!(endpoint.method.as_str(), "GET" | "POST") {
            return Err(CatalogError::InvalidEndpointMethod {
                index,
                method: endpoint.method.clone(),
            });
        }
        if !endpoint.path.starts_with('/') {
            return Err(CatalogError::InvalidEndpointPath {
                index,
                path: endpoint.path.clone(),
            });
        }
    }

    for variant in &catalog.bike_variants {
        if catalog.products.is_empty()
            && !catalog
                .catalog_trees
                .iter()
                .any(|tree| tree.bike_variant_id == variant.id)
        {
            return Err(CatalogError::MissingCatalogTree {
                id: variant.id.clone(),
            });
        }
    }

    for tree in &catalog.catalog_trees {
        if !catalog
            .bike_variants
            .iter()
            .any(|variant| variant.id == tree.bike_variant_id)
        {
            return Err(CatalogError::UnknownTreeVariant {
                id: tree.bike_variant_id.clone(),
            });
        }
        validate_categories(&tree.categories)?;
    }

    validate_categories(&catalog.categories)?;
    validate_product_groups(&catalog.products)?;
    validate_shared_references(catalog)?;

    Ok(())
}

/// Catch dangling category and model references before browser indexing can hide data.
fn validate_shared_references(catalog: &Catalog) -> Result<(), CatalogError> {
    if catalog.metadata.schema_version == 1
        && (!catalog.products.is_empty() || !catalog.categories.is_empty())
    {
        return Err(CatalogError::InvalidSharedCatalog(
            "shared products require schema version 2".to_owned(),
        ));
    }
    if catalog.metadata.schema_version == 2 && !catalog.catalog_trees.is_empty() {
        return Err(CatalogError::InvalidSharedCatalog(
            "schema version 2 uses shared products, not bike trees".to_owned(),
        ));
    }
    fn paths(nodes: &[CategoryNode], output: &mut std::collections::HashSet<Vec<String>>) {
        for node in nodes {
            output.insert(node.path.clone());
            paths(&node.categories, output);
        }
    }
    let mut category_paths = std::collections::HashSet::new();
    paths(&catalog.categories, &mut category_paths);
    let mut codes = std::collections::HashSet::new();
    let mut model_ids = std::collections::HashSet::new();
    for bike in &catalog.bike_variants {
        if !model_ids.insert(&bike.id) {
            return Err(CatalogError::InvalidSharedCatalog(format!(
                "duplicate bike {}",
                bike.id
            )));
        }
    }
    for product in &catalog.products {
        if !codes.insert(&product.code) {
            return Err(CatalogError::InvalidSharedCatalog(format!(
                "duplicate product {}",
                product.code
            )));
        }
        for path in &product.category_paths {
            if !category_paths.contains(path) {
                return Err(CatalogError::InvalidSharedCatalog(format!(
                    "unknown category path {path:?}"
                )));
            }
        }
        for article in &product.articles {
            for fitment in &article.fitments {
                if !catalog
                    .bike_variants
                    .iter()
                    .any(|bike| bike.id == fitment.bike_variant_id)
                {
                    return Err(CatalogError::InvalidSharedCatalog(format!(
                        "unknown fitment model {}",
                        fitment.bike_variant_id
                    )));
                }
            }
        }
    }
    Ok(())
}

fn validate_categories(categories: &[CategoryNode]) -> Result<(), CatalogError> {
    for category in categories {
        validate_categories(&category.categories)?;
        validate_product_groups(&category.product_groups)?;
    }
    Ok(())
}

/// Apply the same URL trust boundary to shared products and legacy tree entries.
fn validate_product_groups(groups: &[ProductGroup]) -> Result<(), CatalogError> {
    for group in groups {
        if let Some(url) = &group.stark_url {
            validate_https_url(url, Some(ALLOWED_STARK_LINK_HOSTS)).map_err(|source| {
                CatalogError::InvalidStarkUrl {
                    url: url.clone(),
                    source,
                }
            })?;
        }
        for url in &group.image_urls {
            validate_https_url(url, Some(ALLOWED_IMAGE_HOSTS)).map_err(|source| {
                CatalogError::InvalidImageUrl {
                    url: url.clone(),
                    source,
                }
            })?;
        }
        for article in &group.articles {
            if let Some(url) = &article.stark_url {
                validate_https_url(url, Some(ALLOWED_STARK_LINK_HOSTS)).map_err(|source| {
                    CatalogError::InvalidStarkUrl {
                        url: url.clone(),
                        source,
                    }
                })?;
            }
            for url in &article.image_urls {
                validate_https_url(url, Some(ALLOWED_IMAGE_HOSTS)).map_err(|source| {
                    CatalogError::InvalidImageUrl {
                        url: url.clone(),
                        source,
                    }
                })?;
            }
            for variant in &article.variants {
                if let Some(url) = &variant.stark_url {
                    validate_https_url(url, Some(ALLOWED_STARK_LINK_HOSTS)).map_err(|source| {
                        CatalogError::InvalidStarkUrl {
                            url: url.clone(),
                            source,
                        }
                    })?;
                }
                for url in &variant.image_urls {
                    validate_https_url(url, Some(ALLOWED_IMAGE_HOSTS)).map_err(|source| {
                        CatalogError::InvalidImageUrl {
                            url: url.clone(),
                            source,
                        }
                    })?;
                }
            }
        }
    }
    Ok(())
}

fn validate_https_url(
    input: &str,
    allowed_hosts: Option<&[&str]>,
) -> Result<(), UrlValidationError> {
    let url = Url::parse(input)?;
    if url.scheme() != "https" {
        return Err(UrlValidationError::NonHttps);
    }
    if url.username() != "" || url.password().is_some() {
        return Err(UrlValidationError::Credentials);
    }
    if url.fragment().is_some() {
        return Err(UrlValidationError::Fragment);
    }
    if let Some(hosts) = allowed_hosts
        && !url.host_str().is_some_and(|host| hosts.contains(&host))
    {
        return Err(UrlValidationError::Host);
    }
    Ok(())
}

fn write_catalog(out: &mut String, catalog: &Catalog) -> Result<(), CatalogError> {
    out.push_str("{\n");
    write_metadata(out, 1, "metadata", &catalog.metadata)?;
    out.push_str(",\n");
    write_bike_variants(out, 1, "bike_variants", &catalog.bike_variants)?;
    out.push_str(",\n");
    write_catalog_trees(out, 1, "catalog_trees", &catalog.catalog_trees)?;
    if !catalog.categories.is_empty() || !catalog.products.is_empty() {
        out.push_str(",\n");
        write_categories(out, 1, "categories", &catalog.categories)?;
        out.push_str(",\n");
        write_product_groups(out, 1, "products", &catalog.products)?;
    }
    out.push_str("\n}\n");
    Ok(())
}

fn write_metadata(
    out: &mut String,
    indent: usize,
    key: &str,
    metadata: &CatalogMetadata,
) -> Result<(), CatalogError> {
    write_key(out, indent, key);
    out.push_str(": {\n");
    write_u32_field(out, indent + 1, "schema_version", metadata.schema_version);
    out.push_str(",\n");
    write_string_field(out, indent + 1, "generated_at", &metadata.generated_at)?;
    out.push_str(",\n");
    write_source_metadata(out, indent + 1, "source", &metadata.source)?;
    out.push('\n');
    indent_line(out, indent);
    out.push('}');
    Ok(())
}

fn write_source_metadata(
    out: &mut String,
    indent: usize,
    key: &str,
    source: &SourceMetadata,
) -> Result<(), CatalogError> {
    write_key(out, indent, key);
    out.push_str(": {\n");
    write_string_field(out, indent + 1, "api_base_url", &source.api_base_url)?;
    out.push_str(",\n");
    write_string_field(out, indent + 1, "country", &source.country)?;
    out.push_str(",\n");
    write_string_field(out, indent + 1, "language", &source.language)?;
    out.push_str(",\n");
    write_source_endpoints(out, indent + 1, "endpoints", &source.endpoints)?;
    out.push('\n');
    indent_line(out, indent);
    out.push('}');
    Ok(())
}

fn write_source_endpoints(
    out: &mut String,
    indent: usize,
    key: &str,
    endpoints: &[SourceEndpoint],
) -> Result<(), CatalogError> {
    write_key(out, indent, key);
    out.push_str(": [");
    if !endpoints.is_empty() {
        out.push('\n');
        for (index, endpoint) in endpoints.iter().enumerate() {
            indent_line(out, indent + 1);
            out.push_str("{\n");
            write_string_field(out, indent + 2, "method", &endpoint.method)?;
            out.push_str(",\n");
            write_string_field(out, indent + 2, "path", &endpoint.path)?;
            out.push('\n');
            indent_line(out, indent + 1);
            out.push('}');
            write_array_separator(out, index, endpoints.len());
        }
        indent_line(out, indent);
    }
    out.push(']');
    Ok(())
}

fn write_bike_variants(
    out: &mut String,
    indent: usize,
    key: &str,
    variants: &[BikeVariant],
) -> Result<(), CatalogError> {
    write_key(out, indent, key);
    out.push_str(": [");
    if !variants.is_empty() {
        out.push('\n');
        for (index, variant) in variants.iter().enumerate() {
            indent_line(out, indent + 1);
            out.push_str("{\n");
            write_string_field(out, indent + 2, "id", &variant.id)?;
            out.push_str(",\n");
            write_string_field(out, indent + 2, "code", &variant.code)?;
            if let Some(display_name) = &variant.display_name {
                out.push_str(",\n");
                write_string_field(out, indent + 2, "display_name", display_name)?;
            }
            out.push('\n');
            indent_line(out, indent + 1);
            out.push('}');
            write_array_separator(out, index, variants.len());
        }
        indent_line(out, indent);
    }
    out.push(']');
    Ok(())
}

fn write_catalog_trees(
    out: &mut String,
    indent: usize,
    key: &str,
    trees: &[BikeCatalogTree],
) -> Result<(), CatalogError> {
    write_key(out, indent, key);
    out.push_str(": [");
    if !trees.is_empty() {
        out.push('\n');
        for (index, tree) in trees.iter().enumerate() {
            indent_line(out, indent + 1);
            out.push_str("{\n");
            write_string_field(out, indent + 2, "bike_variant_id", &tree.bike_variant_id)?;
            out.push_str(",\n");
            write_categories(out, indent + 2, "categories", &tree.categories)?;
            out.push('\n');
            indent_line(out, indent + 1);
            out.push('}');
            write_array_separator(out, index, trees.len());
        }
        indent_line(out, indent);
    }
    out.push(']');
    Ok(())
}

fn write_categories(
    out: &mut String,
    indent: usize,
    key: &str,
    categories: &[CategoryNode],
) -> Result<(), CatalogError> {
    write_key(out, indent, key);
    out.push_str(": [");
    if !categories.is_empty() {
        out.push('\n');
        for (index, category) in categories.iter().enumerate() {
            indent_line(out, indent + 1);
            out.push_str("{\n");
            write_string_field(out, indent + 2, "code", &category.code)?;
            out.push_str(",\n");
            write_string_array_field(out, indent + 2, "path", &category.path)?;
            write_optional_string_after(out, indent + 2, "display_name", &category.display_name)?;
            write_optional_string_after(
                out,
                indent + 2,
                "localization_key",
                &category.localization_key,
            )?;
            out.push_str(",\n");
            write_categories(out, indent + 2, "categories", &category.categories)?;
            out.push_str(",\n");
            write_product_groups(out, indent + 2, "product_groups", &category.product_groups)?;
            out.push('\n');
            indent_line(out, indent + 1);
            out.push('}');
            write_array_separator(out, index, categories.len());
        }
        indent_line(out, indent);
    }
    out.push(']');
    Ok(())
}

fn write_product_groups(
    out: &mut String,
    indent: usize,
    key: &str,
    groups: &[ProductGroup],
) -> Result<(), CatalogError> {
    write_key(out, indent, key);
    out.push_str(": [");
    if !groups.is_empty() {
        out.push('\n');
        for (index, group) in groups.iter().enumerate() {
            indent_line(out, indent + 1);
            out.push_str("{\n");
            write_string_field(out, indent + 2, "code", &group.code)?;
            write_optional_string_after(out, indent + 2, "display_name", &group.display_name)?;
            write_optional_string_after(out, indent + 2, "description", &group.description)?;
            write_optional_string_after(
                out,
                indent + 2,
                "localization_key",
                &group.localization_key,
            )?;
            write_optional_string_after(
                out,
                indent + 2,
                "description_localization_key",
                &group.description_localization_key,
            )?;
            write_optional_string_after(out, indent + 2, "stark_url", &group.stark_url)?;
            out.push_str(",\n");
            write_string_array_field(out, indent + 2, "image_urls", &group.image_urls)?;
            out.push_str(",\n");
            write_articles(out, indent + 2, "articles", &group.articles)?;
            if !group.category_paths.is_empty() {
                out.push_str(",\n");
                write_key(out, indent + 2, "category_paths");
                out.push_str(": [\n");
                for (index, path) in group.category_paths.iter().enumerate() {
                    indent_line(out, indent + 3);
                    out.push('[');
                    out.push_str(
                        &path
                            .iter()
                            .map(serde_json::to_string)
                            .collect::<Result<Vec<_>, _>>()?
                            .join(", "),
                    );
                    out.push(']');
                    write_array_separator(out, index, group.category_paths.len());
                }
                indent_line(out, indent + 2);
                out.push(']');
            }
            out.push('\n');
            indent_line(out, indent + 1);
            out.push('}');
            write_array_separator(out, index, groups.len());
        }
        indent_line(out, indent);
    }
    out.push(']');
    Ok(())
}

fn write_articles(
    out: &mut String,
    indent: usize,
    key: &str,
    articles: &[Article],
) -> Result<(), CatalogError> {
    write_key(out, indent, key);
    out.push_str(": [");
    if !articles.is_empty() {
        out.push('\n');
        for (index, article) in articles.iter().enumerate() {
            indent_line(out, indent + 1);
            out.push_str("{\n");
            write_string_field(out, indent + 2, "code", &article.code)?;
            write_optional_string_after(out, indent + 2, "display_name", &article.display_name)?;
            write_optional_string_after(out, indent + 2, "description", &article.description)?;
            write_optional_string_after(
                out,
                indent + 2,
                "localization_key",
                &article.localization_key,
            )?;
            write_optional_string_after(
                out,
                indent + 2,
                "description_localization_key",
                &article.description_localization_key,
            )?;
            write_optional_string_after(out, indent + 2, "stark_url", &article.stark_url)?;
            out.push_str(",\n");
            write_string_array_field(out, indent + 2, "image_urls", &article.image_urls)?;
            out.push_str(",\n");
            write_string_array_field(out, indent + 2, "kit_memberships", &article.kit_memberships)?;
            out.push_str(",\n");
            write_string_array_field(out, indent + 2, "kit_contents", &article.kit_contents)?;
            out.push_str(",\n");
            write_article_variants(out, indent + 2, "variants", &article.variants)?;
            if !article.fitments.is_empty() {
                out.push_str(",\n");
                write_fitments(out, indent + 2, &article.fitments)?;
            }
            out.push('\n');
            indent_line(out, indent + 1);
            out.push('}');
            write_array_separator(out, index, articles.len());
        }
        indent_line(out, indent);
    }
    out.push(']');
    Ok(())
}

/// Write condition maps in sorted order so upstream object ordering is irrelevant.
fn write_fitments(
    out: &mut String,
    indent: usize,
    fitments: &[Fitment],
) -> Result<(), CatalogError> {
    write_key(out, indent, "fitments");
    out.push_str(": [\n");
    for (index, fitment) in fitments.iter().enumerate() {
        indent_line(out, indent + 1);
        out.push_str("{\n");
        write_string_field(out, indent + 2, "bike_variant_id", &fitment.bike_variant_id)?;
        out.push_str(",\n");
        write_key(out, indent + 2, "conditions");
        out.push_str(": {");
        if !fitment.conditions.is_empty() {
            out.push('\n');
            for (position, (key, value)) in fitment.conditions.iter().enumerate() {
                indent_line(out, indent + 3);
                out.push_str(&serde_json::to_string(key)?);
                out.push_str(": ");
                out.push_str(&serde_json::to_string(value)?);
                write_array_separator(out, position, fitment.conditions.len());
            }
            indent_line(out, indent + 2);
        }
        out.push_str("}\n");
        indent_line(out, indent + 1);
        out.push('}');
        write_array_separator(out, index, fitments.len());
    }
    indent_line(out, indent);
    out.push(']');
    Ok(())
}

fn write_article_variants(
    out: &mut String,
    indent: usize,
    key: &str,
    variants: &[ArticleVariant],
) -> Result<(), CatalogError> {
    write_key(out, indent, key);
    out.push_str(": [");
    if !variants.is_empty() {
        out.push('\n');
        for (index, variant) in variants.iter().enumerate() {
            indent_line(out, indent + 1);
            out.push_str("{\n");
            write_string_field(out, indent + 2, "code", &variant.code)?;
            write_optional_string_after(out, indent + 2, "sku", &variant.sku)?;
            write_optional_string_after(out, indent + 2, "stark_url", &variant.stark_url)?;
            out.push_str(",\n");
            write_string_array_field(out, indent + 2, "image_urls", &variant.image_urls)?;
            out.push_str(",\n");
            write_attributes(out, indent + 2, "attributes", &variant.attributes)?;
            if let Some(price) = &variant.price {
                out.push_str(",\n");
                write_price(out, indent + 2, "price", price)?;
            }
            if let Some(availability) = &variant.availability {
                out.push_str(",\n");
                write_availability(out, indent + 2, "availability", availability)?;
            }
            write_optional_string_after(out, indent + 2, "description", &variant.description)?;
            write_optional_string_after(
                out,
                indent + 2,
                "description_localization_key",
                &variant.description_localization_key,
            )?;
            if let Some(contents) = &variant.kit_contents {
                // An empty override still suppresses legacy article contents.
                out.push_str(",\n");
                write_string_array_field(out, indent + 2, "kit_contents", contents)?;
            }
            out.push('\n');
            indent_line(out, indent + 1);
            out.push('}');
            write_array_separator(out, index, variants.len());
        }
        indent_line(out, indent);
    }
    out.push(']');
    Ok(())
}

fn write_attributes(
    out: &mut String,
    indent: usize,
    key: &str,
    attributes: &[AttributeSelection],
) -> Result<(), CatalogError> {
    write_key(out, indent, key);
    out.push_str(": [");
    if !attributes.is_empty() {
        out.push('\n');
        for (index, attribute) in attributes.iter().enumerate() {
            indent_line(out, indent + 1);
            out.push_str("{\n");
            write_string_field(out, indent + 2, "code", &attribute.code)?;
            out.push_str(",\n");
            write_string_field(out, indent + 2, "option_code", &attribute.option_code)?;
            if let Some(display_name) = &attribute.option_display_name {
                out.push_str(",\n");
                write_string_field(out, indent + 2, "option_display_name", display_name)?;
            }
            if let Some(localization_key) = &attribute.option_localization_key {
                out.push_str(",\n");
                write_string_field(out, indent + 2, "option_localization_key", localization_key)?;
            }
            out.push('\n');
            indent_line(out, indent + 1);
            out.push('}');
            write_array_separator(out, index, attributes.len());
        }
        indent_line(out, indent);
    }
    out.push(']');
    Ok(())
}

fn write_price(
    out: &mut String,
    indent: usize,
    key: &str,
    price: &Price,
) -> Result<(), CatalogError> {
    write_key(out, indent, key);
    out.push_str(": {\n");
    write_i64_field(out, indent + 1, "amount_minor", price.amount_minor);
    out.push_str(",\n");
    write_string_field(out, indent + 1, "currency", &price.currency)?;
    out.push('\n');
    indent_line(out, indent);
    out.push('}');
    Ok(())
}

fn write_availability(
    out: &mut String,
    indent: usize,
    key: &str,
    availability: &Availability,
) -> Result<(), CatalogError> {
    write_key(out, indent, key);
    out.push_str(": {\n");
    write_string_field(out, indent + 1, "status", &availability.status)?;
    if let Some(quantity) = availability.quantity {
        out.push_str(",\n");
        write_u32_field(out, indent + 1, "quantity", quantity);
    }
    out.push('\n');
    indent_line(out, indent);
    out.push('}');
    Ok(())
}

fn write_string_array_field(
    out: &mut String,
    indent: usize,
    key: &str,
    values: &[String],
) -> Result<(), CatalogError> {
    write_key(out, indent, key);
    out.push_str(": [");
    for (index, value) in values.iter().enumerate() {
        if index > 0 {
            out.push_str(", ");
        }
        write!(out, "{}", serde_json::to_string(value)?).expect("writing to String cannot fail");
    }
    out.push(']');
    Ok(())
}

fn write_optional_string_after(
    out: &mut String,
    indent: usize,
    key: &str,
    value: &Option<String>,
) -> Result<(), CatalogError> {
    if let Some(value) = value {
        out.push_str(",\n");
        write_string_field(out, indent, key, value)?;
    }
    Ok(())
}

fn write_string_field(
    out: &mut String,
    indent: usize,
    key: &str,
    value: &str,
) -> Result<(), CatalogError> {
    write_key(out, indent, key);
    write!(out, ": {}", serde_json::to_string(value)?).expect("writing to String cannot fail");
    Ok(())
}

fn write_u32_field(out: &mut String, indent: usize, key: &str, value: u32) {
    write_key(out, indent, key);
    write!(out, ": {value}").expect("writing to String cannot fail");
}

fn write_i64_field(out: &mut String, indent: usize, key: &str, value: i64) {
    write_key(out, indent, key);
    write!(out, ": {value}").expect("writing to String cannot fail");
}

fn write_key(out: &mut String, indent: usize, key: &str) {
    indent_line(out, indent);
    out.push_str(key);
}

fn write_array_separator(out: &mut String, index: usize, len: usize) {
    if index + 1 == len {
        out.push('\n');
    } else {
        out.push_str(",\n");
    }
}

fn indent_line(out: &mut String, indent: usize) {
    for _ in 0..indent {
        out.push_str("  ");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;
    #[cfg(feature = "http")]
    use std::io::{Read, Write};
    #[cfg(feature = "http")]
    use std::net::TcpListener;
    #[cfg(feature = "http")]
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };
    #[cfg(feature = "http")]
    use std::thread;

    #[test]
    /// Both snapshot shapes retain URL-stable filters while the generated data is migrated separately.
    fn generated_catalog_snapshot_covers_discovered_public_variants() {
        let catalog = generated_catalog_snapshot();
        let variant_ids = catalog
            .bike_variants
            .iter()
            .map(|variant| variant.id.as_str())
            .collect::<Vec<_>>();
        let tree_ids = catalog
            .catalog_trees
            .iter()
            .map(|tree| tree.bike_variant_id.as_str())
            .collect::<HashSet<_>>();

        assert!(variant_ids.contains(&"varg-ex"));
        if !catalog.products.is_empty() {
            assert!(catalog.catalog_trees.is_empty());
            assert!(!catalog.categories.is_empty());
            return;
        }
        assert_eq!(tree_ids.len(), variant_ids.len());
        for variant_id in variant_ids {
            assert!(
                tree_ids.contains(variant_id),
                "missing generated catalog tree for {variant_id}"
            );
        }
    }

    #[test]
    /// Committed snapshots must describe their actual source and contain usable part links.
    fn generated_catalog_snapshot_has_sane_metadata_and_content() {
        let catalog = generated_catalog_snapshot();

        assert!(matches!(catalog.metadata.schema_version, 1 | 2));
        if !catalog.products.is_empty() {
            assert_eq!(
                catalog.metadata.source.api_base_url,
                "https://api.starkfuture.com/v3/product-catalog"
            );
            assert_eq!(catalog.metadata.source.country, "US");
            for product in &catalog.products {
                assert!(
                    product
                        .stark_url
                        .as_ref()
                        .unwrap()
                        .starts_with("https://starkfuture.com/parts-and-accessories/product/")
                );
                assert!(!product.articles.is_empty());
                assert!(!product.articles[0].variants.is_empty());
            }
            return;
        }
        assert_eq!(
            catalog.metadata.source.api_base_url,
            "https://api.starkfuture.com/v2"
        );
        assert_eq!(catalog.metadata.source.country, "US");
        assert_eq!(catalog.metadata.source.language, "en");
        assert_eq!(
            catalog
                .metadata
                .source
                .endpoints
                .iter()
                .map(|endpoint| endpoint.path.as_str())
                .collect::<Vec<_>>(),
            [
                "/store/categories",
                "/store/products",
                "/store/products/{code}"
            ]
        );

        for tree in &catalog.catalog_trees {
            let (product_group_count, article_count, sku_count) =
                count_generated_catalog_content(&tree.categories);
            assert!(
                product_group_count > 0,
                "generated tree for {} has no product groups",
                tree.bike_variant_id
            );
            assert!(
                article_count > 0,
                "generated tree for {} has no articles",
                tree.bike_variant_id
            );
            assert!(
                sku_count > 0,
                "generated tree for {} has no SKUs",
                tree.bike_variant_id
            );
            assert_product_group_urls_are_specific(&tree.bike_variant_id, &tree.categories);
        }
    }

    #[test]
    fn generated_catalog_snapshot_is_canonical_json5() {
        let bytes = include_str!("../../../catalog/stark-parts.json5");
        let catalog = parse_catalog_json5(bytes).unwrap();

        assert_eq!(format_catalog_json5(&catalog).unwrap(), bytes);
    }

    #[test]
    fn parses_json5_comments_and_round_trips_to_deterministic_bytes() {
        let input = r#"
        // Stark's source order is preserved inside tree arrays.
        {
          catalog_trees: [{
            categories: [{
              product_groups: [{
                articles: [{
                  variants: [{
                    availability: { quantity: 3, status: "in_stock" },
                    price: { currency: "USD", amount_minor: 1299 },
                    attributes: [{
                      option_display_name: "Black",
                      option_localization_key: "catalog.option.black",
                      option_code: "black",
                      code: "color",
                  }],
                    sku: "SP-123",
                    image_urls: ["https://s3-stark-prod.s3.eu-central-1.amazonaws.com/catalog/washer-variant.png"],
                    stark_url: "https://www.starkfuture.com/us-US/parts/washer?variant=black",
                    code: "washer-black",
                  }],
                  kit_contents: ["bolt", "washer"],
                  kit_memberships: ["frame-service-kit"],
                  image_urls: ["https://s3-stark-prod.s3.eu-central-1.amazonaws.com/catalog/washer-article.png"],
                  stark_url: "https://www.starkfuture.com/us-US/parts/washer",
                  description_localization_key: "catalog.article.washer.description",
                  localization_key: "catalog.article.washer",
                  description: "Replacement washer",
                  display_name: "Washer",
                  code: "washer",
                }],
                image_urls: ["https://s3-stark-prod.s3.eu-central-1.amazonaws.com/catalog/washer.png"],
                stark_url: "https://www.starkfuture.com/us-US/parts/washer?sku=SP-123",
                description_localization_key: "catalog.group.fasteners.description",
                localization_key: "catalog.group.fasteners",
                description: "Fastener group",
                display_name: "Fasteners",
                code: "fasteners",
              }],
              categories: [],
              localization_key: "catalog.category.frame",
              display_name: "Frame",
              path: ["SP", "FRAME"],
              code: "FRAME",
            }],
            bike_variant_id: "varg-ex",
          }],
          bike_variants: [{
            display_name: "Varg EX",
            code: "varg-ex",
            id: "varg-ex",
          }],
          metadata: {
            source: {
              endpoints: [{ path: "/store/products/washer", method: "GET" }],
              language: "en",
              country: "US",
              api_base_url: "https://api.starkfuture.com/v2",
            },
            generated_at: "2026-05-26T12:34:56Z",
            schema_version: 1,
          },
        }
        "#;

        let catalog = parse_catalog_json5(input).unwrap();
        let first = format_catalog_json5(&catalog).unwrap();
        let second = format_catalog_json5(&parse_catalog_json5(&first).unwrap()).unwrap();

        assert_eq!(first, second);
        assert!(first.contains("schema_version: 1"));
        assert!(first.find("metadata").unwrap() < first.find("bike_variants").unwrap());
        assert!(first.find("code: \"FRAME\"").unwrap() < first.find("product_groups").unwrap());
        assert!(
            first.find("amount_minor: 1299").unwrap() < first.find("currency: \"USD\"").unwrap()
        );
    }

    fn generated_catalog_snapshot() -> Catalog {
        parse_catalog_json5(include_str!("../../../catalog/stark-parts.json5")).unwrap()
    }

    fn count_generated_catalog_content(categories: &[CategoryNode]) -> (usize, usize, usize) {
        let mut product_group_count = 0;
        let mut article_count = 0;
        let mut sku_count = 0;
        for category in categories {
            product_group_count += category.product_groups.len();
            for group in &category.product_groups {
                article_count += group.articles.len();
                for article in &group.articles {
                    sku_count += article
                        .variants
                        .iter()
                        .filter(|variant| variant.sku.is_some())
                        .count();
                }
            }
            let (child_product_groups, child_articles, child_skus) =
                count_generated_catalog_content(&category.categories);
            product_group_count += child_product_groups;
            article_count += child_articles;
            sku_count += child_skus;
        }

        (product_group_count, article_count, sku_count)
    }

    fn assert_product_group_urls_are_specific(bike_variant_id: &str, categories: &[CategoryNode]) {
        for category in categories {
            for group in &category.product_groups {
                let url = group
                    .stark_url
                    .as_deref()
                    .expect("generated product groups should have Stark URLs");
                assert!(
                    url.starts_with(&format!("{STARK_SPARE_PARTS_URL}/{bike_variant_id}/")),
                    "product-group URL should include its bike variant: {url}"
                );
                assert!(
                    url.ends_with(&format!("/{}", group.code)),
                    "product-group URL should end with its product code: {url}"
                );
                assert_ne!(
                    url,
                    format!("{STARK_SPARE_PARTS_URL}/{bike_variant_id}"),
                    "product-group URL must not be the bike overview"
                );
            }
            assert_product_group_urls_are_specific(bike_variant_id, &category.categories);
        }
    }

    #[test]
    fn validates_allowed_image_hosts() {
        let mut catalog = representative_catalog();
        catalog.catalog_trees[0].categories[0].product_groups[0].image_urls =
            vec!["https://assets.starkfuture.com/spare-parts-images/washer.png".to_owned()];

        validate_catalog(&catalog).unwrap();

        catalog.catalog_trees[0].categories[0].product_groups[0].image_urls =
            vec!["https://example.com/catalog/washer.png".to_owned()];

        assert!(matches!(
            validate_catalog(&catalog),
            Err(CatalogError::InvalidImageUrl { .. })
        ));
    }

    #[test]
    fn rejects_unsafe_stark_links() {
        let mut catalog = representative_catalog();
        catalog.catalog_trees[0].categories[0].product_groups[0].stark_url =
            Some("https://www.starkfuture.com/us-US/parts/washer#fragment".to_owned());

        assert!(matches!(
            validate_catalog(&catalog),
            Err(CatalogError::InvalidStarkUrl { .. })
        ));
    }

    #[test]
    fn validates_article_and_variant_media_fields() {
        let mut catalog = representative_catalog();
        let article = &mut catalog.catalog_trees[0].categories[0].product_groups[0].articles[0];
        article.stark_url = Some("http://www.starkfuture.com/us-US/parts/washer".to_owned());

        assert!(matches!(
            validate_catalog(&catalog),
            Err(CatalogError::InvalidStarkUrl { .. })
        ));

        let mut catalog = representative_catalog();
        let variant =
            &mut catalog.catalog_trees[0].categories[0].product_groups[0].articles[0].variants[0];
        variant.image_urls = vec!["https://example.com/catalog/washer.png".to_owned()];

        assert!(matches!(
            validate_catalog(&catalog),
            Err(CatalogError::InvalidImageUrl { .. })
        ));
    }

    #[test]
    fn rejects_catalog_trees_without_matching_variants() {
        let mut catalog = representative_catalog();
        catalog.catalog_trees.clear();

        assert!(matches!(
            validate_catalog(&catalog),
            Err(CatalogError::MissingCatalogTree { .. })
        ));
    }

    #[test]
    fn rejects_catalog_trees_for_unknown_variants() {
        let mut catalog = representative_catalog();
        catalog.catalog_trees.push(BikeCatalogTree {
            bike_variant_id: "unknown".to_owned(),
            categories: Vec::new(),
        });

        assert!(matches!(
            validate_catalog(&catalog),
            Err(CatalogError::UnknownTreeVariant { .. })
        ));
    }

    #[test]
    fn rejects_unsupported_source_endpoint_methods() {
        let mut catalog = representative_catalog();
        catalog.metadata.source.endpoints[0].method = "PUT".to_owned();

        assert!(matches!(
            validate_catalog(&catalog),
            Err(CatalogError::InvalidEndpointMethod { .. })
        ));
    }

    #[test]
    fn rejects_source_endpoint_paths_without_leading_slash() {
        let mut catalog = representative_catalog();
        catalog.metadata.source.endpoints[0].path = "store/products".to_owned();

        assert!(matches!(
            validate_catalog(&catalog),
            Err(CatalogError::InvalidEndpointPath { .. })
        ));
    }

    #[test]
    fn generated_timestamps_are_validated_but_not_rewritten() {
        let mut catalog = representative_catalog();
        catalog.metadata.generated_at = "2026-05-26T12:34:56.123Z".to_owned();

        let formatted = format_catalog_json5(&catalog).unwrap();

        assert!(formatted.contains("generated_at: \"2026-05-26T12:34:56.123Z\""));
    }

    #[test]
    #[cfg(feature = "http")]
    fn reqwest_transport_builds_stark_api_urls_without_live_network() {
        let transport = ReqwestTransport::new("https://api.starkfuture.com/v2").unwrap();
        let url = transport
            .build_url(
                "/store/products",
                &[("category", "brakes_front_brake"), ("tags", "varg-ex")],
            )
            .unwrap();

        assert_eq!(
            url.as_str(),
            "https://api.starkfuture.com/v2/store/products?category=brakes_front_brake&tags=varg-ex"
        );
    }

    #[test]
    #[cfg(feature = "http")]
    fn reqwest_transport_retries_transient_server_errors() {
        let (base_url, request_count) =
            status_sequence_server(&[(500, "server error"), (200, r#"{"ok":true}"#)]);
        let transport = ReqwestTransport::new(&base_url).unwrap();

        let value = transport.get_json("/store/products", &[]).unwrap();

        assert_eq!(value, serde_json::json!({ "ok": true }));
        assert_eq!(request_count.load(Ordering::SeqCst), 2);
    }

    #[test]
    #[cfg(feature = "http")]
    fn reqwest_transport_does_not_retry_deterministic_client_errors() {
        let (base_url, request_count) = status_sequence_server(&[(404, "missing")]);
        let transport = ReqwestTransport::new(&base_url).unwrap();

        let error = transport.get_json("/store/products", &[]).unwrap_err();

        assert!(error.to_string().contains("Stark API request failed"));
        assert_eq!(request_count.load(Ordering::SeqCst), 1);
    }

    #[test]
    #[cfg(feature = "http")]
    fn reqwest_transport_rejects_invalid_base_urls_without_live_network() {
        let error = ReqwestTransport::new("not a url").unwrap_err();

        assert!(error.to_string().contains("invalid Stark API base URL"));
    }

    fn representative_catalog() -> Catalog {
        Catalog {
            categories: Vec::new(),
            products: Vec::new(),
            metadata: CatalogMetadata {
                schema_version: 1,
                generated_at: "2026-05-26T12:34:56Z".to_owned(),
                source: SourceMetadata {
                    api_base_url: "https://api.starkfuture.com/v2".to_owned(),
                    country: "US".to_owned(),
                    language: "en".to_owned(),
                    endpoints: vec![SourceEndpoint {
                        method: "GET".to_owned(),
                        path: "/store/products/washer".to_owned(),
                    }],
                },
            },
            bike_variants: vec![BikeVariant {
                id: "varg-ex".to_owned(),
                code: "varg-ex".to_owned(),
                display_name: Some("Varg EX".to_owned()),
            }],
            catalog_trees: vec![BikeCatalogTree {
                bike_variant_id: "varg-ex".to_owned(),
                categories: vec![CategoryNode {
                    code: "FRAME".to_owned(),
                    path: vec!["SP".to_owned(), "FRAME".to_owned()],
                    display_name: Some("Frame".to_owned()),
                    localization_key: Some("catalog.category.frame".to_owned()),
                    categories: Vec::new(),
                    product_groups: vec![ProductGroup {
            category_paths: Vec::new(),
                        code: "fasteners".to_owned(),
                        display_name: Some("Fasteners".to_owned()),
                        description: Some("Fastener group".to_owned()),
                        localization_key: Some("catalog.group.fasteners".to_owned()),
                        description_localization_key: Some("catalog.group.fasteners.description".to_owned()),
                        stark_url: Some("https://www.starkfuture.com/us-US/parts/washer?sku=SP-123".to_owned()),
                        image_urls: vec![
                            "https://s3-stark-prod.s3.eu-central-1.amazonaws.com/catalog/washer.png".to_owned(),
                        ],
                        articles: vec![Article {
                fitments: Vec::new(),
                            code: "washer".to_owned(),
                            display_name: Some("Washer".to_owned()),
                            description: Some("Replacement washer".to_owned()),
                            localization_key: Some("catalog.article.washer".to_owned()),
                            description_localization_key: Some("catalog.article.washer.description".to_owned()),
                            stark_url: Some("https://www.starkfuture.com/us-US/parts/washer".to_owned()),
                            image_urls: vec![
                                "https://s3-stark-prod.s3.eu-central-1.amazonaws.com/catalog/washer-article.png"
                                    .to_owned(),
                            ],
                            kit_memberships: vec!["frame-service-kit".to_owned()],
                            kit_contents: vec!["bolt".to_owned(), "washer".to_owned()],
                            variants: vec![ArticleVariant {
                                description: None,
                                description_localization_key: None,
                                kit_contents: None,
                                code: "washer-black".to_owned(),
                                sku: Some("SP-123".to_owned()),
                                stark_url: Some(
                                    "https://www.starkfuture.com/us-US/parts/washer?variant=black".to_owned(),
                                ),
                                image_urls: vec![
                                    "https://s3-stark-prod.s3.eu-central-1.amazonaws.com/catalog/washer-variant.png"
                                        .to_owned(),
                                ],
                                attributes: vec![AttributeSelection {
                                    code: "color".to_owned(),
                                    option_code: "black".to_owned(),
                                    option_display_name: Some("Black".to_owned()),
                                    option_localization_key: Some("catalog.option.black".to_owned()),
                                }],
                                price: Some(Price {
                                    amount_minor: 1299,
                                    currency: "USD".to_owned(),
                                }),
                                availability: Some(Availability {
                                    status: "in_stock".to_owned(),
                                    quantity: Some(3),
                                }),
                            }],
                        }],
                    }],
                }],
            }],
        }
    }

    #[cfg(feature = "http")]
    fn status_sequence_server(statuses: &[(u16, &str)]) -> (String, Arc<AtomicUsize>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let statuses = statuses
            .iter()
            .map(|(status, body)| (*status, (*body).to_owned()))
            .collect::<Vec<_>>();
        let request_count = Arc::new(AtomicUsize::new(0));
        let server_request_count = Arc::clone(&request_count);

        thread::spawn(move || {
            for (status, body) in statuses {
                let (mut stream, _) = listener.accept().unwrap();
                let mut request = [0; 1024];
                let _ = stream.read(&mut request);
                server_request_count.fetch_add(1, Ordering::SeqCst);

                let reason = match status {
                    200 => "OK",
                    404 => "Not Found",
                    500 => "Internal Server Error",
                    _ => "Status",
                };
                let response = format!(
                    "HTTP/1.1 {status} {reason}\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
                    body.len()
                );
                stream.write_all(response.as_bytes()).unwrap();
            }
        });

        (format!("http://{address}"), request_count)
    }
}

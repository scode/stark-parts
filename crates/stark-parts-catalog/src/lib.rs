//! Committed catalog schema, validation, and deterministic JSON5 formatting.
//!
//! This crate owns the project schema rather than mirroring Stark's upstream
//! API. The crawler can change how it talks to Stark without making the
//! committed file noisy or unstable, while the web app can depend on this
//! smaller contract for search and rendering.

use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fmt::Write as _;
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;
use url::Url;

const SCHEMA_VERSION: u32 = 1;
const MAX_CATEGORY_DEPTH: usize = 32;
const ALLOWED_IMAGE_HOSTS: &[&str] = &["s3-stark-prod.s3.eu-central-1.amazonaws.com"];
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

/// Configuration shared by the trait-backed crawler core and later HTTP client.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CrawlConfig {
    pub api_base_url: String,
    pub country: String,
    pub language: String,
    pub generated_at: String,
    pub root_path: String,
}

impl CrawlConfig {
    pub fn us_storefront(generated_at: impl Into<String>) -> Self {
        Self {
            api_base_url: "https://api.starkfuture.com/v2".to_owned(),
            country: "US".to_owned(),
            language: "en".to_owned(),
            generated_at: generated_at.into(),
            root_path: "SP".to_owned(),
        }
    }
}

/// Upstream access boundary for Stark catalog data.
///
/// Tests implement this trait with fixtures. The real HTTP client in the next
/// plan step should be a thin adapter around these calls, so traversal and
/// schema transformation stay independent from networking.
pub trait UpstreamCatalog {
    fn bike_variants(&self) -> UpstreamResult<Vec<UpstreamBikeVariant>>;

    fn categories(&self, tag: &str, path: &str) -> UpstreamResult<Vec<UpstreamCategory>>;

    fn products(
        &self,
        tag: &str,
        category_code: &str,
    ) -> UpstreamResult<Vec<UpstreamProductSummary>>;

    fn product_detail(
        &self,
        tag: &str,
        country: &str,
        product_code: &str,
    ) -> UpstreamResult<UpstreamProductDetail>;
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
    #[error("upstream discovery returned no bike variants")]
    NoBikeVariants,
    #[error("upstream category code is not a safe path segment: {code}")]
    UnsafeCategoryCode { code: String },
    #[error("category traversal revisited path {path} for bike variant {tag}")]
    CategoryCycle { tag: String, path: String },
    #[error("category traversal exceeded max depth {max_depth} at path {path}")]
    CategoryDepth { max_depth: usize, path: String },
    #[error("catalog validation failed after crawl: {0}")]
    Catalog(#[from] CatalogError),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UpstreamBikeVariant {
    pub tag: String,
    pub display_name: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UpstreamCategory {
    pub code: String,
    pub name_key: Option<String>,
    pub display_name: Option<String>,
    pub image_url: Option<String>,
    pub is_leaf: bool,
    pub path: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UpstreamProductSummary {
    pub code: String,
    pub name_key: Option<String>,
    pub description_key: Option<String>,
    pub display_name: Option<String>,
    pub description: Option<String>,
    pub image_url: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UpstreamProductDetail {
    pub code: String,
    pub name_key: Option<String>,
    pub description_key: Option<String>,
    pub display_name: Option<String>,
    pub description: Option<String>,
    pub feature_image_url: Option<String>,
    pub articles: Vec<UpstreamArticleEntry>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UpstreamArticleEntry {
    pub reference: Option<u32>,
    pub article: UpstreamArticle,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UpstreamArticle {
    pub code: String,
    pub name_key: Option<String>,
    pub description_key: Option<String>,
    pub display_name: Option<String>,
    pub description: Option<String>,
    pub image_url: Option<String>,
    pub tags: Vec<String>,
    pub is_kit: bool,
    pub kit_contain: Vec<String>,
    pub variants: Vec<UpstreamArticleVariant>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UpstreamArticleVariant {
    pub code: String,
    pub skus: Vec<String>,
    pub availability: Option<String>,
    pub price: Option<UpstreamPrice>,
    pub attributes: Vec<UpstreamAttributeSelection>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UpstreamPrice {
    pub total_minor: i64,
    pub currency: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UpstreamAttributeSelection {
    pub attribute_code: String,
    pub option_code: String,
    pub option_display_name: Option<String>,
    pub option_name_key: Option<String>,
}

/// Crawl all discovered bike variants through the upstream trait.
pub fn crawl_catalog(
    client: &impl UpstreamCatalog,
    config: &CrawlConfig,
) -> Result<Catalog, CrawlError> {
    let variants = client.bike_variants()?;
    if variants.is_empty() {
        return Err(CrawlError::NoBikeVariants);
    }

    let bike_variants = variants
        .iter()
        .map(|variant| BikeVariant {
            id: variant.tag.clone(),
            code: variant.tag.clone(),
            display_name: variant.display_name.clone(),
        })
        .collect::<Vec<_>>();

    let mut catalog_trees = Vec::with_capacity(variants.len());
    for variant in &variants {
        let mut visited_paths = HashSet::new();
        let categories = crawl_categories(
            client,
            config,
            &variant.tag,
            &config.root_path,
            Vec::new(),
            &mut visited_paths,
        )?;
        catalog_trees.push(BikeCatalogTree {
            bike_variant_id: variant.tag.clone(),
            categories,
        });
    }

    let catalog = Catalog {
        metadata: CatalogMetadata {
            schema_version: SCHEMA_VERSION,
            generated_at: config.generated_at.clone(),
            source: SourceMetadata {
                api_base_url: config.api_base_url.clone(),
                country: config.country.clone(),
                language: config.language.clone(),
                endpoints: vec![
                    SourceEndpoint {
                        method: "GET".to_owned(),
                        path: "/store/categories".to_owned(),
                    },
                    SourceEndpoint {
                        method: "GET".to_owned(),
                        path: "/store/products".to_owned(),
                    },
                    SourceEndpoint {
                        method: "GET".to_owned(),
                        path: "/store/products/{code}".to_owned(),
                    },
                ],
            },
        },
        bike_variants,
        catalog_trees,
    };

    validate_catalog(&catalog)?;
    Ok(catalog)
}

fn crawl_categories(
    client: &impl UpstreamCatalog,
    config: &CrawlConfig,
    tag: &str,
    request_path: &str,
    parent_path: Vec<String>,
    visited_paths: &mut HashSet<String>,
) -> Result<Vec<CategoryNode>, CrawlError> {
    if parent_path.len() >= MAX_CATEGORY_DEPTH {
        return Err(CrawlError::CategoryDepth {
            max_depth: MAX_CATEGORY_DEPTH,
            path: request_path.to_owned(),
        });
    }
    if !visited_paths.insert(request_path.to_owned()) {
        return Err(CrawlError::CategoryCycle {
            tag: tag.to_owned(),
            path: request_path.to_owned(),
        });
    }

    let upstream_categories = client.categories(tag, request_path)?;
    let mut categories = Vec::with_capacity(upstream_categories.len());

    for upstream in upstream_categories {
        if !is_safe_path_segment(&upstream.code) {
            return Err(CrawlError::UnsafeCategoryCode {
                code: upstream.code,
            });
        }

        let mut path = parent_path.clone();
        path.push(upstream.code.clone());

        let (child_categories, product_groups) = if upstream.is_leaf {
            let products = client.products(tag, &upstream.code)?;
            let mut groups = Vec::with_capacity(products.len());
            for product in products {
                let detail = client.product_detail(tag, &config.country, &product.code)?;
                groups.push(product_group_from_upstream(product, detail));
            }
            (Vec::new(), groups)
        } else {
            let next_request_path = format!("{request_path}/{}", upstream.code);
            (
                crawl_categories(
                    client,
                    config,
                    tag,
                    &next_request_path,
                    path.clone(),
                    visited_paths,
                )?,
                Vec::new(),
            )
        };

        categories.push(CategoryNode {
            code: upstream.code,
            path,
            display_name: upstream.display_name,
            localization_key: upstream.name_key,
            categories: child_categories,
            product_groups,
        });
    }

    Ok(categories)
}

fn is_safe_path_segment(segment: &str) -> bool {
    !segment.is_empty()
        && segment
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
}

fn product_group_from_upstream(
    summary: UpstreamProductSummary,
    detail: UpstreamProductDetail,
) -> ProductGroup {
    let image_urls = detail
        .feature_image_url
        .or(summary.image_url)
        .into_iter()
        .collect();

    ProductGroup {
        code: detail.code,
        display_name: detail.display_name.or(summary.display_name),
        description: detail.description.or(summary.description),
        localization_key: detail.name_key.or(summary.name_key),
        description_localization_key: detail.description_key.or(summary.description_key),
        stark_url: None,
        image_urls,
        articles: detail
            .articles
            .into_iter()
            .map(|entry| article_from_upstream(entry.article))
            .collect(),
    }
}

fn article_from_upstream(article: UpstreamArticle) -> Article {
    Article {
        code: article.code,
        display_name: article.display_name,
        description: article.description,
        localization_key: article.name_key,
        description_localization_key: article.description_key,
        stark_url: None,
        image_urls: article.image_url.into_iter().collect(),
        kit_memberships: Vec::new(),
        kit_contents: article.kit_contain,
        variants: article
            .variants
            .into_iter()
            .flat_map(article_variants_from_upstream)
            .collect(),
    }
}

fn article_variants_from_upstream(variant: UpstreamArticleVariant) -> Vec<ArticleVariant> {
    let price = variant.price.map(|price| Price {
        amount_minor: price.total_minor,
        currency: price.currency,
    });
    let availability = variant.availability.map(|status| Availability {
        status,
        quantity: None,
    });
    let attributes = variant
        .attributes
        .into_iter()
        .map(|attribute| AttributeSelection {
            code: attribute.attribute_code,
            option_code: attribute.option_code,
            option_display_name: attribute.option_display_name,
            option_localization_key: attribute.option_name_key,
        })
        .collect::<Vec<_>>();

    if variant.skus.is_empty() {
        return vec![ArticleVariant {
            code: variant.code,
            sku: None,
            stark_url: None,
            image_urls: Vec::new(),
            attributes,
            price,
            availability,
        }];
    }

    variant
        .skus
        .into_iter()
        .map(|sku| ArticleVariant {
            code: variant.code.clone(),
            sku: Some(sku),
            stark_url: None,
            image_urls: Vec::new(),
            attributes: attributes.clone(),
            price: price.clone(),
            availability: availability.clone(),
        })
        .collect()
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
    if catalog.metadata.schema_version != SCHEMA_VERSION {
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
        if !catalog
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

    Ok(())
}

fn validate_categories(categories: &[CategoryNode]) -> Result<(), CatalogError> {
    for category in categories {
        validate_categories(&category.categories)?;
        for group in &category.product_groups {
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
                        validate_https_url(url, Some(ALLOWED_STARK_LINK_HOSTS)).map_err(
                            |source| CatalogError::InvalidStarkUrl {
                                url: url.clone(),
                                source,
                            },
                        )?;
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
    use std::cell::RefCell;
    use std::collections::HashMap;

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

    #[test]
    fn validates_allowed_image_hosts() {
        let mut catalog = representative_catalog();
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
    fn crawler_discovers_variants_and_traverses_branch_and_leaf_categories() {
        let client = FixtureUpstream::representative();
        let catalog =
            crawl_catalog(&client, &CrawlConfig::us_storefront("2026-05-26T12:34:56Z")).unwrap();

        assert_eq!(
            catalog.bike_variants,
            vec![
                BikeVariant {
                    id: "varg-ex".to_owned(),
                    code: "varg-ex".to_owned(),
                    display_name: Some("Varg EX".to_owned()),
                },
                BikeVariant {
                    id: "varg-sm".to_owned(),
                    code: "varg-sm".to_owned(),
                    display_name: Some("Varg SM".to_owned()),
                },
            ]
        );
        assert_eq!(
            client.calls(),
            vec![
                "bike_variants",
                "categories:varg-ex:SP",
                "categories:varg-ex:SP/brakes",
                "products:varg-ex:brakes_front_brake",
                "detail:varg-ex:US:14_disc",
                "categories:varg-sm:SP",
            ]
        );

        let brakes = &catalog.catalog_trees[0].categories[0];
        assert_eq!(brakes.code, "brakes");
        assert_eq!(brakes.path, vec!["brakes"]);
        let front_brake = &brakes.categories[0];
        assert_eq!(front_brake.code, "brakes_front_brake");
        assert_eq!(front_brake.path, vec!["brakes", "brakes_front_brake"]);
        assert_eq!(front_brake.product_groups[0].articles[0].variants.len(), 2);
        assert_eq!(
            front_brake.product_groups[0].articles[0].variants[0].sku,
            Some("SMX1-BR-FW-260".to_owned())
        );
        assert_eq!(
            front_brake.product_groups[0].articles[0].variants[1].sku,
            Some("I14580-060012-08-P".to_owned())
        );
        assert_eq!(
            front_brake.product_groups[0].display_name,
            Some("Front disc".to_owned())
        );
        assert_eq!(
            front_brake.product_groups[0].description_localization_key,
            Some("spare_parts_product_14_disc_description".to_owned())
        );
        assert_eq!(
            front_brake.product_groups[0].image_urls,
            vec![
                "https://s3-stark-prod.s3.eu-central-1.amazonaws.com/spare-parts-images/Disc.webp"
            ]
        );
        assert_eq!(
            front_brake.product_groups[0].articles[0].description_localization_key,
            Some("spare_parts_product_disc_260mm_description".to_owned())
        );
        assert_eq!(
            front_brake.product_groups[0].articles[0].variants[0].attributes[0]
                .option_localization_key,
            Some("spare_parts_attribute_option_260mm_name".to_owned())
        );
    }

    #[test]
    fn crawler_preserves_region_labeled_parts_instead_of_filtering_by_tags() {
        let client = FixtureUpstream::representative();
        let catalog =
            crawl_catalog(&client, &CrawlConfig::us_storefront("2026-05-26T12:34:56Z")).unwrap();
        let article =
            &catalog.catalog_trees[0].categories[0].categories[0].product_groups[0].articles[0];

        assert_eq!(article.code, "disc_260mm");
        assert_eq!(article.kit_contents, vec!["bolt_m6"]);
        assert_eq!(article.variants[0].price.as_ref().unwrap().currency, "USD");
        assert_eq!(
            article.variants[0].availability.as_ref().unwrap().status,
            "AVAILABLE_HQ"
        );
    }

    #[test]
    fn crawler_reports_upstream_errors() {
        let mut client = FixtureUpstream::representative();
        client.categories.insert(
            ("varg-ex".to_owned(), "SP".to_owned()),
            Err(UpstreamError::new("category failed")),
        );

        assert!(matches!(
            crawl_catalog(&client, &CrawlConfig::us_storefront("2026-05-26T12:34:56Z")),
            Err(CrawlError::Upstream(_))
        ));
    }

    #[test]
    fn crawler_requires_variant_discovery() {
        let mut client = FixtureUpstream::representative();
        client.bike_variants.clear();

        assert!(matches!(
            crawl_catalog(&client, &CrawlConfig::us_storefront("2026-05-26T12:34:56Z")),
            Err(CrawlError::NoBikeVariants)
        ));
    }

    #[test]
    fn crawler_uses_trusted_request_path_for_recursion() {
        let mut client = FixtureUpstream::representative();
        client.categories.insert(
            ("varg-ex".to_owned(), "SP/brakes/controls".to_owned()),
            Ok(Vec::new()),
        );
        let categories = client
            .categories
            .get_mut(&("varg-ex".to_owned(), "SP/brakes".to_owned()))
            .unwrap()
            .as_mut()
            .unwrap();
        categories[0].is_leaf = false;
        categories[0].code = "controls".to_owned();
        categories[0].path = "SP/not-the-real-parent".to_owned();

        let catalog =
            crawl_catalog(&client, &CrawlConfig::us_storefront("2026-05-26T12:34:56Z")).unwrap();

        assert_eq!(
            client.calls(),
            vec![
                "bike_variants",
                "categories:varg-ex:SP",
                "categories:varg-ex:SP/brakes",
                "categories:varg-ex:SP/brakes/controls",
                "categories:varg-sm:SP",
            ]
        );
        assert_eq!(
            catalog.catalog_trees[0].categories[0].categories[0].path,
            vec!["brakes", "controls"]
        );
    }

    #[test]
    fn crawler_rejects_unsafe_category_codes() {
        let mut client = FixtureUpstream::representative();
        client
            .categories
            .get_mut(&("varg-ex".to_owned(), "SP".to_owned()))
            .unwrap()
            .as_mut()
            .unwrap()[0]
            .code = "../brakes".to_owned();

        assert!(matches!(
            crawl_catalog(&client, &CrawlConfig::us_storefront("2026-05-26T12:34:56Z")),
            Err(CrawlError::UnsafeCategoryCode { .. })
        ));
    }

    #[test]
    fn crawler_detects_revisited_category_paths() {
        let mut client = FixtureUpstream::representative();
        client
            .categories
            .get_mut(&("varg-ex".to_owned(), "SP/brakes".to_owned()))
            .unwrap()
            .as_mut()
            .unwrap()
            .clear();
        client
            .categories
            .get_mut(&("varg-ex".to_owned(), "SP".to_owned()))
            .unwrap()
            .as_mut()
            .unwrap()
            .push(UpstreamCategory {
                code: "brakes".to_owned(),
                name_key: Some("duplicate".to_owned()),
                display_name: Some("Duplicate brakes".to_owned()),
                image_url: None,
                is_leaf: false,
                path: "SP".to_owned(),
            });

        assert!(matches!(
            crawl_catalog(&client, &CrawlConfig::us_storefront("2026-05-26T12:34:56Z")),
            Err(CrawlError::CategoryCycle { .. })
        ));
    }

    #[test]
    fn crawler_surfaces_post_crawl_catalog_validation_errors() {
        let client = FixtureUpstream::representative();
        let mut config = CrawlConfig::us_storefront("2026-05-26T12:34:56Z");
        config.api_base_url = "http://api.starkfuture.com/v2".to_owned();

        assert!(matches!(
            crawl_catalog(&client, &config),
            Err(CrawlError::Catalog(CatalogError::InvalidApiBaseUrl(_)))
        ));
    }

    #[test]
    fn crawler_keeps_variant_without_skus() {
        let mut client = FixtureUpstream::representative();
        let detail = client
            .details
            .get_mut(&("varg-ex".to_owned(), "US".to_owned(), "14_disc".to_owned()))
            .unwrap()
            .as_mut()
            .unwrap();
        detail.articles[0].article.variants[0].skus.clear();

        let catalog =
            crawl_catalog(&client, &CrawlConfig::us_storefront("2026-05-26T12:34:56Z")).unwrap();
        let variants = &catalog.catalog_trees[0].categories[0].categories[0].product_groups[0]
            .articles[0]
            .variants;

        assert_eq!(variants.len(), 1);
        assert_eq!(variants[0].sku, None);
    }

    #[test]
    fn crawler_bounds_category_depth() {
        let mut client = FixtureUpstream::representative();
        let root_category = client
            .categories
            .get_mut(&("varg-ex".to_owned(), "SP".to_owned()))
            .unwrap()
            .as_mut()
            .unwrap();
        root_category[0].is_leaf = false;

        for depth in 1..MAX_CATEGORY_DEPTH {
            let path = format!("SP{}", "/brakes".repeat(depth));
            let next_path = format!("{path}/brakes");
            client.categories.insert(
                ("varg-ex".to_owned(), path),
                Ok(vec![UpstreamCategory {
                    code: "brakes".to_owned(),
                    name_key: Some("spare_parts_category_brakes_name".to_owned()),
                    display_name: Some(format!("Brakes {depth}")),
                    image_url: None,
                    is_leaf: false,
                    path: next_path,
                }]),
            );
        }

        assert!(matches!(
            crawl_catalog(&client, &CrawlConfig::us_storefront("2026-05-26T12:34:56Z")),
            Err(CrawlError::CategoryDepth { .. })
        ));
    }

    #[test]
    fn crawler_uses_detail_image_before_summary_image() {
        let mut client = FixtureUpstream::representative();
        let detail = client
            .details
            .get_mut(&("varg-ex".to_owned(), "US".to_owned(), "14_disc".to_owned()))
            .unwrap()
            .as_mut()
            .unwrap();
        detail.feature_image_url =
            Some("https://s3-stark-prod.s3.eu-central-1.amazonaws.com/spare-parts-images/DetailDisc.webp".to_owned());

        let catalog =
            crawl_catalog(&client, &CrawlConfig::us_storefront("2026-05-26T12:34:56Z")).unwrap();
        let product_group = &catalog.catalog_trees[0].categories[0].categories[0].product_groups[0];

        assert_eq!(
            product_group.image_urls,
            vec![
                "https://s3-stark-prod.s3.eu-central-1.amazonaws.com/spare-parts-images/DetailDisc.webp"
            ]
        );
    }

    #[test]
    fn crawler_falls_back_to_summary_text_when_detail_text_is_missing() {
        let mut client = FixtureUpstream::representative();
        let detail = client
            .details
            .get_mut(&("varg-ex".to_owned(), "US".to_owned(), "14_disc".to_owned()))
            .unwrap()
            .as_mut()
            .unwrap();
        detail.display_name = None;
        detail.description = None;
        detail.name_key = None;
        detail.description_key = None;

        let catalog =
            crawl_catalog(&client, &CrawlConfig::us_storefront("2026-05-26T12:34:56Z")).unwrap();
        let product_group = &catalog.catalog_trees[0].categories[0].categories[0].product_groups[0];

        assert_eq!(product_group.display_name, Some("Disc".to_owned()));
        assert_eq!(
            product_group.description,
            Some("Front disc group".to_owned())
        );
        assert_eq!(
            product_group.localization_key,
            Some("spare_parts_product_14_disc_name".to_owned())
        );
        assert_eq!(
            product_group.description_localization_key,
            Some("spare_parts_product_14_disc_description".to_owned())
        );
    }

    fn representative_catalog() -> Catalog {
        Catalog {
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

    struct FixtureUpstream {
        bike_variants: Vec<UpstreamBikeVariant>,
        categories: HashMap<(String, String), UpstreamResult<Vec<UpstreamCategory>>>,
        products: HashMap<(String, String), UpstreamResult<Vec<UpstreamProductSummary>>>,
        details: HashMap<(String, String, String), UpstreamResult<UpstreamProductDetail>>,
        calls: RefCell<Vec<String>>,
    }

    impl FixtureUpstream {
        fn representative() -> Self {
            let mut categories = HashMap::new();
            categories.insert(
                ("varg-ex".to_owned(), "SP".to_owned()),
                Ok(vec![UpstreamCategory {
                    code: "brakes".to_owned(),
                    name_key: Some("spare_parts_category_brakes_name".to_owned()),
                    display_name: Some("Brakes".to_owned()),
                    image_url: None,
                    is_leaf: false,
                    path: "SP".to_owned(),
                }]),
            );
            categories.insert(
                ("varg-ex".to_owned(), "SP/brakes".to_owned()),
                Ok(vec![UpstreamCategory {
                    code: "brakes_front_brake".to_owned(),
                    name_key: Some("spare_parts_category_front_brake_name".to_owned()),
                    display_name: Some("Front brake".to_owned()),
                    image_url: None,
                    is_leaf: true,
                    path: "SP".to_owned(),
                }]),
            );
            categories.insert(("varg-sm".to_owned(), "SP".to_owned()), Ok(Vec::new()));

            let mut products = HashMap::new();
            products.insert(
                ("varg-ex".to_owned(), "brakes_front_brake".to_owned()),
                Ok(vec![UpstreamProductSummary {
                    code: "14_disc".to_owned(),
                    name_key: Some("spare_parts_product_14_disc_name".to_owned()),
                    description_key: Some("spare_parts_product_14_disc_description".to_owned()),
                    display_name: Some("Disc".to_owned()),
                    description: Some("Front disc group".to_owned()),
                    image_url: Some(
                        "https://s3-stark-prod.s3.eu-central-1.amazonaws.com/spare-parts-images/Disc.webp"
                            .to_owned(),
                    ),
                }]),
            );

            let mut details = HashMap::new();
            details.insert(
                ("varg-ex".to_owned(), "US".to_owned(), "14_disc".to_owned()),
                Ok(UpstreamProductDetail {
                    code: "14_disc".to_owned(),
                    name_key: Some("spare_parts_product_14_disc_name".to_owned()),
                    description_key: Some("spare_parts_product_14_disc_description".to_owned()),
                    display_name: Some("Front disc".to_owned()),
                    description: Some("Front brake disc".to_owned()),
                    feature_image_url: None,
                    articles: vec![UpstreamArticleEntry {
                        reference: Some(14),
                        article: UpstreamArticle {
                            code: "disc_260mm".to_owned(),
                            name_key: Some("spare_parts_product_disc_260mm_name".to_owned()),
                            description_key: Some("spare_parts_product_disc_260mm_description".to_owned()),
                            display_name: Some("260mm disc".to_owned()),
                            description: Some("US labeled part that still fits the bike".to_owned()),
                            image_url: Some(
                                "https://s3-stark-prod.s3.eu-central-1.amazonaws.com/spare-parts-images/Disc260.webp"
                                    .to_owned(),
                            ),
                            tags: vec!["region-us".to_owned()],
                            is_kit: false,
                            kit_contain: vec!["bolt_m6".to_owned()],
                            variants: vec![UpstreamArticleVariant {
                                code: "disc_260mm-standard".to_owned(),
                                skus: vec!["SMX1-BR-FW-260".to_owned(), "I14580-060012-08-P".to_owned()],
                                availability: Some("AVAILABLE_HQ".to_owned()),
                                price: Some(UpstreamPrice {
                                    total_minor: 14900,
                                    currency: "USD".to_owned(),
                                }),
                                attributes: vec![UpstreamAttributeSelection {
                                    attribute_code: "disc_size".to_owned(),
                                    option_code: "260mm".to_owned(),
                                    option_display_name: None,
                                    option_name_key: Some("spare_parts_attribute_option_260mm_name".to_owned()),
                                }],
                            }],
                        },
                    }],
                }),
            );

            Self {
                bike_variants: vec![
                    UpstreamBikeVariant {
                        tag: "varg-ex".to_owned(),
                        display_name: Some("Varg EX".to_owned()),
                    },
                    UpstreamBikeVariant {
                        tag: "varg-sm".to_owned(),
                        display_name: Some("Varg SM".to_owned()),
                    },
                ],
                categories,
                products,
                details,
                calls: RefCell::new(Vec::new()),
            }
        }

        fn calls(&self) -> Vec<String> {
            self.calls.borrow().clone()
        }
    }

    impl UpstreamCatalog for FixtureUpstream {
        fn bike_variants(&self) -> UpstreamResult<Vec<UpstreamBikeVariant>> {
            self.calls.borrow_mut().push("bike_variants".to_owned());
            Ok(self.bike_variants.clone())
        }

        fn categories(&self, tag: &str, path: &str) -> UpstreamResult<Vec<UpstreamCategory>> {
            self.calls
                .borrow_mut()
                .push(format!("categories:{tag}:{path}"));
            self.categories
                .get(&(tag.to_owned(), path.to_owned()))
                .cloned()
                .unwrap_or_else(|| {
                    Err(UpstreamError::new(format!(
                        "missing categories fixture for {tag} {path}"
                    )))
                })
        }

        fn products(
            &self,
            tag: &str,
            category_code: &str,
        ) -> UpstreamResult<Vec<UpstreamProductSummary>> {
            self.calls
                .borrow_mut()
                .push(format!("products:{tag}:{category_code}"));
            self.products
                .get(&(tag.to_owned(), category_code.to_owned()))
                .cloned()
                .unwrap_or_else(|| {
                    Err(UpstreamError::new(format!(
                        "missing products fixture for {tag} {category_code}"
                    )))
                })
        }

        fn product_detail(
            &self,
            tag: &str,
            country: &str,
            product_code: &str,
        ) -> UpstreamResult<UpstreamProductDetail> {
            self.calls
                .borrow_mut()
                .push(format!("detail:{tag}:{country}:{product_code}"));
            self.details
                .get(&(tag.to_owned(), country.to_owned(), product_code.to_owned()))
                .cloned()
                .unwrap_or_else(|| {
                    Err(UpstreamError::new(format!(
                        "missing product detail fixture for {tag} {country} {product_code}"
                    )))
                })
        }
    }
}

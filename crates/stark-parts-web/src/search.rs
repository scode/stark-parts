use stark_parts_catalog::{
    Article, ArticleVariant, AttributeSelection, Availability, Catalog, CategoryNode, Price,
    ProductGroup,
};
use std::collections::{HashMap, HashSet};
use url::form_urlencoded;

/// Browser-local search index derived entirely from the committed catalog.
///
/// The index is deliberately UI-agnostic. It knows how to normalize text and
/// apply bike filters, but it does not know anything about Leptos components
/// or browser events.
pub struct SearchIndex {
    bike_variants: Vec<BikeVariantSummary>,
    rows: Vec<SearchRow>,
}

impl SearchIndex {
    /// Build search rows in catalog order, including articles without variants.
    pub fn from_catalog(catalog: &Catalog) -> Self {
        let bike_variants = catalog
            .bike_variants
            .iter()
            .map(|variant| BikeVariantSummary {
                id: variant.id.clone(),
                code: variant.code.clone(),
                display_name: variant.display_name.clone(),
            })
            .collect::<Vec<_>>();
        let bike_summaries = bike_variants
            .iter()
            .map(|variant| (variant.id.clone(), variant.clone()))
            .collect::<HashMap<_, _>>();

        let mut rows = Vec::new();
        for tree in &catalog.catalog_trees {
            let bike = bike_summaries.get(&tree.bike_variant_id);
            collect_category_rows(
                &tree.bike_variant_id,
                bike.map(|bike| bike.code.as_str()),
                bike.and_then(|bike| bike.display_name.as_deref()),
                &tree.categories,
                &mut Vec::new(),
                &mut rows,
            );
        }

        Self {
            bike_variants,
            rows,
        }
    }

    /// Return the committed bike variants in the URL-stable order.
    pub fn bike_variants(&self) -> &[BikeVariantSummary] {
        &self.bike_variants
    }

    /// Search rows in catalog order without doing any renderer-specific work.
    pub fn search(&self, request: &SearchRequest) -> SearchResults {
        let selected_bikes = request
            .selected_bike_variant_ids
            .iter()
            .cloned()
            .collect::<HashSet<_>>();
        let query_tokens = normalize_tokens(&request.query);
        let compact_query = compact_search_text(&request.query);
        let selected_all_bikes = selected_bikes.is_empty();

        let mut matched_rows = Vec::new();
        for row in &self.rows {
            if !selected_all_bikes && !selected_bikes.contains(&row.bike_variant_id) {
                continue;
            }
            if let Some(match_source) = row_match_source(row, &query_tokens, &compact_query) {
                matched_rows.push((row, match_source));
            }
        }

        SearchResults {
            is_empty_query: query_tokens.is_empty() && compact_query.is_empty(),
            rows: merge_result_rows(matched_rows),
        }
    }
}

/// URL-restorable search state controlled by the search model.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct SearchRequest {
    pub query: String,
    pub selected_bike_variant_ids: Vec<String>,
}

impl SearchRequest {
    /// Encode search state into a stable query string without a leading `?`.
    pub fn to_query_string(&self) -> String {
        let mut serializer = form_urlencoded::Serializer::new(String::new());
        if !self.query.is_empty() {
            serializer.append_pair("q", &self.query);
        }
        for bike in &self.selected_bike_variant_ids {
            serializer.append_pair("bike", bike);
        }
        serializer.finish()
    }

    /// Decode search state from a query string with or without a leading `?`.
    pub fn from_query_string(query: &str) -> Self {
        let query = query.strip_prefix('?').unwrap_or(query);
        let mut request = SearchRequest::default();

        for (key, value) in form_urlencoded::parse(query.as_bytes()) {
            match key.as_ref() {
                "q" => request.query = value.into_owned(),
                "bike" => request.selected_bike_variant_ids.push(value.into_owned()),
                _ => {}
            }
        }

        request
    }
}

/// A bike variant users can select as a search filter.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BikeVariantSummary {
    pub id: String,
    pub code: String,
    pub display_name: Option<String>,
}

/// Search result rows in the order the catalog exposes them.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SearchResults {
    pub is_empty_query: bool,
    pub rows: Vec<SearchResultRow>,
}

impl SearchResults {
    pub fn has_matches(&self) -> bool {
        !self.rows.is_empty()
    }
}

/// One matching part-level row with enough context for details panes.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SearchResultRow {
    pub bike_variant_id: String,
    pub bike_code: Option<String>,
    pub bike_display_name: Option<String>,
    pub compatible_bikes: Vec<BikeVariantSummary>,
    pub category_path: Vec<String>,
    pub category_display_path: Vec<String>,
    pub product_group: ProductGroupSummary,
    pub article: ArticleSummary,
    pub variant: Option<ArticleVariantSummary>,
    pub match_feedback: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProductGroupSummary {
    pub code: String,
    pub display_name: Option<String>,
    pub description: Option<String>,
    pub stark_url: Option<String>,
    pub image_urls: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ArticleSummary {
    pub code: String,
    pub display_name: Option<String>,
    pub description: Option<String>,
    pub stark_url: Option<String>,
    pub image_urls: Vec<String>,
    pub kit_memberships: Vec<String>,
    pub kit_contents: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ArticleVariantSummary {
    pub code: String,
    pub sku: Option<String>,
    pub stark_url: Option<String>,
    pub image_urls: Vec<String>,
    pub attributes: Vec<AttributeSummary>,
    pub price: Option<Price>,
    pub availability: Option<Availability>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AttributeSummary {
    pub code: String,
    pub option_code: String,
    pub option_display_name: Option<String>,
}

struct SearchRow {
    bike_variant_id: String,
    search_text: SearchText,
    result: SearchResultRow,
}

struct SearchText {
    exact_part: SearchTextBucket,
    product_group: SearchTextBucket,
    context: SearchTextBucket,
    combined: SearchTextBucket,
}

impl SearchText {
    fn from_result(result: &SearchResultRow) -> Self {
        let exact_part_fields = exact_part_searchable_fields(result);
        let product_group_fields = product_group_searchable_fields(result);
        let context_fields = context_searchable_fields(result);
        let combined_fields = exact_part_fields
            .iter()
            .chain(product_group_fields.iter())
            .chain(context_fields.iter())
            .cloned()
            .collect();

        Self {
            exact_part: SearchTextBucket::from_fields(exact_part_fields),
            product_group: SearchTextBucket::from_fields(product_group_fields),
            context: SearchTextBucket::from_fields(context_fields),
            combined: SearchTextBucket::from_fields(combined_fields),
        }
    }

    fn matches(&self, query_tokens: &[String], compact_query: &str) -> bool {
        let buckets = [&self.exact_part, &self.product_group, &self.context];
        query_tokens
            .iter()
            .all(|token| buckets.iter().any(|bucket| bucket.contains_token(token)))
            || self.combined.matches_compact(compact_query)
    }

    fn feedback_source(&self, query_tokens: &[String], compact_query: &str) -> Option<MatchSource> {
        if self.exact_part.matches_query(query_tokens, compact_query) {
            return None;
        }
        if self
            .product_group
            .matches_query(query_tokens, compact_query)
        {
            return Some(MatchSource::ProductGroup);
        }
        None
    }
}

struct SearchTextBucket {
    normalized_text: String,
    compact_text: String,
}

impl SearchTextBucket {
    fn from_fields(fields: Vec<String>) -> Self {
        let searchable_text = fields.join(" ");

        Self {
            normalized_text: normalize_tokens(&searchable_text).join(" "),
            compact_text: compact_search_text(&searchable_text),
        }
    }

    fn contains_token(&self, token: &str) -> bool {
        self.normalized_text.contains(token)
    }

    fn matches_compact(&self, compact_query: &str) -> bool {
        !compact_query.is_empty() && self.compact_text.contains(compact_query)
    }

    fn matches_query(&self, query_tokens: &[String], compact_query: &str) -> bool {
        query_tokens.iter().all(|token| self.contains_token(token))
            || self.matches_compact(compact_query)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum MatchSource {
    ProductGroup,
}

fn collect_category_rows(
    bike_variant_id: &str,
    bike_code: Option<&str>,
    bike_display_name: Option<&str>,
    categories: &[CategoryNode],
    ancestors: &mut Vec<CategoryCrumb>,
    rows: &mut Vec<SearchRow>,
) {
    for category in categories {
        ancestors.push(CategoryCrumb {
            code: category.code.clone(),
            display_name: category.display_name.clone(),
        });
        for group in &category.product_groups {
            collect_product_group_rows(
                bike_variant_id,
                bike_code,
                bike_display_name,
                ancestors,
                group,
                rows,
            );
        }
        collect_category_rows(
            bike_variant_id,
            bike_code,
            bike_display_name,
            &category.categories,
            ancestors,
            rows,
        );
        ancestors.pop();
    }
}

fn collect_product_group_rows(
    bike_variant_id: &str,
    bike_code: Option<&str>,
    bike_display_name: Option<&str>,
    ancestors: &[CategoryCrumb],
    group: &ProductGroup,
    rows: &mut Vec<SearchRow>,
) {
    for article in &group.articles {
        if article.variants.is_empty() {
            rows.push(search_row_from_variant(
                bike_variant_id,
                bike_code,
                bike_display_name,
                ancestors,
                group,
                article,
                None,
            ));
        } else {
            for variant in &article.variants {
                rows.push(search_row_from_variant(
                    bike_variant_id,
                    bike_code,
                    bike_display_name,
                    ancestors,
                    group,
                    article,
                    Some(variant),
                ));
            }
        }
    }
}

fn search_row_from_variant(
    bike_variant_id: &str,
    bike_code: Option<&str>,
    bike_display_name: Option<&str>,
    ancestors: &[CategoryCrumb],
    group: &ProductGroup,
    article: &Article,
    variant: Option<&ArticleVariant>,
) -> SearchRow {
    let category_path = ancestors
        .iter()
        .map(|category| category.code.clone())
        .collect::<Vec<_>>();
    let category_display_path = ancestors
        .iter()
        .map(|category| {
            category
                .display_name
                .clone()
                .unwrap_or_else(|| category.code.clone())
        })
        .collect::<Vec<_>>();
    let result = SearchResultRow {
        bike_variant_id: bike_variant_id.to_owned(),
        bike_code: bike_code.map(str::to_owned),
        bike_display_name: bike_display_name.map(str::to_owned),
        compatible_bikes: vec![BikeVariantSummary {
            id: bike_variant_id.to_owned(),
            code: bike_code.unwrap_or(bike_variant_id).to_owned(),
            display_name: bike_display_name.map(str::to_owned),
        }],
        category_path,
        category_display_path,
        product_group: ProductGroupSummary {
            code: group.code.clone(),
            display_name: group.display_name.clone(),
            description: group.description.clone(),
            stark_url: group.stark_url.clone(),
            image_urls: group.image_urls.clone(),
        },
        article: ArticleSummary {
            code: article.code.clone(),
            display_name: article.display_name.clone(),
            description: article.description.clone(),
            stark_url: article.stark_url.clone(),
            image_urls: article.image_urls.clone(),
            kit_memberships: article.kit_memberships.clone(),
            kit_contents: article.kit_contents.clone(),
        },
        variant: variant.map(article_variant_summary),
        match_feedback: None,
    };
    let search_text = SearchText::from_result(&result);

    SearchRow {
        bike_variant_id: bike_variant_id.to_owned(),
        search_text,
        result,
    }
}

fn attribute_summary(attribute: &AttributeSelection) -> AttributeSummary {
    AttributeSummary {
        code: attribute.code.clone(),
        option_code: attribute.option_code.clone(),
        option_display_name: attribute.option_display_name.clone(),
    }
}

fn article_variant_summary(variant: &ArticleVariant) -> ArticleVariantSummary {
    ArticleVariantSummary {
        code: variant.code.clone(),
        sku: variant.sku.clone(),
        stark_url: variant.stark_url.clone(),
        image_urls: variant.image_urls.clone(),
        attributes: variant.attributes.iter().map(attribute_summary).collect(),
        price: variant.price.clone(),
        availability: variant.availability.clone(),
    }
}

fn merge_result_rows(rows: Vec<(&SearchRow, Option<MatchSource>)>) -> Vec<SearchResultRow> {
    let mut merged = Vec::<SearchResultRow>::new();
    let mut indexes = HashMap::<SearchResultKey, usize>::new();

    for (row, match_source) in rows {
        let key = SearchResultKey::from(&row.result);
        if let Some(index) = indexes.get(&key).copied() {
            merge_compatible_bikes(&mut merged[index], &row.result);
            merge_match_feedback(&mut merged[index], row, match_source);
        } else {
            let mut result = row.result.clone();
            result.match_feedback = match_source.and_then(|source| match_feedback(row, source));
            indexes.insert(key, merged.len());
            merged.push(result);
        }
    }

    merged
}

fn merge_compatible_bikes(target: &mut SearchResultRow, source: &SearchResultRow) {
    for bike in &source.compatible_bikes {
        if !target
            .compatible_bikes
            .iter()
            .any(|existing| existing.id == bike.id)
        {
            target.compatible_bikes.push(bike.clone());
        }
    }
}

fn merge_match_feedback(
    target: &mut SearchResultRow,
    source: &SearchRow,
    match_source: Option<MatchSource>,
) {
    if target.match_feedback.is_none() {
        target.match_feedback =
            match_source.and_then(|source_kind| match_feedback(source, source_kind));
    }
}

fn match_feedback(row: &SearchRow, source: MatchSource) -> Option<String> {
    match source {
        MatchSource::ProductGroup => {
            let label = row
                .result
                .product_group
                .display_name
                .as_ref()
                .unwrap_or(&row.result.product_group.code);
            Some(format!("matched group: {label}"))
        }
    }
}

#[derive(Debug, Eq, Hash, PartialEq)]
struct SearchResultKey {
    product_group_code: String,
    article_code: String,
    variant_code: Option<String>,
    sku: Option<String>,
}

impl SearchResultKey {
    fn from(row: &SearchResultRow) -> Self {
        Self {
            product_group_code: row.product_group.code.clone(),
            article_code: row.article.code.clone(),
            variant_code: row.variant.as_ref().map(|variant| variant.code.clone()),
            sku: row.variant.as_ref().and_then(|variant| variant.sku.clone()),
        }
    }
}

fn exact_part_searchable_fields(result: &SearchResultRow) -> Vec<String> {
    let mut fields = Vec::new();
    fields.push(result.article.code.clone());
    push_optional(&mut fields, &result.article.display_name);
    push_optional(&mut fields, &result.article.description);
    if let Some(variant) = &result.variant {
        fields.push(variant.code.clone());
        push_optional(&mut fields, &variant.sku);
        for attribute in &variant.attributes {
            fields.push(attribute.code.clone());
            fields.push(attribute.option_code.clone());
            push_optional(&mut fields, &attribute.option_display_name);
        }
    }
    fields.extend(result.article.kit_memberships.iter().cloned());
    fields.extend(result.article.kit_contents.iter().cloned());
    fields
}

fn product_group_searchable_fields(result: &SearchResultRow) -> Vec<String> {
    let mut fields = Vec::new();
    fields.push(result.product_group.code.clone());
    push_optional(&mut fields, &result.product_group.display_name);
    push_optional(&mut fields, &result.product_group.description);
    fields
}

fn context_searchable_fields(result: &SearchResultRow) -> Vec<String> {
    let mut fields = Vec::new();
    fields.push(result.bike_variant_id.clone());
    push_optional(&mut fields, &result.bike_code);
    push_optional(&mut fields, &result.bike_display_name);
    for bike in &result.compatible_bikes {
        fields.push(bike.id.clone());
        fields.push(bike.code.clone());
        push_optional(&mut fields, &bike.display_name);
    }
    fields.extend(result.category_path.iter().cloned());
    fields.extend(result.category_display_path.iter().cloned());
    fields
}

fn push_optional(fields: &mut Vec<String>, value: &Option<String>) {
    if let Some(value) = value {
        fields.push(value.clone());
    }
}

fn row_match_source(
    row: &SearchRow,
    query_tokens: &[String],
    compact_query: &str,
) -> Option<Option<MatchSource>> {
    if query_tokens.is_empty() && compact_query.is_empty() {
        return Some(None);
    }

    if !row.search_text.matches(query_tokens, compact_query) {
        return None;
    }

    Some(row.search_text.feedback_source(query_tokens, compact_query))
}

fn normalize_tokens(input: &str) -> Vec<String> {
    input
        .split(|character: char| !character.is_alphanumeric())
        .filter(|token| !token.is_empty())
        .map(|token| token.to_lowercase())
        .collect()
}

fn compact_search_text(input: &str) -> String {
    input
        .chars()
        .filter(|character| character.is_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

#[derive(Clone)]
struct CategoryCrumb {
    code: String,
    display_name: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use stark_parts_catalog::{
        Availability, BikeCatalogTree, BikeVariant, CatalogMetadata, Price, SourceEndpoint,
        SourceMetadata, parse_catalog_json5,
    };

    #[test]
    fn empty_query_returns_all_result_rows_for_all_bikes() {
        let index = SearchIndex::from_catalog(&fixture_catalog());
        let results = index.search(&SearchRequest::default());

        assert!(results.is_empty_query);
        assert_eq!(results.rows.len(), 4);
        assert_eq!(
            results
                .rows
                .iter()
                .map(|row| row.bike_variant_id.as_str())
                .collect::<Vec<_>>(),
            ["varg-ex", "varg-ex", "varg-ex", "varg-sm"]
        );
    }

    #[test]
    fn bike_filter_limits_results_and_none_selected_means_all() {
        let index = SearchIndex::from_catalog(&fixture_catalog());
        let filtered = index.search(&SearchRequest {
            query: String::new(),
            selected_bike_variant_ids: vec!["varg-sm".to_owned()],
        });
        let multi_selected = index.search(&SearchRequest {
            query: String::new(),
            selected_bike_variant_ids: vec!["varg-ex".to_owned(), "varg-sm".to_owned()],
        });
        let multi_selected_with_missing = index.search(&SearchRequest {
            query: String::new(),
            selected_bike_variant_ids: vec!["varg-sm".to_owned(), "missing-bike".to_owned()],
        });
        let unfiltered = index.search(&SearchRequest {
            query: String::new(),
            selected_bike_variant_ids: Vec::new(),
        });

        assert_eq!(filtered.rows.len(), 1);
        assert_eq!(filtered.rows[0].bike_variant_id, "varg-sm");
        assert_eq!(multi_selected.rows.len(), 4);
        assert_eq!(
            multi_selected
                .rows
                .iter()
                .map(|row| row.bike_variant_id.as_str())
                .collect::<Vec<_>>(),
            ["varg-ex", "varg-ex", "varg-ex", "varg-sm"]
        );
        assert_eq!(multi_selected_with_missing.rows.len(), 1);
        assert!(
            multi_selected_with_missing
                .rows
                .iter()
                .all(|row| row.bike_variant_id == "varg-sm")
        );
        assert_eq!(unfiltered.rows.len(), 4);
    }

    #[test]
    fn repeated_variants_across_bike_trees_render_as_one_result() {
        let mut catalog = fixture_catalog();
        catalog.catalog_trees[1].categories.push(CategoryNode {
            code: "shared_brakes".to_owned(),
            path: vec!["shared_brakes".to_owned()],
            display_name: Some("Shared brakes".to_owned()),
            localization_key: None,
            categories: Vec::new(),
            product_groups: vec![disc_group()],
        });
        let index = SearchIndex::from_catalog(&catalog);

        let unfiltered = index.search(&SearchRequest {
            query: "disc_260mm-standard".to_owned(),
            selected_bike_variant_ids: Vec::new(),
        });
        let filtered = index.search(&SearchRequest {
            query: "disc_260mm-standard".to_owned(),
            selected_bike_variant_ids: vec!["varg-sm".to_owned()],
        });

        assert_eq!(unfiltered.rows.len(), 1);
        assert_eq!(
            unfiltered.rows[0]
                .compatible_bikes
                .iter()
                .map(|bike| bike.id.as_str())
                .collect::<Vec<_>>(),
            ["varg-ex", "varg-sm"]
        );
        assert_eq!(filtered.rows.len(), 1);
        assert_eq!(filtered.rows[0].bike_variant_id, "varg-sm");
        assert_eq!(
            filtered.rows[0]
                .compatible_bikes
                .iter()
                .map(|bike| bike.id.as_str())
                .collect::<Vec<_>>(),
            ["varg-sm"]
        );
    }

    #[test]
    fn search_matches_required_fields_case_insensitively() {
        let index = SearchIndex::from_catalog(&fixture_catalog());

        assert_match(&index, "varg ex", "SMX1-BR-FW-260");
        assert_match(&index, "front brake", "SMX1-BR-FW-260");
        assert_match(&index, "disc group", "SMX1-BR-FW-260");
        assert_match(&index, "group-only-description", "SMX1-BR-FW-260");
        assert_match(&index, "article-only-description", "SMX1-BR-FW-260");
        assert_match(&index, "disc_size 260mm", "SMX1-BR-FW-260");
        assert_match(&index, "mount bolt", "SMX1-BR-FW-260");
        assert_match(&index, "supermoto-code", "SMX1-MIRROR");
        assert_match(&index, "front_brake", "SMX1-BR-FW-260");
        assert_match(&index, "disc_group", "SMX1-BR-FW-260");
        assert_match(&index, "disc_260mm", "SMX1-BR-FW-260");
        assert_match(&index, "disc_260mm-standard", "SMX1-BR-FW-260");
    }

    #[test]
    fn sku_search_ignores_hyphen_punctuation() {
        let index = SearchIndex::from_catalog(&fixture_catalog());

        assert_match(&index, "smx1brfw260", "SMX1-BR-FW-260");
        assert_match(&index, "SMX1 BR FW 260", "SMX1-BR-FW-260");
    }

    #[test]
    fn no_result_query_returns_clear_empty_rows() {
        let index = SearchIndex::from_catalog(&fixture_catalog());
        let results = index.search(&SearchRequest {
            query: "does-not-exist".to_owned(),
            selected_bike_variant_ids: Vec::new(),
        });

        assert!(!results.has_matches());
        assert!(results.rows.is_empty());
    }

    #[test]
    fn result_rows_preserve_context_needed_by_details() {
        let index = SearchIndex::from_catalog(&fixture_catalog());
        let results = index.search(&SearchRequest {
            query: "SMX1-BR-FW-260".to_owned(),
            selected_bike_variant_ids: Vec::new(),
        });
        let row = &results.rows[0];
        let variant = row.variant.as_ref().expect("query should match a variant");

        assert_eq!(row.bike_variant_id, "varg-ex");
        assert_eq!(row.category_path, ["brakes", "front_brake"]);
        assert_eq!(row.product_group.code, "disc_group");
        assert_eq!(
            row.product_group.image_urls,
            ["https://s3-stark-prod.s3.eu-central-1.amazonaws.com/catalog/disc-group.png"]
        );
        assert_eq!(row.article.code, "disc_260mm");
        assert_eq!(
            row.article.image_urls,
            ["https://s3-stark-prod.s3.eu-central-1.amazonaws.com/catalog/disc-260mm.png"]
        );
        assert_eq!(variant.sku.as_deref(), Some("SMX1-BR-FW-260"));
        assert_eq!(
            variant.image_urls,
            ["https://s3-stark-prod.s3.eu-central-1.amazonaws.com/catalog/disc-variant.png"]
        );
    }

    #[test]
    fn variantless_articles_remain_searchable() {
        let index = SearchIndex::from_catalog(&fixture_catalog());
        let results = index.search(&SearchRequest {
            query: "brake manual".to_owned(),
            selected_bike_variant_ids: Vec::new(),
        });

        assert_eq!(results.rows.len(), 1);
        assert!(results.rows[0].variant.is_none());
        assert_eq!(results.rows[0].article.code, "brake_manual");
    }

    #[test]
    fn result_rows_carry_part_detail_fields_for_ui() {
        let index = SearchIndex::from_catalog(&fixture_catalog());
        let results = index.search(&SearchRequest {
            query: "SMX1-BR-FW-260".to_owned(),
            selected_bike_variant_ids: Vec::new(),
        });
        let row = &results.rows[0];
        let variant = row.variant.as_ref().unwrap();

        assert_eq!(
            row.product_group.stark_url.as_deref(),
            Some("https://www.starkfuture.com/us-US/parts/disc-group")
        );
        assert_eq!(
            row.product_group.image_urls,
            ["https://s3-stark-prod.s3.eu-central-1.amazonaws.com/catalog/disc-group.png"]
        );
        assert_eq!(
            row.article.stark_url.as_deref(),
            Some("https://www.starkfuture.com/us-US/parts/disc-260mm")
        );
        assert_eq!(
            row.article.image_urls,
            ["https://s3-stark-prod.s3.eu-central-1.amazonaws.com/catalog/disc-260mm.png"]
        );
        assert_eq!(row.article.kit_memberships, ["brake-service-kit"]);
        assert_eq!(row.article.kit_contents, ["mount bolt"]);
        assert_eq!(
            variant.stark_url.as_deref(),
            Some("https://www.starkfuture.com/us-US/parts/disc-260mm?sku=SMX1-BR-FW-260")
        );
        assert_eq!(
            variant.image_urls,
            ["https://s3-stark-prod.s3.eu-central-1.amazonaws.com/catalog/disc-variant.png"]
        );
        assert_eq!(variant.attributes[0].code, "disc_size");
        assert_eq!(variant.attributes[0].option_code, "260mm");
        assert_eq!(variant.price.as_ref().unwrap().amount_minor, 14900);
        assert_eq!(variant.price.as_ref().unwrap().currency, "USD");
        assert_eq!(variant.availability.as_ref().unwrap().status, "AVAILABLE");
    }

    #[test]
    fn search_state_round_trips_through_query_string() {
        let request = SearchRequest {
            query: "front disc".to_owned(),
            selected_bike_variant_ids: vec!["varg-ex".to_owned(), "varg-sm".to_owned()],
        };

        let encoded = request.to_query_string();

        assert_eq!(SearchRequest::from_query_string(&encoded), request);
        assert_eq!(
            SearchRequest::from_query_string(&format!("?{encoded}")),
            request
        );
    }

    #[test]
    fn committed_catalog_builds_search_index() {
        let catalog =
            parse_catalog_json5(include_str!("../../../catalog/stark-parts.json5")).unwrap();
        let index = SearchIndex::from_catalog(&catalog);
        let results = index.search(&SearchRequest {
            query: "SMX1-TOOLBOX".to_owned(),
            selected_bike_variant_ids: Vec::new(),
        });

        assert_eq!(index.bike_variants().len(), 4);
        assert!(results.has_matches());
        assert!(results.rows.iter().any(|row| {
            row.variant
                .as_ref()
                .and_then(|variant| variant.sku.as_deref())
                == Some("SMX1-TOOLBOX")
        }));
    }

    #[test]
    fn committed_catalog_matches_parts_through_group_descriptions() {
        let catalog =
            parse_catalog_json5(include_str!("../../../catalog/stark-parts.json5")).unwrap();
        let index = SearchIndex::from_catalog(&catalog);
        let results = index.search(&SearchRequest {
            query: "wiring harness".to_owned(),
            selected_bike_variant_ids: Vec::new(),
        });

        assert!(results.rows.iter().any(|row| {
            row.variant
                .as_ref()
                .and_then(|variant| variant.sku.as_deref())
                == Some("SMX1-WH-F-01")
                && row.article.display_name.as_deref() == Some("Frame cable holder")
                && row.product_group.description.as_deref().is_some()
        }));
    }

    #[test]
    fn group_description_matches_explain_the_source() {
        let index = SearchIndex::from_catalog(&fixture_catalog());
        let results = index.search(&SearchRequest {
            query: "group-only-description".to_owned(),
            selected_bike_variant_ids: Vec::new(),
        });

        let row = results
            .rows
            .iter()
            .find(|row| {
                row.variant
                    .as_ref()
                    .and_then(|variant| variant.sku.as_deref())
                    == Some("SMX1-BR-FW-260")
            })
            .expect("group description should match the brake disc variant");

        assert_eq!(
            row.match_feedback.as_deref(),
            Some("matched group: Disc group")
        );
    }

    #[test]
    fn search_text_keeps_part_and_group_wording_separate() {
        let index = SearchIndex::from_catalog(&fixture_catalog());
        let row = index
            .rows
            .iter()
            .find(|row| {
                row.result
                    .variant
                    .as_ref()
                    .and_then(|variant| variant.sku.as_deref())
                    == Some("SMX1-BR-FW-260")
            })
            .expect("fixture should include the brake disc variant");

        assert!(
            row.search_text
                .exact_part
                .normalized_text
                .contains("article")
        );
        assert!(!row.search_text.exact_part.normalized_text.contains("group"));
        assert!(
            row.search_text
                .product_group
                .normalized_text
                .contains("group")
        );
        assert!(
            !row.search_text
                .product_group
                .normalized_text
                .contains("article")
        );
    }

    #[test]
    fn search_tokens_can_match_across_separate_text_sources() {
        let index = SearchIndex::from_catalog(&fixture_catalog());
        let results = index.search(&SearchRequest {
            query: "article-only-description group-only-description".to_owned(),
            selected_bike_variant_ids: Vec::new(),
        });

        assert!(results.rows.iter().any(|row| {
            row.variant
                .as_ref()
                .and_then(|variant| variant.sku.as_deref())
                == Some("SMX1-BR-FW-260")
        }));
    }

    fn assert_match(index: &SearchIndex, query: &str, expected_sku: &str) {
        let results = index.search(&SearchRequest {
            query: query.to_owned(),
            selected_bike_variant_ids: Vec::new(),
        });

        assert!(
            results.rows.iter().any(|row| row
                .variant
                .as_ref()
                .and_then(|variant| variant.sku.as_deref())
                == Some(expected_sku)),
            "query {query:?} did not match {expected_sku}"
        );
    }

    fn fixture_catalog() -> Catalog {
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
                        path: "/store/products".to_owned(),
                    }],
                },
            },
            bike_variants: vec![
                BikeVariant {
                    id: "varg-ex".to_owned(),
                    code: "varg-ex".to_owned(),
                    display_name: Some("VARG EX".to_owned()),
                },
                BikeVariant {
                    id: "varg-sm".to_owned(),
                    code: "supermoto-code".to_owned(),
                    display_name: Some("VARG SM".to_owned()),
                },
            ],
            catalog_trees: vec![
                BikeCatalogTree {
                    bike_variant_id: "varg-ex".to_owned(),
                    categories: vec![
                        CategoryNode {
                            code: "brakes".to_owned(),
                            path: vec!["brakes".to_owned()],
                            display_name: Some("Brakes".to_owned()),
                            localization_key: None,
                            categories: vec![CategoryNode {
                                code: "front_brake".to_owned(),
                                path: vec!["brakes".to_owned(), "front_brake".to_owned()],
                                display_name: Some("Front brake".to_owned()),
                                localization_key: None,
                                categories: Vec::new(),
                                product_groups: vec![disc_group()],
                            }],
                            product_groups: Vec::new(),
                        },
                        CategoryNode {
                            code: "empty_category".to_owned(),
                            path: vec!["empty_category".to_owned()],
                            display_name: Some("Empty category".to_owned()),
                            localization_key: None,
                            categories: Vec::new(),
                            product_groups: Vec::new(),
                        },
                    ],
                },
                BikeCatalogTree {
                    bike_variant_id: "varg-sm".to_owned(),
                    categories: vec![CategoryNode {
                        code: "bodywork".to_owned(),
                        path: vec!["bodywork".to_owned()],
                        display_name: Some("Bodywork".to_owned()),
                        localization_key: None,
                        categories: Vec::new(),
                        product_groups: vec![ProductGroup {
                            code: "mirror_group".to_owned(),
                            display_name: Some("Mirror group".to_owned()),
                            description: None,
                            localization_key: None,
                            description_localization_key: None,
                            stark_url: None,
                            image_urls: Vec::new(),
                            articles: vec![Article {
                                code: "mirror".to_owned(),
                                display_name: Some("Mirror".to_owned()),
                                description: None,
                                localization_key: None,
                                description_localization_key: None,
                                stark_url: None,
                                image_urls: Vec::new(),
                                kit_memberships: Vec::new(),
                                kit_contents: Vec::new(),
                                variants: vec![variant("mirror", "SMX1-MIRROR")],
                            }],
                        }],
                    }],
                },
            ],
        }
    }

    fn disc_group() -> ProductGroup {
        ProductGroup {
            code: "disc_group".to_owned(),
            display_name: Some("Disc group".to_owned()),
            description: Some("group-only-description".to_owned()),
            localization_key: None,
            description_localization_key: None,
            stark_url: Some("https://www.starkfuture.com/us-US/parts/disc-group".to_owned()),
            image_urls: vec!["https://s3-stark-prod.s3.eu-central-1.amazonaws.com/catalog/disc-group.png".to_owned()],
            articles: vec![Article {
                code: "disc_260mm".to_owned(),
                display_name: Some("260mm Disc".to_owned()),
                description: Some("article-only-description".to_owned()),
                localization_key: None,
                description_localization_key: None,
                stark_url: Some("https://www.starkfuture.com/us-US/parts/disc-260mm".to_owned()),
                image_urls: vec!["https://s3-stark-prod.s3.eu-central-1.amazonaws.com/catalog/disc-260mm.png".to_owned()],
                kit_memberships: vec!["brake-service-kit".to_owned()],
                kit_contents: vec!["mount bolt".to_owned()],
                variants: vec![
                    ArticleVariant {
                        code: "disc_260mm-standard".to_owned(),
                        sku: Some("SMX1-BR-FW-260".to_owned()),
                        stark_url: Some("https://www.starkfuture.com/us-US/parts/disc-260mm?sku=SMX1-BR-FW-260".to_owned()),
                        image_urls: vec!["https://s3-stark-prod.s3.eu-central-1.amazonaws.com/catalog/disc-variant.png".to_owned()],
                        attributes: vec![AttributeSelection {
                            code: "disc_size".to_owned(),
                            option_code: "260mm".to_owned(),
                            option_display_name: Some("260mm".to_owned()),
                            option_localization_key: None,
                        }],
                        price: Some(Price {
                            amount_minor: 14900,
                            currency: "USD".to_owned(),
                        }),
                        availability: Some(Availability {
                            status: "AVAILABLE".to_owned(),
                            quantity: None,
                        }),
                    },
                    variant("disc_260mm-small", "SMX1-BR-FW-240"),
                ],
            }, Article {
                code: "brake_manual".to_owned(),
                display_name: Some("Brake manual".to_owned()),
                description: Some("Variantless reference article".to_owned()),
                localization_key: None,
                description_localization_key: None,
                stark_url: None,
                image_urls: Vec::new(),
                kit_memberships: Vec::new(),
                kit_contents: Vec::new(),
                variants: Vec::new(),
            }],
        }
    }

    fn variant(code: &str, sku: &str) -> ArticleVariant {
        ArticleVariant {
            code: code.to_owned(),
            sku: Some(sku.to_owned()),
            stark_url: None,
            image_urls: Vec::new(),
            attributes: Vec::new(),
            price: None,
            availability: None,
        }
    }
}

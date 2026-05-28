use stark_parts_catalog::{
    Article, ArticleVariant, AttributeSelection, Availability, Catalog, CategoryNode, Price,
    ProductGroup,
};
use std::collections::{HashMap, HashSet};
use url::form_urlencoded;

/// Browser-local search index derived entirely from the committed catalog.
///
/// The index is deliberately UI-agnostic. It knows how to normalize text,
/// apply bike filters, and project matching rows back into the catalog tree,
/// but it does not know anything about Leptos components or browser events.
pub struct SearchIndex {
    bike_variants: Vec<BikeVariantSummary>,
    full_trees: Vec<ProjectedCatalogTree>,
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

        let mut full_trees = Vec::new();
        let mut rows = Vec::new();
        for tree in &catalog.catalog_trees {
            let bike = bike_summaries.get(&tree.bike_variant_id);
            full_trees.push(project_catalog_tree(
                tree,
                bike.and_then(|bike| bike.display_name.clone()),
            ));
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
            full_trees,
            rows,
        }
    }

    /// Return the committed bike variants in the URL-stable order.
    pub fn bike_variants(&self) -> &[BikeVariantSummary] {
        &self.bike_variants
    }

    /// Search rows and project matching rows plus their ancestors as a tree.
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
            if row_matches_query(row, &query_tokens, &compact_query) {
                matched_rows.push(row);
            }
        }

        let is_empty_query = query_tokens.is_empty() && compact_query.is_empty();
        let trees = if is_empty_query {
            self.full_trees
                .iter()
                .filter(|tree| selected_all_bikes || selected_bikes.contains(&tree.bike_variant_id))
                .cloned()
                .collect()
        } else {
            project_rows(&matched_rows)
        };

        SearchResults {
            is_empty_query,
            rows: matched_rows.iter().map(|row| row.result.clone()).collect(),
            trees,
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

/// Search result rows plus the ancestor-preserving tree projection.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SearchResults {
    pub is_empty_query: bool,
    pub rows: Vec<SearchResultRow>,
    pub trees: Vec<ProjectedCatalogTree>,
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
    pub category_path: Vec<String>,
    pub category_display_path: Vec<String>,
    pub product_group: ProductGroupSummary,
    pub article: ArticleSummary,
    pub variant: Option<ArticleVariantSummary>,
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

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectedCatalogTree {
    pub bike_variant_id: String,
    pub bike_display_name: Option<String>,
    pub categories: Vec<ProjectedCategory>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectedCategory {
    pub code: String,
    pub display_name: Option<String>,
    pub categories: Vec<ProjectedCategory>,
    pub product_groups: Vec<ProjectedProductGroup>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectedProductGroup {
    pub code: String,
    pub display_name: Option<String>,
    pub articles: Vec<ProjectedArticle>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectedArticle {
    pub code: String,
    pub display_name: Option<String>,
    pub variants: Vec<ProjectedArticleVariant>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectedArticleVariant {
    pub code: String,
    pub sku: Option<String>,
}

struct SearchRow {
    bike_variant_id: String,
    normalized_text: String,
    compact_text: String,
    result: SearchResultRow,
}

fn project_catalog_tree(
    tree: &stark_parts_catalog::BikeCatalogTree,
    bike_display_name: Option<String>,
) -> ProjectedCatalogTree {
    ProjectedCatalogTree {
        bike_variant_id: tree.bike_variant_id.clone(),
        bike_display_name,
        categories: tree.categories.iter().map(project_category).collect(),
    }
}

fn project_category(category: &CategoryNode) -> ProjectedCategory {
    ProjectedCategory {
        code: category.code.clone(),
        display_name: category.display_name.clone(),
        categories: category.categories.iter().map(project_category).collect(),
        product_groups: category.product_groups.iter().map(project_group).collect(),
    }
}

fn project_group(group: &ProductGroup) -> ProjectedProductGroup {
    ProjectedProductGroup {
        code: group.code.clone(),
        display_name: group.display_name.clone(),
        articles: group.articles.iter().map(project_article).collect(),
    }
}

fn project_article(article: &Article) -> ProjectedArticle {
    ProjectedArticle {
        code: article.code.clone(),
        display_name: article.display_name.clone(),
        variants: article
            .variants
            .iter()
            .map(|variant| ProjectedArticleVariant {
                code: variant.code.clone(),
                sku: variant.sku.clone(),
            })
            .collect(),
    }
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
    };
    let searchable_text = searchable_fields(&result).join(" ");

    SearchRow {
        bike_variant_id: bike_variant_id.to_owned(),
        normalized_text: normalize_tokens(&searchable_text).join(" "),
        compact_text: compact_search_text(&searchable_text),
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

fn searchable_fields(result: &SearchResultRow) -> Vec<String> {
    let mut fields = Vec::new();
    fields.push(result.bike_variant_id.clone());
    push_optional(&mut fields, &result.bike_code);
    push_optional(&mut fields, &result.bike_display_name);
    fields.extend(result.category_path.iter().cloned());
    fields.extend(result.category_display_path.iter().cloned());
    fields.push(result.product_group.code.clone());
    push_optional(&mut fields, &result.product_group.display_name);
    push_optional(&mut fields, &result.product_group.description);
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

fn push_optional(fields: &mut Vec<String>, value: &Option<String>) {
    if let Some(value) = value {
        fields.push(value.clone());
    }
}

fn row_matches_query(row: &SearchRow, query_tokens: &[String], compact_query: &str) -> bool {
    if query_tokens.is_empty() && compact_query.is_empty() {
        return true;
    }

    query_tokens
        .iter()
        .all(|token| row.normalized_text.contains(token))
        || (!compact_query.is_empty() && row.compact_text.contains(compact_query))
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

fn project_rows(rows: &[&SearchRow]) -> Vec<ProjectedCatalogTree> {
    let mut trees = Vec::new();
    for row in rows {
        let tree = get_or_insert_tree(
            &mut trees,
            &row.result.bike_variant_id,
            row.result.bike_display_name.clone(),
        );
        let categories = insert_category_path(
            &mut tree.categories,
            &row.result.category_path,
            &row.result.category_display_path,
        );
        let group = get_or_insert_group(categories, &row.result.product_group);
        let article = get_or_insert_article(group, &row.result.article);
        let Some(variant) = &row.result.variant else {
            continue;
        };
        if !article
            .variants
            .iter()
            .any(|existing| existing.code == variant.code && existing.sku == variant.sku)
        {
            article.variants.push(ProjectedArticleVariant {
                code: variant.code.clone(),
                sku: variant.sku.clone(),
            });
        }
    }
    trees
}

fn get_or_insert_tree<'a>(
    trees: &'a mut Vec<ProjectedCatalogTree>,
    bike_variant_id: &str,
    bike_display_name: Option<String>,
) -> &'a mut ProjectedCatalogTree {
    if let Some(index) = trees
        .iter()
        .position(|tree| tree.bike_variant_id == bike_variant_id)
    {
        return &mut trees[index];
    }

    trees.push(ProjectedCatalogTree {
        bike_variant_id: bike_variant_id.to_owned(),
        bike_display_name,
        categories: Vec::new(),
    });
    trees.last_mut().expect("tree was just inserted")
}

fn insert_category_path<'a>(
    categories: &'a mut Vec<ProjectedCategory>,
    path: &[String],
    display_path: &[String],
) -> &'a mut Vec<ProjectedProductGroup> {
    let mut current_categories = categories;
    for (index, code) in path.iter().enumerate() {
        let category_index = current_categories
            .iter()
            .position(|category| category.code == *code)
            .unwrap_or_else(|| {
                current_categories.push(ProjectedCategory {
                    code: code.clone(),
                    display_name: display_path.get(index).cloned(),
                    categories: Vec::new(),
                    product_groups: Vec::new(),
                });
                current_categories.len() - 1
            });
        if index + 1 == path.len() {
            return &mut current_categories[category_index].product_groups;
        }
        current_categories = &mut current_categories[category_index].categories;
    }

    panic!("catalog rows must have at least one category")
}

fn get_or_insert_group<'a>(
    groups: &'a mut Vec<ProjectedProductGroup>,
    summary: &ProductGroupSummary,
) -> &'a mut ProjectedProductGroup {
    if let Some(index) = groups.iter().position(|group| group.code == summary.code) {
        return &mut groups[index];
    }

    groups.push(ProjectedProductGroup {
        code: summary.code.clone(),
        display_name: summary.display_name.clone(),
        articles: Vec::new(),
    });
    groups.last_mut().expect("group was just inserted")
}

fn get_or_insert_article<'a>(
    group: &'a mut ProjectedProductGroup,
    summary: &ArticleSummary,
) -> &'a mut ProjectedArticle {
    if let Some(index) = group
        .articles
        .iter()
        .position(|article| article.code == summary.code)
    {
        return &mut group.articles[index];
    }

    group.articles.push(ProjectedArticle {
        code: summary.code.clone(),
        display_name: summary.display_name.clone(),
        variants: Vec::new(),
    });
    group
        .articles
        .last_mut()
        .expect("article was just inserted")
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
    fn empty_query_returns_full_tree_for_all_bikes() {
        let index = SearchIndex::from_catalog(&fixture_catalog());
        let results = index.search(&SearchRequest::default());

        assert!(results.is_empty_query);
        assert_eq!(results.rows.len(), 4);
        assert_eq!(
            results
                .trees
                .iter()
                .map(|tree| tree.bike_variant_id.as_str())
                .collect::<Vec<_>>(),
            ["varg-ex", "varg-sm"]
        );
        assert_eq!(results.trees[0].categories[1].code, "empty_category");
        assert!(results.trees[0].categories[1].product_groups.is_empty());
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
                .trees
                .iter()
                .map(|tree| tree.bike_variant_id.as_str())
                .collect::<Vec<_>>(),
            ["varg-ex", "varg-sm"]
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
    fn no_result_query_returns_clear_empty_projection() {
        let index = SearchIndex::from_catalog(&fixture_catalog());
        let results = index.search(&SearchRequest {
            query: "does-not-exist".to_owned(),
            selected_bike_variant_ids: Vec::new(),
        });

        assert!(!results.has_matches());
        assert!(results.trees.is_empty());
    }

    #[test]
    fn projection_preserves_matching_row_ancestors() {
        let index = SearchIndex::from_catalog(&fixture_catalog());
        let results = index.search(&SearchRequest {
            query: "SMX1-BR-FW-260".to_owned(),
            selected_bike_variant_ids: Vec::new(),
        });

        let tree = &results.trees[0];
        let category = &tree.categories[0];
        let subcategory = &category.categories[0];
        let group = &subcategory.product_groups[0];
        let article = &group.articles[0];

        assert_eq!(tree.bike_variant_id, "varg-ex");
        assert_eq!(category.code, "brakes");
        assert_eq!(subcategory.code, "front_brake");
        assert_eq!(group.code, "disc_group");
        assert_eq!(article.code, "disc_260mm");
        assert_eq!(article.variants[0].sku.as_deref(), Some("SMX1-BR-FW-260"));
    }

    #[test]
    fn variantless_articles_remain_searchable_and_projected() {
        let index = SearchIndex::from_catalog(&fixture_catalog());
        let results = index.search(&SearchRequest {
            query: "brake manual".to_owned(),
            selected_bike_variant_ids: Vec::new(),
        });

        assert_eq!(results.rows.len(), 1);
        assert!(results.rows[0].variant.is_none());
        assert_eq!(
            results.trees[0].categories[0].categories[0].product_groups[0].articles[0].code,
            "brake_manual"
        );
        assert!(
            results.trees[0].categories[0].categories[0].product_groups[0].articles[0]
                .variants
                .is_empty()
        );
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

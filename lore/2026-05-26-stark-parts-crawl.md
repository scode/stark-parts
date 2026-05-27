# Stark VARG EX Parts Catalog API Notes

NOTE: This is not a complete catalog dump. This is a research handoff for another agent so it can crawl the Stark VARG EX public parts catalog without repeating the discovery work. The endpoints below were verified against the public website on May 25, 2026 from a US locale.

The useful data is not in the initial static page in a clean browser session. The page first shows a country/language confirmation modal, then the catalog UI hydrates. Once hydrated, the visible catalog is ordinary links, but scraping those links is the wrong layer. The frontend uses a public JSON API under `https://api.starkfuture.com/v2/store`, and that API exposes the catalog tree, product groups, product details, articles, variants, SKUs, availability, prices, images, and technical drawing image URLs.

The catalog page I started from was:

```text
https://starkfuture.com/parts-and-accessories/spare-parts/varg-ex
```

The relevant frontend bundle showed these endpoint definitions:

```text
base: https://api.starkfuture.com/v2/

GET store/categories
GET store/categories/{code}
GET store/products
GET store/products/{code}
GET store/articles/suggestions
```

## The TLDR

Use the API directly.

Root categories:

```text
GET https://api.starkfuture.com/v2/store/categories?product_tag=varg-ex&path=SP
```

Children of a branch category:

```text
GET https://api.starkfuture.com/v2/store/categories?product_tag=varg-ex&path=SP/brakes
GET https://api.starkfuture.com/v2/store/categories?product_tag=varg-ex&path=SP/suspension
```

Product groups inside a leaf category:

```text
GET https://api.starkfuture.com/v2/store/products?category=bodywork&tags=varg-ex
GET https://api.starkfuture.com/v2/store/products?category=brakes_front_brake&tags=varg-ex
```

Product detail, including article leaves, variants, SKUs, prices, and availability:

```text
GET https://api.starkfuture.com/v2/store/products/9_seat?tags=varg-ex&country=US
```

Search suggestions:

```text
GET https://api.starkfuture.com/v2/store/articles/suggestions?tag=varg-ex&query=seat&lang=en-US&limit=5
```

The API uses `varg-ex` as the model tag. Use `country=US` on product detail if you want US pricing. Other countries may change price, currency, availability, or tax behavior. I did not test non-US countries.

## Catalog Tree Shape

The root endpoint returned 13 top-level categories. Categories have this shape:

```json
{
  "code": "bodywork",
  "name_key": "spare_parts_category_bodywork_name",
  "image_url": "https://s3-stark-prod.s3.eu-central-1.amazonaws.com/spare-parts-images/Bodywork.webp",
  "is_leaf": true,
  "path": "SP"
}
```

The important fields are:

- `code`: the category code used for routing and product-list API calls.
- `name_key`: localization key, not the display text.
- `image_url`: category image.
- `is_leaf`: if `true`, call `store/products`; if `false`, recurse with `store/categories`.
- `path`: parent path used to construct the next category request.

Observed VARG EX tree:

```text
varg-ex
  accessories
  apparel
  bodywork
  brakes
    brakes_front_brake
    brakes_rear_foot_brake
    brakes_rear_hand_brake
  chain_sprockets
  cooling
  electronics
  frame_cat
  handlebar_controls
  powertrain
  suspension
    suspension_fork
    suspension_linkage
    suspension_shock
    triple_clamp
  titanium_parts
  wheels
```

I found 2 branch categories (`brakes`, `suspension`) and 18 leaf categories. The leaf product-group counts I measured were:

```text
accessories: 4
apparel: 3
bodywork: 10
brakes/brakes_front_brake: 6
brakes/brakes_rear_foot_brake: 7
brakes/brakes_rear_hand_brake: 7
chain_sprockets: 4
cooling: 3
electronics: 6
frame_cat: 7
handlebar_controls: 6
powertrain: 4
suspension/suspension_fork: 1
suspension/suspension_linkage: 2
suspension/suspension_shock: 1
suspension/triple_clamp: 1
titanium_parts: 2
wheels: 5
```

That totaled 79 product groups at the time I checked. Treat this as a sanity check, not a permanent invariant.

## Traversal Logic

Here is the crawl logic I would use:

1. Start with `GET /store/categories?product_tag=varg-ex&path=SP`.
2. For each returned category:
   - If `is_leaf` is `true`, fetch `GET /store/products?category={code}&tags=varg-ex`.
   - If `is_leaf` is `false`, recurse into `GET /store/categories?product_tag=varg-ex&path={path}/{code}`.
3. For each product group from `store/products`, fetch `GET /store/products/{product_code}?tags=varg-ex&country=US`.
4. Flatten product details into index rows. The best leaf-level searchable unit is probably a variant/SKU row with denormalized category, product, article, and variant data.

Pseudo-code:

```js
const API = "https://api.starkfuture.com/v2";
const MODEL = "varg-ex";
const COUNTRY = "US";

async function getJson(path, params) {
  const url = new URL(API + path);
  for (const [key, value] of Object.entries(params)) {
    url.searchParams.set(key, value);
  }

  const response = await fetch(url);
  if (!response.ok) {
    throw new Error(`${response.status} ${url}`);
  }
  return response.json();
}

async function walkCategories(path = "SP", trail = []) {
  const categories = await getJson("/store/categories", {
    product_tag: MODEL,
    path,
  });

  const leaves = [];
  for (const category of categories) {
    const nextTrail = [...trail, category.code];
    if (category.is_leaf) {
      leaves.push({ category, trail: nextTrail });
    } else {
      leaves.push(...await walkCategories(`${category.path}/${category.code}`, nextTrail));
    }
  }
  return leaves;
}

async function fetchLeafProducts(categoryCode) {
  return getJson("/store/products", {
    category: categoryCode,
    tags: MODEL,
  });
}

async function fetchProductDetail(productCode) {
  return getJson(`/store/products/${productCode}`, {
    tags: MODEL,
    country: COUNTRY,
  });
}
```

## Product Group Shape

The product-list endpoint returns product groups, not the final SKU leaves. Example from `category=bodywork&tags=varg-ex`:

```json
{
  "code": "9_seat",
  "name_key": "spare_parts_product_9_seat_name",
  "description_key": "spare_parts_product_9_seat_description",
  "image_url": "https://s3-stark-prod.s3.eu-central-1.amazonaws.com/spare-parts-images/Seat.webp",
  "category_code": "bodywork"
}
```

Fetch the detail endpoint for each `code` if you need SKUs, prices, variants, article names, technical drawings, or kit contents.

## Product Detail Shape

Example endpoint:

```text
GET https://api.starkfuture.com/v2/store/products/9_seat?tags=varg-ex&country=US
```

The top-level response has the product group:

```json
{
  "code": "9_seat",
  "name_key": "spare_parts_product_9_seat_name",
  "description_key": "spare_parts_product_9_seat_description",
  "feature_image_url": "https://s3-stark-prod.s3.eu-central-1.amazonaws.com/spare-parts-images/Seat.webp",
  "category": {
    "code": "bodywork",
    "name_key": "spare_parts_category_bodywork_name",
    "image_url": "https://s3-stark-prod.s3.eu-central-1.amazonaws.com/spare-parts-images/Bodywork.webp",
    "path": "SP"
  },
  "articles": []
}
```

Each `articles[]` entry wraps a specific article with a drawing reference and default variant:

```json
{
  "reference": 2,
  "default_variant": "28_seat_assembly-seat_color.jet_black",
  "article": {
    "code": "28_seat_assembly",
    "name_key": "spare_parts_product_28_seat_assembly_name",
    "description_key": "spare_parts_product_28_seat_assembly_description",
    "tags": ["varg", "varg-ex", "varg-1.2", "varg-sm"],
    "image_url": "https://s3-stark-prod.s3.eu-central-1.amazonaws.com/spare-parts-images/SMX1-P-ST.webp",
    "attributes": [],
    "variants": [],
    "is_kit": false,
    "kit_contain": []
  }
}
```

The final SKU-level data is in `article.variants[]`:

```json
{
  "code": "28_seat_assembly-seat_color.jet_black",
  "tags": [],
  "skus": ["SMX1-P-ST-B"],
  "availability": "AVAILABLE",
  "price": {
    "subtotal": 149,
    "vat_percentage": 0,
    "discount_percentage": 0,
    "total": 149
  },
  "attributes": [
    {
      "attribute": {
        "_id": "01ebe27b-a622-4bdd-ae33-6ee86f81f30f",
        "code": "seat_color"
      },
      "selectedOption": {
        "code": "jet_black",
        "nameKey": "spare_parts_attribute_option_jet_black_name"
      }
    }
  ]
}
```

For `9_seat`, I saw:

```text
articles: 5
variants: 6
skus:
  SMX1-P-ST-B
  SMX1-P-ST-G
  SMX1-P-ST-B-RB
  SMX1-P-ST-B-SG
  STD-SC-0007
  STD-SC-0019
```

For `brakes_front_brake`, the product group `14_disc` had:

```text
articles: 3
variants: 3
skus:
  SMX1-BR-FW-260
  SMX1-BR-FW-260-X
  I14580-060012-08-P
```

## Suggested Index Row

For search, I would flatten to one row per variant/SKU. A variant can contain multiple SKUs, so either keep `skus` as an array or explode to one row per SKU depending on how exact SKU lookup should behave.

Useful fields:

```json
{
  "model": "varg-ex",
  "country": "US",
  "category_path": ["bodywork"],
  "category_code": "bodywork",
  "category_name_key": "spare_parts_category_bodywork_name",
  "product_code": "9_seat",
  "product_name_key": "spare_parts_product_9_seat_name",
  "product_description_key": "spare_parts_product_9_seat_description",
  "product_image_url": "https://...",
  "article_reference": 2,
  "article_code": "28_seat_assembly",
  "article_name_key": "spare_parts_product_28_seat_assembly_name",
  "article_description_key": "spare_parts_product_28_seat_assembly_description",
  "article_image_url": "https://...",
  "is_kit": false,
  "kit_contain": [],
  "variant_code": "28_seat_assembly-seat_color.jet_black",
  "skus": ["SMX1-P-ST-B"],
  "availability": "AVAILABLE",
  "price_total": 149,
  "price_subtotal": 149,
  "vat_percentage": 0,
  "discount_percentage": 0,
  "attributes": [
    {
      "attribute_code": "seat_color",
      "option_code": "jet_black",
      "option_name_key": "spare_parts_attribute_option_jet_black_name"
    }
  ]
}
```

For a usable text index, include the raw codes and SKUs even if localization fails. Codes like `28_seat_assembly` and SKUs like `SMX1-P-ST-B` are valuable search terms on their own.

## Localization

The API returns localization keys rather than English display strings. The site page includes a large Next.js payload with the English translation table. I verified these mappings in the page HTML:

```text
spare_parts_category_bodywork_name => Bodywork
spare_parts_product_9_seat_name => Seat
spare_parts_product_28_seat_assembly_name => Original Seat
spare_parts_attribute_option_jet_black_name => Jet Black
```

Options for another agent:

- Good enough: index both localization keys and machine-readable codes. This avoids needing translation extraction for a first pass.
- Better: fetch the catalog page HTML and extract the translation map from the `self.__next_f.push(...)` payload.
- Also useful: the `store/articles/suggestions` endpoint accepts `lang=en-US`, but in my check it still returned localization keys in the JSON. The frontend resolves those keys client-side.

I did not identify a clean standalone `/translations/en-US` endpoint. It may exist, but the page payload is enough if display strings matter.

## Search Suggestions Endpoint

The frontend search box uses:

```text
GET /store/articles/suggestions?tag=varg-ex&query={query}&lang={lang}&limit={limit}
```

Example:

```text
GET https://api.starkfuture.com/v2/store/articles/suggestions?tag=varg-ex&query=seat&lang=en-US&limit=5
```

The response shape is paginated:

```json
{
  "totalDocs": 7,
  "docs": [
    {
      "code": "28_seat_assembly",
      "name_key": "spare_parts_product_28_seat_assembly_name",
      "description_key": "spare_parts_product_28_seat_assembly_description",
      "image_url": "https://...",
      "tags": ["varg", "varg-ex", "varg-1.2", "varg-sm"],
      "related_products": [
        {
          "code": "9_seat",
          "name_key": "spare_parts_product_9_seat_name",
          "category_path": "SP",
          "category_code": "bodywork",
          "category_name_key": "spare_parts_category_bodywork_name",
          "product_code": "9_seat"
        }
      ]
    }
  ],
  "page": 1,
  "hasPrevPage": false,
  "hasNextPage": false,
  "prevPage": null,
  "nextPage": null,
  "totalPages": 2
}
```

This is useful as a comparison point for your own index, but I would not use it as the primary crawl source because it is query-driven and limited. The category/product endpoints are better for full ingestion.

## Gotchas

The `path` field is a parent path, not always the full path to the category. When recursing, use `{category.path}/{category.code}` for the next `path` value. For example, root `brakes` has `path: "SP"`, so its children are fetched with `path=SP/brakes`.

Leaf categories are not all under `SP` according to their `path` field. I saw `cooling` with `path: "SP/cooling"` and `powertrain` with `path: "SP/powertrain"` even though they were returned by the root query. Do not rely on `path` alone to infer hierarchy; carry your own traversal trail while crawling.

The website can render empty until the country/language confirmation state is set. This matters if you are using browser automation to inspect the UI. It does not appear to matter for direct API calls.

The product-list endpoint returns product groups. Those are not the final parts. Always fetch product detail if you want the leaves.

Prices are country-dependent. I only checked `country=US`.

Availability values I saw include `AVAILABLE`. The frontend code also handles `AVAILABLE_HQ` and `NOT_AVAILABLE`.

Images are mostly S3 URLs. Keep them as URLs rather than trying to download them during initial indexing.

## Minimal Crawl Script Sketch

This is intentionally plain. It is meant to show the API sequence, not be production code.

```js
const API = "https://api.starkfuture.com/v2";
const MODEL = "varg-ex";
const COUNTRY = "US";

async function getJson(path, params = {}) {
  const url = new URL(API + path);
  for (const [key, value] of Object.entries(params)) {
    url.searchParams.set(key, value);
  }
  const response = await fetch(url);
  if (!response.ok) {
    throw new Error(`${response.status} ${response.statusText}: ${url}`);
  }
  return response.json();
}

async function categoryLeaves(path = "SP", trail = []) {
  const categories = await getJson("/store/categories", {
    product_tag: MODEL,
    path,
  });

  const out = [];
  for (const category of categories) {
    const nextTrail = [...trail, category.code];
    if (category.is_leaf) {
      out.push({ category, trail: nextTrail });
    } else {
      out.push(...await categoryLeaves(`${category.path}/${category.code}`, nextTrail));
    }
  }
  return out;
}

async function crawl() {
  const rows = [];
  const leaves = await categoryLeaves();

  for (const leaf of leaves) {
    const products = await getJson("/store/products", {
      category: leaf.category.code,
      tags: MODEL,
    });

    for (const product of products) {
      const detail = await getJson(`/store/products/${product.code}`, {
        tags: MODEL,
        country: COUNTRY,
      });

      for (const articleWrapper of detail.articles ?? []) {
        const article = articleWrapper.article;
        for (const variant of article.variants ?? []) {
          rows.push({
            model: MODEL,
            country: COUNTRY,
            categoryPath: leaf.trail,
            categoryCode: leaf.category.code,
            categoryNameKey: leaf.category.name_key,
            productCode: detail.code,
            productNameKey: detail.name_key,
            articleReference: articleWrapper.reference,
            articleCode: article.code,
            articleNameKey: article.name_key,
            variantCode: variant.code,
            skus: variant.skus ?? [],
            availability: variant.availability ?? null,
            price: variant.price ?? null,
            articleImageUrl: article.image_url ?? null,
            productImageUrl: detail.feature_image_url ?? product.image_url ?? null,
            isKit: article.is_kit ?? false,
            kitContain: article.kit_contain ?? [],
            attributes: (variant.attributes ?? []).map((item) => ({
              attributeCode: item.attribute?.code ?? null,
              optionCode: item.selectedOption?.code ?? null,
              optionNameKey: item.selectedOption?.nameKey ?? null,
            })),
          });
        }
      }
    }
  }

  return rows;
}

console.log(JSON.stringify(await crawl(), null, 2));
```

## Discovery Notes

The first browser view showed only page chrome and the search input because the country confirmation modal blocked the useful catalog content. After confirming `United States / English`, the top-level category links appeared in the DOM.

Direct navigation to a guessed category URL was unreliable before state was settled. The API was much more reliable once the frontend bundle revealed the endpoint names.

The relevant route logic was in a Next/Turbopack chunk and used RTK Query. The important bit was:

```text
getCategories:         url "store/categories", params { product_tag: model, path: `${path}/${subCategory}` }
getCategoryByCode:     url `store/categories/${code}`
getProductsByCategory: url "store/products", params { category, tags: tags.join(",") }
getProductByCode:      url `store/products/${code}`, params { tags: tags.join(","), country }
searchArticles:        url "store/articles/suggestions", params { tag, query, lang, limit }
```

That is the observed evidence behind the crawl plan above.

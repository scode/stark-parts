import { expect, test } from "@playwright/test";

test("static app restores URL state and searches without Stark API calls", async ({ page }) => {
  // Fixed interaction examples keep catalog removals separate from UI regressions.
  // A separate smoke test below loads the actual deployed snapshot.
  await page.route("**/stark-parts.json5", (route) =>
    route.fulfill({ path: "tests/fixtures/interaction-catalog.json", contentType: "application/json5" }),
  );
  const catalogApiRequests = [];
  const staticCatalogRequests = [];
  const analyticsScriptRequests = [];
  await page.route("https://s3-stark-*/**", (route) => route.abort());
  await page.route("https://assets.starkfuture.com/**", (route) => route.abort());
  await page.route("**/_vercel/insights/script.js", (route) => {
    analyticsScriptRequests.push(route.request().url());
    route.fulfill({
      contentType: "application/javascript",
      body: "window.__starkPartsAnalyticsLoaded = true;",
    });
  });
  page.on("request", (request) => {
    const url = request.url();
    if (url.endsWith("/stark-parts.json5")) {
      staticCatalogRequests.push(url);
    }
    if (url.includes("api.starkfuture.com/") || url.includes("/product-catalog/")) {
      catalogApiRequests.push(url);
    }
  });

  await page.goto("/?q=SSM1-P-FF-01-G&bike=varg-sm");

  expect(analyticsScriptRequests).toHaveLength(1);
  await expect(page.getByText("Unofficial but fast parts catalog source")).toBeVisible();
  await expect(page.locator(".notice em")).toHaveText("fast");
  await expect(page.getByLabel("Search")).toHaveValue("SSM1-P-FF-01-G");
  await expect(page.getByLabel("Search")).toBeFocused();
  await expect(page.locator('input[type="checkbox"][value="varg-sm"]')).toBeChecked();
  await expect(page.getByLabel("Bike filters")).toBeVisible();
  const searchBox = await page.getByLabel("Search").boundingBox();
  const bikeFilters = await page.getByLabel("Bike filters").boundingBox();
  expect(searchBox).not.toBeNull();
  expect(bikeFilters).not.toBeNull();
  expect(bikeFilters.y).toBeGreaterThan(searchBox.y + searchBox.height);
  await expect(page.getByText("default: all bikes")).toHaveCount(0);
  await page.locator('input[type="checkbox"][value="varg-sm"]').uncheck();
  await expect(page.getByText("default: all bikes")).toBeVisible();
  await page.locator('input[type="checkbox"][value="varg-sm"]').check();
  await expect(page.getByText("default: all bikes")).toHaveCount(0);
  await expect(page.getByText("Parts data last updated")).toBeVisible();
  await expect(page.getByText(/\d{4}-\d{2}-\d{2}/).first()).toBeVisible();
  await expect(page.getByText(/\d{2}:\d{2}:\d{2}/)).toHaveCount(0);
  await expect(page.getByText("US storefront")).toBeVisible();
  await expect(page.getByText("Feedback/contact")).toBeVisible();
  await expect(page.getByText("peter.schuller@infidyne.com")).toBeVisible();
  await expect(page.getByRole("link", { name: "github.com/scode/stark-parts" })).toHaveAttribute(
    "href",
    "https://github.com/scode/stark-parts",
  );
  await expect(page.getByText("SSM1-P-FF-01-G").first()).toBeVisible();
  await expect(page.locator(".result-detail-popover .result-card")).toHaveCount(0);
  await page.getByText("SSM1-P-FF-01-G").first().hover();
  await expect(page.getByText("Price and availability are from the committed catalog snapshot").first()).toBeVisible();
  await expect(page.locator(".result-detail-popover .part-image").first()).toBeAttached();
  await expect(page.locator(".result-detail-popover .part-image-frame.image-frame-missing").first()).toBeVisible();
  await expect(page.locator(".result-thumb-frame.image-frame-missing").first()).toBeVisible();

  await page.getByLabel("Search").fill("SMX1-TOOLBOX");
  await expect(page.locator(".result-row")).toHaveCount(1);
  await expect(page.locator(".result-detail-popover .result-card")).toHaveCount(1);
  await expect(page.locator(".result-label").first()).toHaveText("Stark VARG toolbox");
  await expect(page.locator(".result-meta").first()).toHaveText("SMX1-TOOLBOX");
  await expect(page.getByText("SMX1-TOOLBOX").first()).toBeVisible();
  await page.getByText("SMX1-TOOLBOX").first().hover();
  await expect(page.locator(".result-row").first()).toHaveClass(/result-row-active/);

  const starkLinks = await page
    .locator(".result-detail-popover a.stark-link")
    .evaluateAll((links) => links.map((link) => link.href));
  await page.locator(".result-detail-popover a.stark-link").first().hover();
  await expect(page.locator(".result-detail-popover a.stark-link").first()).toBeVisible();
  await page.getByLabel("Search").hover();
  await expect(page.locator(".result-detail-popover a.stark-link").first()).toBeVisible();
  await expect(page.locator(".result-row").first()).toHaveClass(/result-row-active/);

  await page.getByLabel("Search").fill("SMX1");
  await expect(page.getByText("SMX1-TOOLBOX").first()).toBeVisible();
  await expect(page.getByText("SMX1-TRAILSAVER").first()).toBeVisible();
  await expect(page.locator(".result-list")).not.toHaveClass(/result-list-compact/);
  await page.getByRole("button", { name: "Compact" }).click();
  await expect(page.locator(".result-list")).toHaveClass(/result-list-compact/);
  await expect(page.locator(".result-thumb-frame").first()).not.toBeVisible();
  await page.getByRole("button", { name: "Default" }).click();
  await expect(page.locator(".result-list")).not.toHaveClass(/result-list-compact/);
  await expect(page.locator(".result-thumb-frame").first()).toBeVisible();
  await page.getByText("SMX1-TOOLBOX").first().hover();
  await expect(page.locator(".result-row").first()).toHaveClass(/result-row-active/);
  await expect(page.locator(".result-detail-popover").getByText("SMX1-TOOLBOX").first()).toBeVisible();
  await page.getByText("SMX1-TRAILSAVER").first().hover();
  await expect(page.locator(".result-row").first()).not.toHaveClass(/result-row-active/);
  await expect(page.locator(".result-row").nth(1)).toHaveClass(/result-row-active/);
  await expect(page.locator(".result-detail-popover").getByText("SMX1-TRAILSAVER").first()).toBeVisible();

  expect(starkLinks.length).toBeGreaterThan(0);
  for (const href of starkLinks) {
    const url = new URL(href);
    expect(url.protocol).toBe("https:");
    expect(["starkfuture.com", "www.starkfuture.com"]).toContain(url.hostname);
    expect(url.username).toBe("");
    expect(url.password).toBe("");
    expect(url.hash).toBe("");
  }

  await page.getByLabel("Search").fill("definitely-not-a-real-part");
  await expect(page.getByText("No matching catalog entries").first()).toBeVisible();
  await expect(page.locator(".result-detail-popover .result-card")).toHaveCount(0);
  await expect(page.locator(".result-row-active")).toHaveCount(0);

  await page.getByLabel("Search").fill("wiring harness");
  await expect(page.getByText("Frame front cable holder").first()).toBeVisible();
  await expect(page.getByText("matched group: Wiring harness").first()).toBeVisible();

  expect(catalogApiRequests).toEqual([]);
  expect(staticCatalogRequests).toHaveLength(1);
});

test("deployed catalog remains searchable after an upstream refresh", async ({ page }) => {
  const requests = [];
  page.on("request", (request) => {
    if (request.url().includes("api.starkfuture.com/")) requests.push(request.url());
  });
  await page.route("https://assets.starkfuture.com/**", (route) => route.abort());
  await page.goto("/");
  // Discover a current SKU instead of requiring a product to stay on sale forever.
  const firstSku = page.locator(".result-meta").first();
  await expect(firstSku).toBeVisible();
  const sku = await firstSku.innerText();
  expect(sku.trim()).not.toBe("");
  await page.getByLabel("Search").fill(sku);
  await expect(page.locator(".result-meta").first()).toHaveText(sku);
  await page.locator(".result-row").first().hover();
  await expect(page.locator(".result-detail-popover")).toContainText(sku);
  expect(requests).toEqual([]);
});

test("shared kit SKUs show their own contents, links, and fitment restrictions", async ({ page }) => {
  // Frozen schema-2 data covers behavior independently of current source membership.
  await page.route("**/stark-parts.json5", (route) =>
    route.fulfill({ path: "tests/fixtures/shared-catalog.json", contentType: "application/json5" }),
  );
  await page.goto("/?q=body+kit+snow+white");
  await expect(page.locator(".result-row")).toHaveCount(1);
  await expect(page.locator(".result-meta").first()).toHaveText("FIXTURE-KIT-W");
  await page.locator(".result-row").first().hover();
  const details = page.locator(".result-detail-popover");
  await expect(details).toContainText("2 × Snow White panel");
  await expect(details).not.toContainText("Forest grey");
  await expect(details).toContainText("Fitment restrictions");
  await expect(details).toContainText("gen2");
  await expect(details.locator("a.stark-link")).toHaveAttribute(
    "href",
    "https://starkfuture.com/product/fixture-body-kit?variant=white",
  );
  await page.getByLabel("Search").fill("body kit stark red");
  await expect(page.locator(".result-row")).toHaveCount(1);
  await expect(page.locator(".result-meta").first()).toHaveText("FIXTURE-KIT-R");
  await page.locator(".result-row").first().hover();
  await expect(details).toContainText("2 × Stark Red panel");
  await expect(details).not.toContainText("Snow White");
});

test("static app distinguishes a catalog load failure from empty search results", async ({ page }) => {
  await page.route("**/stark-parts.json5", (route) => route.abort());

  await page.goto("/");

  await expect(page.getByRole("alert")).toContainText("The parts catalog could not be loaded");
  await expect(page.getByText("No matching catalog entries")).toHaveCount(0);
  await expect(page.getByLabel("Search")).toBeDisabled();
});

test("static app keeps its orientation visible while the catalog is loading", async ({ page }) => {
  let releaseCatalog;
  const catalogGate = new Promise((resolve) => {
    releaseCatalog = resolve;
  });
  await page.route("**/stark-parts.json5", async (route) => {
    await catalogGate;
    await route.continue();
  });

  await page.goto("/");

  await expect(page.getByText("Unofficial but fast parts catalog source")).toBeVisible();
  await expect(page.getByText("Loading parts catalog…")).toBeVisible();
  await expect(page.getByLabel("Search")).toBeDisabled();

  releaseCatalog();
  await expect(page.getByLabel("Search")).toBeEnabled();
});

test("static app rejects a malformed catalog response", async ({ page }) => {
  await page.route("**/stark-parts.json5", (route) =>
    route.fulfill({
      contentType: "application/json5",
      body: "this is not a catalog",
    }),
  );

  await page.goto("/");

  await expect(page.getByRole("alert")).toContainText("The parts catalog could not be loaded");
  await expect(page.getByRole("alert")).toContainText("catalog response was invalid");
  await expect(page.getByLabel("Search")).toBeDisabled();
});

import { expect, test } from "@playwright/test";

test("static app restores URL state and searches without Stark API calls", async ({ page }) => {
  const catalogApiRequests = [];
  const analyticsScriptRequests = [];
  await page.route("https://s3-stark-*/**", (route) => route.abort());
  await page.route("**/_vercel/insights/script.js", (route) => {
    analyticsScriptRequests.push(route.request().url());
    route.fulfill({
      contentType: "application/javascript",
      body: "window.__starkPartsAnalyticsLoaded = true;",
    });
  });
  page.on("request", (request) => {
    const url = request.url();
    if (url.includes("api.starkfuture.com/v2/store") || url.includes("/v2/store/")) {
      catalogApiRequests.push(url);
    }
  });

  await page.goto("/?q=SSM1-P-FF-01-G&bike=varg-sm");

  expect(analyticsScriptRequests).toHaveLength(1);
  await expect(page.getByText("Unofficial catalog helper")).toBeVisible();
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
  await expect(page.getByText("Frame cable holder").first()).toBeVisible();
  await expect(page.getByText("matched group: Wiring harness").first()).toBeVisible();

  expect(catalogApiRequests).toEqual([]);
});

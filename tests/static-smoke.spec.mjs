import { expect, test } from "@playwright/test";

test("static app restores URL state and searches without Stark API calls", async ({ page }) => {
  const catalogApiRequests = [];
  await page.route("https://s3-stark-*/**", (route) => route.abort());
  page.on("request", (request) => {
    const url = request.url();
    if (url.includes("api.starkfuture.com/v2/store") || url.includes("/v2/store/")) {
      catalogApiRequests.push(url);
    }
  });

  await page.goto("/?q=SSM1-P-FF-01-G&bike=varg-sm");

  await expect(page.getByText("Unofficial catalog helper")).toBeVisible();
  await expect(page.getByLabel("Search")).toHaveValue("SSM1-P-FF-01-G");
  await expect(page.locator('input[type="checkbox"][value="varg-sm"]')).toBeChecked();
  await expect(page.getByText("Generated")).toBeVisible();
  await expect(page.getByText("US storefront")).toBeVisible();
  await expect(page.getByText("SSM1-P-FF-01-G").first()).toBeVisible();
  await expect(page.getByText("Price and availability are from the committed catalog snapshot").first()).toBeVisible();
  await expect(page.locator('img[loading="lazy"][referrerpolicy="no-referrer"]').first()).toBeAttached();

  await page.getByLabel("Search").fill("SMX1-TOOLBOX");
  await expect(page.getByText("SMX1-TOOLBOX").first()).toBeVisible();

  const starkLinks = await page.locator("a.stark-link").evaluateAll((links) => links.map((link) => link.href));
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

  expect(catalogApiRequests).toEqual([]);
});

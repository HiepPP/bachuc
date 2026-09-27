import { expect, test } from "../support/fixtures";
import { gotoAppShell } from "../support/helpers/app";
import { getServerId } from "../support/helpers/server-id";
import { installUsageReportsFixture } from "../support/helpers/usage-reports";

test.describe("usage screen", () => {
  test("opens from the sidebar and groups reports under their host", async ({ page }) => {
    test.setTimeout(120_000);
    const serverId = getServerId();
    const usage = await installUsageReportsFixture(page, {
      lists: [
        [
          {
            id: "alpha:a",
            account: {},
            fetchedAt: "2026-01-01T00:00:00.000Z",
            sourceId: "alpha",
            sourceLabel: "Alpha plan",
            report: {
              status: "available",
              windows: [{ id: "weekly", label: "Weekly", usedPct: 31, headline: true }],
            },
          },
          {
            id: "beta:b",
            account: {},
            fetchedAt: "2026-01-01T00:00:00.000Z",
            sourceId: "beta",
            sourceLabel: "Beta plan",
            report: { status: "unavailable", windows: [] },
          },
        ],
      ],
    });

    await gotoAppShell(page);
    await page.locator('[data-testid="sidebar-usage"]:visible').first().click();
    await expect(page).toHaveURL(/\/usage$/);
    await usage.waitForListRequests(1);

    const group = page.getByTestId(`usage-host-${serverId}`);
    await expect(group.getByText("Alpha plan", { exact: true })).toBeVisible({ timeout: 10_000 });
    await expect(group.getByText("31%")).toBeVisible();
    await expect(group.getByText("Beta plan", { exact: true })).toBeVisible();
    await expect(group.getByText("Unavailable", { exact: true })).toBeVisible();
  });

  test("shows the host once it connects after a cold load on a phone", async ({ page }) => {
    test.setTimeout(120_000);
    const serverId = getServerId();
    const usage = await installUsageReportsFixture(page, {
      lists: [
        [
          {
            id: "alpha:a",
            account: {},
            fetchedAt: "2026-01-01T00:00:00.000Z",
            sourceId: "alpha",
            sourceLabel: "Alpha plan",
            report: { status: "available", windows: [] },
          },
        ],
      ],
    });

    await page.setViewportSize({ width: 390, height: 844 });
    await page.goto("/usage");
    await usage.waitForListRequests(1);

    await expect(
      page.getByTestId(`usage-host-${serverId}`).getByText("Alpha plan", { exact: true }),
    ).toBeVisible({ timeout: 10_000 });
  });

  test("tells the user to update a host without usage sources", async ({ page }) => {
    test.setTimeout(120_000);
    const serverId = getServerId();
    const usage = await installUsageReportsFixture(page, { usageSources: false });

    await gotoAppShell(page);
    await page.locator('[data-testid="sidebar-usage"]:visible').first().click();

    await expect(
      page.getByTestId(`usage-host-${serverId}`).getByText("Update the host to see usage", {
        exact: true,
      }),
    ).toBeVisible({ timeout: 10_000 });
    expect(usage.listRequests()).toHaveLength(0);
  });
});

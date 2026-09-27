import type { UsageReportEntry } from "@getpaseo/protocol/messages";
import { expect, test, type Page } from "../support/fixtures";
import { expectComposerVisible, submitMessage } from "../support/helpers/composer";
import { openAgentRoute, seedMockAgentWorkspace } from "../support/helpers/mock-agent";
import { installUsageReportsFixture } from "../support/helpers/usage-reports";

function agentEntry(usedPct: number): UsageReportEntry {
  return {
    id: "fixture:fixture-account",
    account: {},
    fetchedAt: "2026-01-01T00:00:00.000Z",
    sourceId: "fixture",
    sourceLabel: "Fixture plan",
    icon: '<svg viewBox="0 0 24 24"><rect width="24" height="24" fill="currentColor"/></svg>',
    report: {
      status: "available",
      planLabel: "Test plan",
      windows: [
        { id: "session", label: "Session", usedPct: 3 },
        { id: "weekly", label: "Weekly", usedPct, headline: true },
      ],
    },
  };
}

async function openMockAgent(page: Page) {
  const session = await seedMockAgentWorkspace({
    repoPrefix: "usage-composer-pill-",
    title: "Usage composer pill e2e",
    initialPrompt: "emit 1 coalesced agent stream update for usage composer pill.",
  });
  await openAgentRoute(page, session);
  await expectComposerVisible(page);
  return session;
}

test.describe("usage composer pill", () => {
  test("shows the headline percent and opens the usage card with a forced refresh", async ({
    page,
  }) => {
    test.setTimeout(180_000);
    const usage = await installUsageReportsFixture(page, {
      agentReportIds: ["fixture:fixture-account"],
      lists: [[agentEntry(42)], [agentEntry(64)]],
    });
    const session = await openMockAgent(page);
    try {
      const pill = page.getByTestId("usage-composer-pill");
      await expect(pill).toContainText("42%", { timeout: 30_000 });
      expect(usage.agentRequests()[0]).toEqual({ agentId: session.agentId });

      const requestsBeforeOpen = usage.listRequests().length;
      await pill.click();
      await usage.waitForListRequests(requestsBeforeOpen + 1);
      expect(usage.listRequests().at(-1)?.forceRefresh).toBe(true);

      const popover = page.getByTestId("usage-composer-popover");
      await expect(popover.getByText("Fixture plan", { exact: true })).toBeVisible();
      await expect(popover.getByText("Test plan")).toBeVisible();
      await expect(popover.getByText("64%")).toBeVisible();
      await expect(pill).toContainText("64%");
    } finally {
      await session.cleanup();
    }
  });

  test("re-resolves the report ID after a turn and reuses the cached report", async ({ page }) => {
    test.setTimeout(180_000);
    const usage = await installUsageReportsFixture(page, {
      agentReportIds: ["fixture:fixture-account"],
      lists: [[agentEntry(42)]],
    });
    const session = await openMockAgent(page);
    try {
      const pill = page.getByTestId("usage-composer-pill");
      await expect(pill).toContainText("42%", { timeout: 30_000 });
      const requestsBeforeTurn = usage.agentRequests().length;
      const listRequestsBeforeTurn = usage.listRequests().length;

      await submitMessage(page, "emit 1 coalesced agent stream update for usage composer pill.");

      await usage.waitForAgentRequests(requestsBeforeTurn + 1);
      expect(usage.agentRequests().at(-1)).toEqual({
        agentId: session.agentId,
      });
      await expect(pill).toContainText("42%");
      expect(usage.listRequests()).toHaveLength(listRequestsBeforeTurn);
    } finally {
      await session.cleanup();
    }
  });

  test("is hidden when the agent has no usage report", async ({ page }) => {
    test.setTimeout(180_000);
    const usage = await installUsageReportsFixture(page, { agentReportIds: [null] });
    const session = await openMockAgent(page);
    try {
      await usage.waitForAgentRequests(1);
      await expect(page.getByTestId("usage-composer-pill")).toHaveCount(0);
      expect(usage.listRequests()).toHaveLength(0);
    } finally {
      await session.cleanup();
    }
  });

  test("is hidden on a host without usage sources", async ({ page }) => {
    test.setTimeout(180_000);
    const usage = await installUsageReportsFixture(page, {
      usageSources: false,
      agentReportIds: ["fixture:fixture-account"],
      lists: [[agentEntry(42)]],
    });
    const session = await openMockAgent(page);
    try {
      await expect(page.getByTestId("context-window-meter")).toBeVisible({ timeout: 30_000 });
      await expect(page.getByTestId("usage-composer-pill")).toHaveCount(0);
      expect(usage.agentRequests()).toHaveLength(0);
    } finally {
      await session.cleanup();
    }
  });
});

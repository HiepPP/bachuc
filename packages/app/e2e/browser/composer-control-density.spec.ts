import { test, expect } from "../support/fixtures";
import { openWorkspaceWithAgents } from "../support/helpers/archive-tab";
import {
  expectNoCollapsedComposerToolbarFrame,
  recordComposerToolbarFrames,
} from "../support/helpers/composer-control-density";
import { clickNewChat, gotoWorkspace } from "../support/helpers/launcher";
import { seedWorkspace, type SeededWorkspace } from "../support/helpers/seed-client";

const SETTLE_MS = 1_000;

async function seedSettledMockAgent(
  workspace: SeededWorkspace,
  title: string,
  thinkingOptionId = "low",
) {
  const agent = await workspace.client.createAgent({
    provider: "mock",
    model: "ten-second-stream",
    modeId: "load-test",
    thinkingOptionId,
    cwd: workspace.repoPath,
    workspaceId: workspace.workspaceId,
    title,
  });
  await workspace.client.waitForAgentUpsert(
    agent.id,
    (snapshot) => snapshot.status === "idle",
    30_000,
  );
  return { id: agent.id, title, cwd: workspace.repoPath, workspaceId: workspace.workspaceId };
}

function visibleAgentTab(page: import("@playwright/test").Page, agentId: string) {
  return page.getByTestId(`workspace-tab-agent_${agentId}`).filter({ visible: true }).first();
}

test.describe("Composer control density across tab switches", () => {
  test.describe.configure({ timeout: 180_000 });

  test("selected effort stays visible in narrow agent threads and after tab switches", async ({
    page,
  }) => {
    const workspace = await seedWorkspace({ repoPrefix: "composer-effort-threads-" });
    try {
      const first = await seedSettledMockAgent(workspace, "High effort chat", "high");
      const second = await seedSettledMockAgent(workspace, "Default effort chat");
      await openWorkspaceWithAgents(page, [first, second]);

      const thinking = page
        .getByTestId("agent-thinking-selector")
        .filter({ visible: true })
        .first();
      for (const width of [1280, 900]) {
        await page.setViewportSize({ width, height: 900 });
        await visibleAgentTab(page, first.id).click({ position: { x: 12, y: 13 } });
        await expect(thinking).toHaveText("High");
        await visibleAgentTab(page, second.id).click({ position: { x: 12, y: 13 } });
        await expect(thinking).toHaveText("Low");
      }

      await page.setViewportSize({ width: 760, height: 900 });
      const compactModel = page
        .getByTestId("combined-model-selector")
        .filter({ visible: true })
        .first();
      await visibleAgentTab(page, first.id).click({ position: { x: 12, y: 13 } });
      await expect(compactModel).toContainText("High");
      await visibleAgentTab(page, second.id).click({ position: { x: 12, y: 13 } });
      await expect(compactModel).toContainText("Low");
    } finally {
      await workspace.cleanup();
    }
  });

  test("switching between agent tabs never paints a collapsed composer toolbar", async ({
    page,
  }) => {
    const workspace = await seedWorkspace({ repoPrefix: "composer-density-agents-" });
    try {
      const first = await seedSettledMockAgent(workspace, "First chat");
      const second = await seedSettledMockAgent(workspace, "Second chat");
      await openWorkspaceWithAgents(page, [first, second]);

      await recordComposerToolbarFrames(page);
      await visibleAgentTab(page, first.id).click();
      await page.waitForTimeout(SETTLE_MS);
      await visibleAgentTab(page, second.id).click();
      await page.waitForTimeout(SETTLE_MS);

      await expectNoCollapsedComposerToolbarFrame(page);
    } finally {
      await workspace.cleanup();
    }
  });

  test("switching between draft tabs never paints a collapsed composer toolbar", async ({
    page,
  }) => {
    const workspace = await seedWorkspace({ repoPrefix: "composer-density-drafts-" });
    try {
      await page.addInitScript(() => {
        localStorage.setItem(
          "@paseo:create-agent-preferences",
          JSON.stringify({
            provider: "mock",
            providerPreferences: { mock: { mode: "load-test" } },
          }),
        );
      });
      await gotoWorkspace(page, workspace.workspaceId);
      await clickNewChat(page);
      await clickNewChat(page);

      const draftTabs = page
        .locator('[data-testid^="workspace-tab-draft"]')
        .filter({ visible: true });
      await expect(draftTabs).toHaveCount(2, { timeout: 30_000 });
      await expect(
        page.locator('[data-testid="mode-control"]').filter({ visible: true }).first(),
      ).toBeVisible({ timeout: 30_000 });

      await recordComposerToolbarFrames(page);
      await draftTabs.nth(0).click();
      await page.waitForTimeout(SETTLE_MS);
      await draftTabs.nth(1).click();
      await page.waitForTimeout(SETTLE_MS);

      await expectNoCollapsedComposerToolbarFrame(page);
    } finally {
      await workspace.cleanup();
    }
  });
});

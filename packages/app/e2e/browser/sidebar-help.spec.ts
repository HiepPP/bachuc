import { expect, test, type Page } from "../support/fixtures";
import { gotoAppShell, openSettings } from "../support/helpers/app";
import { openSettingsSection } from "../support/helpers/settings";

// The name and the version are separate cells of a key/value row, so they meet with no space
// between them in the row's text content. Fork builds show `hiep-X.Y.Z`, others `vX.Y.Z`.
const APP_VERSION = /^Bachuc\s*(?:v|hiep-)\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?(?:\+[0-9A-Za-z.-]+)?$/;

async function openHelpMenu(page: Page): Promise<void> {
  await page.getByTestId("sidebar-help").click();
  await expect(page.getByTestId("sidebar-help-menu")).toBeVisible();
}

async function expectDiagnosticReport(page: Page): Promise<void> {
  const sheet = page.getByTestId("app-diagnostic-sheet");
  await expect(sheet).toBeVisible();
  await expect(sheet.getByRole("button", { name: "Copy diagnostic" })).toBeEnabled();
  await expect(page.getByText(/App version:/).first()).toBeVisible();
}

async function closeSheet(page: Page, testID: string): Promise<void> {
  const sheet = page.getByTestId(testID);
  await sheet.getByLabel("Close").click();
  await expect(sheet).not.toBeVisible();
}

// The fork hides the sidebar Help button (`SHOW_FOOTER_EXTRAS` in left-sidebar.tsx), so the
// tests that open the help menu are marked fixme until the button shows again.
test.fixme("opens troubleshooting destinations", async ({ page }) => {
  await gotoAppShell(page);
  await expect(page.getByTestId("sidebar-help")).toBeVisible();

  await test.step("opens diagnostics and keyboard shortcuts", async () => {
    await openHelpMenu(page);
    await expect(page.getByText("Help", { exact: true })).toBeVisible();
    await expect(page.getByTestId("sidebar-help-version")).toHaveText(APP_VERSION);

    await page.getByTestId("sidebar-help-diagnostics").click();
    await expectDiagnosticReport(page);
    await closeSheet(page, "app-diagnostic-sheet");

    await openHelpMenu(page);
    await page.getByTestId("sidebar-help-shortcuts").click();
    await expect(page.getByTestId("keyboard-shortcuts-dialog")).toBeVisible();
    await closeSheet(page, "keyboard-shortcuts-dialog");
  });
});

test.fixme("searches keyboard shortcuts from the sidebar help menu", async ({ page }) => {
  await page.addInitScript(() => {
    Object.defineProperty(navigator, "platform", { get: () => "MacIntel" });
  });
  await gotoAppShell(page);
  await openHelpMenu(page);
  await page.getByTestId("sidebar-help-shortcuts").click();

  const dialog = page.getByTestId("keyboard-shortcuts-dialog");
  const search = page.getByPlaceholder("Search shortcuts");

  await search.fill("command+n");
  await expect(dialog.getByText("New workspace", { exact: true })).toBeVisible();

  await search.fill("interrupt");

  await expect(dialog.getByText("Interrupt agent", { exact: true })).toBeVisible();
  await expect(dialog.getByText("New workspace", { exact: true })).toHaveCount(0);

  await search.fill("no matching shortcut");
  await expect(dialog.getByText("No results found", { exact: true })).toBeVisible();

  await search.fill("");
  await expect(dialog.getByText("New workspace", { exact: true })).toBeVisible();
});

test("keeps diagnostics available from Settings after globalizing the sheet", async ({ page }) => {
  await gotoAppShell(page);
  await openSettings(page);
  await openSettingsSection(page, "diagnostics");

  await page.getByRole("button", { name: "Run", exact: true }).click();
  await expectDiagnosticReport(page);
});

test.describe("compact sidebar help", () => {
  test.use({ viewport: { width: 390, height: 844 }, isMobile: true, hasTouch: true });

  test.fixme("offers diagnostics without advertising disabled keyboard shortcuts", async ({
    page,
  }) => {
    await gotoAppShell(page);
    await page.getByRole("button", { name: "Open menu", exact: true }).click();

    await openHelpMenu(page);
    await expect(page.getByTestId("sidebar-help-shortcuts")).toHaveCount(0);
    await page.getByTestId("sidebar-help-diagnostics").click();
    await expectDiagnosticReport(page);
  });
});

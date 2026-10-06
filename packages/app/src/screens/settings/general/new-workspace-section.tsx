import { useCallback } from "react";
import { useTranslation } from "react-i18next";
import { SettingsCard, SettingsSection, SettingsSwitch } from "@/components/settings";
import { useAppSettings } from "@/hooks/use-settings";

export function NewWorkspaceSection() {
  const { t } = useTranslation();
  const { settings, updateSettings } = useAppSettings();
  const change = useCallback(
    (newWorkspaceSidePanel: boolean) => void updateSettings({ newWorkspaceSidePanel }),
    [updateSettings],
  );
  return (
    <SettingsSection title={t("settings.general.newWorkspace.title")}>
      <SettingsCard>
        <SettingsSwitch
          label={t("settings.general.newWorkspace.sidePanel.label")}
          hint={t("settings.general.newWorkspace.sidePanel.description")}
          value={settings.newWorkspaceSidePanel}
          onValueChange={change}
        />
      </SettingsCard>
    </SettingsSection>
  );
}

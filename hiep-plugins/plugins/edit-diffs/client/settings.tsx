import { useSettings, type PluginSurfaceProps } from "@getpaseo/plugin/client";
import {
  SettingsAction,
  SettingsCard,
  SettingsSection,
  SettingsSwitch,
} from "@getpaseo/plugin/client/ui";
import { Text } from "react-native";
import { displaySettings } from "../shared/settings";

export function DisplaySettingsScreen({ theme }: PluginSurfaceProps) {
  const settings = useSettings(displaySettings);
  if (settings.status === "loading" || settings.status === "error") {
    return (
      <Text style={{ color: theme.colors.foregroundMuted, fontSize: 13 }}>
        {settings.status === "loading"
          ? "Loading edit diff settings…"
          : `Edit diff settings are unavailable: ${settings.error}`}
      </Text>
    );
  }
  if (settings.status === "invalid") {
    return (
      <SettingsSection title="Edit diffs">
        <SettingsCard>
          <SettingsAction
            label="Stored settings are invalid"
            hint={settings.error}
            error={settings.saveError}
            actionLabel="Reset to defaults"
            disabled={settings.saving}
            onPress={() => void settings.reset()}
          />
        </SettingsCard>
      </SettingsSection>
    );
  }
  const { values, revision } = settings;
  return (
    <SettingsSection title="Edit diffs">
      <SettingsCard>
        <SettingsSwitch
          label="Auto-expand edit diffs"
          hint="Show each edit's diff open in the chat. When off, press the row to open it."
          error={settings.saveError}
          value={values.autoExpand}
          disabled={settings.saving}
          onValueChange={(autoExpand) => void settings.save({ ...values, autoExpand }, revision)}
        />
      </SettingsCard>
    </SettingsSection>
  );
}

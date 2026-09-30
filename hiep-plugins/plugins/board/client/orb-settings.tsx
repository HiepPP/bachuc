import { useSettings, type PluginSurfaceProps } from "@getpaseo/plugin/client";
import {
  SettingsAction,
  SettingsCard,
  SettingsSection,
  SettingsSelect,
  SettingsSwitch,
} from "@getpaseo/plugin/client/ui";
import { Text } from "react-native";
import { ORB_STATES, orbSettings, type OrbSettings } from "../shared/orb";

const OPACITY_OPTIONS = [100, 90, 80, 70, 60, 50, 40, 30, 20].map((value) => ({
  label: `${value}%`,
  value: String(value),
}));
const STATE_OPTIONS = ORB_STATES.map((value) => ({
  label: value[0].toUpperCase() + value.slice(1),
  value,
}));

export function OrbSettingsScreen({ theme }: PluginSurfaceProps) {
  const settings = useSettings(orbSettings);
  if (settings.status === "loading" || settings.status === "error") {
    return (
      <Text style={{ color: theme.colors.foregroundMuted, fontSize: 13 }}>
        {settings.status === "loading"
          ? "Loading thinking orb settings…"
          : `Thinking orb settings are unavailable: ${settings.error}`}
      </Text>
    );
  }
  if (settings.status === "invalid") {
    return (
      <SettingsSection title="Thinking orb">
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
  const update = (change: Partial<OrbSettings>) =>
    void settings.save({ ...values, ...change }, revision);
  return (
    <SettingsSection title="Thinking orb">
      <SettingsCard>
        <SettingsSwitch
          label="Show thinking orb"
          hint="Running Board cards on desktop and web. Mobile keeps the spinner."
          value={values.enabled}
          disabled={settings.saving}
          onValueChange={(enabled) => update({ enabled })}
        />
        <SettingsSelect
          label="Animation"
          hint="The avatar edge follows the orb outline."
          value={values.state}
          options={STATE_OPTIONS}
          disabled={settings.saving || !values.enabled}
          onValueChange={(state) => update({ state })}
        />
        <SettingsSelect
          label="Avatar opacity"
          hint="Lower values make the orb stand out."
          error={settings.saveError}
          value={String(values.avatarOpacity)}
          options={OPACITY_OPTIONS}
          disabled={settings.saving || !values.enabled}
          onValueChange={(value) => update({ avatarOpacity: Number(value) })}
        />
      </SettingsCard>
    </SettingsSection>
  );
}

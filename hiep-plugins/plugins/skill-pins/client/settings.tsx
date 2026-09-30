import { useSettings, type PluginSurfaceProps } from "@getpaseo/plugin/client";
import {
  SettingsAction,
  SettingsCard,
  SettingsSection,
  SettingsSwitch,
} from "@getpaseo/plugin/client/ui";
import { Text } from "react-native";
import { catalog } from "../shared/catalog";
import { pinSettings } from "../shared/settings";

export function SkillPinsSettingsScreen({ theme }: PluginSurfaceProps) {
  const settings = useSettings(pinSettings);
  if (settings.status === "loading" || settings.status === "error") {
    return (
      <Text style={{ color: theme.colors.foregroundMuted, fontSize: 13 }}>
        {settings.status === "loading"
          ? "Loading skill pins settings…"
          : `Skill pins settings are unavailable: ${settings.error}`}
      </Text>
    );
  }
  if (settings.status === "invalid") {
    return (
      <SettingsSection title="Skill pins">
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
    <SettingsSection title="Default pins for new agents">
      <SettingsCard>
        {catalog.map((skill) => (
          <SettingsSwitch
            key={skill.id}
            label={skill.label}
            hint={"warn" in skill ? `${skill.description}. ${skill.warn}.` : skill.description}
            error={settings.saveError}
            value={values.defaults.includes(skill.id)}
            disabled={settings.saving}
            onValueChange={(on) =>
              void settings.save(
                {
                  defaults: on
                    ? [...values.defaults, skill.id]
                    : values.defaults.filter((id) => id !== skill.id),
                },
                revision,
              )
            }
          />
        ))}
      </SettingsCard>
    </SettingsSection>
  );
}

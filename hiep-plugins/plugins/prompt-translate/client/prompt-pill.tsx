import type { PluginButtonContentProps, PluginButtonLabelProps } from "@getpaseo/plugin/client";
import { Text, View } from "react-native";
import { MenuRow } from "./menu-row";
import { modes } from "./modes";
import { usePromptPrefs } from "./prefs";

/** The pill names the Caveman mode of its own composer. */
export function PromptPillLabel(props: PluginButtonLabelProps) {
  const { prefs } = usePromptPrefs(props);
  const mode = modes.find((candidate) => candidate.value === prefs?.mode);
  return mode ? `Caveman: ${mode.label}` : "Caveman";
}

// Opened from the shared composer pill; the button context names the composer's agent.
export function CavemanModeMenu(props: PluginButtonContentProps) {
  const { theme, layout, close } = props;
  const { prefs, error, save } = usePromptPrefs(props);
  return (
    <View accessibilityRole="menu" style={{ paddingVertical: 4 }}>
      {error ? (
        <Text
          style={{
            paddingHorizontal: 13,
            paddingVertical: 4,
            color: theme.colors.statusDanger,
            fontSize: 12,
            lineHeight: 16,
          }}
        >
          {error}
        </Text>
      ) : null}
      {modes.map((mode) => (
        <MenuRow
          key={mode.value}
          theme={theme}
          compact={layout.compact}
          label={mode.label}
          selected={prefs?.mode === mode.value}
          disabled={!prefs}
          onPress={() => {
            save({ mode: mode.value });
            close();
          }}
        />
      ))}
    </View>
  );
}

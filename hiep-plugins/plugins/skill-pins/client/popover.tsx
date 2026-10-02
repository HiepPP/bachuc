import type { PluginButtonContentProps, PluginButtonLabelProps } from "@getpaseo/plugin/client";
import { Text, View } from "react-native";
import { catalog, pillLabel } from "../shared/catalog";
import { MenuRow } from "./menu-row";
import { useSkillPins } from "./pins";

/** The pill names the skills pinned on its own composer. */
export function SkillsPillLabel(props: PluginButtonLabelProps) {
  return pillLabel(useSkillPins(props).skills ?? []);
}

// Shown from the shared composer pill; the button context names the composer it belongs to.
export function SkillsMenu(props: PluginButtonContentProps) {
  const { theme, layout } = props;
  const { skills, error, save } = useSkillPins(props);
  const pinned = skills ?? [];
  const note = { paddingHorizontal: 13, fontSize: 12, lineHeight: 16 } as const;
  return (
    <View accessibilityRole="menu" style={{ paddingVertical: 4 }}>
      <Text
        style={{ ...note, paddingTop: 8, paddingBottom: 4, color: theme.colors.foregroundMuted }}
      >
        Pinned for this chat
      </Text>
      {error ? (
        <Text style={{ ...note, paddingBottom: 4, color: theme.colors.statusDanger }}>{error}</Text>
      ) : null}
      {catalog.map((skill) => {
        const selected = pinned.includes(skill.id);
        return (
          // Toggling keeps the menu open so several skills can be picked in a row.
          <MenuRow
            key={skill.id}
            theme={theme}
            compact={layout.compact}
            label={skill.label}
            description={skill.description}
            warn={"warn" in skill ? skill.warn : undefined}
            selected={selected}
            disabled={!skills}
            onPress={() =>
              save(selected ? pinned.filter((id) => id !== skill.id) : [...pinned, skill.id])
            }
          />
        );
      })}
      <View style={{ height: 1, marginVertical: 4, backgroundColor: theme.colors.border }} />
      <MenuRow
        theme={theme}
        compact={layout.compact}
        label="Bỏ chọn tất cả"
        muted
        disabled={pinned.length === 0}
        onPress={() => save([])}
      />
    </View>
  );
}

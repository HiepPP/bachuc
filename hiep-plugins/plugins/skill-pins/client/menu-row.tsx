import type { PluginTheme } from "@getpaseo/plugin";
import { Icon } from "@getpaseo/plugin/client/react-native";
import { Pressable, Text, View } from "react-native";

// React Native's types omit `hovered` and `focused`, which the web renderer supplies.
type RowState = { pressed: boolean; hovered?: boolean; focused?: boolean };

interface MenuRowProps {
  theme: PluginTheme;
  compact: boolean;
  label: string;
  description?: string;
  warn?: string;
  /** Omit on an action row; a boolean marks a toggle and announces its state. */
  selected?: boolean;
  muted?: boolean;
  disabled?: boolean;
  onPress(): void;
}

/**
 * Follows Paseo's own menu rows (docs/menus.md): 28pt for a pointer, 40pt for a thumb, a hover
 * chip inset 4pt from the surface, the label on the 13pt rail, and a trailing check with no fill.
 */
export function MenuRow(props: MenuRowProps) {
  const { theme, compact, label, description, warn, selected, muted, disabled, onPress } = props;
  return (
    <Pressable
      // The host menu's row marker: arrow keys and the pill's mouse wheel move focus across it.
      {...{ dataSet: { menuItem: "true", menuDisabled: disabled ? "true" : "false" } }}
      accessibilityRole="menuitem"
      accessibilityState={{ checked: selected, disabled }}
      disabled={disabled}
      onPress={onPress}
      style={({ pressed, hovered, focused }: RowState) => ({
        flexDirection: "row",
        alignItems: "center",
        gap: 8,
        minHeight: compact ? 40 : 28,
        marginHorizontal: 4,
        paddingHorizontal: 9,
        paddingVertical: 5,
        borderRadius: 6,
        opacity: disabled ? 0.5 : 1,
        backgroundColor:
          !disabled && (pressed || hovered || focused) ? theme.colors.surface2 : "transparent",
      })}
    >
      <View style={{ flex: 1, minWidth: 0 }}>
        <Text
          numberOfLines={1}
          style={{
            color: muted ? theme.colors.foregroundMuted : theme.colors.foreground,
            fontSize: 14,
            lineHeight: 18,
          }}
        >
          {label}
        </Text>
        {description ? (
          <Text
            numberOfLines={2}
            style={{
              marginTop: 2,
              color: theme.colors.foregroundMuted,
              fontSize: 12,
              lineHeight: 16,
            }}
          >
            {description}
          </Text>
        ) : null}
        {warn ? (
          <Text
            numberOfLines={2}
            style={{ color: theme.colors.statusWarning, fontSize: 12, lineHeight: 16 }}
          >
            {warn}
          </Text>
        ) : null}
      </View>
      {selected ? <Icon name="Check" size={16} color={theme.colors.foregroundMuted} /> : null}
    </Pressable>
  );
}

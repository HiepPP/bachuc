import { useSettings, type PluginWorkspaceHeaderSubtitleProps } from "@getpaseo/plugin/client";
import { Pressable, Text, View } from "react-native";
import { projectColors } from "../shared/project-colors";
import { chipColors, projectInitial } from "./project-header";
import { workspaceActions } from "./workspace";

/**
 * The workspace header's project name as a chip with the project mark and its Board color.
 * Pressing it opens the new workspace screen for the project.
 */
export function ProjectHeaderChip({
  theme,
  layout,
  projectId,
  projectDisplayName,
  projectRootPath,
}: PluginWorkspaceHeaderSubtitleProps) {
  const palette = useSettings(projectColors);
  const hue = palette.status === "ready" ? palette.values.hues[projectId] : undefined;
  const colors = chipColors(hue, theme.colors);
  // Matches the size of the project name it replaces.
  const fontSize = layout.compact ? 12 : 14;
  const mark = Math.round(fontSize * 1.05);
  const openNewWorkspace = projectRootPath
    ? () => workspaceActions.open?.({ cwd: projectRootPath, projectId, name: projectDisplayName })
    : undefined;
  return (
    <Pressable
      accessibilityRole="button"
      accessibilityLabel={`New workspace in ${projectDisplayName}`}
      disabled={!openNewWorkspace}
      onPress={openNewWorkspace}
      style={({ pressed }) => ({
        opacity: pressed ? 0.7 : 1,
        flexShrink: 1,
        minWidth: 0,
      })}
    >
      <View
        style={{
          flexDirection: "row",
          alignItems: "center",
          gap: 6,
          flexShrink: 1,
          minWidth: 0,
          paddingVertical: 1,
          paddingLeft: 4,
          paddingRight: 8,
          borderRadius: 6,
          borderWidth: 1,
          borderColor: colors.border,
          backgroundColor: colors.background,
        }}
      >
        <View
          accessibilityElementsHidden
          importantForAccessibility="no-hide-descendants"
          style={{
            width: mark,
            height: mark,
            borderRadius: mark * 0.35,
            backgroundColor: colors.markBackground,
            alignItems: "center",
            justifyContent: "center",
            flexShrink: 0,
          }}
        >
          <Text
            style={{
              color: colors.markText,
              fontSize: fontSize * 0.75,
              lineHeight: mark,
              fontWeight: "700",
            }}
          >
            {projectInitial(projectDisplayName)}
          </Text>
        </View>
        <Text
          numberOfLines={1}
          style={{
            color: theme.colors.foregroundMuted,
            fontSize,
            fontWeight: "700",
            flexShrink: 1,
            minWidth: 0,
          }}
        >
          {projectDisplayName}
        </Text>
      </View>
    </Pressable>
  );
}

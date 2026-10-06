import { useMemo } from "react";
import { View } from "react-native";
import { StyleSheet, withUnistyles } from "react-native-unistyles";
import type { HtmlFrameProps } from "@getpaseo/plugin/client/ui";
import { SandboxedHtmlView } from "@/file-pane/html-preview";
import { cssVariablesStyle, withPreviewCsp } from "@/file-pane/html-preview-csp";
import { htmlFrameCssVariables } from "./html-frame-theme";

const DEFAULT_HEIGHT = 360;

interface HtmlFrameViewProps extends HtmlFrameProps {
  // A plugin could pass this prop too; it gains nothing, because it already owns `html`.
  themeStyle?: string;
}

function HtmlFrameView({
  html,
  height = DEFAULT_HEIGHT,
  network = false,
  title = "HTML",
  testID,
  themeStyle,
}: HtmlFrameViewProps) {
  // Plugins are plain JS, so `html` can arrive undefined while their data loads.
  const source = typeof html === "string" ? html : "";
  const document = useMemo(
    () => withPreviewCsp(source, { network, themeStyle }),
    [network, source, themeStyle],
  );
  const frameStyle = useMemo(() => [styles.frame, { height }], [height]);
  return (
    <View style={frameStyle}>
      <SandboxedHtmlView document={document} title={title} testID={testID} nestedScroll />
    </View>
  );
}

/**
 * Plugin UI: a self-contained HTML document in the file preview's sandbox. The mapping returns
 * a string, so a re-render with the same theme rebuilds nothing; a theme change reloads the page.
 */
export const HtmlFrame = withUnistyles(HtmlFrameView, (theme) => ({
  themeStyle: cssVariablesStyle(htmlFrameCssVariables(theme)),
}));

const styles = StyleSheet.create((theme) => ({
  frame: {
    width: "100%",
    overflow: "hidden",
    borderRadius: theme.borderRadius.md,
    borderWidth: theme.borderWidth[1],
    borderColor: theme.colors.border,
  },
}));

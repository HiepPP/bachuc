import { useCallback, useMemo, useState } from "react";
import { View } from "react-native";
import { StyleSheet, withUnistyles } from "react-native-unistyles";
import type { HtmlFrameProps } from "@getpaseo/plugin/client/ui";
import { SandboxedHtmlView } from "@/file-pane/html-preview";
import { cssVariablesStyle, withPreviewCsp } from "@/file-pane/html-preview-csp";
import { INITIAL_AUTO_HEIGHT, nextAutoHeight, readReportedHeight } from "./html-frame-height";
import { htmlFrameCssVariables } from "./html-frame-theme";

const DEFAULT_HEIGHT = 360;

interface FrameBodyProps {
  document: string;
  title: string;
  testID?: string;
}

function FixedHeightFrame({
  document,
  title,
  testID,
  height,
}: FrameBodyProps & { height: number }) {
  const bodyStyle = useMemo(() => ({ height }), [height]);
  return (
    <View style={bodyStyle}>
      <SandboxedHtmlView document={document} title={title} testID={testID} nestedScroll />
    </View>
  );
}

// Keyed by document, so a new page starts again from the initial height.
function AutoHeightFrame({ document, title, testID }: FrameBodyProps) {
  const [size, setSize] = useState(INITIAL_AUTO_HEIGHT);
  const handleFrameMessage = useCallback((data: unknown) => {
    const reported = readReportedHeight(data);
    if (reported !== null) setSize((current) => nextAutoHeight(current, reported));
  }, []);
  const bodyStyle = useMemo(() => ({ height: size.height }), [size.height]);
  return (
    <View style={bodyStyle}>
      <SandboxedHtmlView
        document={document}
        title={title}
        testID={testID}
        nestedScroll
        onFrameMessage={handleFrameMessage}
      />
    </View>
  );
}

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
  const auto = height === "auto";
  const document = useMemo(
    () => withPreviewCsp(source, { network, themeStyle, reportHeight: auto }),
    [auto, network, source, themeStyle],
  );
  return (
    <View style={styles.frame}>
      {auto ? (
        <AutoHeightFrame key={document} document={document} title={title} testID={testID} />
      ) : (
        <FixedHeightFrame document={document} title={title} testID={testID} height={height} />
      )}
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

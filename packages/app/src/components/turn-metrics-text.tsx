import { Fragment, memo, type ReactElement } from "react";
import { Text, type StyleProp, type TextStyle } from "react-native";
import { StyleSheet } from "react-native-unistyles";
import {
  TURN_METRIC_SEPARATOR,
  type TurnMetricPart,
  type TurnMetricTone,
} from "@/timeline/turn-time";

/** TTFT and output rate, each value colored by how good it is; separators keep `style`. */
export const TurnMetricsText = memo(function TurnMetricsText({
  parts,
  style,
  testID,
}: {
  parts: TurnMetricPart[];
  style?: StyleProp<TextStyle>;
  testID?: string;
}): ReactElement | null {
  if (parts.length === 0) {
    return null;
  }
  return (
    <Text style={[style, styles.tabular]} testID={testID}>
      {parts.map((part, index) => (
        <Fragment key={part.text}>
          {index > 0 ? TURN_METRIC_SEPARATOR : null}
          <Text style={toneStyles[part.tone]}>{part.text}</Text>
        </Fragment>
      ))}
    </Text>
  );
});

const styles = StyleSheet.create((theme) => ({
  tabular: { fontVariant: ["tabular-nums"] },
  good: { color: theme.colors.statusSuccess },
  fair: { color: theme.colors.statusWarning },
  poor: { color: theme.colors.statusDanger },
}));

const toneStyles: Record<TurnMetricTone, StyleProp<TextStyle>> = {
  good: styles.good,
  fair: styles.fair,
  poor: styles.poor,
};

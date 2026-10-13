import { Text, View } from "react-native";
import { StyleSheet } from "react-native-unistyles";
import { useTranslation } from "react-i18next";

export interface FleetMapViewProps {
  serverId: string;
}

// Native fallback: the map renders only on web and Electron, in view.web.tsx.
export function FleetMapView(_props: FleetMapViewProps) {
  const { t } = useTranslation();
  return (
    <View style={styles.centerState}>
      <Text style={styles.message}>{t("panels.fleetMap.webglUnavailable")}</Text>
    </View>
  );
}

const styles = StyleSheet.create((theme) => ({
  centerState: {
    flex: 1,
    alignItems: "center",
    justifyContent: "center",
    padding: theme.spacing[4],
  },
  message: {
    color: theme.colors.foregroundMuted,
    textAlign: "center",
  },
}));

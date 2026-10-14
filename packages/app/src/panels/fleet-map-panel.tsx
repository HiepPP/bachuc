import { Network } from "lucide-react-native";
import { withUnistyles } from "react-native-unistyles";
import invariant from "tiny-invariant";
import { FleetMapView } from "@/fleet-map/view";
import { usePaneContext } from "@/panels/pane-context";
import { definePanel, type PanelPresentation } from "@/panels/panel-registry";

const ThemedNetwork = withUnistyles(Network);
const fleetMapPanelPresentation = {
  label: (t) => t("panels.fleetMap.label"),
  subtitle: (t) => t("panels.fleetMap.subtitle"),
  tooltip: (t) => t("panels.fleetMap.label"),
  icon: ThemedNetwork,
} satisfies PanelPresentation;

function FleetMapPanel() {
  const { serverId, target } = usePaneContext();
  invariant(target.kind === "fleet_map", "FleetMapPanel requires fleet_map target");
  return <FleetMapView serverId={serverId} />;
}

export const fleetMapPanelRegistration = definePanel("fleet_map", {
  component: FleetMapPanel,
  presentation: fleetMapPanelPresentation,
});

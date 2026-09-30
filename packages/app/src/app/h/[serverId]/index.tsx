import { Redirect } from "expo-router";
import { useHostRouteServerId } from "@/navigation/host-route-context";
import {
  resolveHostIndexRoute,
  resolveWorkspaceSelectionStatus,
} from "@/navigation/host-runtime-bootstrap";
import { StartupSplashScreen } from "@/screens/startup-splash-screen";
import { useHasHydratedWorkspaces, useWorkspaceExists } from "@/stores/session-store-hooks";
import {
  useIsLastWorkspaceSelectionHydrated,
  useLastWorkspaceSelection,
} from "@/stores/navigation-active-workspace-store";
import { useActiveHostStore } from "@/stores/active-host-store";
import { useActiveHostHydrated } from "@/hosts/use-visible-hosts";

export default function HostIndexRoute() {
  const serverId = useHostRouteServerId();
  const lastWorkspaceSelection = useLastWorkspaceSelection();
  const isActiveHostHydrated = useActiveHostHydrated();
  const isWorkspaceSelectionLoaded = useIsLastWorkspaceSelectionHydrated() && isActiveHostHydrated;
  // The global selection only remembers one host; switching hosts restores that host's own.
  const rememberedWorkspaceId = useActiveHostStore((state) =>
    serverId ? (state.lastWorkspaceIdByServerId[serverId] ?? null) : null,
  );
  const workspaceSelection =
    lastWorkspaceSelection?.serverId === serverId || !serverId || !rememberedWorkspaceId
      ? lastWorkspaceSelection
      : { serverId, workspaceId: rememberedWorkspaceId };
  const workspaceSelectionWorkspaceId =
    workspaceSelection?.serverId === serverId ? workspaceSelection.workspaceId : null;
  const hasHydratedWorkspaces = useHasHydratedWorkspaces(serverId);
  const workspaceSelectionExists = useWorkspaceExists(serverId, workspaceSelectionWorkspaceId);

  if (!serverId || !isWorkspaceSelectionLoaded) {
    return <StartupSplashScreen />;
  }

  return (
    <Redirect
      href={resolveHostIndexRoute({
        serverId,
        workspaceSelection,
        workspaceSelectionStatus: resolveWorkspaceSelectionStatus({
          hasHydratedWorkspaces,
          workspaceExists: workspaceSelectionExists,
        }),
      })}
    />
  );
}

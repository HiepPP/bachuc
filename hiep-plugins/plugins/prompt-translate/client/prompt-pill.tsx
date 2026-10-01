import { useRpc, type PluginButtonContentProps } from "@getpaseo/plugin/client";
import { SettingsCard, SettingsSelect, SettingsSwitch } from "@getpaseo/plugin/client/ui";
import { useEffect, useState } from "react";
import { Text, View } from "react-native";
import { modeReadRpc, modeWriteRpc } from "../shared/contracts";
import { cavemanModeSchema } from "../shared/settings";
import { modes, type Mode } from "./modes";

type Prefs = { mode: Mode; rewrite: boolean };

// Opened from the shared composer pill; the button context names the composer's agent.
export function PromptPrefsPopover(props: PluginButtonContentProps) {
  const agentId = props.context === "agent" ? props.agentId : undefined;
  const read = useRpc(modeReadRpc);
  const write = useRpc(modeWriteRpc);
  const [prefs, setPrefs] = useState<Prefs | null>(null);
  const [error, setError] = useState<string | null>(null);
  useEffect(() => {
    if (!agentId) return;
    let active = true;
    read({ agentId })
      .then((value) => active && setPrefs(value))
      .catch((failure: unknown) => active && setError(String(failure)));
    return () => {
      active = false;
    };
  }, [agentId, read]);
  const save = (patch: Partial<Prefs>) => {
    if (!agentId || !prefs) return;
    setPrefs({ ...prefs, ...patch });
    write({ agentId, ...patch })
      .then(setPrefs)
      .catch((failure: unknown) => setError(String(failure)));
  };
  return (
    <View style={{ padding: 8, minWidth: 280 }}>
      {error ? <Text style={{ color: props.theme.colors.foregroundMuted }}>{error}</Text> : null}
      <SettingsCard>
        <SettingsSwitch
          label="Rewrite Vietnamese to English"
          hint="Sends a Vietnamese draft as a clear English prompt."
          value={prefs?.rewrite ?? false}
          disabled={!prefs}
          onValueChange={(rewrite) => save({ rewrite })}
        />
        <SettingsSelect
          label="Caveman mode"
          value={prefs?.mode ?? "follow-agent"}
          options={modes.map(({ label, value }) => ({ label, value }))}
          disabled={!prefs}
          onValueChange={(mode) => save({ mode: cavemanModeSchema.parse(mode) })}
        />
      </SettingsCard>
    </View>
  );
}

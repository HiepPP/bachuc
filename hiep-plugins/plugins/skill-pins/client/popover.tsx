import { useRpc, type PluginButtonContentProps } from "@getpaseo/plugin/client";
import { SettingsCard, SettingsSwitch } from "@getpaseo/plugin/client/ui";
import { useCallback, useEffect, useState } from "react";
import { Text, View } from "react-native";
import { catalog, type SkillId } from "../shared/catalog";
import { skillsReadRpc, skillsWriteRpc } from "../shared/contracts";

// Shown from the shared composer pill; `agentId` is the composer the pill belongs to.
export function SkillsPopover(props: PluginButtonContentProps) {
  const { theme } = props;
  const id = props.context === "agent" ? props.agentId : undefined;
  const read = useRpc(skillsReadRpc);
  const write = useRpc(skillsWriteRpc);
  const [skills, setSkills] = useState<SkillId[] | null>(null);
  const [error, setError] = useState<string | null>(null);
  useEffect(() => {
    if (!id) return;
    let active = true;
    read({ agentId: id })
      .then((value) => active && setSkills(value.skills))
      .catch((failure: unknown) => active && setError(String(failure)));
    return () => {
      active = false;
    };
  }, [id, read]);
  const toggle = useCallback(
    (skill: SkillId, on: boolean) => {
      if (!id || !skills) return;
      const next = on ? [...skills, skill] : skills.filter((value) => value !== skill);
      setSkills(next);
      write({ agentId: id, skills: next })
        .then((saved) => setSkills(saved.skills))
        .catch((failure: unknown) => setError(String(failure)));
    },
    [id, skills, write],
  );
  return (
    <View style={{ padding: 8, minWidth: 260 }}>
      {error ? <Text style={{ color: theme.colors.foregroundMuted }}>{error}</Text> : null}
      <SettingsCard>
        {catalog.map((skill) => (
          <SettingsSwitch
            key={skill.id}
            label={skill.label}
            hint={"warn" in skill ? skill.warn : skill.description}
            value={skills?.includes(skill.id) ?? false}
            disabled={!skills}
            onValueChange={(on) => toggle(skill.id, on)}
          />
        ))}
      </SettingsCard>
    </View>
  );
}

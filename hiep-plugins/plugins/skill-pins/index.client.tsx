import type { PluginClientContext } from "@getpaseo/plugin/client";
import { SkillsMenu, SkillsPillLabel } from "./client/popover";
import { SkillPinsSettingsScreen } from "./client/settings";

export default function contribute(client: PluginClientContext) {
  const settings = client.addSettingsScreen({
    id: "skill-pins",
    title: "Skill pins",
    icon: "Pin",
    Component: SkillPinsSettingsScreen,
  });
  // One pill for every agent composer, on desktop and mobile.
  const pill = client.addComposerPill({
    id: "skills",
    showOnDraft: true,
    placement: "toolbar",
    button: {
      title: "Pinned skills",
      label: SkillsPillLabel,
      behavior: { kind: "popover", Content: SkillsMenu, flush: true },
    },
  });
  return () => {
    pill.remove();
    settings();
  };
}

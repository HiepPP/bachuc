import type { PluginClientContext } from "@getpaseo/plugin/client";
import { SkillsPopover } from "./client/popover";
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
    button: {
      title: "Pinned skills",
      icon: "Pin",
      label: "Skills",
      behavior: { kind: "popover", Content: SkillsPopover },
    },
  });
  return () => {
    pill.remove();
    settings();
  };
}

import type {
  PluginButton,
  PluginButtonBehavior,
  PluginButtonIcon,
  PluginComposerPillButton,
} from "@getpaseo/plugin/client";

export interface ButtonValidation {
  validateIconName(name: string): void;
}

export function requireButtonId(value: string): string {
  if (typeof value !== "string" || !/^[a-z][a-z0-9-]*$/.test(value))
    throw new Error(`Invalid plugin button id: ${value}`);
  return value;
}

function optionalBoolean(value: boolean | undefined, fallback: boolean): boolean {
  if (value === undefined) return fallback;
  if (typeof value !== "boolean")
    throw new Error("Plugin button visibility and disabled state must be booleans");
  return value;
}

function requireText(value: string, field: string): string {
  if (typeof value !== "string" || !value.trim()) throw new Error(`Plugin button needs ${field}`);
  return value.trim();
}

function isComponent(value: unknown): boolean {
  return (
    typeof value === "function" ||
    (typeof value === "object" && value !== null && "$$typeof" in value)
  );
}

function validateIcon(icon: PluginButtonIcon, validation: ButtonValidation): void {
  if (typeof icon === "string") validation.validateIconName(icon);
  else if (!isComponent(icon))
    throw new Error("Plugin button icon must be a Lucide name or component");
}

function validateBehavior(
  behavior: PluginButtonBehavior,
  validation: ButtonValidation,
): PluginButtonBehavior {
  switch (behavior.kind) {
    case "action":
      if (typeof behavior.onPress !== "function")
        throw new Error("Plugin button action needs onPress");
      if (
        behavior.newWorkspacePanel !== undefined &&
        typeof behavior.newWorkspacePanel !== "string"
      )
        throw new Error("Plugin button newWorkspacePanel must be a panel id");
      return { ...behavior };
    case "popover":
      if (!isComponent(behavior.Content)) throw new Error("Plugin button popover needs Content");
      return { ...behavior };
    case "menu": {
      const ids = new Set<string>();
      return {
        kind: "menu",
        items: behavior.items.map((item) => {
          requireButtonId(item.id);
          if (ids.has(item.id)) throw new Error(`Duplicate plugin button menu item: ${item.id}`);
          ids.add(item.id);
          if (item.kind === "separator") return { ...item };
          if (item.kind !== "item") throw new Error("Invalid plugin button menu entry");
          if (item.icon !== undefined) validateIcon(item.icon, validation);
          return {
            ...item,
            title: requireText(item.title, "menu item title"),
            visible: optionalBoolean(item.visible, true),
            disabled: optionalBoolean(item.disabled, false),
            behavior: validateBehavior(item.behavior, validation),
          };
        }),
      };
    }
    default:
      throw new Error("Invalid plugin button behavior");
  }
}

function validateLabel(label: PluginButton["label"]): PluginButton["label"] {
  if (label === undefined) return undefined;
  if (typeof label === "string") return requireText(label, "label");
  if (!isComponent(label)) throw new Error("Plugin button label must be text or a component");
  return label;
}

function validateColor(color: string | undefined): string | undefined {
  if (color === undefined) return undefined;
  if (typeof color !== "string" || !/^#(?:[0-9a-f]{3,4}|[0-9a-f]{6}|[0-9a-f]{8})$/i.test(color))
    throw new Error("Plugin button color must be a hex color");
  return color;
}

export type ResolvedPluginButton = PluginComposerPillButton & {
  visible: boolean;
  disabled: boolean;
};

/** Header buttons collapse to their icon, so they need one. Composer pills always show text. */
export function validateButton(
  button: PluginComposerPillButton,
  validation: ButtonValidation,
  requireIcon: boolean,
): ResolvedPluginButton {
  if (button.icon !== undefined) validateIcon(button.icon, validation);
  else if (requireIcon) throw new Error("Plugin button needs icon");
  return {
    ...button,
    title: requireText(button.title, "title"),
    label: validateLabel(button.label),
    color: validateColor(button.color),
    visible: optionalBoolean(button.visible, true),
    disabled: optionalBoolean(button.disabled, false),
    behavior: validateBehavior(button.behavior, validation),
  };
}

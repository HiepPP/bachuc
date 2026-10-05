import { createContext, useContext } from "react";
import { StyleSheet } from "react-native-unistyles";
import { SPACING } from "@/styles/theme";

/** Padding inside the composer's bordered surface. The docked track strip bleeds through it. */
export const COMPOSER_SURFACE_PADDING = {
  vertical: SPACING[2],
  horizontal: SPACING[3],
} as const;
export const COMPOSER_DOCKED_STRIP_HEIGHT = 28;
const COMPOSER_DOCKED_PILL_HEIGHT = 24;

export const COMPOSER_PILL_CLEARANCE = {
  compact: SPACING[2],
  wide: 10,
} as const;
export const COMPOSER_PILL_MIN_HEIGHT = 32;

export function resolveComposerPillClearance(isCompact: boolean): number {
  return isCompact ? COMPOSER_PILL_CLEARANCE.compact : COMPOSER_PILL_CLEARANCE.wide;
}

export function resolveComposerTrackTailClearance(isCompact: boolean): number {
  const composerClearance = resolveComposerPillClearance(isCompact);
  const transcriptClearance = isCompact ? SPACING[6] : 20;
  return transcriptClearance + COMPOSER_PILL_MIN_HEIGHT + composerClearance;
}

export function resolveComposerTrackControlClearance(isCompact: boolean): number {
  const clearance = resolveComposerPillClearance(isCompact);
  return clearance + COMPOSER_PILL_MIN_HEIGHT + clearance;
}

/** Shared visual contract for pills that float over the transcript, above the composer. */
export const composerPillStyles = StyleSheet.create((theme) => ({
  body: {
    flexDirection: "row",
    alignItems: "center",
    gap: theme.spacing[2],
    minHeight: COMPOSER_PILL_MIN_HEIGHT,
    paddingHorizontal: theme.spacing[3],
    paddingVertical: theme.spacing[1],
    // Match the composer's 16px tangent point. The 32px floor prevents short labels from
    // clamping the radius and pulling the point where the curve meets the straight edge inward.
    borderRadius: theme.borderRadius["2xl"],
    borderWidth: theme.borderWidth[1],
    borderColor: theme.colors.borderAccent,
    backgroundColor: theme.colors.surface1,
  },
  bodyActive: {
    backgroundColor: theme.colors.surface2,
  },
  label: {
    fontSize: theme.fontSize.sm,
    color: theme.colors.foregroundMuted,
  },
  labelActive: {
    color: theme.colors.foreground,
  },
}));

/**
 * Docked pills sit in the strip at the top of the composer surface. The strip already frames
 * them, so they drop the border and fill and show a fill only when hovered, pressed, or open.
 */
const dockedComposerPillStyles = StyleSheet.create((theme) => ({
  body: {
    flexDirection: "row",
    alignItems: "center",
    gap: theme.spacing[1.5],
    minHeight: COMPOSER_DOCKED_PILL_HEIGHT,
    paddingHorizontal: theme.spacing[1.5],
    borderRadius: theme.borderRadius.md,
  },
  bodyActive: {
    backgroundColor: theme.colors.surface2,
  },
  label: {
    fontSize: theme.fontSize.sm,
    color: theme.colors.foregroundMuted,
  },
  labelActive: {
    color: theme.colors.foreground,
  },
}));

export type ComposerPillPlacement = "floating" | "docked";

const ComposerPillPlacementContext = createContext<ComposerPillPlacement>("floating");

export const ComposerPillPlacementProvider = ComposerPillPlacementContext.Provider;

/** Pills float over the transcript unless a docked strip says otherwise. */
export function useComposerPillStyles() {
  return useContext(ComposerPillPlacementContext) === "docked"
    ? dockedComposerPillStyles
    : composerPillStyles;
}

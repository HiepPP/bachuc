import type { OrbSettings } from "../shared/orb";

// Opening a thread unmounts the Board, and the host drops unobserved settings after five minutes.
// These last-known values let a remounted Board paint as before instead of with defaults.
export const lastSettings: {
  orb?: OrbSettings;
  hues?: Record<string, number>;
  size?: number;
} = {};

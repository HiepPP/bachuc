import { readFileSync } from "node:fs";
import path from "node:path";

// scripts/paseo-switch.sh writes this file into fork data only. Data without it belongs to the
// release app.
const SWITCH_SIDE_LABEL = ".paseo-switch-side";

export function isForkDataLive(dataDir: string): boolean {
  try {
    return readFileSync(path.join(dataDir, SWITCH_SIDE_LABEL), "utf-8").trim() === "fork";
  } catch {
    return false;
  }
}

// The switch moves the home and the userData folder one after the other, so an interrupted switch
// can leave only one of them holding fork data. Returns the folders that do not.
export function findNonForkData(dataDirs: string[]): string[] {
  return dataDirs.filter((dataDir) => !isForkDataLive(dataDir));
}

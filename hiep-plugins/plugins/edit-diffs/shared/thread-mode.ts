/** Agent label that turns edit diffs off for one thread. Board sets it too, so keep it stable. */
export const EDIT_DIFFS_MODE_LABEL = "edit-diffs.mode";

export function editDiffsOff(labels: Readonly<Record<string, string>> | undefined): boolean {
  return labels?.[EDIT_DIFFS_MODE_LABEL] === "off";
}

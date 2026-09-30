# TASK-008 Outcome

## Outcome

Status: DONE

Changed:
- Restricted the Watchtower panel to the Explorer location.
- The open command now explicitly targets Explorer beside Files and Changes.
- The board header now shows the project name instead of the conversation title.
- Updated the plugin README.

Contract:
- The query key includes the host and exact workspace ID.
- The backend reads only the requested workspace directory.
- Task selection remains scoped to its workspace. Attachments and read-only behavior are unchanged.

Verified:
- The installed Paseo 0.8 SDK supports Explorer locations and explicit open targets.
- Formatter, typecheck, lint, and all 10 tests passed.
- Plugin reload reported running. No commit or push was made.
- Source inspection confirmed the existing workspace data boundary.
- The user manually confirmed Explorer placement and correct data when switching projects.

- On 2026-09-20, native UI inspection found no obsolete Watchtower center tab in either project workspace: `wks_43b925b9485acc55` and `wks_6f12ecc578f25a6d`. No close action was needed.
- Final native UI inspection through `cua.getApp("sh.paseo.desktop")` confirmed the original `Determine next steps` workspace (`wks_6f12ecc578f25a6d`) restored, with only `what next` in the center and Watchtower beside Files and Changes in Explorer. The board header showed `hiep-paseo-plugin`.
- This follow-up changed only plan records; earlier code, test, reload, and user-confirmed project-switching evidence above was retained.

Resolved blocker:
- Earlier native UI startup and popover-only observations prevented final cleanup verification. The final app binding returned the full window and confirmed the required state.

Handoff:
- None.

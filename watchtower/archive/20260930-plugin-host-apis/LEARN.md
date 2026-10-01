# Learn 20260930-plugin-host-apis

## Summary

Discrepancy: 9 found. 16 of 17 TASKs shipped in commit `26a8b71db`; TASK-015 is BLOCKED and archived unresolved. Seven TASKs shipped with a deviation from their spec, and one Plan Verify line does not pass.

## Per TASK

- TASK-001: plan said the hook covers turns and steers -> shipped for both, but only the turn path has a test. Mistake: the spec asked for a steer result the fake provider cannot show. Fix: add steer support to the fake provider, or mark the steer path as unverified in the spec. Two bugs came from the host check: plugin text showed in the user bubble for prompts with no client message ID, and again after a daemon restart.
- TASK-002: plan said the anti-goal test shows the permission as pending -> shipped a test where the normal event fires and the plugin answers it. Mistake: the Verify line named one end state, the test proves another. Fix: write the Verify line as "the permission reaches the normal flow".
- TASK-003: plan said plugins send a `tools.changed` message -> shipped tools at setup time only. Native tool catalogs for OpenCode and OMP are built before plugins load, so they do not list plugin tools. Mistake: the spec did not check when each catalog is built. Fix: read the catalog build order before planning a tool API.
- TASK-004: match.
- TASK-005: plan said a `composerTarget` prop from 4 callers -> shipped without the prop; the target comes from the session store. Browser tests were not run, because the Playwright Chromium binary is missing. Mistake: the spec planned a prop the code did not need. Fix: none needed for the prop; install Chromium before the next composer TASK.
- TASK-006: match.
- TASK-007: plan said menu items are `PluginButtonMenuEntry[]` with submenus -> shipped flat `{ id, title, disabled?, onSelect }` items. Mistake: the project menus use two different menu primitives, which the spec did not check. Fix: read the target menu component before naming a menu type. React Compiler dropped a memo input read only through `void`; found in the host check.
- TASK-008: plan said an unknown plugin or surface throws a clear error -> shipped no check for another plugin's surface; the route shows "This plugin surface is unavailable.". Mistake: spec gap, the calling plugin cannot see another host's surfaces. Fix: state the missing-surface behavior in the spec.
- TASK-009: plan said version `0.10.2-hiep.0` first -> shipped `0.10.2-beta.900` after the desktop build failed. Mistake: the version format rule in `packages/app/native-release-version.js` was not read before choosing. Fix: now a Planning Rule in `watchtower/MEMORY.md`.
- TASK-010: match. `npm run lint` fails in every plugin under the repo root config; plugins now pass `--disable-nested-config`.
- TASK-011: plan said keep the native hook and installer -> shipped with both deleted. Mistake: the spec assumed the hook served sessions outside Paseo; it returned nothing without `PASEO_AGENT_ID`. Fix: check what a file does before writing "keep" in Boundaries.
- TASK-012: plan said a rewrite toggle pill plus a separate mode pill, and keep `install-hooks.mjs` -> shipped one `Prompt` pill for both. A Caveman timeout showed only after a daemon restart inside the desktop app. Mistake: the host check ran before a restart. Fix: restart the daemon in every host check.
- TASK-013: plan said the panel holds the Recap and What Next, and keep `client/recap.ts` -> shipped the panel for What Next only; the Recap keeps native rendering and `client/recap.ts` is deleted. Mistake: the spec carried the old DOM layout into the new API. Fix: decide what stays native before writing the transformer TASK.
- TASK-014: plan said remove horizontal trackpad switching and `client/gesture.ts` -> shipped with both kept, and a new host `onSwipe` on `addSidebarProjectFilter`, after user feedback. Mistake: the plan removed a feature the user still wanted. Fix: confirm every "removed" line in Boundaries with the user before implement.
- TASK-015: plan said move all 5 tools to `server.registerTool` and remove the bridge -> shipped nothing. Blocker: the native Codex and Claude hooks call the bridge over HTTP, and the global hook matcher names `mcp__jev_orchestrator__prepare_native_delegate`. Mistake: the plan did not read `server/native-hook-command.ts` and `server/native-install.ts`. Fix: pick one of the three options in `tasks/TASK-015-outcome.md` in a new plan.
- TASK-016: plan said remove Board's DOM code -> shipped with DOM kept in `client/project-header.ts` and `client/latest-prompt.ts`; the TASK Verify line was corrected. The send-failure toast is gone. Mistake: no plugin API covers header colors or the latest-prompt reveal. Fix: add those APIs, or accept the DOM code in the plan.
- TASK-017: plan said check 7 plugins -> shipped checks for 6; jev-orchestrator was not changed. The plan said `cua_repl` for UI checks; the TASK-014 outcome records a check over CDP. iOS and Android were not run.

## Plan-Level

- Plan Verify fails as written: the DOM grep covers `board/client`, which still matches `querySelector` and `MutationObserver` in two files. The TASK-016 Verify line was corrected, the plan line was not. The TASK-017 outcome reports the grep over four folders.
- Scope creep: three host changes shipped without a TASK row. They are the `onSwipe` sidebar gesture (in the TASK-014 outcome), the active-host default for `openSurface` (TASK-016), and loading plugin client code only for the active host in `packages/app/src/plugins/catalog-sync.tsx` (TASK-017).
- Dep gap: TASK-008 changed during TASK-016, because two Board installations needed one target host.
- Plan misdescribes reality: `CONTEXT.md` says the stable daemon loads plugins from `~/Projects/hiep-paseo-plugin`. That repo is frozen now, and `hiep-plugins` is the only plugin source.
- No ADR recorded. The user declined the three candidates at archive: daemon-served plugin tools, active-host plugin client loading, and the prompt hook that hides plugin context.

## Lessons

- Run the real host check after a daemon restart, and with a prompt sent by `paseo run`.
- When a TASK Verify line changes, update the matching Plan Verify line in the same edit.
- Before a TASK removes a bridge or a process, list every caller of it, including global provider hook configs.
- Add a TASK row when a host change appears during a plugin TASK, so it gets its own Verify.
- Confirm each removed user feature with the user before implement.

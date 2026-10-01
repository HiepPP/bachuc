# TASK-014 P5 workspace-spaces uses the sidebar API

Group: H (hiep-plugins/plugins/workspace-spaces)
Class: risky

## Brief

Goal: Keep numbered Spaces in the sidebar on every client, with no DOM code.

Change: DOM sidebar injection and a private localStorage read -> `addSidebarProjectFilter`, `addSidebarProjectMenuItems`, and `addSidebarSection`.

Boundaries:

- Build on the uncommitted per-host Spaces work already in the checkout. Do not revert it.
- The section renders the numbered strip with create, rename, and remove. It gets `activeServerId` from props, not localStorage.
- "Move to workspace" is a project menu submenu.
- Horizontal trackpad switching and the Space name in the Workspaces heading are removed.
- Keep the standalone Spaces page.
- Raise `requirements.paseo` to `>=0.10.2-beta.900`.

How:

- Remove [hiep-plugins/plugins/workspace-spaces/client/web.ts](hiep-plugins/plugins/workspace-spaces/client/web.ts) and [hiep-plugins/plugins/workspace-spaces/client/gesture.ts](hiep-plugins/plugins/workspace-spaces/client/gesture.ts).
- Adapt [hiep-plugins/plugins/workspace-spaces/client/sidebar-state.ts](hiep-plugins/plugins/workspace-spaces/client/sidebar-state.ts) to feed the filter and menu.
- Add the strip section component.
- Update tests and the README.

Files:

- [hiep-plugins/plugins/workspace-spaces/index.client.tsx](hiep-plugins/plugins/workspace-spaces/index.client.tsx) (contributions)
- Files under [hiep-plugins/plugins/workspace-spaces/client](hiep-plugins/plugins/workspace-spaces/client), [hiep-plugins/plugins/workspace-spaces/shared](hiep-plugins/plugins/workspace-spaces/shared), and [hiep-plugins/plugins/workspace-spaces/tests](hiep-plugins/plugins/workspace-spaces/tests)
- [hiep-plugins/plugins/workspace-spaces/paseo-plugin.json](hiep-plugins/plugins/workspace-spaces/paseo-plugin.json), [hiep-plugins/plugins/workspace-spaces/package.json](hiep-plugins/plugins/workspace-spaces/package.json), [hiep-plugins/plugins/workspace-spaces/README.md](hiep-plugins/plugins/workspace-spaces/README.md)

Expected result:

- The strip shows above the footer. Picking a Space filters projects.
- Move to workspace moves a project and the list updates.

Anti-goal: Saved Spaces survive the change; tripwire; limit settings schema version in [hiep-plugins/plugins/workspace-spaces/shared/spaces.ts](hiep-plugins/plugins/workspace-spaces/shared/spaces.ts) stays readable by the new code, checked by a test that loads the current saved shape; read before changes and at completion.

## Verify

- `rg -n "querySelector|MutationObserver|localStorage|document\." hiep-plugins/plugins/workspace-spaces/client hiep-plugins/plugins/workspace-spaces/index.client.tsx` -> no match.
- `cd hiep-plugins/plugins/workspace-spaces && npm run typecheck && npm run lint && npm test` -> exit 0.
- Real host check in Paseo Dev (TASK-017): the strip filters projects and Move to workspace works.
- Anti-goal: the saved-shape test passes.

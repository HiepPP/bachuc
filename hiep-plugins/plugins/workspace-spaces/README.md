# Workspace Spaces

Numbered Spaces in Paseo's left sidebar, on desktop and mobile. Requires the Paseo fork
`>=0.10.2-beta.900`; the stock 0.10.1 host refuses the plugin.

- A strip of numbered tabs sits above the sidebar footer. The Workspaces heading shows the selected
  Space's name. Tap a tab to show only that Space's projects. Use **+** to create a Space and switch to it. An
  empty Space keeps the Workspaces heading. The sidebar refreshes Spaces every 15 seconds.
- Swipe horizontally with two fingers on the trackpad over the project list to move to the
  adjacent Space, without wrapping (desktop). Changing Space slides the project list in the
  direction of travel; reduced motion skips the slide.
- Right-click a tab, or long-press it, to open its menu with **Rename…**, **Move projects…**, and
  **Remove…**. A press outside the menu or Escape closes it. **Move projects…** lists every project
  with its Space; pick one, then pick where it goes. **Remove…** asks for confirmation first.
  Enter or **Save** saves a name; **Cancel** or Escape discards it. Blank names are rejected.
  Removing a Space moves its projects to the preceding Space, or the next one when removing the
  first; **Remove → <Space>** names that Space, and the sidebar switches to it. The last Space
  cannot be removed.
- A project's context menu and three-dot menu have a **Move to** submenu listing every Space, with
  the current one checked. The whole project group, including its coding sessions, follows.
- Each host picked in the sidebar's active host switch has its own Spaces, names, and project
  memberships. **All hosts** keeps a separate set, which holds Spaces saved before per-host support.
- The standalone Spaces page stays available.

## Implementation and limits

The plugin uses the sidebar API: `addSidebarProjectFilter`, `addSidebarProjectMenuItems`, and
`addSidebarSection`. The active host comes from the sidebar context, not from app storage. No DOM
code. The host sends trackpad swipes to the filter's `onSwipe`, and `getTitle` names the Workspaces
heading.

Settings belong to the daemon where this plugin is installed, with revision-checked atomic saves.
The sidebar uses one installation per plugin: the active host's, else the first. Add Project
saves new projects in the selected Space before opening the workspace form. Reopening an assigned
project preserves its Space. Projects added outside the app default to the first Space.
Settings or catalog errors show every project instead of hiding any.
Space selection resets to the first Space on reload. Removing the registration deletes its settings.

## Checks and installation

```sh
npm ci
npm run format
npm run typecheck
npm run lint
npm test
PASEO_HOME=~/.paseo paseo plugin install "$PWD" --id workspace-spaces
PASEO_HOME=~/.paseo paseo plugin reload workspace-spaces
PASEO_HOME=~/.paseo paseo plugin ls workspace-spaces --json
PASEO_HOME=~/.paseo paseo plugin logs workspace-spaces
```

Tests cover the sidebar filter per host and Space, the Move to menu, errors that show every
project, revision conflicts, stable identities, and project reassignment when removing a Space.

Plugins are trusted code running in the daemon and app. This plugin reads the project/workspace
catalog, writes its own settings, and contributes sidebar UI through the plugin API.

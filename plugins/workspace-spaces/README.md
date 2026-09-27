# Workspace Spaces

Arc-style numbered Spaces inside Paseo's existing desktop left sidebar.
The sidebar Workspaces heading shows the selected Space name.
The compact bottom strip always uses numbered tabs; hover a tab to see its name. Use **+** to create another Space.
Open a project's existing context menu or three-dot menu and choose a destination under
**Move to workspace**. The whole project group, including its coding sessions, follows.
The menu closes after a successful move and stays open if saving fails.
Horizontal trackpad gestures across the project list switch adjacent Spaces without wrapping.
Right-click a tab (or focus it and press the context-menu key) and choose **Rename workspace**.
Enter a name, then press Enter or **Save**. **Cancel** or Escape discards the edit.
Names are trimmed; blank names are rejected. Renaming preserves projects and the selected Space.
The standalone Spaces page also offers **Rename workspace**.
The same tab menu (also opened with Delete) lets you remove that Space.
Its projects move atomically to the preceding Space, or the next one when removing the first.
For example, removing Space 2 moves its projects to Space 1.
The last Space cannot be removed. A failed save preserves both the Space and its memberships.
Clicking tabs also works. The main chat stays open and running agents are untouched.

## Implementation and limits

Supports Paseo daemon and app 0.8.x and 0.9.0-beta.2. No keys or configuration required.
This desktop-only adapter uses private DOM identifiers, not a supported sidebar extension API.
It adds its own controls and CSS attributes; it does not edit Paseo source, binaries, project paths,
Git worktrees, workspace identities, or agents. App updates may require selector changes.
Browser and native mobile clients keep the standalone Spaces page as a fallback.

Project view IDs group rows across connected hosts, including equivalent projects.
Settings belong to the daemon where this plugin is installed, with revision-checked atomic saves.
Install on one host per desktop: the first active adapter owns the shared sidebar and others do not
compete. Space selection resets to the first remaining Space on reload. Project membership survives reload,
disable, and updates. Removing the registration deletes its settings.
New project groups default to the first remaining Space. Project rows with unrecognized DOM structure are left
visible. Settings/catalog errors reveal hidden groups instead of leaving projects inaccessible.
Pinned workspaces belonging to catalogued projects follow membership; pinned workspaces from
other hosts remain visible when their project identity cannot be resolved.
Existing native sidebar filters still apply; Spaces do not reveal rows the app already filtered.
Native keyboard shortcuts, search, and history retain their normal global behavior.

A MutationObserver restores controls when the sidebar rerenders. Disabling removes the observer,
timer, wheel listener, injected menus, styles, visibility attributes, and footer controls.
Switching Spaces slides the project list out and the destination list in over 170 ms, following the direction
of travel. The numbered controls stay fixed. Reduced-motion preferences skip the animation.
Disabling cancels any active animation. Real trackpad hardware remains a manual acceptance check.

## Checks and installation

```sh
npm ci
npm run format
npm run typecheck
npm run lint
npm test
paseo plugin install "$PWD" --id workspace-spaces
paseo plugin reload workspace-spaces
paseo plugin ls workspace-spaces --json
paseo plugin logs workspace-spaces
```

DOM tests cover scoped wheel events, project filtering, rerenders, errors, menu extension,
success-only menu dismissal, Space removal, last-Space protection, and cleanup.
State tests cover stable identities, invalid settings, pagination, revision conflicts, gesture boundaries,
and project reassignment when removing a Space.
Live desktop checks cover project moves, filtered tabs, and restoring the original sidebar on disable.

Plugins are trusted code running in the daemon and app. This plugin reads the project/workspace
catalog, writes its own settings, and changes only the desktop UI at runtime.

# Plan Context

## Shared Context

- The user approved desktop-only injection into the existing left sidebar. Keep all Paseo application source unchanged.
- Numbered tabs sit below the native project list. Existing project menus gain Space destinations.
- Spaces group existing projects; they do not create Git worktrees or coding workspaces.
- Use host-scoped settings for sidebar view-key membership and client-local selection.
- Projects without explicit membership appear in the first remaining Space. Moves include all coding workspaces of that project.
- Opening a coding workspace uses supported navigation and preserves its agents.

## Decisions

- Create numbered Spaces without a fixed three-Space limit.
- Support project menu moves and click/trackpad navigation inside the desktop sidebar.
- Preserve native row contents and actions. Filter group visibility using owned attributes; restore on disable.
- Never store credentials or real prompts. Preserve existing plugin and Paseo checkout changes.

## Board

- Add a separate plugin named Board, with the menu immediately below Schedules.
- Each Board item represents one Paseo agent conversation, per the latest user correction. New turns update the same item.
- The user approved the generated UI on 2026-09-20. The design approval gate is satisfied.
- Verify actual lifecycle states and sidebar placement support against the installed host before implementation.
- Existing Spaces sidebar approval applies to Spaces only. Board must not assume private sidebar integration is approved.
- Board uses standard sidebar registration; the installed desktop places it directly below Schedules without injection.
- Board uses lifecycle outcomes and active snapshots. Recent history resets when the plugin stops or reloads.
- TASK-014 groups unstarred cards by project within each existing status column.
- Starred cards appear above all project groups in their column. Unstarring returns cards to their project.
- The user requested TASK-014 implementation on 2026-09-22 using the [new concept](../../designs/TASK-014-board-project-groups.png).
- Keep the current UI size control. Mockup chevrons do not require collapsible groups.

## References

- TASK-013 adds inline Send for suggested prompts and optional Jev auto-run. The desktop plugin is implemented and installed.
- [Prompt button concept](../../designs/next-step-prompts.png) uses synthetic text; the final UI places Send at the bottom right.
- TASK-013 must verify its own integration support. Earlier sidebar approval does not authorize a private timeline adapter.

- [Arc Spaces](https://resources.arc.net/hc/en-us/articles/19228064149143-Spaces-Distinct-Browsing-Areas): bottom controls and horizontal swipes.
- [Plugin reference](https://paseo.sh/docs/plugins/reference): surfaces, navigation, host settings, and runtime boundaries.
- [Plugin](../../../plugins/workspace-spaces/README.md): implementation and supported behavior.

## Desktop integration

- The adapter uses private DOM markers and a MutationObserver, gated to Electron.
- The first active installation owns the shared sidebar. Unknown DOM shapes remain visible.
- Browser and native mobile retain the standalone page fallback.
- Pinned rows from uncatalogued hosts remain visible when project identity cannot be resolved.
- Fifteen tests and live sidebar move/filter/disable/enable checks pass. Physical trackpad input remains unverified.

## Jev autonomous delegation

- TASK-007–012 are the requested implementation scope. TASK-004 remains a separate physical-input check.
- Implement sequentially in this session. Use one standalone plugin, with no Paseo source edits or worktrees.
- User authorized installation, real Jev evaluations, and bounded real child runs to measure effectiveness.
- Use saved profiles and exact provider/model/mode/effort/features. Do not invent model tiers or alter profiles.
- Use named tasks, explicit file ownership, dependency IDs, resource locks, and bounded attempts.
- Check results are measured by the plugin. Child claims and Jev judgments never count as verified success.
- Local job state and measurements must remain outside Git. Do not store credentials or real prompts in Git.
- Record exact evidence and limitations; a small synthetic benchmark is not production calibration.

- TASK-007–012 completed on 2026-09-21. See the orchestrator verification report for real runtime evidence and retained failures.
- Current routing is functional; two small synthetic cases were slower than direct baseline. Cost savings and production calibration remain unproven.

- Whole-workflow token follow-up: native session accounting and Jev ledger implemented; two fresh cases used 5.06x and 11.16x baseline tokens. No token savings demonstrated. Final counts reconciled after actor closure.

## Next prompt actions

- The user authorized TASK-013 implementation and the private desktop adapter.
- Place Send inside suggested prompt blocks under What Next or Next Steps.
- Optional Jev auto-send starts disabled. Its per-conversation control belongs in Command Center, outside prompt blocks.
- Jev evaluates relevance; it does not grant permissions or execute coding work itself.
- Verify installed timeline extension support before choosing integration. Keep Paseo source unchanged.
- Existing private sidebar approval does not approve private timeline injection.
- Use synthetic text in the generated design. Keep real conversation content outside Git.
- On 2026-09-21, the user approved the Send concept and requested implementation of TASK-013.
- TASK-013 public API preflight found no inline Markdown action slot. The user approved a desktop DOM adapter.

# Plan Context

## Shared Context

- All code lives in the `watchtower-board` plugin: [hiep-plugins/plugins/watchtower-board](hiep-plugins/plugins/watchtower-board). Read [hiep-plugins/AGENTS.md](hiep-plugins/AGENTS.md) before the first edit.
- The design is option 3f, "Process overview", of round 3. It is a published HTML mock outside the repo: `/Users/hiep/.paseo/plugin-data/html-reply/74a6aa0e-4c34-4281-8649-d11f81d34b08.html`. Open it in a browser and look at the section with id `3f`. Option 3a in the same file shows the pill states.
- The overview has six parts: a header, a lifecycle stepper (Plan, Implement, Verify, Review, Archive), a branch timeline with a stats row, Needs you, the run log, manual checks, and decisions and history.
- Plugin checks run in the plugin folder: `npm test` (runs `tsx --test tests/*.test.ts`), `npm run typecheck`, and `npm run lint`. Tests import `.ts` files only. A test cannot import a `.tsx` file that imports `react-native`, so put logic that needs a test in a `.ts` file.
- The server reads plan files only through `readFile` in [hiep-plugins/plugins/watchtower-board/server/board.ts](hiep-plugins/plugins/watchtower-board/server/board.ts). It checks the real path stays inside `watchtower/`, opens with `O_NOFOLLOW`, and caps a file at 128 KiB. New readers reuse it and keep the 1 MiB total cap.
- The client has no SVG. The plugin SDK exports `Icon`, `ScrollView`, `FlatList`, `TextInput`, `Modal`, and `copyText` from `@getpaseo/plugin/client/react-native`, plus plain `react-native`. Draw the timeline with Views.
- Style with `theme.colors` only: `surface0`, `surface1`, `surface2`, `foreground`, `foregroundMuted`, `border`, `accent`, `accentForeground`, `statusSuccess`, `statusWarning`, and `statusDanger`. This keeps light and dark themes correct.
- The app runs React Compiler. A `useMemo` input that the body reads only through `void` drops out of the cache keys. See [watchtower/MEMORY.md](watchtower/MEMORY.md).
- `layout.compact` in plugin surface props is true below the host `md` breakpoint (720 px). See [hiep-plugins/plugins/watchtower-board/client/dashboard.ts](hiep-plugins/plugins/watchtower-board/client/dashboard.ts).
- Composer pills and surfaces are documented in [public-docs/plugins/reference.md](public-docs/plugins/reference.md) under Composer pills, Button descriptor, and Surfaces and sidebar items.

## Decisions

- Add a new RPC, `watchtower.overview.read`, for the overview. Keep the `watchtower.read` output unchanged, because the Explorer panel and attachment search use it.
- Lifecycle phase rules: Plan is done when the Tracker has a task. Implement is current while any task is not `DONE`. Verify is current when every task is `DONE` and `RUN.md` has no `Finished:` value. Review is current when `RUN.md` has a `Finished:` value. Archive is current only when the plan `Status:` is `ARCHIVED`.
- Branch timeline rules: done bars come from the `RUN.md` log. A running task starts at the end of the latest log row, or at the run start when the log is empty, and ends now. Planned tasks run one at a time in Tracker order, never before their deps end, at the average done duration. The default duration is 20 minutes when nothing is done yet.
- Draw timeline connectors as orthogonal lines made of Views. Do not add a host SVG API in this plan, because the fork extends through plugins first.
- Wide layouts show the overview on the full-screen surface. Compact layouts keep the existing board.
- The pill goes in the composer track of every agent composer on the host, beside Tasks and Subagents. It also shows on the New workspace composer (Q-002).
- The overview is read-only. Its actions copy text only (Q-001).

## Open Decisions

- None.

## References

- [hiep-plugins/plugins/watchtower-board/README.md](hiep-plugins/plugins/watchtower-board/README.md)
- [hiep-plugins/plugins/watchtower-board/shared/board.ts](hiep-plugins/plugins/watchtower-board/shared/board.ts)
- [hiep-plugins/plugins/watchtower-board/client/page.tsx](hiep-plugins/plugins/watchtower-board/client/page.tsx)
- [hiep-plugins/plugins/watchtower-board/index.client.tsx](hiep-plugins/plugins/watchtower-board/index.client.tsx)
- [watchtower/archive/20261006-t3code-orchestration-port/](watchtower/archive/20261006-t3code-orchestration-port/) (a real finished plan to use as test data)

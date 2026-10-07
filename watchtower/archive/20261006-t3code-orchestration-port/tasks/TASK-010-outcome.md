# TASK-010 Outcome

## Outcome

Status: DONE

Changed:

- Plugin SDK: `revealTimelinePassage` and `PluginTimelinePassage` in [packages/plugin/src/client/contracts.ts](packages/plugin/src/client/contracts.ts). `PluginAttachmentSourceContribution` gets `onOpen(item)`, and its picker fields become optional, in [packages/plugin/src/contracts.ts](packages/plugin/src/contracts.ts) (ADR-0003, proposed).
- Registration in [packages/app/src/plugins/evaluate.ts](packages/app/src/plugins/evaluate.ts): reveal input validation, and `parseAttachmentSource` (a source needs `search` or `onOpen`).
- Reveal path: [packages/app/src/plugins/client-runtime.ts](packages/app/src/plugins/client-runtime.ts) calls `navigateToAgent`, then the registry in [packages/app/src/plugins/timeline-reveal.ts](packages/app/src/plugins/timeline-reveal.ts). The per-stream revealer is in [packages/app/src/agent-stream/passage-reveal/](packages/app/src/agent-stream/passage-reveal/), mounted in [packages/app/src/agent-stream/view.tsx](packages/app/src/agent-stream/view.tsx). [packages/app/src/agent-stream/strategy-web.tsx](packages/app/src/agent-stream/strategy-web.tsx) pins the reveal's message rows like chat find. `findUniqueMessageMatch` is new in [packages/app/src/agent-stream/chat-find/ranges.web.ts](packages/app/src/agent-stream/chat-find/ranges.web.ts).
- Chip press: `openPluginResource` in [packages/app/src/composer/actions.ts](packages/app/src/composer/actions.ts), [packages/app/src/plugins/attachments/open.ts](packages/app/src/plugins/attachments/open.ts), wired in [packages/app/src/composer/index.tsx](packages/app/src/composer/index.tsx). The picker skips sources without `search` ([packages/app/src/plugins/attachments/picker.tsx](packages/app/src/plugins/attachments/picker.tsx)).
- New label `agentStream.passageNotLoaded` in all 9 locales. Docs in [docs/plugins.md](docs/plugins.md) and [public-docs/plugins/reference.md](public-docs/plugins/reference.md).

Contract:

- Q-007 answered: a chip press calls the source `onOpen`, and the cite plugin (TASK-007) registers a chip-only source with `onOpen` (ADR-0003).
- Q-010 DEFAULTED: native opens the agent only; it does not scroll. The native stream has no `scrollToMessage`.
- A request made before the stream mounts waits up to 10 s, and the stream holds it until authoritative history is ready.
- The highlight needs exactly one match inside one block, ignoring case and whitespace. It clears after 2.5 s, because `::highlight()` cannot animate.
- A message that is not loaded, or that never becomes visible within 5 s, shows a toast. The agent tab still opens first.
- Spec deviation: `serverId` is optional and defaults to the plugin's host.

Review (reviewer agent, claude-opus-5-5 xhigh): 2 high, 2 medium, 3 low. Fixed:

- High: a reveal on an agent that was not open showed a false "not loaded" toast, because the stream mounts before its history. Fixed: the request waits for `isAuthoritativeHistoryReady`, with a test.
- High: the uniqueness check counted only mounted rows of a long message. Fixed: the reveal pins its message in the web virtualizer, and counts only after the pin commits.
- Medium: a message in `items` that never becomes visible polled silently for 5 s. Fixed: toast at once when the window cannot show it, and at the deadline.
- Medium: native called `revealLoadedMessage` for nothing. Fixed: native does nothing beyond opening the agent (Q-010).
- Low: added tests for the hook and the chip press lookup, and documented the one-block, case-insensitive match.
- Accepted, not fixed: the agent tab opens before the not-loaded toast. Speculation, not fixed: a hidden stale stream of the same agent can take a request.

Verified:

- From `packages/app`: `npx vitest run src/agent-stream/passage-reveal/use-passage-reveal.test.tsx src/plugins/attachments/open.test.ts src/plugins/timeline-reveal.test.ts src/plugins/evaluate.test.ts src/composer/actions.test.ts src/plugins/attachments/model.test.ts src/agent-stream/strategy-web.test.tsx --bail=1` -> 7 files, 123 passed.
- From `packages/app`: `npx vitest run --project browser src/agent-stream/chat-find/ranges.browser.test.ts --bail=1` -> 5 passed, including the unique-match and duplicate-text cases.
- `npm run typecheck` -> exit 0. `npm run lint` on the changed files -> 0 warnings, 0 errors.
- GitNexus impact: `openComposerAttachment` and `PluginAttachmentSourceContribution` CRITICAL; both changes only add optional fields. Detect-changes: risk critical, 16 flows, all through `handleCancelAgent` line shifts in `composer/index.tsx`; that function is not edited.
- Browser check (a test plugin reveals an older message, the timeline scrolls and highlights) -> UNVERIFIED (autonomous run). The live daemon on port 6768 was stopped.

Anti-goal:

- Before changes: effect and subscription counts `view.tsx` 3, `strategy-web.tsx` 16, `strategy-native.tsx` 6; 2026-10-07 01:15.
- After the reveal change: 3, 16, 6; 01:23.
- Final: 3, 16, 6; 01:36. The hook test "registers once per stream however often the timeline re-renders" passed. The revealer adds two effects per stream, none per row.
- Result: PASS.

Lessons:

- The stream view mounts before its authoritative history arrives. Code that checks loaded items on mount must wait for `isAuthoritativeHistoryReady`.
- Counting DOM matches in a virtualized message needs every row of that message pinned, as chat find does in `strategy-web.tsx`.
- The vitest Lucide stub (`packages/app/test-stubs/lucide-react-native.ts`) lacks many icons, such as `Quote`. Use `Blocks` in plugin tests.
- After a `packages/plugin` type change, run `npm run build:plugin` before `npm run typecheck`, or the app sees stale declarations.

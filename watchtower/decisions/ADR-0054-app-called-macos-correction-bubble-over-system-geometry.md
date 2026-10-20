# ADR-0054 App-called macOS correction bubble over system geometry or a GPUI pill

Status: accepted
Date: 2026-10-20
Supersedes: -
Scope: `packages/desktop-gpui/vendor/gpui-base/src/input/**`, `packages/desktop-gpui/vendor/gpui-base/Cargo.toml`, `packages/desktop-gpui/vendor/gpui-base/BACHUC-PATCH.md`

## Context

- Hiep wants the GPUI composer to preview a macOS Text Replacement as Electron does: a bubble with the phrase and an x under the typed shortcut, for example `ss`.
- macOS shows that bubble only for AppKit text views. A custom `NSTextInputClient`, such as GPUI's view, never gets it. A trace of every geometry call stayed empty while Hiep typed `ss`: macOS never asked GPUI where the text was.
- `-[NSSpellChecker showCorrectionIndicatorOfType:primaryString:alternativeStrings:forStringInRect:view:completionHandler:]` draws the bubble on request. The 2026-10-20 crash stack shows it runs a nested event loop (`NSCorrectionPanel _interceptEvents` calls `nextEventMatchingMask`), and that loop drains the main dispatch queue.
- GPUI runs its tasks from the main queue and holds the `AppCell` borrow during paint. A task run inside that nested loop hit `panic_already_borrowed` in `AppCell::borrow_mut`, and the app aborted (`SIGABRT`, report `bachuc-desktop-2026-10-20-120945.ips`).

## Decision

- The input asks for the shortcut at a lone cursor on each paint of a focused field, and `native.rs` shows, keeps, or dismisses the macOS correction bubble for it.
- Every `showCorrectionIndicator` and `dismissCorrectionIndicatorForView` call runs as its own main-queue item (`dispatch_async_f`), outside any GPUI borrow. The paint updates only the bubble state.
- A bubble closed with nil while the current event is a left mouse click counts as the x. That shortcut then stays as typed at the next boundary. Any other close with nil, such as a key press, does not reject it.
- Hiep chose the macOS bubble over a GPUI-drawn one and confirmed the result on 2026-10-20.

## Consequences

- The bubble looks and acts like the one in Electron and AppKit, with no pixels drawn by Bachuc.
- The patch lives in vendored gpui-base and must be re-applied after a gpui-kit upgrade ([BACHUC-PATCH.md](../../packages/desktop-gpui/vendor/gpui-base/BACHUC-PATCH.md)).
- A focused input reads the replacement list on each frame, so the list is cached for 2 s and the word scan stops at 64 characters.
- Any future AppKit call that can spin an event loop must also leave the GPUI paint and event path.

## Options Considered

- Add the macOS 14 `documentVisibleRect` and `unionRectInVisibleSelectedRange` to GPUI's view and let macOS draw the bubble: rejected. macOS never asks a custom `NSTextInputClient` for this bubble (the trace was empty). The idea came from a misread screenshot: the x box in it was an image attachment in the composer.
- Call the bubble API inside the input paint: rejected. It crashed the app on the first typed shortcut (Context).
- Draw a GPUI pill under the caret: rejected. It would copy the macOS look by hand, and it would need its own x and hit testing.
- Read a nil close as a rejection every time: rejected. AppKit can close the bubble on a key press, and the space after `ss` would then keep the shortcut.

## Revisit If

- macOS starts to show the Text Replacement bubble for a custom `NSTextInputClient` by itself.
- gpui-pre gains a hook that runs after paint outside the `AppCell` borrow, so the main-queue dispatch can go.
- Hiep reports the x does not keep the shortcut, or a space after a shown bubble does not replace it.

## Lessons

- A stale cargo build hid the first patch: the patched files had mtimes older than the last `gpui-base` build, so cargo reused it. The binary had no `userReplacementsDictionary`. Check that a new symbol is in the binary (`strings -a`) before you debug the logic, and `touch` the files to force a rebuild.
- Trace the platform calls before you act on a screenshot. One empty log disproved the geometry theory that a misread image suggested.
- An AppKit UI call (panel, menu, sheet, correction bubble) can spin a nested run loop. Never make one inside GPUI paint or an entity update; dispatch it to the main queue.

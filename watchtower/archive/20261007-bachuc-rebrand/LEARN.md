# Learn 20261007-bachuc-rebrand

## Summary

Discrepancy: 11 found. All 12 TASKs shipped, but three brand changes broke tests that no Verify line ran, and two specs assumed wrong tool behavior.

## Per TASK

- TASK-001: plan a Codex agent makes concepts and the SVG -> shipped concepts before Start, then a hand-written SVG in the iteration. Mistake: none; the spec was rewritten after the owner picked early. Fix: none. The owner check at 16 px and 1024 px is still pending.
- TASK-002: plan production name only -> shipped the development name too, because live runs with `APP_VARIANT=development`. Mistake: the spec missed that live uses the development variant, and it missed [packages/app/src/desktop/daemon/daemon-management-error.test.ts](packages/app/src/desktop/daemon/daemon-management-error.test.ts), which asserts UI text with "Paseo". CI failed after the merge; PR #47 fixed it. Fix: grep test files for the literal text before a rename TASK, and add them to Verify.
- TASK-003: plan `executableName: Bachuc` -> shipped `Paseo` (Q-005 defaulted), and edited `runtime-paths.ts` beyond `Files:`. Mistake: the spec assumed the executable name is separate from the bundle name. Fix: read the electron-builder naming code before writing a packaging spec.
- TASK-004: match.
- TASK-005: plan at most 60 changed lines -> needed 104 and went BLOCKED until Q-007 raised the limit. The change also broke the CLI tests `17-onboard` and `03-daemon`, which Verify did not list; PR #47 fixed them. Mistake: the limit was a guess, and the spec did not list tests that rely on the removed defaults. Fix: estimate the diff before setting a limit, and grep tests for each default value a TASK removes.
- TASK-006: match. It added [packages/desktop/src/features/app-updates-enabled.ts](packages/desktop/src/features/app-updates-enabled.ts) beyond `Files:`, so a test can flip the gate.
- TASK-007: match. It left [packages/app/e2e/browser/sidebar-help.spec.ts](packages/app/e2e/browser/sidebar-help.spec.ts) stale, and TASK-010 fixed it.
- TASK-008: plan `rg "Paseo Fork"` has no match -> one match, the signing identity that stays on purpose. Mistake: the Verify grep was broader than the goal. Fix: grep the exact stale token, here `Paseo Fork\.app`.
- TASK-009: plan a top-level `executableName` -> shipped `mac.executableName`, and fixed [.github/workflows/nix.yml](.github/workflows/nix.yml) beyond `Files:`. Mistake: the spec did not know that a top-level name also renames the Linux and Windows binaries. Fix: as for TASK-003.
- TASK-010: plan assert the current help menu -> three help menu tests are `test.fixme` (Q-008 defaulted), because the fork hides the Help button. Mistake: the spec did not check that the menu is reachable. Fix: open the UI path in the source before writing an e2e TASK.
- TASK-011: plan `rg "paseo\.sh|hosted"` has no match -> two matches on `self-hosted`. Mistake: a Verify grep for a bare word. Fix: quote the token, as in `'"hosted"'`. The nix parse check is UNVERIFIED, because nix is not installed.
- TASK-012: match. It also fixed [public-docs/hub/configuration/index.md](public-docs/hub/configuration/index.md), outside `Files:` but inside the Goal.

## Plan-Level

- Verify ran only the test files each TASK changed. Brand text and default changes broke tests in other files, and only CI found them, after the merge of PR #38.
- PR #38 merged with 11 failing CI jobs, by the owner's choice after a local build passed. `main` had no CI baseline. Later checks found the other failures were there before the rebrand: `agent-loading`, `agent-manager`, `migration-doc`, the RPM package name with a space, the Nix Update Hash secret, and the Metro warmup timeouts.
- The run could not start live, so every UI check is UNVERIFIED. The release install is not verified either.
- The second pass needed four new TASKs (TASK-009 to TASK-012) from review leftovers and a defaulted question.
- Follow-ups left open: `paseo open` still looks for `/Applications/Paseo.app` ([packages/cli/src/commands/open.ts](packages/cli/src/commands/open.ts)), and [.github/workflows/nix.yml](.github/workflows/nix.yml) line 107 still expects `sh.paseo.desktop`.

## Lessons

- Before a rename TASK, grep `*.test.*`, `tests/`, and `e2e/` for the old text and the removed defaults. Put each hit in Verify.
- Set an anti-goal limit from a measured estimate, not a guess.
- Read the build tool's naming code before writing a packaging spec.
- Get a CI baseline on `main` before a big merge, so new and old failures can be told apart.
- Plan a time for UI checks on live, or accept that a run without live leaves them all UNVERIFIED.

# Learn 20261025-gpui-native-mermaid-pele-quickjs

## Summary

Discrepancy: 5 found. Everything planned shipped. The gaps were spec facts about usvg and Rust raw strings, plus repo hooks the plan did not name.

## Per TASK

- TASK-301: plan vendored Pele, rquickjs, and a bench -> shipped as planned, GO at init 117-142 ms and p95 7.4-9.6 ms. Mistake: `Files:` missed `.oxlintrc.json`, `.oxfmtrc.json`, and `lefthook.yml`. The commit hook linted the vendored JS and reported 2757 errors. Cause: the plan did not check the repo hooks for a vendored asset. Fix: list the lint and format ignore files when a plan vendors JS.
- TASK-302: match. The builder also added an `## Avoid` line for ADR-0067 under R4; this was a safe default.
- TASK-303: plan engine, `resolve_vars`, and `rasterize` -> shipped. Mistake 1: the spec red tests used `r#"..."#` around text that holds `"#`, so they did not compile. The builder changed only the delimiters. Mistake 2: the spec did not know that usvg stops reading a `style` attribute at the first custom property, so the font was lost. `resolve_vars` now drops those declarations. Fix: compile red tests before writing a spec, and render a real Pele sample through usvg at plan time.
- TASK-304: plan Mermaid through Pele and WebView for KaTeX only -> shipped. Screen check PASS; Hiep checked light and dark (Q-191). Gap: the anti-goal Ready-path timing read was never taken, because the example prints no timings and the unfocused window drew only 2 frames. Fix: plan a timing read that does not need a drawn window, for example a log in the crate behind an env var, or drop the read point.

## Plan-Level

- Wrong order: Group A held TASK-301, TASK-303, and TASK-304, but TASK-303 depended on TASK-302 in Group B, which depended on TASK-301. The groups depended on each other, and the run had to split Group A into two phases. Fix: put a gate spike in its own group when a later TASK in the same files depends on a TASK that needs the spike result.
- Plan misdescribes reality: during the run, another agent merged the plan commit as PR #92 and moved the checkout to `main`. Later, Hiep moved the run to `feat/gpui-pele-spike`. The run recovered without lost work.
- ADR-0067 links `watchtower/tasks/TASK-301-outcome.md`. After the archive, that file lives under this archive folder.

## Lessons

- Before planning SVG work for GPUI, render one real sample through usvg 0.46 and check the text style, not only the colors.
- A vendored JS file needs ignore lines in the lint config, the format config, and the lint hook.
- Write a `## Verify` timing read that works with no window focus, or mark it as an owner check in the plan.

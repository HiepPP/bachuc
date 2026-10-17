# ADR-0034 Rust and GPUI desktop client over Swift and Electron

Status: accepted
Date: 2026-10-15
Supersedes: -
Scope: `packages/desktop-gpui/**`, `packages/desktop/**`, `prototypes/**`

## Context

- On 2026-10-15 three clients showed the same live agent: a mock `thirty-minute-stream` on the live daemon (port 6768), about 10 stream events per second. Each app was measured in the foreground for 60 seconds, two rounds each, with a `measure.sh` script (CPU time delta and `footprint`).
- Rust and GPUI prototype (gpui-kit 0.7.3 on the gpui-pre 0.3.9 snapshot, full markdown): 6.5% to 6.6% CPU of one core, about 197 MB phys_footprint including the WebKit helper processes of one mermaid WebView.
- Swift prototype (SwiftUI, inline markdown only): 23.2% to 26.1% CPU, about 123 MB phys_footprint including WebKit.
- Electron live dev build: 54.4% to 108.5% CPU, 1195 MB to 1551 MB phys_footprint. The release Electron app showed 43.0% CPU and 1237 MB under a different workload.
- The GPUI prototype exposed almost nothing to the macOS accessibility tree. The gpui-pre 0.3.9 snapshot ships accesskit 0.24 and an `a11y` API that the prototype did not use.
- A GPUI build needs the rustup toolchain 1.97.1, because Homebrew `rustc` 1.79 is too old. The `runtime_shaders` feature of gpui-kit removes the need for the Xcode Metal compiler.
- 15 of 16 client plugins in `hiep-plugins` render React UI, which GPUI cannot run.

## Decision

- Build the replacement desktop client in Rust with GPUI, starting from the prototype in [prototypes/bachuc-gpui](prototypes/bachuc-gpui).
- The client lives in `packages/desktop-gpui` as a Cargo workspace. It talks to the existing TypeScript daemon over the WebSocket protocol. The daemon stays as it is.
- The owner chose this on 2026-10-15, after the measurements above. The Swift prototype was removed the same day.

## Consequences

- Each UI part needs `a11y` roles and labels from the start, because GPUI gives no accessibility tree for free.
- Plugin UI needs a new host design, because React plugin surfaces cannot run in GPUI.
- The client depends on a pre-1.0 GPUI snapshot that gpui-kit pins with `=0.3.9`. A snapshot bump can break the API.
- Electron stays the daily app until the GPUI client covers the daily flows.

## Options Considered

- Swift with SwiftUI and AppKit: rejected. It used 23.2% to 26.1% CPU with only inline markdown, against 6.5% for GPUI with full markdown. It had better accessibility, SwiftTerm, a 4.8 MB binary, and iOS code sharing.
- Keep Electron: rejected as the long-term client. It used 1195 MB to 1551 MB, 6 to 12 times the native clients.

## Revisit If

- A Swift timeline built on TextKit renders full markdown at 10% CPU or less on the same workload.
- A gpui-kit or gpui-pre snapshot bump blocks the build for more than two weeks.
- VoiceOver cannot read the sidebar, the timeline, and the composer after the accessibility audit TASK.

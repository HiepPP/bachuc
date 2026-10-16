//! The thinking orb engine of the `thinking-orbs` library as pure functions, for the seven
//! states the Thinking orb settings offer. Each function ports the one of the same name in
//! `hiep-plugins/plugins/board/node_modules/thinking-orbs/dist/index-B8WsUNf5.js` (line numbers
//! below) with `f64` math, so a frame matches the Electron orb. `modes` holds the frame of each
//! mode; this module holds the profiles, the presets, and the ink.

mod modes;

use crate::model::OrbState;
use std::collections::BTreeMap;

/// The engine mode that draws a state (`STATE_TO_MODE`, line 780). `ring` and `morph` draw the
/// `breathing` and `shaping` states, which the settings do not offer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Mode {
    Rubik,
    Orbits,
    Globe,
    Wave,
    Braid,
    Ribbon,
    Web,
}

impl Mode {
    pub(crate) fn of(state: OrbState) -> Self {
        match state {
            OrbState::Solving => Self::Rubik,
            OrbState::Working => Self::Orbits,
            OrbState::Searching => Self::Globe,
            OrbState::Listening => Self::Wave,
            OrbState::Weaving => Self::Braid,
            OrbState::Composing => Self::Ribbon,
            OrbState::Connecting => Self::Web,
        }
    }
}

/// One dot of a frame: canvas position, depth, radius, ink weight, and alpha.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Dot {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub r: f64,
    pub white: f64,
    pub a: f64,
}

/// One stroke of the `web` mode.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Line {
    pub x1: f64,
    pub y1: f64,
    pub x2: f64,
    pub y2: f64,
    pub white: f64,
    pub a: f64,
    pub w: f64,
}

/// The dots sorted from the far side to the near side, and the lines drawn under them.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct Frame {
    pub dots: Vec<Dot>,
    pub lines: Vec<Line>,
}

/// A mode's options by their engine names; a missing key takes the default its frame names.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct Opts(BTreeMap<&'static str, f64>);

impl Opts {
    fn new(pairs: &[(&'static str, f64)]) -> Self {
        Self(pairs.iter().copied().collect())
    }

    pub(crate) fn get(&self, key: &str, default: f64) -> f64 {
        self.0.get(key).copied().unwrap_or(default)
    }

    fn has(&self, key: &str) -> bool {
        self.0.contains_key(key)
    }
}

/// `Math.round`: halves round up.
pub(crate) fn js_round(value: f64) -> f64 {
    (value + 0.5).floor()
}

/// `BASE_PROFILES` (lines 163-250) of the seven ported modes.
fn base_profile(mode: Mode) -> Opts {
    Opts::new(match mode {
        Mode::Globe => &[
            ("latRings", 17.),
            ("lonDensity", 44.),
            ("rBase", 0.6),
            ("rDepth", 1.7),
            ("rBoost", 1.),
            ("inkFar", 0.62),
            ("inkSpan", 0.54),
            ("rsPow", 0.6),
            ("rMin", 0.3),
        ],
        Mode::Orbits => &[
            ("orbitN", 12.),
            ("ghostN", 40.),
            ("ghostR", 0.9),
            ("ghostA", 0.5),
            ("particles", 3.),
            ("partR", 1.2),
            ("partRDepth", 1.6),
            ("rsPow", 0.6),
            ("rMin", 0.3),
        ],
        Mode::Rubik => &[
            ("latRings", 15.),
            ("lonDensity", 40.),
            ("moveCount", 14.),
            ("rBase", 0.6),
            ("rDepth", 1.7),
            ("rActive", 0.3),
            ("inkFar", 0.62),
            ("inkSpan", 0.54),
            ("rsPow", 0.6),
            ("rMin", 0.3),
        ],
        Mode::Wave => &[
            ("rings", 15.),
            ("lonDensity", 40.),
            ("rBase", 0.6),
            ("rDepth", 1.7),
            ("rsPow", 0.6),
            ("rMin", 0.3),
        ],
        Mode::Web => &[
            ("nodeN", 30.),
            ("thr", 0.72),
            ("signals", 5.),
            ("nodeR", 1.4),
            ("nodeRDepth", 1.8),
            ("lineW", 0.8),
            ("rsPow", 0.6),
            ("rMin", 0.3),
        ],
        Mode::Braid => &[
            ("strandN", 52.),
            ("turns", 3.),
            ("ghostN", 150.),
            ("rBase", 1.2),
            ("rDepth", 1.8),
            ("rsPow", 0.6),
            ("rMin", 0.3),
        ],
        Mode::Ribbon => &[
            ("lanes", 5.),
            ("segs", 88.),
            ("ghostN", 150.),
            ("rBase", 1.1),
            ("rDepth", 1.7),
            ("rsPow", 0.6),
            ("rMin", 0.3),
        ],
    })
}

const COUNT_PAIRS: [(&str, &str); 3] = [
    ("latRings", "lonDensity"),
    ("rings", "lonDensity"),
    ("lanes", "segs"),
];
const COUNT_KEYS: [&str; 5] = ["orbitN", "ghostN", "nodeN", "strandN", "signals"];
const RADIUS_KEYS: [&str; 9] = [
    "rBase",
    "rDepth",
    "rActive",
    "rDot",
    "ghostR",
    "partR",
    "partRDepth",
    "nodeR",
    "nodeRDepth",
];

/// `scaleCounts` (line 112): a pair scales by the square root, a single count linearly.
fn scale_counts(opts: &Opts, scale: f64) -> Opts {
    let mut out = opts.clone();
    let mut done: Vec<&str> = Vec::new();
    let root = scale.sqrt();
    for (a, b) in COUNT_PAIRS {
        if out.has(a) && out.has(b) && !done.contains(&a) && !done.contains(&b) {
            let (va, vb) = (out.get(a, 0.), out.get(b, 0.));
            out.0.insert(a, js_round(va * root).max(2.));
            out.0.insert(b, js_round(vb * root).max(2.));
            done.extend([a, b]);
        }
    }
    for key in COUNT_KEYS {
        let value = out.get(key, 0.);
        if out.has(key) && value != 0. && !done.contains(&key) {
            out.0.insert(key, js_round(value * scale).max(1.));
        }
    }
    out
}

/// `scaleRadii` (line 136). `rSizeMul` is not read by any frame, so it is left out.
fn scale_radii(opts: &Opts, scale: f64) -> Opts {
    let mut out = opts.clone();
    for key in RADIUS_KEYS {
        if let Some(value) = out.0.get_mut(key) {
            *value *= scale;
        }
    }
    out
}

/// One `PRESETS` entry (lines 791-838): the speed, count scale, radius scale, and extras.
struct Preset {
    speed: f64,
    count: f64,
    size: f64,
    extra: &'static [(&'static str, f64)],
}

const fn preset(speed: f64, count: f64, size: f64) -> Preset {
    Preset {
        speed,
        count,
        size,
        extra: &[],
    }
}

/// `PRESETS[mode][size]` for the preset sizes 20, 32, and 64.
fn preset_of(mode: Mode, size: u16) -> Preset {
    let pick = |small: Preset, middle: Preset, large: Preset| match size {
        64 => large,
        32 => middle,
        _ => small,
    };
    match mode {
        Mode::Orbits => pick(
            preset(3.9, 0.238, 2.4),
            preset(2.9072, 0.4251, 1.6849),
            preset(1.885, 1., 1.),
        ),
        Mode::Globe => pick(
            Preset {
                extra: &[("scanMul", 4.335), ("dimBase", 0.45)],
                ..preset(2.665, 0.105, 1.75)
            },
            Preset {
                extra: &[("scanMul", 4.2301), ("dimBase", 0.45)],
                ..preset(2.3803, 0.1839, 1.4769)
            },
            Preset {
                extra: &[("scanMul", 4.08), ("dimBase", 0.45)],
                ..preset(2.015, 0.42, 1.15)
            },
        ),
        Mode::Rubik => pick(
            preset(1.95, 0.088, 1.9),
            preset(1.8964, 0.1537, 1.4951),
            preset(1.82, 0.35, 1.05),
        ),
        Mode::Wave => pick(
            preset(3.998, 0.105, 1.6),
            preset(4.1512, 0.169, 1.3232),
            preset(4.388, 0.341, 1.),
        ),
        Mode::Web => pick(
            preset(6.63, 0.25, 1.52),
            preset(5.0104, 0.4942, 1.2571),
            preset(3.315, 1.35, 0.95),
        ),
        Mode::Braid => pick(
            preset(2.75, 0.1125, 1.36),
            preset(2.2234, 0.2056, 1.2011),
            preset(1.625, 0.5, 1.),
        ),
        Mode::Ribbon => pick(
            Preset {
                extra: &[("spin", 0.), ("bandMul", 4.94), ("wobMul", 1.)],
                ..preset(3.12, 0.051, 1.073)
            },
            Preset {
                extra: &[("spin", 0.), ("bandMul", 4.49), ("wobMul", 1.)],
                ..preset(2.7776, 0.0969, 0.9766)
            },
            Preset {
                extra: &[("spin", 0.), ("bandMul", 3.9), ("wobMul", 1.)],
                ..preset(2.34, 0.25, 0.85)
            },
        ),
    }
}

/// A mode's options at one preset size and the speed its seconds are multiplied by.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Resolved {
    pub mode: Mode,
    pub speed: f64,
    pub opts: Opts,
}

/// `resolvePreset` (line 839). `size` is 20, 32, or 64.
pub(crate) fn resolve_preset(state: OrbState, size: u16) -> Resolved {
    let mode = Mode::of(state);
    let preset = preset_of(mode, size);
    let mut opts = base_profile(mode);
    if preset.count != 1. {
        opts = scale_counts(&opts, preset.count);
    }
    if preset.size != 1. {
        opts = scale_radii(&opts, preset.size);
    }
    for (key, value) in preset.extra {
        opts.0.insert(key, *value);
    }
    Resolved {
        mode,
        speed: preset.speed,
        opts,
    }
}

/// The frame of `mode` at time `t` on a `size` px canvas.
pub(crate) fn frame(mode: Mode, size: f64, t: f64, opts: &Opts) -> Frame {
    match mode {
        Mode::Rubik => modes::frame_rubik(size, t, opts),
        Mode::Orbits => modes::frame_orbits(size, t, opts),
        Mode::Globe => modes::frame_globe(size, t, opts),
        Mode::Wave => modes::frame_wave(size, t, opts),
        Mode::Braid => modes::frame_braid(size, t, opts),
        Mode::Ribbon => modes::frame_ribbon(size, t, opts),
        Mode::Web => modes::frame_web(size, t, opts),
    }
}

/// The monochrome ink of `inkColor` (line 47): gray `round((1 - w) * 255)` on a dark surface
/// and `round(w * 255)` on a light one, with `w` limited to 0 to 1.
pub(crate) fn ink_gray(white: f64, dark: bool) -> u8 {
    let w = white.clamp(0., 1.);
    js_round(if dark { 1. - w } else { w } * 255.) as u8
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

    const STATES: [OrbState; 7] = [
        OrbState::Solving,
        OrbState::Working,
        OrbState::Searching,
        OrbState::Listening,
        OrbState::Weaving,
        OrbState::Composing,
        OrbState::Connecting,
    ];

    fn frame_at(state: OrbState, size: u16, t: f64) -> Frame {
        let resolved = resolve_preset(state, size);
        frame(resolved.mode, f64::from(size), t, &resolved.opts)
    }

    #[test]
    fn every_mode_sorts_by_depth_floors_the_radius_and_repeats_a_time() {
        for state in STATES {
            for size in [20, 32, 64] {
                for t in [0., 0.6, 3.7, 41.25] {
                    let frame = frame_at(state, size, t);
                    assert!(!frame.dots.is_empty(), "{state:?} {size}");
                    assert!(
                        frame.dots.windows(2).all(|pair| pair[0].z <= pair[1].z),
                        "{state:?} {size} sorted"
                    );
                    assert!(frame.dots.iter().all(|dot| dot.r >= 0.3), "{state:?}");
                    assert!(frame.dots.iter().all(|dot| dot.a >= 0.02), "{state:?}");
                    assert_eq!(frame, frame_at(state, size, t), "{state:?} deterministic");
                }
            }
        }
    }

    #[test]
    fn only_web_draws_lines() {
        for state in STATES {
            let lines = frame_at(state, 32, 2.).lines;
            if state == OrbState::Connecting {
                assert!(!lines.is_empty());
                assert!(lines.iter().all(|line| line.a >= 0.02 && line.w >= 0.6));
            } else {
                assert!(lines.is_empty(), "{state:?}");
            }
        }
    }

    #[test]
    fn the_presets_scale_counts_and_radii_as_the_engine() {
        // Rubik at 32: rings and density by sqrt(0.1537), radii by 1.4951.
        let rubik = resolve_preset(OrbState::Solving, 32);
        assert_eq!(rubik.mode, Mode::Rubik);
        assert_eq!(rubik.speed, 1.8964);
        assert_eq!(rubik.opts.get("latRings", 0.), 6.);
        assert_eq!(rubik.opts.get("lonDensity", 0.), 16.);
        assert_eq!(rubik.opts.get("moveCount", 0.), 14.);
        assert!((rubik.opts.get("rBase", 0.) - 0.6 * 1.4951).abs() < 1e-12);
        // Wave scales its own ring pair; braid scales the single counts.
        let wave = resolve_preset(OrbState::Listening, 20);
        assert_eq!(wave.opts.get("rings", 0.), 5.);
        assert_eq!(wave.opts.get("lonDensity", 0.), 13.);
        let braid = resolve_preset(OrbState::Weaving, 64);
        assert_eq!(braid.opts.get("strandN", 0.), 26.);
        assert_eq!(braid.opts.get("ghostN", 0.), 75.);
        assert_eq!(braid.opts.get("rBase", 0.), 1.2);
        // Extras land after the scale.
        let globe = resolve_preset(OrbState::Searching, 64);
        assert_eq!(globe.opts.get("scanMul", 1.), 4.08);
        assert_eq!(globe.opts.get("dimBase", 1.), 0.45);
        let ribbon = resolve_preset(OrbState::Composing, 20);
        assert_eq!(ribbon.opts.get("lanes", 0.), 2.);
        assert_eq!(ribbon.opts.get("bandMul", 1.), 4.94);
        // Orbits at 64 keeps its base counts.
        let orbits = resolve_preset(OrbState::Working, 64);
        assert_eq!(orbits.opts, base_profile(Mode::Orbits));
    }

    #[test]
    fn the_ink_is_monochrome_and_inverts_on_a_light_surface() {
        assert_eq!(ink_gray(0., true), 255);
        assert_eq!(ink_gray(1., true), 0);
        assert_eq!(ink_gray(0.5, false), 128);
        assert_eq!(ink_gray(-2., false), 0);
        assert_eq!(ink_gray(0.62, true), 97);
    }

    /// Anti-goal tripwire: 1,000 frames of each mode at the 32 px preset in under 500 ms.
    #[test]
    fn a_thousand_frames_of_each_mode_stay_under_500_ms() {
        for state in STATES {
            let resolved = resolve_preset(state, 32);
            let started = Instant::now();
            let mut dots = 0;
            for step in 0..1_000 {
                let t = f64::from(step) / 60. * resolved.speed;
                dots += frame(resolved.mode, 32., t, &resolved.opts).dots.len();
            }
            let elapsed = started.elapsed();
            println!("orb {state:?}: 1000 frames in {elapsed:?}, {dots} dots");
            assert!(dots > 0);
            assert!(elapsed.as_millis() < 500, "{state:?} took {elapsed:?}");
        }
    }
}

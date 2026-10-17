//! The `solving` orb of the `thinking-orbs` library as pure functions: a sphere of dots whose
//! slices turn like a Rubik's cube. Each function ports the one of the same name in
//! `node_modules/thinking-orbs/dist/index-B8WsUNf5.js` (line numbers below), with `f64` math, so a
//! frame matches the Electron orb. Sizing is `shared/orb.ts`.

use std::f64::consts::PI;

/// `PRESETS` of `shared/orb.ts`: the sizes the library ships tuned designs for.
const PRESET_SIZES: [u32; 3] = [20, 32, 64];
/// `SOLVING_SPHERE`: the sphere's diameter as a fraction of its canvas (`R = size / 2 * 0.82`).
const SOLVING_SPHERE: f64 = 0.82;
/// How much faster the orb spins under the pointer (`HOVER_SPEED` of `client/stop-orb.tsx`).
pub(crate) const HOVER_SPEED: f64 = 1.8;
/// The bolder dots that match the weight of the icon strokes (`dotSize` of `client/stop-orb.tsx`).
pub(crate) const DOT_SIZE: f64 = 1.4;

/// The preset canvas and the scale that draw the solving sphere exactly `diameter` wide.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Orb {
    pub(crate) size: u32,
    pub(crate) scale: f64,
}

/// `solvingOrb`: the nearest preset to the canvas that holds a sphere of `diameter`.
pub(crate) fn solving_orb(diameter: f64) -> Orb {
    let canvas = diameter / SOLVING_SPHERE;
    let size = PRESET_SIZES
        .into_iter()
        .fold(PRESET_SIZES[0], |best, preset| {
            if (f64::from(preset) - canvas).abs() < (f64::from(best) - canvas).abs() {
                preset
            } else {
                best
            }
        });
    Orb {
        size,
        scale: canvas / f64::from(size),
    }
}

/// One point of the sphere: `x` and `y` on the canvas, `z` the depth in -1 to 1, `r` the radius,
/// `white` the ink weight (`finalizeFrame` dots).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Dot {
    pub(crate) x: f64,
    pub(crate) y: f64,
    pub(crate) z: f64,
    pub(crate) r: f64,
    pub(crate) white: f64,
}

/// One turn of a slice of the sphere (`makeMoves`).
#[derive(Clone, Copy, Debug, PartialEq)]
struct Move {
    axis: u8,
    lo: f64,
    hi: f64,
    ang: f64,
}

/// The `rubik` profile after `resolvePreset("solving", size)` and the `dotSize` scale
/// (`BASE_PROFILES.rubik` line 186, `PRESETS.rubik` line 802).
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Options {
    lat_rings: usize,
    lon_density: usize,
    move_count: usize,
    r_base: f64,
    r_depth: f64,
    r_active: f64,
    ink_far: f64,
    ink_span: f64,
    rs_pow: f64,
    r_min: f64,
    moves: Vec<Move>,
    /// The preset's `speed`: seconds are multiplied by it.
    pub(crate) speed: f64,
}

impl Options {
    /// `resolvePreset("solving", size)` (line 839), then `scaleRadii(.., dot_size)` as the
    /// component does for `dotSize` (`index.es.js:686`).
    pub(crate) fn solving(size: u32, dot_size: f64) -> Self {
        let (speed, count, radius) = match size {
            64 => (1.82, 0.35, 1.05),
            32 => (1.8964, 0.1537, 1.4951),
            _ => (1.95, 0.088, 1.9),
        };
        let move_count = 14;
        // `scaleCounts` (line 112): the ring pair scales by the square root of the count.
        let root = f64::sqrt(count);
        let lat_rings = ((15. * root).round() as usize).max(2);
        let lon_density = ((40. * root).round() as usize).max(2);
        // `scaleRadii` (line 136), once for the preset and once for `dotSize`.
        let scaled = |value: f64| value * radius * dot_size.max(0.1);
        Self {
            lat_rings,
            lon_density,
            move_count,
            r_base: scaled(0.6),
            r_depth: scaled(1.7),
            r_active: scaled(0.3),
            ink_far: 0.62,
            ink_span: 0.54,
            rs_pow: 0.6,
            r_min: 0.3,
            moves: make_moves(move_count),
            speed,
        }
    }
}

/// `hashD` (line 20).
fn hash_d(a: f64, b: f64) -> f64 {
    let h = (a * 12.9898 + b * 78.233).sin() * 43758.5453;
    h - h.floor()
}

/// `makeMoves` (line 342).
fn make_moves(count: usize) -> Vec<Move> {
    (0..count)
        .map(|i| {
            let i = i as f64;
            let axis = ((hash_d(i, 2.3) * 3.).floor()).min(2.) as u8;
            let lo = -1. + 0.5 * (hash_d(i, 5.9) * 4.).floor().min(3.);
            let dir = if hash_d(i, 7.7) < 0.5 { 1. } else { -1. };
            Move {
                axis,
                lo,
                hi: lo + 0.5,
                ang: dir * PI / 2.,
            }
        })
        .collect()
}

/// How far each move has turned, and which one turns now (`solveCycle`, line 291).
struct Cycle {
    amount: Vec<f64>,
    active: Option<usize>,
}

fn solve_cycle(time: f64, count: usize, slot_dur: f64, rest: f64) -> Cycle {
    let total = 2. * count as f64 * slot_dur;
    let tc = time % (total + rest);
    let mut amount = vec![0.; count];
    if tc >= total {
        return Cycle {
            amount,
            active: None,
        };
    }
    let slot = ((tc / slot_dur).floor() as usize).min(2 * count - 1);
    let p = (tc - slot as f64 * slot_dur) / slot_dur;
    let cl = (p / 0.7).min(1.);
    let ep = 1. - (1. - cl).powi(3);
    let active = if slot < count {
        amount[..slot].fill(1.);
        amount[slot] = ep;
        slot
    } else {
        let undo = 2 * count - 1 - slot;
        amount[..undo].fill(1.);
        amount[undo] = 1. - ep;
        undo
    };
    Cycle {
        amount,
        active: Some(active),
    }
}

/// `applyMoves` (line 314): the point after every move that holds it, and whether the move that
/// turns now holds it.
fn apply_moves(point: [f64; 3], moves: &[Move], cycle: &Cycle) -> ([f64; 3], bool) {
    let [mut x, mut y, mut z] = point;
    let mut in_active = false;
    for (i, mv) in moves.iter().enumerate() {
        if cycle.amount[i] <= 0. {
            continue;
        }
        let coord = match mv.axis {
            0 => x,
            1 => y,
            _ => z,
        };
        if coord < mv.lo || coord >= mv.hi {
            continue;
        }
        if cycle.active == Some(i) {
            in_active = true;
        }
        let angle = mv.ang * cycle.amount[i];
        let (sa, ca) = angle.sin_cos();
        match mv.axis {
            0 => {
                let y2 = y * ca - z * sa;
                z = y * sa + z * ca;
                y = y2;
            }
            1 => {
                let x2 = x * ca + z * sa;
                z = -x * sa + z * ca;
                x = x2;
            }
            _ => {
                let x2 = x * ca - y * sa;
                y = x * sa + y * ca;
                x = x2;
            }
        }
    }
    ([x, y, z], in_active)
}

/// `frameRubik` (line 389) with `makeProj` (line 34) and `finalizeFrame` (line 77): the dots of
/// the frame at time `t`, sorted from the far side to the near side. `size` is the preset canvas.
pub(crate) fn frame_rubik(size: u32, t: f64, options: &Options) -> Vec<Dot> {
    let size = f64::from(size);
    let (cx, cy) = (size / 2., size / 2.);
    let radius = size / 2. * 0.82;
    let yaw = t * 0.55;
    let tilt = 0.35 + 0.1 * (t * 0.9).sin();
    let (sin_tilt, cos_tilt) = tilt.sin_cos();
    let (sin_yaw, cos_yaw) = yaw.sin_cos();
    let project = |x: f64, y: f64, z: f64| {
        let x1 = x * cos_yaw + z * sin_yaw;
        let z1 = -x * sin_yaw + z * cos_yaw;
        let y1 = y * cos_tilt - z1 * sin_tilt;
        let z2 = y * sin_tilt + z1 * cos_tilt;
        (cx + x1 * radius, cy - y1 * radius, z2)
    };
    // `radiusScale` (line 91).
    let radius_scale = (size / 300.).powf(options.rs_pow);
    let cycle = solve_cycle(t, options.move_count, 0.42, 1.2);
    let mut dots = Vec::new();
    for ring in 0..=options.lat_rings {
        let lat = -PI / 2. + ring as f64 / options.lat_rings as f64 * PI;
        let (sin_lat, cos_lat) = lat.sin_cos();
        let lon_count = ((cos_lat.abs() * options.lon_density as f64).round() as usize).max(1);
        for slot in 0..lon_count {
            let lon = slot as f64 / lon_count as f64 * 2. * PI;
            let ([x, y, z], in_active) = apply_moves(
                [cos_lat * lon.cos(), sin_lat, cos_lat * lon.sin()],
                &options.moves,
                &cycle,
            );
            let (px, py, depth_z) = project(x, y, z);
            let depth = (depth_z + 1.) / 2.;
            let active = if in_active { 1. } else { 0. };
            dots.push(Dot {
                x: px,
                y: py,
                z: depth_z,
                r: (options.r_base + options.r_depth * depth + options.r_active * active)
                    * radius_scale,
                white: options.ink_far - options.ink_span * depth - 0.14 * active,
            });
        }
    }
    // `finalizeFrame`: every dot is opaque, so only the radius floor and the depth sort apply.
    for dot in &mut dots {
        dot.r = dot.r.max(options.r_min);
    }
    dots.sort_by(|a, b| a.z.total_cmp(&b.z));
    dots
}

/// The tint ramp of `inkColor` (line 47): `c * (1 - w)` on a dark surface, `c + (255 - c) * w`
/// on a light one, with `w` the dot's weight limited to 0 to 1.
pub(crate) fn ink_rgb(white: f64, dark: bool, tint: [u8; 3]) -> [u8; 3] {
    let w = white.clamp(0., 1.);
    tint.map(|c| {
        let c = f64::from(c);
        let ramped = if dark {
            c * (1. - w)
        } else {
            c + (255. - c) * w
        };
        ramped.round() as u8
    })
}

/// The speed factor of the frame clock: faster under the pointer, unless the agent stops.
pub(crate) fn speed_multiplier(hovered: bool, cancelling: bool) -> f64 {
    if hovered && !cancelling {
        HOVER_SPEED
    } else {
        1.
    }
}

/// The time of each frame. The library draws at `seconds * speed` and, when paused, keeps the
/// frame it showed at that moment; this keeps that frame while the agent stops.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct OrbClock {
    frozen: Option<f64>,
}

impl OrbClock {
    /// The frame time for `seconds` on a clock that started with the process.
    pub(crate) fn time(
        &mut self,
        seconds: f64,
        preset_speed: f64,
        hovered: bool,
        cancelling: bool,
    ) -> f64 {
        let t = seconds * preset_speed * speed_multiplier(hovered, cancelling);
        if cancelling {
            *self.frozen.get_or_insert(t)
        } else {
            self.frozen = None;
            t
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

    fn options() -> Options {
        Options::solving(20, DOT_SIZE)
    }

    #[test]
    fn solving_orb_picks_nearest_preset() {
        let orb = solving_orb(16.);
        assert_eq!(orb.size, 20);
        assert_eq!(orb.scale, 16. / 0.82 / 20.);
        // 26 px of canvas is the middle of 20 and 32; 48 is the middle of 32 and 64.
        assert_eq!(solving_orb(25.9 * 0.82).size, 20);
        assert_eq!(solving_orb(26.1 * 0.82).size, 32);
        assert_eq!(solving_orb(47.9 * 0.82).size, 32);
        assert_eq!(solving_orb(48.1 * 0.82).size, 64);
        assert_eq!(solving_orb(500.).size, 64);
    }

    #[test]
    fn the_20_px_preset_matches_the_library() {
        // resolvePreset("solving", 20) then scaleRadii(.., 1.4), run in Node.
        let o = options();
        assert_eq!((o.lat_rings, o.lon_density, o.move_count), (4, 12, 14));
        assert_eq!(o.speed, 1.95);
        for (got, want) in [
            (o.r_base, 1.5959999999999999),
            (o.r_depth, 4.521999999999999),
            (o.r_active, 0.7979999999999999),
        ] {
            assert!((got - want).abs() < 1e-12, "{got} vs {want}");
        }
        let o32 = Options::solving(32, DOT_SIZE);
        assert_eq!((o32.lat_rings, o32.lon_density), (6, 16));
        let o64 = Options::solving(64, DOT_SIZE);
        assert_eq!((o64.lat_rings, o64.lon_density), (9, 24));
    }

    /// The first frame values below come from `MODE_FRAMES.rubik(20, 0.6, ..)` of the library,
    /// run in Node (a move turns at that time).
    #[test]
    fn frame_matches_the_library() {
        const GOLDEN: [[f64; 5]; 30] = [
            [
                8.121109213,
                13.194107460,
                -0.892058349,
                0.362390081,
                0.590855754,
            ],
            [
                7.342847167,
                6.968971432,
                -0.870841052,
                0.371837994,
                0.585127084,
            ],
            [
                12.550197192,
                13.302752701,
                -0.860843620,
                0.376289779,
                0.582427777,
            ],
            [
                13.779136878,
                12.047292362,
                -0.851624043,
                0.537557405,
                0.439938492,
            ],
            [
                13.606523456,
                7.122619005,
                -0.826696759,
                0.391495136,
                0.573208125,
            ],
            [
                3.820064537,
                7.894153590,
                -0.605028116,
                0.490202617,
                0.513357591,
            ],
            [
                11.999882822,
                3.563548487,
                -0.569559989,
                0.505996323,
                0.503781197,
            ],
            [
                10.000000000,
                17.548178658,
                -0.390719964,
                0.585632523,
                0.455494390,
            ],
            [
                2.854280114,
                12.533172107,
                -0.381021483,
                0.747113401,
                0.312875800,
            ],
            [
                8.121109213,
                2.519370831,
                -0.339496877,
                0.608441802,
                0.441664157,
            ],
            [
                12.550197192,
                2.628016072,
                -0.308282148,
                0.622341500,
                0.433236180,
            ],
            [
                17.757547217,
                8.961797341,
                -0.298284716,
                0.626793286,
                0.430536873,
            ],
            [
                1.953190623,
                9.383595593,
                -0.177098384,
                0.680756704,
                0.397816564,
            ],
            [
                17.044796233,
                5.896714118,
                -0.107281799,
                0.869007749,
                0.238966086,
            ],
            [
                4.514585758,
                16.071488455,
                -0.065361591,
                0.730512309,
                0.367647629,
            ],
            [
                16.718232961,
                5.326800794,
                -0.062962160,
                0.731580759,
                0.366999783,
            ],
            [
                14.476904704,
                3.236469238,
                0.146948735,
                0.982214811,
                0.170323842,
            ],
            [
                13.878773608,
                2.943984200,
                0.189231056,
                0.843880617,
                0.298907615,
            ],
            [
                2.242452783,
                11.038202659,
                0.298284716,
                0.892441441,
                0.269463127,
            ],
            [
                7.449802808,
                17.371983928,
                0.308282148,
                0.896893227,
                0.266763820,
            ],
            [
                11.878890787,
                17.480629169,
                0.339496877,
                0.910792925,
                0.258335843,
            ],
            [
                10.000000000,
                2.451821342,
                0.390719964,
                0.933602204,
                0.244505610,
            ],
            [
                17.364305029,
                11.409140714,
                0.404858468,
                0.939897980,
                0.240688214,
            ],
            [
                4.514585758,
                5.396751825,
                0.487199881,
                0.976564030,
                0.218456032,
            ],
            [
                15.757664395,
                7.850070196,
                0.661995237,
                1.054399167,
                0.171261286,
            ],
            [
                4.610343455,
                12.414624161,
                0.693742668,
                1.068536074,
                0.162689480,
            ],
            [
                7.449802808,
                6.697247299,
                0.860843620,
                1.142944948,
                0.117572223,
            ],
            [
                12.657152833,
                13.031028568,
                0.870841052,
                1.147396734,
                0.114872916,
            ],
            [
                11.878890787,
                6.805892540,
                0.892058349,
                1.156844646,
                0.109144246,
            ],
            [
                8.422388247,
                13.144049069,
                0.903312832,
                1.161856188,
                0.106105535,
            ],
        ];
        let frame = frame_rubik(20, 0.6, &options());
        assert_eq!(frame.len(), GOLDEN.len());
        for (dot, want) in frame.iter().zip(GOLDEN) {
            let got = [dot.x, dot.y, dot.z, dot.r, dot.white];
            for (got, want) in got.into_iter().zip(want) {
                assert!((got - want).abs() < 1e-8, "{dot:?} vs {want:?}");
            }
        }
        // t = 3.1 undoes a move; the three dots at the back.
        let frame = frame_rubik(20, 3.1, &options());
        assert_eq!(frame.len(), 30);
        let back = [
            [
                9.224183841,
                13.220024012,
                -0.914793113,
                0.352266451,
                0.596994141,
            ],
            [
                7.710791181,
                7.046947658,
                -0.890152057,
                0.520401153,
                0.450341055,
            ],
            [
                13.949027865,
                7.304794626,
                -0.812428068,
                0.555011096,
                0.429355578,
            ],
        ];
        for (dot, want) in frame.iter().zip(back) {
            let got = [dot.x, dot.y, dot.z, dot.r, dot.white];
            for (got, want) in got.into_iter().zip(want) {
                assert!((got - want).abs() < 1e-8, "{dot:?} vs {want:?}");
            }
        }
    }

    #[test]
    fn frame_dots_sorted_by_depth() {
        for t in [0., 0.6, 3.1, 7.3, 19.77, 100.4] {
            let frame = frame_rubik(20, t, &options());
            assert!(frame.windows(2).all(|pair| pair[0].z <= pair[1].z), "t {t}");
        }
    }

    #[test]
    fn frame_radius_floor() {
        for size in PRESET_SIZES {
            let options = Options::solving(size, DOT_SIZE);
            for t in [0., 0.6, 3.1, 7.3, 19.77] {
                let frame = frame_rubik(size, t, &options);
                assert!(!frame.is_empty());
                assert!(frame.iter().all(|dot| dot.r >= 0.3), "size {size} t {t}");
            }
        }
    }

    #[test]
    fn ink_ramp_dark_and_light() {
        let red = [220, 38, 38];
        // w = 0 keeps the tint on both; w = 1 goes to black on dark and white on light.
        assert_eq!(ink_rgb(0., true, red), red);
        assert_eq!(ink_rgb(0., false, red), red);
        assert_eq!(ink_rgb(1., true, red), [0, 0, 0]);
        assert_eq!(ink_rgb(1., false, red), [255, 255, 255]);
        // w = 0.5: c * 0.5 on dark, c + (255 - c) * 0.5 on light, rounded.
        assert_eq!(ink_rgb(0.5, true, red), [110, 19, 19]);
        assert_eq!(ink_rgb(0.5, false, red), [238, 147, 147]);
        // The weight is limited to 0 to 1.
        assert_eq!(ink_rgb(-3., true, red), red);
        assert_eq!(ink_rgb(7., false, red), [255, 255, 255]);
    }

    #[test]
    fn hover_speed_multiplier() {
        assert_eq!(speed_multiplier(false, false), 1.);
        assert_eq!(speed_multiplier(true, false), 1.8);
        // A stopping orb does not speed up under the pointer.
        assert_eq!(speed_multiplier(true, true), 1.);
        let mut clock = OrbClock::default();
        assert_eq!(clock.time(10., 1.95, false, false), 10. * 1.95);
        assert_eq!(clock.time(10., 1.95, true, false), 10. * 1.95 * 1.8);
    }

    #[test]
    fn cancelling_freezes_frame() {
        let options = options();
        let mut clock = OrbClock::default();
        let running = clock.time(10., options.speed, false, false);
        let frozen = clock.time(11., options.speed, false, true);
        assert_eq!(frozen, 11. * options.speed);
        // Later frames, with or without the pointer, keep the same time and the same dots.
        for (seconds, hovered) in [(11.5, false), (12., true), (40., false)] {
            let t = clock.time(seconds, options.speed, hovered, true);
            assert_eq!(t, frozen);
            assert_eq!(
                frame_rubik(20, t, &options),
                frame_rubik(20, frozen, &options)
            );
        }
        assert_ne!(
            frame_rubik(20, running, &options),
            frame_rubik(20, frozen, &options)
        );
        // When the agent runs again, the clock follows the seconds again.
        assert_eq!(
            clock.time(41., options.speed, false, false),
            41. * options.speed
        );
    }

    #[test]
    fn one_frame_under_budget() {
        let options = options();
        let started = Instant::now();
        let mut dots = 0;
        for frame in 0..1000 {
            dots += frame_rubik(20, f64::from(frame) * 0.016 * options.speed, &options).len();
        }
        let elapsed = started.elapsed();
        assert!(dots > 0);
        assert!(elapsed.as_millis() < 500, "1000 frames took {elapsed:?}");
    }
}

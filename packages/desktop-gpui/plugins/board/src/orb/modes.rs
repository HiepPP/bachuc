//! The frame of each ported mode and the helpers they share. Line numbers are in
//! `thinking-orbs/dist/index-B8WsUNf5.js`.

use super::{Dot, Frame, Line, Opts, js_round};
use std::f64::consts::PI;

/// `lerp` (line 1).
fn lerp(a: f64, b: f64, f: f64) -> f64 {
    a + (b - a) * f
}

/// `frac` (line 4).
fn frac(x: f64) -> f64 {
    x - x.floor()
}

/// `vnoise` (line 7).
fn vnoise(x: f64, y: f64) -> f64 {
    let (xi, yi) = (x.floor(), y.floor());
    let (mut fx, mut fy) = (x - xi, y - yi);
    fx = fx * fx * (3. - 2. * fx);
    fy = fy * fy * (3. - 2. * fy);
    let a = hash_d(xi, yi);
    let b = hash_d(xi + 1., yi);
    let c = hash_d(xi, yi + 1.);
    let d = hash_d(xi + 1., yi + 1.);
    a + (b - a) * fx + (c - a) * fy + (a - b - c + d) * fx * fy
}

/// `hashD` (line 20).
fn hash_d(a: f64, b: f64) -> f64 {
    let h = (a * 12.9898 + b * 78.233).sin() * 43758.5453;
    h - h.floor()
}

/// `fibDir` (line 24).
fn fib_dir(i: f64, n: f64) -> [f64; 3] {
    let golden = PI * (3. - 5f64.sqrt());
    let y = 1. - 2. * (i + 0.5) / n;
    let rad = (1. - y * y).sqrt();
    let a = i * golden;
    [rad * a.cos(), y, rad * a.sin()]
}

/// `angleDelta` (line 31).
fn angle_delta(a: f64, b: f64) -> f64 {
    (a - b).sin().atan2((a - b).cos())
}

/// `makeProj` (line 34): yaw, then tilt, then the canvas position and depth.
struct Proj {
    st: f64,
    ct: f64,
    sy: f64,
    cyw: f64,
    cx: f64,
    cy: f64,
    scale: f64,
}

impl Proj {
    fn new(yaw: f64, tilt: f64, cx: f64, cy: f64, scale: f64) -> Self {
        Self {
            st: tilt.sin(),
            ct: tilt.cos(),
            sy: yaw.sin(),
            cyw: yaw.cos(),
            cx,
            cy,
            scale,
        }
    }

    fn at(&self, x: f64, y: f64, z: f64) -> (f64, f64, f64) {
        let x1 = x * self.cyw + z * self.sy;
        let z1 = -x * self.sy + z * self.cyw;
        let y1 = y * self.ct - z1 * self.st;
        let z2 = y * self.st + z1 * self.ct;
        (self.cx + x1 * self.scale, self.cy - y1 * self.scale, z2)
    }
}

/// `radiusScale` (line 91).
fn radius_scale(size: f64, pow: f64) -> f64 {
    (size / 300.).powf(pow)
}

/// `finalizeFrame` (line 77): drops near-invisible dots and lines, floors the radius, and
/// sorts the dots by depth. The sort is stable, as `Array.prototype.sort`.
fn finalize(dots: Vec<Dot>, lines: Vec<Line>, r_min: f64) -> Frame {
    let mut visible: Vec<Dot> = dots
        .into_iter()
        .filter(|dot| dot.a >= 0.02)
        .map(|dot| Dot {
            r: dot.r.max(r_min),
            ..dot
        })
        .collect();
    visible.sort_by(|a, b| a.z.total_cmp(&b.z));
    Frame {
        dots: visible,
        lines: lines.into_iter().filter(|line| line.a >= 0.02).collect(),
    }
}

/// A count option as a loop bound.
fn count(opts: &Opts, key: &str, default: f64) -> usize {
    opts.get(key, default).max(0.) as usize
}

/// `frameBraid` (line 252).
pub(super) fn frame_braid(size: f64, t: f64, o: &Opts) -> Frame {
    let (cx, cy) = (size / 2., size / 2.);
    let big_r = size / 2. * 0.76;
    let pt = Proj::new(t * 0.4, 0.3, cx, cy, 1.);
    let rs = radius_scale(size, o.get("rsPow", 0.6));
    let mut dots = Vec::new();
    let ghost_n = o.get("ghostN", 150.);
    for i in 0..count(o, "ghostN", 150.) {
        let d = fib_dir(i as f64, ghost_n);
        let (px, py, z) = pt.at(d[0] * big_r, d[1] * big_r, d[2] * big_r);
        let depth = (z / big_r + 1.) / 2.;
        dots.push(Dot {
            x: px,
            y: py,
            z,
            r: 0.8 * rs,
            white: 0.78,
            a: 0.1 + 0.22 * depth,
        });
    }
    let strand_n = o.get("strandN", 52.);
    let turns = o.get("turns", 3.);
    let (r_base, r_depth) = (o.get("rBase", 1.2), o.get("rDepth", 1.8));
    for s in 0..3 {
        let phase = f64::from(s) / 3. * 2. * PI;
        for i in 0..count(o, "strandN", 52.) {
            let u = (frac(i as f64 / strand_n + t * 0.045) * 2. - 1.) * 0.96;
            let surf = (1. - u * u).max(0.).sqrt();
            let end_fade = ((1. - u.abs()) / 0.1).min(1.);
            let a = u * PI * turns + phase;
            let weave = 1. + 0.075 * (u * PI * turns * 2. + phase * 2. + t * 0.8).sin();
            let rr = surf * big_r * weave;
            let (px, py, zr) = pt.at(a.cos() * rr, u * big_r * weave, a.sin() * rr);
            let depth = (zr / big_r + 1.) / 2.;
            dots.push(Dot {
                x: px,
                y: py,
                z: zr,
                r: (r_base + r_depth * depth) * rs,
                white: 0.55 - 0.45 * depth,
                a: end_fade * (0.45 + 0.55 * depth),
            });
        }
    }
    finalize(dots, Vec::new(), o.get("rMin", 0.3))
}

/// How far each move has turned, and which one turns now (`solveCycle`, line 291).
struct Cycle {
    amount: Vec<f64>,
    active: Option<usize>,
}

fn solve_cycle(time: f64, count: usize, slot_dur: f64, rest: f64) -> Cycle {
    let cyc = 2. * count as f64 * slot_dur + rest;
    let tc = time % cyc;
    let mut amount = vec![0.; count];
    let mut active = None;
    if count > 0 && tc < 2. * count as f64 * slot_dur {
        let slot = ((tc / slot_dur).floor() as usize).min(2 * count - 1);
        let p = (tc - slot as f64 * slot_dur) / slot_dur;
        let cl = (p / 0.7).min(1.);
        let ep = 1. - (1. - cl).powi(3);
        if slot < count {
            amount[..slot].fill(1.);
            amount[slot] = ep;
            active = Some(slot);
        } else {
            let u = 2 * count - 1 - slot;
            amount[..u].fill(1.);
            amount[u] = 1. - ep;
            active = Some(u);
        }
    }
    Cycle { amount, active }
}

/// One turn of a slice of the sphere (`makeMoves`, line 342).
struct Move {
    axis: u8,
    lo: f64,
    hi: f64,
    ang: f64,
}

fn make_moves(count: usize) -> Vec<Move> {
    (0..count)
        .map(|i| {
            let i = i as f64;
            let axis = (hash_d(i, 2.3) * 3.).floor().min(2.) as u8;
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

/// `applyMoves` (line 314).
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
        let (sa, ca) = (mv.ang * cycle.amount[i]).sin_cos();
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

/// The latitude rings of a dotted sphere: `(sin, cos)` of each ring and its dot count.
fn rings(ring_count: usize, lon_density: f64) -> impl Iterator<Item = (f64, f64, usize)> {
    (0..=ring_count).map(move |ring| {
        let lat = -PI / 2. + ring as f64 / ring_count as f64 * PI;
        let (sin_lat, cos_lat) = lat.sin_cos();
        let lon_count = (js_round(cos_lat.abs() * lon_density) as usize).max(1);
        (sin_lat, cos_lat, lon_count)
    })
}

fn longitude(slot: usize, lon_count: usize) -> f64 {
    slot as f64 / lon_count as f64 * 2. * PI
}

/// `frameGlobe` (line 352).
pub(super) fn frame_globe(size: f64, t: f64, o: &Opts) -> Frame {
    let spin = 0.5;
    let (cx, cy) = (size / 2., size / 2.);
    let radius = size / 2. * 0.82;
    let tilt = 0.4 + 0.06 * (t * 0.35).sin();
    let pt = Proj::new(t * spin, tilt, cx, cy, radius);
    let scan = t * (spin + (1.7 - spin) * o.get("scanMul", 1.));
    let rs = radius_scale(size, o.get("rsPow", 0.6));
    let dim_base = o.get("dimBase", 1.);
    let (r_base, r_depth, r_boost) = (
        o.get("rBase", 0.6),
        o.get("rDepth", 1.7),
        o.get("rBoost", 1.),
    );
    let (ink_far, ink_span) = (o.get("inkFar", 0.62), o.get("inkSpan", 0.54));
    let mut dots = Vec::new();
    for (sin_lat, cos_lat, lon_count) in rings(count(o, "latRings", 17.), o.get("lonDensity", 44.))
    {
        for slot in 0..lon_count {
            let lon = longitude(slot, lon_count);
            let (px, py, z) = pt.at(cos_lat * lon.cos(), sin_lat, cos_lat * lon.sin());
            let depth = (z + 1.) / 2.;
            let d = angle_delta(lon + t * spin, scan);
            let boost = (-(d * d) / 0.18).exp() * z.max(0.);
            dots.push(Dot {
                x: px,
                y: py,
                z,
                r: (r_base + r_depth * depth + r_boost * boost) * rs,
                white: ink_far - ink_span * depth,
                a: dim_base + (1. - dim_base) * boost.min(1.),
            });
        }
    }
    finalize(dots, Vec::new(), o.get("rMin", 0.3))
}

/// `frameRubik` (line 389).
pub(super) fn frame_rubik(size: f64, t: f64, o: &Opts) -> Frame {
    let (cx, cy) = (size / 2., size / 2.);
    let big_r = size / 2. * 0.82;
    let pt = Proj::new(t * 0.55, 0.35 + 0.1 * (t * 0.9).sin(), cx, cy, big_r);
    let rs = radius_scale(size, o.get("rsPow", 0.6));
    let move_count = count(o, "moveCount", 14.);
    let moves = make_moves(move_count);
    let cycle = solve_cycle(t, move_count, 0.42, 1.2);
    let (r_base, r_depth, r_active) = (
        o.get("rBase", 0.6),
        o.get("rDepth", 1.7),
        o.get("rActive", 0.3),
    );
    let (ink_far, ink_span) = (o.get("inkFar", 0.62), o.get("inkSpan", 0.54));
    let mut dots = Vec::new();
    for (sin_lat, cos_lat, lon_count) in rings(count(o, "latRings", 15.), o.get("lonDensity", 40.))
    {
        for slot in 0..lon_count {
            let lon = longitude(slot, lon_count);
            let ([x, y, z], in_active) = apply_moves(
                [cos_lat * lon.cos(), sin_lat, cos_lat * lon.sin()],
                &moves,
                &cycle,
            );
            let (px, py, zr) = pt.at(x, y, z);
            let depth = (zr + 1.) / 2.;
            let active = if in_active { 1. } else { 0. };
            dots.push(Dot {
                x: px,
                y: py,
                z: zr,
                r: (r_base + r_depth * depth + r_active * active) * rs,
                white: ink_far - ink_span * depth - 0.14 * active,
                a: 1.,
            });
        }
    }
    finalize(dots, Vec::new(), o.get("rMin", 0.3))
}

/// `frameWave` (line 422).
pub(super) fn frame_wave(size: f64, t: f64, o: &Opts) -> Frame {
    let (cx, cy) = (size / 2., size / 2.);
    let big_r = size / 2. * 0.874;
    let pt = Proj::new(t * 0.18, 0.38, cx, cy, 1.);
    let rs = radius_scale(size, o.get("rsPow", 0.6));
    let (r_base, r_depth) = (o.get("rBase", 0.6), o.get("rDepth", 1.7));
    let mut dots = Vec::new();
    for (ri, (sin_lat, cos_lat, lon_count)) in
        rings(count(o, "rings", 15.), o.get("lonDensity", 40.)).enumerate()
    {
        let ri = ri as f64;
        let w = 0.62 * (t * 2.1 - ri * 0.52).sin() + 0.38 * (t * 1.27 + ri * 0.83).sin();
        let rr = big_r * (0.88 + 0.105 * w);
        let crest = w.max(0.);
        for slot in 0..lon_count {
            let lon = longitude(slot, lon_count);
            let (px, py, z) = pt.at(
                cos_lat * lon.cos() * rr,
                sin_lat * rr,
                cos_lat * lon.sin() * rr,
            );
            let depth = (z / big_r + 1.) / 2.;
            dots.push(Dot {
                x: px,
                y: py,
                z,
                r: (r_base + r_depth * depth) * (1. + 0.4 * crest) * rs,
                white: 0.66 - 0.56 * depth - 0.1 * crest,
                a: 1.,
            });
        }
    }
    finalize(dots, Vec::new(), o.get("rMin", 0.3))
}

/// `frameOrbits` (line 559).
pub(super) fn frame_orbits(size: f64, t: f64, o: &Opts) -> Frame {
    let (cx, cy) = (size / 2., size / 2.);
    let big_r = size / 2. * 0.82;
    let pt = Proj::new(t * 0.12, 0.3, cx, cy, 1.);
    let rs = radius_scale(size, o.get("rsPow", 0.6));
    let ghost_n = count(o, "ghostN", 40.);
    let particles = count(o, "particles", 3.);
    let (ghost_r, ghost_a) = (o.get("ghostR", 0.9), o.get("ghostA", 0.5));
    let (part_r, part_r_depth) = (o.get("partR", 1.2), o.get("partRDepth", 1.6));
    let mut dots = Vec::new();
    for orb in 0..count(o, "orbitN", 12.) {
        let orb = orb as f64;
        let h1 = hash_d(orb, 1.7);
        let h2 = hash_d(orb, 5.2);
        let h3 = hash_d(orb, 8.9);
        let ro = big_r * (0.45 + 0.52 * h1);
        let th = h1 * 2. * PI;
        let phi = (2. * h2 - 1.).acos();
        let nx = phi.sin() * th.cos();
        let ny = phi.cos();
        let nz = phi.sin() * th.sin();
        let (mut ux, mut uy, uz) = (-ny, nx, 0.);
        let ul = (ux * ux + uy * uy).sqrt().max(1e-6);
        ux /= ul;
        uy /= ul;
        let vx = ny * uz - nz * uy;
        let vy = nz * ux - nx * uz;
        let vz = nx * uy - ny * ux;
        let speed = (0.25 + 0.55 * h3) * if h3 > 0.5 { 1. } else { -1. };
        let ring = |a: f64| {
            let (sa, ca) = a.sin_cos();
            pt.at(
                (ux * ca + vx * sa) * ro,
                (uy * ca + vy * sa) * ro,
                (uz * ca + vz * sa) * ro,
            )
        };
        for k in 0..ghost_n {
            let (px, py, z) = ring(k as f64 / ghost_n as f64 * 2. * PI);
            let depth = (z / ro + 1.) / 2.;
            dots.push(Dot {
                x: px,
                y: py,
                z,
                r: ghost_r * rs,
                white: 0.72,
                a: ghost_a * (0.4 + 0.6 * depth),
            });
        }
        for m in 0..particles {
            let a = t * speed + m as f64 / particles as f64 * 2. * PI + h2 * 6.;
            let (px, py, z) = ring(a);
            let depth = (z / ro + 1.) / 2.;
            dots.push(Dot {
                x: px,
                y: py,
                z,
                r: (part_r + part_r_depth * depth) * rs,
                white: 0.3 - 0.22 * depth,
                a: 1.,
            });
        }
    }
    finalize(dots, Vec::new(), o.get("rMin", 0.3))
}

/// `frameRibbon` (line 625) without the `faceOn` branch, which only `ring` sets.
pub(super) fn frame_ribbon(size: f64, t: f64, o: &Opts) -> Frame {
    let (cx, cy) = (size / 2., size / 2.);
    let big_r = size / 2. * 0.78;
    let spin = o.get("spin", 1.);
    let pt = Proj::new(t * 0.1 * spin, 0.3, cx, cy, 1.);
    let rs = radius_scale(size, o.get("rsPow", 0.6));
    let mut dots = Vec::new();
    let ghost_n = o.get("ghostN", 150.);
    for i in 0..count(o, "ghostN", 150.) {
        let d = fib_dir(i as f64, ghost_n);
        let (px, py, z) = pt.at(d[0] * big_r, d[1] * big_r, d[2] * big_r);
        let depth = (z / big_r + 1.) / 2.;
        dots.push(Dot {
            x: px,
            y: py,
            z,
            r: 0.8 * rs,
            white: 0.78,
            a: 0.1 + 0.22 * depth,
        });
    }
    let ya = t * 0.24 * spin;
    let ta = 0.55 + 0.3 * (t * 0.18).sin() * spin;
    let (ux, uy, uz) = (ya.cos(), 0., ya.sin());
    let vx = -uz * ta.sin();
    let vy = ta.cos();
    let vz = ux * ta.sin();
    let nx = uy * vz - uz * vy;
    let ny = uz * vx - ux * vz;
    let nz = ux * vy - uy * vx;
    let wob_mul = o.get("wobMul", 1.);
    let segs = count(o, "segs", 88.);
    let lanes = (js_round(o.get("lanes", 5.) * o.get("bandMul", 1.)) as usize).max(1);
    let (r_base, r_depth) = (o.get("rBase", 1.1), o.get("rDepth", 1.7));
    let middle = (lanes as f64 - 1.) / 2.;
    for w in 0..lanes {
        let w = w as f64;
        let lane_off = (w - middle) * 0.075;
        let edge = (w - middle).abs() / middle.max(1.);
        for k in 0..segs {
            let a = k as f64 / segs as f64 * 2. * PI;
            let wob = (0.16 * (a * 3. - t * 1.7 + w * 0.22).sin()
                + 0.07 * (a * 5. + t * 1.1).sin())
                * wob_mul;
            let off = lane_off + wob;
            let x = ux * a.cos() + vx * a.sin() + nx * off;
            let y = uy * a.cos() + vy * a.sin() + ny * off;
            let z = uz * a.cos() + vz * a.sin() + nz * off;
            let l = (x * x + y * y + z * z).sqrt();
            let (px, py, zr) = pt.at(x / l * big_r, y / l * big_r, z / l * big_r);
            let depth = (zr / big_r + 1.) / 2.;
            dots.push(Dot {
                x: px,
                y: py,
                z: zr,
                r: (r_base + r_depth * depth) * (1. - 0.25 * edge) * rs,
                white: 0.52 - 0.44 * depth + 0.18 * edge,
                a: 0.4 + 0.6 * depth,
            });
        }
    }
    finalize(dots, Vec::new(), o.get("rMin", 0.3))
}

/// `frameWeb` (line 684).
pub(super) fn frame_web(size: f64, t: f64, o: &Opts) -> Frame {
    let (cx, cy) = (size / 2., size / 2.);
    let big_r = size / 2. * 0.8 * o.get("spread", 1.);
    let pt = Proj::new(t * 0.12, 0.32, cx, cy, big_r);
    let rs = radius_scale(size, o.get("rsPow", 0.6));
    let node_count = count(o, "nodeN", 30.);
    let node_n = node_count as f64;
    let thr = o.get("thr", 0.72);
    let (node_r, node_r_depth) = (o.get("nodeR", 1.4), o.get("nodeRDepth", 1.8));
    let nodes: Vec<[f64; 3]> = (0..node_count)
        .map(|i| {
            let fi = i as f64;
            let d = fib_dir(fi, node_n);
            let x = d[0] + 0.3 * (vnoise(fi * 0.31 + 9., t * 0.24) - 0.5) * 2.;
            let y = d[1] + 0.3 * (vnoise(fi * 0.53 + 27., t * 0.21) - 0.5) * 2.;
            let z = d[2] + 0.3 * (vnoise(fi * 0.77 + 55., t * 0.27) - 0.5) * 2.;
            let l = (x * x + y * y + z * z).sqrt();
            [x / l, y / l, z / l]
        })
        .collect();
    let line_w = (o.get("lineW", 0.8) * rs).max(0.6);
    let mut lines = Vec::new();
    for i in 0..node_count {
        for j in i + 1..node_count {
            let [ax, ay, az] = nodes[i];
            let [bx, by, bz] = nodes[j];
            let dist = ((ax - bx).powi(2) + (ay - by).powi(2) + (az - bz).powi(2)).sqrt();
            if dist >= thr {
                continue;
            }
            let (x1, y1, z1) = pt.at(ax, ay, az);
            let (x2, y2, z2) = pt.at(bx, by, bz);
            let depth = ((z1 + z2) / 2. + 1.) / 2.;
            lines.push(Line {
                x1,
                y1,
                x2,
                y2,
                white: 0.42,
                a: (1. - dist / thr) * (0.3 + 0.55 * depth),
                w: line_w,
            });
        }
    }
    let mut dots = Vec::new();
    for (i, &[x, y, z]) in nodes.iter().enumerate() {
        let (px, py, z) = pt.at(x, y, z);
        let depth = (z + 1.) / 2.;
        let pulse = 1. + 0.25 * (t * 1.4 + i as f64 * 2.7).sin();
        dots.push(Dot {
            x: px,
            y: py,
            z,
            r: (node_r + node_r_depth * depth) * pulse * rs,
            white: 0.55 - 0.45 * depth,
            a: 1.,
        });
    }
    for s in 0..count(o, "signals", 5.) {
        let s = s as f64;
        let seg = (t * 0.55 + s * 7.31).floor();
        let a = (hash_d(seg, s * 3.1 + 1.7) * node_n).floor() as usize;
        let b = (hash_d(seg, s * 5.7 + 4.2) * node_n).floor() as usize;
        if a == b || a >= node_count || b >= node_count {
            continue;
        }
        let f = frac(t * 0.55 + s * 7.31);
        let x = lerp(nodes[a][0], nodes[b][0], f);
        let y = lerp(nodes[a][1], nodes[b][1], f);
        let z = lerp(nodes[a][2], nodes[b][2], f);
        let l = (x * x + y * y + z * z).sqrt().max(1e-6);
        let (px, py, zr) = pt.at(x / l, y / l, z / l);
        let depth = (zr + 1.) / 2.;
        dots.push(Dot {
            x: px,
            y: py,
            z: zr,
            r: (node_r * 1.5 + node_r_depth * depth) * rs,
            white: 0.05,
            a: 0.5 + 0.5 * depth,
        });
    }
    finalize(dots, lines, o.get("rMin", 0.3))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_helpers_match_the_engine() {
        assert_eq!(frac(-0.25), 0.75);
        assert_eq!(lerp(2., 4., 0.25), 2.5);
        // `hashD(0, 0)` is 0; `fibDir(0, 2)` sits on the upper half.
        assert_eq!(hash_d(0., 0.), 0.);
        let [x, y, z] = fib_dir(0., 2.);
        assert!((y - 0.5).abs() < 1e-12 && (x * x + y * y + z * z - 1.).abs() < 1e-12);
        assert!((angle_delta(0.1, 2. * PI) - 0.1).abs() < 1e-12);
        let v = vnoise(3.3, 7.9);
        assert!((0. ..=1.).contains(&v));
        assert_eq!(radius_scale(300., 0.6), 1.);
    }

    #[test]
    fn the_solve_cycle_turns_then_undoes_every_move_then_rests() {
        let start = solve_cycle(0., 3, 0.42, 1.2);
        assert_eq!(start.active, Some(0));
        assert_eq!(start.amount, vec![0., 0., 0.]);
        let turned = solve_cycle(2. * 0.42 + 0.3, 3, 0.42, 1.2);
        assert_eq!(turned.active, Some(2));
        assert_eq!(turned.amount, vec![1., 1., 1.]);
        let rest = solve_cycle(6. * 0.42 + 0.5, 3, 0.42, 1.2);
        assert_eq!(rest.active, None);
        assert_eq!(rest.amount, vec![0., 0., 0.]);
    }

    #[test]
    fn finalize_drops_faint_dots_and_lines_and_floors_the_radius() {
        let dot = |z: f64, r: f64, a: f64| Dot {
            x: 0.,
            y: 0.,
            z,
            r,
            white: 0.,
            a,
        };
        let line = |a: f64| Line {
            x1: 0.,
            y1: 0.,
            x2: 1.,
            y2: 1.,
            white: 0.,
            a,
            w: 1.,
        };
        let frame = finalize(
            vec![dot(0.5, 0.1, 1.), dot(-0.5, 2., 0.01), dot(-0.2, 1., 0.5)],
            vec![line(0.01), line(0.5)],
            0.3,
        );
        assert_eq!(frame.dots, vec![dot(-0.2, 1., 0.5), dot(0.5, 0.3, 1.)]);
        assert_eq!(frame.lines, vec![line(0.5)]);
    }
}

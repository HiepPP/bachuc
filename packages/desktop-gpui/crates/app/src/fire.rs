//! The flames on the dock icon while a thread waits for input, drawn in plain Rust one frame at
//! a time. The dock only shows a still snapshot of its tile, so Core Animation cannot run there.

/// Side of a frame in pixels. The dock scales it up; soft flames stay smooth.
pub const SIDE: usize = 96;

/// The flames at `time` seconds, as premultiplied RGBA rows, top row first.
pub fn frame(time: f32) -> Vec<u8> {
    let mut pixels = vec![0u8; SIDE * SIDE * 4];
    for row in 0..SIDE {
        let y = 1. - (row as f32 + 0.5) / SIDE as f32;
        for col in 0..SIDE {
            let x = (col as f32 + 0.5) / SIDE as f32;
            let [r, g, b, a] = color(heat(x, y, time));
            let at = (row * SIDE + col) * 4;
            pixels[at..at + 4].copy_from_slice(&[byte(r * a), byte(g * a), byte(b * a), byte(a)]);
        }
    }
    pixels
}

/// Above zero is flame. `y` is zero at the bottom edge.
fn heat(x: f32, y: f32, time: f32) -> f32 {
    // A slow noise bends the fast one, so the tongues sway.
    let sway = fbm(x * 2., y * 1.5 - time * 0.7) - 0.5;
    // Stretched up, so the noise makes tall thin tongues.
    let flicker = fbm(x * 6. + sway * 2., y * 2.5 - time * 2.2);
    // The flames climb higher at the sides, so they wrap the icon.
    let edge = ((x - 0.5).abs() * 2.).powi(4);
    // The icon art leaves a margin; the flames stay inside it.
    let inside = smoothstep((x - 0.06) / 0.08) * smoothstep((0.94 - x) / 0.08);
    (flicker - (y * 2.2 - 0.12 - edge * 0.6)) * inside
}

/// Dark red at the flame tips, through orange, to yellow at the core. The core stays a little
/// clear, so the icon shows through.
fn color(heat: f32) -> [f32; 4] {
    const STOPS: [[f32; 3]; 4] = [
        [0.6, 0.04, 0.],
        [0.95, 0.22, 0.],
        [1., 0.5, 0.04],
        [1., 0.82, 0.3],
    ];
    let level = (heat * 1.6).clamp(0., 1.);
    let alpha = smoothstep(level / 0.35) * 0.92;
    let scaled = level * (STOPS.len() - 1) as f32;
    let low = (scaled as usize).min(STOPS.len() - 2);
    let mix = scaled - low as f32;
    let [from, to] = [STOPS[low], STOPS[low + 1]];
    let channel = |i: usize| from[i] + (to[i] - from[i]) * mix;
    [channel(0), channel(1), channel(2), alpha]
}

/// Three octaves of value noise, from zero to one. A fourth adds detail too fine to see in
/// the dock.
fn fbm(x: f32, y: f32) -> f32 {
    let (mut sum, mut amplitude, mut frequency) = (0., 0.5, 1.);
    for _ in 0..3 {
        sum += amplitude * noise(x * frequency, y * frequency);
        amplitude *= 0.5;
        frequency *= 2.;
    }
    sum / 0.875
}

fn noise(x: f32, y: f32) -> f32 {
    let (cell_x, cell_y) = (x.floor(), y.floor());
    let (fx, fy) = (smoothstep(x - cell_x), smoothstep(y - cell_y));
    let (i, j) = (cell_x as i32, cell_y as i32);
    let top = lerp(hash(i, j), hash(i + 1, j), fx);
    let bottom = lerp(hash(i, j + 1), hash(i + 1, j + 1), fx);
    lerp(top, bottom, fy)
}

fn hash(x: i32, y: i32) -> f32 {
    let mut h = (x as u32).wrapping_mul(374_761_393) ^ (y as u32).wrapping_mul(668_265_263);
    h = (h ^ (h >> 13)).wrapping_mul(1_274_126_177);
    (h ^ (h >> 16)) as f32 / u32::MAX as f32
}

fn smoothstep(t: f32) -> f32 {
    let t = t.clamp(0., 1.);
    t * t * (3. - 2. * t)
}

fn lerp(from: f32, to: f32, t: f32) -> f32 {
    from + (to - from) * t
}

fn byte(value: f32) -> u8 {
    (value.clamp(0., 1.) * 255.).round() as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    fn alphas(pixels: &[u8], row: usize) -> Vec<u8> {
        (0..SIDE)
            .map(|col| pixels[(row * SIDE + col) * 4 + 3])
            .collect()
    }

    #[test]
    fn the_flames_fill_the_bottom_and_leave_the_top_clear() {
        let pixels = frame(1.5);
        let bottom = alphas(&pixels, SIDE - 1);
        let mean = bottom.iter().map(|&a| a as usize).sum::<usize>() / SIDE;
        assert!(mean > 150, "bottom alpha {mean}");
        assert!(alphas(&pixels, 0).iter().all(|&a| a == 0));
    }

    #[test]
    fn the_colors_are_premultiplied() {
        let pixels = frame(0.7);
        for px in pixels.chunks_exact(4) {
            assert!(px[0] <= px[3] && px[1] <= px[3] && px[2] <= px[3]);
        }
    }

    #[test]
    fn the_flames_move_over_time() {
        assert_eq!(frame(2.), frame(2.));
        assert_ne!(frame(2.), frame(2.1));
    }
}

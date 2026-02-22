use macroquad::prelude::*;

/// Converts a color from HSV to RGB.
/// 'h' -> hue in degrees [0,360], 's' -> saturation, and 'v' -> value.
/// Returns (r, g, b) in range [0,1].
fn hsv_to_rgb(h: f32, s: f32, v: f32) -> (f32, f32, f32) {
    let chroma = v * s;
    let x = chroma * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
    let m = v - chroma;

    let (r, g, b) = match h as i32 {
        0..=59 => (chroma, x, 0.0),
        60..=119 => (x, chroma, 0.0),
        120..=179 => (0.0, chroma, x),
        180..=239 => (0.0, x, chroma),
        240..=299 => (x, 0.0, chroma),
        _ => (chroma, 0.0, x),
    };

    (r + m, g + m, b + m)
}

pub fn mandelbrot_iter(c: (f64, f64), max_iter: u32) -> u32 {
    let mut z = (0.0_f64, 0.0_f64);
    let mut i = 0;

    while i < max_iter {
        // (a=bi)^2 = (a^2 - b^2) + (2ab)i
        let a = z.0;
        let b = z.1;
        let aa = a * a;
        let bb = b * b;

        if aa + bb > 4.0 {
            break;
        }

        z = (aa - bb + c.0, 2.0 * a * b + c.1);
        i += 1;
    }

    i
}

pub fn iter_to_color(iter: u32, max_iter: u32) -> Color {
    if iter >= max_iter {
        return BLACK;
    }

    let t = iter as f32 / max_iter as f32;
    let hue = (360.0 * (0.85 + 2.0 * t)) % 360.0;
    let sat = 0.85;
    let val = 1.0;

    let (r, g, b) = hsv_to_rgb(hue, sat, val);
    Color::new(r, g, b, 1.0)
}

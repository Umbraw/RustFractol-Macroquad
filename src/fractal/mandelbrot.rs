use macroquad::prelude::*;

fn hsv_to_rgb(h: f32, s: f32, v: f32) -> (f32, f32, f32) {
    let c = v * s;
    let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
    let m = v - c;

    let (r, g, b) = match h as i32 {
        0..=59 => (c, x, 0.0),
        60..=119 => (x, c, 0.0),
        120..=179 => (0.0, c, x),
        180..=239 => (0.0, x, c),
        240..=299 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };

    (r + m, g + m, b + m)
}

pub fn mandelbrot_iter(c: Vec2, max_iter: u32) -> u32 {
    let mut z = vec2(0.0, 0.0);
    let mut i = 0;

    while i < max_iter {
        // (a=bi)^2 = (a^2 - b^2) + (2ab)i
        let a = z.x;
        let b = z.y;
        let aa = a * b;
        let bb = b * b;

        if aa + bb > 4.0 {
            break;
        }

        z = vec2(aa - bb + c.x, 2.0 * a * b + c.y);
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
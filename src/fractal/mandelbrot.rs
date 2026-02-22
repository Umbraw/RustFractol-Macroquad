use macroquad::prelude::*;
use rug::Float;

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

pub struct PerturbationRef {
    pub center: (f64, f64),
    pub orbit: Vec<(f64, f64)>,
    pub escaped_iter: u32,
}

pub fn precision_bits_for_scale(scale: f64) -> u32 {
    let s = scale.abs().max(1e-300);
    let digits = (-s.log10()).max(0.0) + 10.0;
    let bits = (digits * 3.321_928_094_89 + 32.0).round() as u32;
    bits.clamp(128, 1024)
}

pub fn build_reference(center: (f64, f64), max_iter: u32, prec_bits: u32) -> PerturbationRef {
    let mut orbit: Vec<(f64, f64)> = Vec::with_capacity(max_iter as usize);

    let mut zr = Float::with_val(prec_bits, 0.0);
    let mut zi = Float::with_val(prec_bits, 0.0);
    let cr = Float::with_val(prec_bits, center.0);
    let ci = Float::with_val(prec_bits, center.1);

    let mut escaped_iter = 0;
    for i in 0..max_iter {
        let aa = Float::with_val(prec_bits, &zr * &zr);
        let bb = Float::with_val(prec_bits, &zi * &zi);
        let mag2 = Float::with_val(prec_bits, &aa + &bb);

        if mag2 > 4.0 {
            escaped_iter = i;
            break;
        }

        let zrzi = Float::with_val(prec_bits, &zr * &zi);
        let mut new_zr = Float::with_val(prec_bits, &aa);
        new_zr -= &bb;
        new_zr += &cr;
        let mut new_zi = Float::with_val(prec_bits, zrzi * 2.0);
        new_zi += &ci;

        zr = new_zr;
        zi = new_zi;

        orbit.push((zr.to_f64(), zi.to_f64()));
    }

    while orbit.len() < max_iter as usize {
        orbit.push((0.0, 0.0));
    }

    PerturbationRef {
        center,
        orbit,
        escaped_iter,
    }
}

pub fn perturbation_iter(dc: (f64, f64), pref: &PerturbationRef, max_iter: u32) -> u32 {
    if pref.escaped_iter > 0 && pref.escaped_iter < max_iter {
        let c = (pref.center.0 + dc.0, pref.center.1 + dc.1);
        return mandelbrot_iter(c, max_iter);
    }

    let mut dzr = 0.0_f64;
    let mut dzi = 0.0_f64;

    for i in 0..max_iter {
        let (zr, zi) = pref.orbit[i as usize];
        let zcr = zr + dzr;
        let zci = zi + dzi;

        if zcr * zcr + zci * zci > 4.0 {
            return i;
        }

        let two_zr = 2.0 * zr;
        let two_zi = 2.0 * zi;

        let dzr2 = dzr * dzr - dzi * dzi;
        let dzi2 = 2.0 * dzr * dzi;

        let new_dzr = two_zr * dzr - two_zi * dzi + dzr2 + dc.0;
        let new_dzi = two_zr * dzi + two_zi * dzr + dzi2 + dc.1;

        dzr = new_dzr;
        dzi = new_dzi;
    }

    max_iter
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

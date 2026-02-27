use macroquad::prelude::*;
use rayon::prelude::*;
use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::fractal;

#[derive(Clone, Copy)]
struct View {
    center: (f64, f64),
    scale: f64,
    max_iter: u32,
}

pub struct App {
    frames: u64,
    tex: Texture2D,
    minimap_tex: Texture2D,
    minimap_w: u16,
    minimap_h: u16,
    render_w: u16,
    render_h: u16,
    view: View,
    palette: u8,
    dirty: bool,
    render_job: Option<RenderJob>,
    last_mouse: Vec2,
    screenshot_requested: bool,
    palette_select_mode: bool,

    preview_w: u16,
    preview_h: u16,
    hq_w: u16,
    hq_h: u16,
    idle_frames: u32,
    next_preview_render_time: f64,
}

struct RenderJob {
    w: u16,
    h: u16,
    view: View,
    palette: u8,
    image: Image,
    upload_image: Image,
    next_row: u16,
    pass_stage: usize,
    pass_offset_idx: usize,
    x_coords: Vec<f64>,
    ref_orbit: Option<fractal::mandelbrot::PerturbationRef>,
    sh: f64,
}

impl RenderJob {
    fn stage_steps() -> [(u16, &'static [u16]); 3] {
        [
            (4, &[0, 2, 1, 3]),
            (2, &[0, 1]),
            (1, &[0]),
        ]
    }

    fn new(w: u16, h: u16, view: View, palette: u8, image: Image) -> Self {
        let sw = w as f64;
        let sh = h as f64;
        let aspect = sw / sh;

        let mut x_coords = Vec::with_capacity(w as usize);
        for x in 0..w {
            let nx = (x as f64 + 0.5) / sw - 0.5;
            x_coords.push(view.center.0 + nx * view.scale * aspect);
        }

        let ref_orbit = if view.scale < 1e-6 {
            let bits = fractal::mandelbrot::precision_bits_for_scale(view.scale);
            Some(fractal::mandelbrot::build_reference(
                view.center,
                view.max_iter,
                bits,
            ))
        } else {
            None
        };

        Self {
            w,
            h,
            view,
            palette,
            image,
            upload_image: Image::empty(),
            next_row: 0,
            pass_stage: 0,
            pass_offset_idx: 0,
            x_coords,
            ref_orbit,
            sh,
        }
    }

    fn render_rows_parallel(&mut self, rows: &[u16]) {
        let w = self.w as usize;
        let max_iter = self.view.max_iter;
        let center_y = self.view.center.1;
        let scale = self.view.scale;
        let sh = self.sh;
        let x_coords = &self.x_coords;
        let center = self.view.center;
        let ref_orbit = self.ref_orbit.as_ref();
        let palette = self.palette;

        let row_bytes: Vec<(u16, Vec<u8>)> = rows
            .par_iter()
            .map(|&y| {
                let ny = (y as f64 + 0.5) / sh - 0.5;
                let im = center_y + ny * scale;
                let mut buf = vec![0u8; w * 4];
                for x in 0..w {
                    let re = x_coords[x];
                    let it = if let Some(pref) = ref_orbit {
                        let dc = (re - center.0, im - center.1);
                        fractal::mandelbrot::perturbation_iter(dc, pref, max_iter)
                    } else {
                        fractal::mandelbrot::mandelbrot_iter((re, im), max_iter)
                    };
                    let col = fractal::mandelbrot::iter_to_color(it, max_iter, palette);
                    let rgba = color_to_rgba8(col);
                    let idx = x * 4;
                    buf[idx] = rgba[0];
                    buf[idx + 1] = rgba[1];
                    buf[idx + 2] = rgba[2];
                    buf[idx + 3] = rgba[3];
                }
                (y, buf)
            })
            .collect();

        for (y, buf) in row_bytes {
            let row_start = y as usize * w * 4;
            self.image.bytes[row_start..row_start + w * 4].copy_from_slice(&buf);
        }
    }

    fn current_step_and_offset(&self) -> (u16, u16) {
        let stages = Self::stage_steps();
        let (step, offsets) = stages[self.pass_stage];
        let offset = offsets[self.pass_offset_idx];
        (step, offset)
    }

    fn advance_row_cursor(&mut self) {
        let stages = Self::stage_steps();
        let (step, offsets) = stages[self.pass_stage];

        self.next_row = self.next_row.saturating_add(step);
        if self.next_row >= self.h {
            self.pass_offset_idx += 1;
            if self.pass_offset_idx >= offsets.len() {
                self.pass_stage += 1;
                self.pass_offset_idx = 0;
                if self.pass_stage >= stages.len() {
                    return;
                }
            }
            let (_, new_offset) = self.current_step_and_offset();
            self.next_row = new_offset;
        }
    }

    fn step(&mut self, tex: &Texture2D, time_budget: f64) -> bool {
        let start = get_time();

        while self.pass_stage < Self::stage_steps().len() && (get_time() - start) < time_budget {
            let mut rows: Vec<u16> = Vec::new();
            let mut rows_budget: usize = if self.w >= 1024 { 6 } else { 10 };

            while rows_budget > 0 && self.pass_stage < Self::stage_steps().len() {
                if self.next_row >= self.h {
                    self.advance_row_cursor();
                    continue;
                }
                rows.push(self.next_row);
                rows_budget -= 1;
                self.advance_row_cursor();
            }

            if rows.is_empty() {
                break;
            }

            self.render_rows_parallel(&rows);

            let needed = self.w as usize * 4;
            if self.upload_image.bytes.len() != needed {
                self.upload_image.bytes.resize(needed, 0);
            }
            self.upload_image.width = self.w;
            self.upload_image.height = 1;

            for y in rows {
                let row_start = y as usize * self.w as usize * 4;
                let row_end = row_start + needed;
                self.upload_image.bytes[..needed]
                    .copy_from_slice(&self.image.bytes[row_start..row_end]);
                tex.update_part(&self.upload_image, 0, y as i32, self.w as i32, 1);
            }
        }

        self.pass_stage >= Self::stage_steps().len()
    }
}

impl App {
    pub fn new() -> Self {
        let preview_w: u16 = 480;
        let preview_h: u16 = 270;
        let hq_w: u16 = 1280;
        let hq_h: u16 = 720;

        // Start in preview for responsiveness
        let render_w = preview_w;
        let render_h = preview_h;

        let view = View {
            center: (-0.5, 0.0),
            scale: 3.0,
            max_iter: 200,
        };

        let image = Image::gen_image_color(render_w, render_h, BLACK);
        let tex = Texture2D::from_image(&image);
        tex.set_filter(FilterMode::Linear);

        let minimap_w: u16 = 160;
        let minimap_h: u16 = 160;
        let minimap_view = View {
            center: (-0.5, 0.0),
            scale: 3.0,
            max_iter: 200,
        };
        let minimap_img = render_mandelbrot_image(minimap_w, minimap_h, minimap_view, 0);
        let minimap_tex = Texture2D::from_image(&minimap_img);
        minimap_tex.set_filter(FilterMode::Linear);

        Self {
            frames: 0,
            tex,
            minimap_tex,
            minimap_w,
            minimap_h,
            render_w,
            render_h,
            view,
            palette: 0,
            dirty: true,
            render_job: None,
            last_mouse: vec2(0.0, 0.0),
            screenshot_requested: false,
            palette_select_mode: false,

            preview_w,
            preview_h,
            hq_w,
            hq_h,
            idle_frames: 0,
            next_preview_render_time: 0.0,
        }
    }

    fn is_preview(&self) -> bool {
        self.render_w == self.preview_w && self.render_h == self.preview_h
    }

    fn start_render_job(&mut self) {
        let tex_w = self.tex.width() as u16;
        let tex_h = self.tex.height() as u16;
        if tex_w != self.render_w || tex_h != self.render_h {
            let image = Image::gen_image_color(self.render_w, self.render_h, BLACK);
            self.tex = Texture2D::from_image(&image);
            self.tex.set_filter(FilterMode::Linear);
        }

        let image = Image::gen_image_color(self.render_w, self.render_h, BLACK);
        let job_view = self.effective_view(self.is_preview());
        self.render_job = Some(RenderJob::new(
            self.render_w,
            self.render_h,
            job_view,
            self.palette,
            image,
        ));
    }

    pub fn update(&mut self) {
        let mut moved = false;
        self.frames += 1;

        // Keep render resolution in sync with window size (for crisp full-screen output)
        let sw = screen_width().round().max(1.0) as u16;
        let sh = screen_height().round().max(1.0) as u16;
        if sw != self.hq_w || sh != self.hq_h {
            self.hq_w = sw;
            self.hq_h = sh;
            if !self.is_preview() && (self.render_w != sw || self.render_h != sh) {
                self.render_w = sw;
                self.render_h = sh;
                self.dirty = true;
                self.render_job = None;
            }
        }

        let preview_scale = 0.35_f32;
        let pw = (screen_width() * preview_scale).round().max(200.0) as u16;
        let ph = (screen_height() * preview_scale).round().max(112.0) as u16;
        if pw != self.preview_w || ph != self.preview_h {
            self.preview_w = pw;
            self.preview_h = ph;
            if self.is_preview() && (self.render_w != pw || self.render_h != ph) {
                self.render_w = pw;
                self.render_h = ph;
                self.dirty = true;
                self.render_job = None;
            }
        }

        let mouse = vec2(mouse_position().0, mouse_position().1);
        let mouse_delta = mouse - self.last_mouse;
        self.last_mouse = mouse;

        let (_wx, wy) = mouse_wheel();
        if wy.abs() > 0.0 {
            let factor = if wy > 0.0 { 0.85 } else { 1.0 / 0.85 };
            zoom_at_mouse(&mut self.view, factor);
            moved = true;
        }

        if is_mouse_button_down(MouseButton::Left) {
            pan_with_mouse(&mut self.view, mouse_delta);
            moved = true;
        }

        if is_key_pressed(KeyCode::Up) {
            self.view.max_iter = (self.view.max_iter + 50).min(10_000);
            moved = true;
        }
        if is_key_pressed(KeyCode::Down) {
            self.view.max_iter = self.view.max_iter.saturating_sub(50).max(50);
            moved = true;
        }

        if is_key_pressed(KeyCode::R) {
            self.view = View {
                center: (-0.5, 0.0),
                scale: 3.0,
                max_iter: 200,
            };
            moved = true;
        }

        if is_key_pressed(KeyCode::P) {
            self.palette_select_mode = true;
        }

        if self.palette_select_mode {
            let mut selected: Option<u8> = None;
            if is_key_pressed(KeyCode::Key1) {
                selected = Some(0);
            } else if is_key_pressed(KeyCode::Key2) {
                selected = Some(1);
            } else if is_key_pressed(KeyCode::Key3) {
                selected = Some(2);
            } else if is_key_pressed(KeyCode::Key4) {
                selected = Some(3);
            } else if is_key_pressed(KeyCode::Key5) {
                selected = Some(4);
            } else if is_key_pressed(KeyCode::Key6) {
                selected = Some(5);
            } else if is_key_pressed(KeyCode::Escape) {
                self.palette_select_mode = false;
            }

            if let Some(p) = selected {
                self.palette = p;
                self.dirty = true;
                self.render_job = None;
                self.palette_select_mode = false;
            }
        }

        if is_key_pressed(KeyCode::S) {
            self.screenshot_requested = true;
        }

        // --- Preview while moving, HQ after short idle ---
        if moved {
            self.idle_frames = 0;
            self.dirty = true;
            self.render_job = None;

            // If we were in HQ, drop back to preview to stay responsive
            if self.render_w != self.preview_w || self.render_h != self.preview_h {
                self.render_w = self.preview_w;
                self.render_h = self.preview_h;
            }
        } else {
            self.idle_frames += 1;
        }

        // Switch to HQ once after ~100ms of inactivity
        let idle_threshold: u32 = 6; // ~6 frames @ 60fps ~ 100ms
        if !moved && self.idle_frames == idle_threshold {
            if self.render_w != self.hq_w || self.render_h != self.hq_h {
                self.render_w = self.hq_w;
                self.render_h = self.hq_h;
                self.dirty = true;
                self.render_job = None;
            }
        }

        let now = get_time();
        let is_preview = self.is_preview();

        // In preview, limit to ~30 renders/sec to avoid bursts on fast scrolling.
        let preview_interval = 1.0 / 30.0;
        let can_start_render = if is_preview {
            if now >= self.next_preview_render_time {
                self.next_preview_render_time = now + preview_interval;
                true
            } else {
                false
            }
        } else {
            true
        };

        if self.dirty && self.render_job.is_none() && can_start_render {
            self.start_render_job();
        }

        if let Some(job) = &mut self.render_job {
            let frame_time = get_frame_time() as f64;
            let base_budget = if is_preview { 0.004 } else { 0.012 };
            let time_budget = (frame_time * 0.35).clamp(0.002, base_budget);
            let done = job.step(&self.tex, time_budget);
            if done {
                self.render_job = None;
                self.dirty = false;
            }
        }
    }

    pub fn draw(&mut self) {
        clear_background(BLACK);

        let sw = screen_width();
        let sh = screen_height();

        draw_texture_ex(
            &self.tex,
            0.0,
            0.0,
            WHITE,
            DrawTextureParams {
                dest_size: Some(vec2(sw, sh)),
                ..Default::default()
            },
        );

        // If a screenshot is requested, capture only the fractal (no UI).
        if self.screenshot_requested {
            let img = get_screen_data();
            save_screenshot_image(&img);
            self.screenshot_requested = false;
            return;
        }

        // HUD
        let hud_x = 16.0;
        let hud_y = 16.0;
        let hud_w = 360.0;
        let hud_h = 124.0;
        draw_hud_card(hud_x, hud_y, hud_w, hud_h);

        let title_size = 24.0;
        draw_text_shadow(
            "Fractol - Mandelbrot",
            hud_x + 14.0,
            hud_y + 30.0,
            title_size,
            WHITE,
        );

        draw_line(
            hud_x + 14.0,
            hud_y + 38.0,
            hud_x + hud_w - 14.0,
            hud_y + 38.0,
            1.0,
            Color::new(1.0, 1.0, 1.0, 0.12),
        );

        let zoom = 3.0_f64 / self.view.scale.max(1e-300);
        let line1 = format!(
            "Zoom x{}   Iter {}",
            fmt_zoom(zoom),
            self.view.max_iter
        );
        draw_text(&line1, hud_x + 14.0, hud_y + 64.0, 18.0, GRAY);

        let line2 = format!(
            "Center  {:.6}, {:.6}",
            self.view.center.0,
            self.view.center.1
        );
        draw_text(&line2, hud_x + 14.0, hud_y + 86.0, 16.0, Color::new(1.0, 1.0, 1.0, 0.75));

        let line3 = "S: screenshot (no UI)";
        draw_text(&line3, hud_x + 14.0, hud_y + 104.0, 14.0, Color::new(1.0, 1.0, 1.0, 0.60));

        let hint = "Wheel: zoom   LMB drag: pan   Up/Down: iterations   R: reset   P: palette   S: screenshot";
        draw_text(
            hint,
            16.0,
            sh - 18.0,
            16.0,
            Color::new(1.0, 1.0, 1.0, 0.65),
        );

        if self.palette_select_mode {
            let msg = "Select palette: 1 / 2 / 3 / 4 / 5 / 6   (Esc to cancel)";
            draw_text(
                msg,
                16.0,
                sh - 40.0,
                16.0,
                Color::new(1.0, 1.0, 1.0, 0.85),
            );
        }

        draw_minimap(
            &self.minimap_tex,
            self.minimap_w,
            self.minimap_h,
            sw,
            sh,
            self.view,
        );

        // Status pill
        let mode = if self.is_preview() { "PREVIEW" } else { "HQ" };
        let pill_w = 96.0;
        let pill_h = 26.0;
        let pill_x = sw - pill_w - 16.0;
        let pill_y = 16.0;
        draw_rectangle(pill_x, pill_y, pill_w, pill_h, Color::new(0.0, 0.0, 0.0, 0.55));
        draw_rectangle_lines(
            pill_x,
            pill_y,
            pill_w,
            pill_h,
            1.0,
            Color::new(1.0, 1.0, 1.0, 0.15),
        );
        draw_text(
            mode,
            pill_x + 12.0,
            pill_y + 18.0,
            16.0,
            Color::new(1.0, 1.0, 1.0, 0.85),
        );

    }

    fn effective_view(&self, is_preview: bool) -> View {
        if is_preview {
            View {
                max_iter: preview_max_iter(self.view.max_iter),
                ..self.view
            }
        } else {
            self.view
        }
    }
}

fn screen_to_complex(p: Vec2, view: View, screen_w: f32, screen_h: f32) -> (f64, f64) {
    let aspect = screen_w as f64 / screen_h as f64;

    let x = (p.x as f64 / screen_w as f64 - 0.5) * view.scale * aspect;
    let y = (p.y as f64 / screen_h as f64 - 0.5) * view.scale;

    (view.center.0 + x, view.center.1 + y)
}

fn zoom_at_mouse(view: &mut View, zoom_factor: f32) {
    let (sw, sh) = (screen_width(), screen_height());
    let mouse = vec2(mouse_position().0, mouse_position().1);

    let before = screen_to_complex(mouse, *view, sw, sh);
    view.scale *= zoom_factor as f64;
    let after = screen_to_complex(mouse, *view, sw, sh);

    view.center.0 += before.0 - after.0;
    view.center.1 += before.1 - after.1;
}

fn pan_with_mouse(view: &mut View, delta: Vec2) {
    let (sw, sh) = (screen_width(), screen_height());
    let aspect = sw as f64 / sh as f64;

    let dx = -(delta.x as f64) / sw as f64 * view.scale * aspect;
    let dy = -(delta.y as f64) / sh as f64 * view.scale;

    view.center.0 += dx;
    view.center.1 += dy;
}

fn draw_hud_card(x: f32, y: f32, w: f32, h: f32) {
    draw_rectangle(x, y, w, h, Color::new(0.0, 0.0, 0.0, 0.60));
    draw_rectangle_lines(x, y, w, h, 1.0, Color::new(1.0, 1.0, 1.0, 0.12));
}

fn fmt_zoom(v: f64) -> String {
    if v >= 1e6 {
        format!("{:.2e}", v)
    } else if v >= 10.0 {
        format!("{:.1}", v)
    } else {
        format!("{:.2}", v)
    }
}

fn draw_text_shadow(text: &str, x: f32, y: f32, size: f32, color: Color) {
    draw_text(text, x + 1.0, y + 1.0, size, Color::new(0.0, 0.0, 0.0, 0.6));
    draw_text(text, x, y, size, color);
}

fn render_mandelbrot_image(w: u16, h: u16, view: View, palette: u8) -> Image {
    let mut img = Image::gen_image_color(w, h, BLACK);
    let sw = w as f64;
    let sh = h as f64;
    let aspect = sw / sh;

    for y in 0..h {
        let ny = (y as f64 + 0.5) / sh - 0.5;
        let im = view.center.1 + ny * view.scale;
        for x in 0..w {
            let nx = (x as f64 + 0.5) / sw - 0.5;
            let re = view.center.0 + nx * view.scale * aspect;
            let it = fractal::mandelbrot::mandelbrot_iter((re, im), view.max_iter);
            let col = fractal::mandelbrot::iter_to_color(it, view.max_iter, palette);
            img.set_pixel(x as u32, y as u32, col);
        }
    }

    img
}

fn draw_minimap(
    tex: &Texture2D,
    mw: u16,
    mh: u16,
    sw: f32,
    sh: f32,
    view: View,
) {
    let pad = 12.0;
    let mm_w = mw as f32;
    let mm_h = mh as f32;
    let x = sw - mm_w - pad;
    let y = sh - mm_h - pad - 24.0;

    draw_rectangle(
        x - 6.0,
        y - 6.0,
        mm_w + 12.0,
        mm_h + 12.0,
        Color::new(0.0, 0.0, 0.0, 0.40),
    );
    draw_rectangle_lines(
        x - 6.0,
        y - 6.0,
        mm_w + 12.0,
        mm_h + 12.0,
        1.0,
        Color::new(1.0, 1.0, 1.0, 0.08),
    );

    draw_texture_ex(
        tex,
        x,
        y,
        WHITE,
        DrawTextureParams {
            dest_size: Some(vec2(mm_w, mm_h)),
            ..Default::default()
        },
    );

    let base_center = (-0.5_f64, 0.0_f64);
    let base_scale = 3.0_f64;
    let base_aspect = 1.0_f64;
    let base_w = base_scale * base_aspect;
    let base_h = base_scale;
    let base_left = base_center.0 - base_w * 0.5;
    let base_top = base_center.1 - base_h * 0.5;

    let screen_aspect = sw as f64 / sh as f64;
    let view_w = view.scale * screen_aspect;
    let view_h = view.scale;
    let view_left = view.center.0 - view_w * 0.5;
    let view_top = view.center.1 - view_h * 0.5;

    let rx = ((view_left - base_left) / base_w * mm_w as f64) as f32;
    let ry = ((view_top - base_top) / base_h * mm_h as f64) as f32;
    let mut rw = (view_w / base_w * mm_w as f64) as f32;
    let mut rh = (view_h / base_h * mm_h as f64) as f32;

    let mut rx = rx;
    let mut ry = ry;
    if rw < 2.0 {
        rw = 2.0;
    }
    if rh < 2.0 {
        rh = 2.0;
    }
    if rw > mm_w {
        rw = mm_w;
        rx = 0.0;
    } else {
        rx = rx.clamp(0.0, mm_w - rw);
    }
    if rh > mm_h {
        rh = mm_h;
        ry = 0.0;
    } else {
        ry = ry.clamp(0.0, mm_h - rh);
    }

    draw_rectangle_lines(
        x + rx,
        y + ry,
        rw,
        rh,
        2.0,
        Color::new(1.0, 1.0, 1.0, 0.85),
    );
}

fn save_screenshot_image(img: &Image) {
    let dir = "screenshots";
    if fs::create_dir_all(dir).is_err() {
        return;
    }

    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let path = format!("{}/shot_{}.png", dir, ts);
    img.export_png(&path);
}

fn preview_max_iter(max_iter: u32) -> u32 {
    if max_iter <= 50 {
        return max_iter;
    }
    let reduced = (max_iter as f32 * 0.35).round() as u32;
    reduced.clamp(50, max_iter)
}

fn color_to_rgba8(c: Color) -> [u8; 4] {
    let r = (c.r * 255.0).clamp(0.0, 255.0) as u8;
    let g = (c.g * 255.0).clamp(0.0, 255.0) as u8;
    let b = (c.b * 255.0).clamp(0.0, 255.0) as u8;
    let a = (c.a * 255.0).clamp(0.0, 255.0) as u8;
    [r, g, b, a]
}

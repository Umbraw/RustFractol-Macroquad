use macroquad::prelude::*;
use rayon::prelude::*;

use crate::fractal;

#[derive(Clone, Copy)]
struct View {
    center: Vec2,
    scale: f32,
    max_iter: u32,
}

pub struct App {
    frames: u64,
    tex: Texture2D,
    render_w: u16,
    render_h: u16,
    view: View,
    dirty: bool,
    render_job: Option<RenderJob>,
    last_mouse: Vec2,

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
    image: Image,
    upload_image: Image,
    next_row: u16,
    pass_stage: usize,
    pass_offset_idx: usize,
    x_coords: Vec<f32>,
    sh: f32,
}

impl RenderJob {
    fn stage_steps() -> [(u16, &'static [u16]); 3] {
        [
            (4, &[0, 2, 1, 3]),
            (2, &[0, 1]),
            (1, &[0]),
        ]
    }

    fn new(w: u16, h: u16, view: View, image: Image) -> Self {
        let sw = w as f32;
        let sh = h as f32;
        let aspect = sw / sh;

        let mut x_coords = Vec::with_capacity(w as usize);
        for x in 0..w {
            let nx = (x as f32 + 0.5) / sw - 0.5;
            x_coords.push(view.center.x + nx * view.scale * aspect);
        }

        Self {
            w,
            h,
            view,
            image,
            upload_image: Image::empty(),
            next_row: 0,
            pass_stage: 0,
            pass_offset_idx: 0,
            x_coords,
            sh,
        }
    }

    fn render_rows_parallel(&mut self, rows: &[u16]) {
        let w = self.w as usize;
        let max_iter = self.view.max_iter;
        let center_y = self.view.center.y;
        let scale = self.view.scale;
        let sh = self.sh;
        let x_coords = &self.x_coords;

        let row_bytes: Vec<(u16, Vec<u8>)> = rows
            .par_iter()
            .map(|&y| {
                let ny = (y as f32 + 0.5) / sh - 0.5;
                let im = center_y + ny * scale;
                let mut buf = vec![0u8; w * 4];
                for x in 0..w {
                    let re = x_coords[x];
                    let c = vec2(re, im);
                    let it = fractal::mandelbrot::mandelbrot_iter(c, max_iter);
                    let col = fractal::mandelbrot::iter_to_color(it, max_iter);
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
            center: vec2(-0.5, 0.0),
            scale: 3.0,
            max_iter: 200,
        };

        let image = Image::gen_image_color(render_w, render_h, BLACK);
        let tex = Texture2D::from_image(&image);
        tex.set_filter(FilterMode::Linear);

        Self {
            frames: 0,
            tex,
            render_w,
            render_h,
            view,
            dirty: true,
            render_job: None,
            last_mouse: vec2(0.0, 0.0),

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
        self.render_job = Some(RenderJob::new(self.render_w, self.render_h, job_view, image));
    }

    pub fn update(&mut self) {
        let mut moved = false;
        self.frames += 1;

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
                center: vec2(-0.5, 0.0),
                scale: 3.0,
                max_iter: 200,
            };
            moved = true;
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
        let idle_threshold: u32 = 6; // ~6 frames @ 60fps ≈ 100ms
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

    pub fn draw(&self) {
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

        // HUD
        let hud_x = 14.0;
        let hud_y = 14.0;
        let hud_w = 520.0;
        let hud_h = 86.0;

        draw_hud_card(hud_x, hud_y, hud_w, hud_h);

        let title_size = 22.0;
        draw_text("Fractol - Mandelbrot", hud_x + 12.0, hud_y + 28.0, title_size, WHITE);

        draw_line(
            hud_x + 12.0,
            hud_y + 36.0,
            hud_x + hud_w - 12.0,
            hud_y + 36.0,
            1.0,
            Color::new(1.0, 1.0, 1.0, 0.10),
        );

        let text_size = 18.0;
        let line1 = format!(
            "res {}x{}  -  iter {}  -  scale {}",
            self.render_w,
            self.render_h,
            self.view.max_iter,
            fmt_f32(self.view.scale, 6),
        );
        draw_text(&line1, hud_x + 12.0, hud_y + 58.0, text_size, GRAY);

        let line2 = "Wheel: zoom  -  LMB drag: pan  -  Up/Down: iterations  -  R: reset";
        draw_text(
            line2,
            hud_x + 12.0,
            hud_y + 78.0,
            16.0,
            Color::new(1.0, 1.0, 1.0, 0.70),
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

fn screen_to_complex(p: Vec2, view: View, screen_w: f32, screen_h: f32) -> Vec2 {
    let aspect = screen_w / screen_h;

    let x = (p.x / screen_w - 0.5) * view.scale * aspect;
    let y = (p.y / screen_h - 0.5) * view.scale;

    view.center + vec2(x, y)
}

fn zoom_at_mouse(view: &mut View, zoom_factor: f32) {
    let (sw, sh) = (screen_width(), screen_height());
    let mouse = vec2(mouse_position().0, mouse_position().1);

    let before = screen_to_complex(mouse, *view, sw, sh);
    view.scale *= zoom_factor;
    let after = screen_to_complex(mouse, *view, sw, sh);

    view.center += before - after;
}

fn pan_with_mouse(view: &mut View, delta: Vec2) {
    let (sw, sh) = (screen_width(), screen_height());
    let aspect = sw / sh;

    let dx = -delta.x / sw * view.scale * aspect;
    let dy = -delta.y / sh * view.scale;

    view.center += vec2(dx, dy);
}

fn draw_hud_card(x: f32, y: f32, w: f32, h: f32) {
    draw_rectangle(x, y, w, h, Color::new(0.0, 0.0, 0.0, 0.60));
    draw_rectangle_lines(x, y, w, h, 1.0, Color::new(1.0, 1.0, 1.0, 0.12));
}

fn fmt_f32(v: f32, digits: usize) -> String {
    format!("{:.*}", digits, v)
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

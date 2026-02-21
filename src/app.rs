use macroquad::prelude::*;

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
    last_mouse: Vec2,
}

impl App {
    pub fn new() -> Self {
        let render_w: u16 = 480;
        let render_h: u16 = 270;

        let view = View {
            center: vec2(-0.5, 0.0),
            scale: 3.0,
            max_iter: 200,
        };

        let image = render_mandelbrot(render_w, render_h, view);
        let tex = Texture2D::from_image(&image);
        tex.set_filter(FilterMode::Nearest);

        Self {
            frames: 0,
            tex,
            render_w,
            render_h,
            view,
            dirty: false,
            last_mouse: vec2(0.0, 0.0),
        }
    }

    pub fn update(&mut self) {
        self.frames += 1;
        let mouse = vec2(mouse_position().0, mouse_position().1);
        let mouse_delta = mouse - self.last_mouse;
        self.last_mouse = mouse;

        let (_wx, wy) = mouse_wheel();
        if wy.abs() > 0.0 {
            let factor = if wy > 0.0 { 0.85 } else { 1.0 / 0.85 };
            zoom_at_mouse(&mut self.view, factor);
            self.dirty = true;
        }

        if is_mouse_button_down(MouseButton::Left) {
            pan_with_mouse(&mut self.view, mouse_delta);
            self.dirty = true;
        }

        if is_key_pressed(KeyCode::Up) {
            self.view.max_iter = (self.view.max_iter + 50).min(10_000);
            self.dirty = true;
        }
        if is_key_pressed(KeyCode::Down) {
            self.view.max_iter = self.view.max_iter.saturating_sub(50).max(50);
            self.dirty = true;
        }

        if is_key_pressed(KeyCode::R) {
            self.view = View {
                center: vec2(-0.5, 0.0),
                scale: 3.0,
                max_iter: 200,
            };
            self.dirty = true;
        }

        if self.dirty {
            let image = render_mandelbrot(self.render_w, self.render_h, self.view);
            self.tex = Texture2D::from_image(&image);
            self.tex.set_filter(FilterMode::Nearest);
            self.dirty = false;
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

        let line2 = "Wheel: zoom  -  LMB drag: pan  -  up/down: iterations  -  R: reset";
        draw_text(line2, hud_x + 12.0, hud_y + 78.0, 16.0, Color::new(1.0, 1.0, 1.0, 0.70));
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

fn render_mandelbrot(w: u16, h: u16, view: View) -> Image {
    let mut img = Image::gen_image_color(w, h, BLACK);

    let center = view.center;
    let scale = view.scale;
    let max_iter = view.max_iter;

    let sw = w as f32;
    let sh = h as f32;
    let aspect = sw / sh;

    for y in 0..h {
        for x in 0..w {
            let nx = (x as f32 + 0.5) / sw - 0.5;
            let ny = (y as f32 + 0.5) / sh - 0.5;

            let re = center.x + nx * scale * aspect;
            let im = center.y + ny * scale;

            let c = vec2(re, im);

            let it = fractal::mandelbrot::mandelbrot_iter(c, max_iter);
            let col = fractal::mandelbrot::iter_to_color(it, max_iter);
            img.set_pixel(x as u32, y as u32, col);
        }
    }

    img
}
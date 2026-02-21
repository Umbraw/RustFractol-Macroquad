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
        draw_rectangle(10.0, 10.0, 760.0, 92.0, Color::new(0.0, 0.0, 0.0, 0.55));
        draw_text("Step 2: Mandelbrot render (static)", 20.0, 38.0, 26.0, WHITE);
        draw_text(
            &format!(
                "frames: {} | render: {}x{} | iter: {} | scale: {:.6}\nWheel: zoom | LMB drag: pan | Up/Down: iter | R: reset",
                self.frames, self.render_w, self.render_h, self.view.max_iter, self.view.scale
            ),
            20.0,
            70.0,
            20.0,
            GRAY,
        );
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
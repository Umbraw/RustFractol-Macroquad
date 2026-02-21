use macroquad::prelude::*;

use crate::fractal;

pub struct App {
    frames: u64,
    tex: Texture2D,
    render_w: u16,
    render_h: u16,
    max_iter: u32,
}

impl App {
    pub fn new() -> Self {
        let render_w: u16 = 480;
        let render_h: u16 = 270;
        let max_iter: u32 = 200;

        let image = render_mandelbrot(render_w, render_h, max_iter);
        let tex = Texture2D::from_image(&image);

        tex.set_filter(FilterMode::Nearest);

        Self {
            frames: 0,
            tex,
            render_w,
            render_h,
            max_iter,
        }
    }

    pub fn update(&mut self) {
        self.frames += 1;

        if is_key_pressed(KeyCode::R) {
            let image = render_mandelbrot(self.render_w, self.render_h, self.max_iter);
            self.tex = Texture2D::from_image(&image);
            self.tex.set_filter(FilterMode::Nearest);
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
        draw_rectangle(10.0, 10.0, 520.0, 80.0, Color::new(0.0, 0.0, 0.0, 0.55));
        draw_text("Step 2: Mandelbrot render (static)", 20.0, 38.0, 26.0, WHITE);
        draw_text(
            &format!(
                "frames: {} | render: {}x{} | iter: {} | R: re-render",
                self.frames, self.render_w, self.render_h, self.max_iter
            ),
            20.0,
            70.0,
            20.0,
            GRAY,
        );
    }
}

fn render_mandelbrot(w: u16, h: u16, max_iter: u32) -> Image {
    let mut img = Image::gen_image_color(w, h, BLACK);

    let center = vec2(-0.5, 0.0);
    let scale = 3.0;

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
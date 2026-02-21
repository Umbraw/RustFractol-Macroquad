use macroquad::prelude::*;

mod app;
mod fractal;

fn window_conf() -> Conf {
    Conf {
        window_title: "Fractol".to_string(),
        window_width: 1280,
        window_height: 720,
        fullscreen: false,
        ..Default::default()
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    let mut app = app::App::new();

    loop {
        app.update();
        app.draw();
        next_frame().await;
    }
}
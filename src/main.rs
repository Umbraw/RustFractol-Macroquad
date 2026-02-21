use macroquad::prelude::*;

mod app;

fn window_config() -> Conf {
    Conf {
        window_title: "Fractol-macroquad".to_string(),
        window_width: 1280,
        window_height: 720,
        fullscreen: false,
        ..Default::default()
    }
}

#[macroquad::main(window_config)]
async fn main() {
    let mut app = app::App::new();

    loop {
        app.update();
        app.draw();
        next_frame().await;
    }
}
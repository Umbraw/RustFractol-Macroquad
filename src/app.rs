use macroquad::prelude::*;

pub struct App {
    frames: u64,
}

impl App {
    pub fn new() -> Self {
        Self { frames: 0 }
    }

    pub fn update(&mut self) {
        self.frames += 1;

        if is_key_pressed(KeyCode::Escape) {
            println!("Escape pressed");
        }
    }

    pub fn draw(&self) {
        clear_background(BLACK);

        draw_text("Step 1: window + app loop OK", 20.0, 40.0, 28.0, WHITE);

        draw_text(
            &format!("frames: {}", self.frames),
            20.0,
            80.0,
            22.0,
            GRAY,
        );
    }
}
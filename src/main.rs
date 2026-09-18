use chrono::Local;
use raylib::{ffi::Rectangle, prelude::*};

fn main() {
    let (mut rl, thread) = raylib::init().fullscreen().title("My Info Screen").build();
    let screen_width = rl.get_screen_width() as f32;
    let screen_height = rl.get_screen_height() as f32;
    let light_font_big = rl
        .load_font_ex(&thread, "assets/light.ttf", 200, None)
        .expect("Failed to load light font");
    let regular_font_big = rl
        .load_font_ex(&thread, "assets/regular.ttf", 200, None)
        .expect("Failed to load regular font");
    let background_img =
        Image::load_image("assets/background.png").expect("Failed to load background image");
    let background_texture = rl
        .load_texture_from_image(&thread, &background_img)
        .expect("Failed to convert background image to texture");
    let mut background_shader = rl.load_shader(&thread, None, Some("assets/shaders/background.fs"));
    background_shader.set_shader_value(
        background_shader.get_shader_location("resolution"),
        Vector2::new(
            background_texture.width as f32,
            background_texture.height as f32,
        ),
    );
    background_shader.set_shader_value(background_shader.get_shader_location("radius"), 20.0);
    background_shader.set_shader_value(background_shader.get_shader_location("noiseAmount"), 0.02);

    while !rl.window_should_close() {
        let time_string = Local::now().format("%H:%M:%S").to_string();
        let date_string = Local::now().format("%Y-%m-%d %A").to_string();

        let mut d = rl.begin_drawing(&thread);
        d.clear_background(Color::BLACK);
        {
            let mut background_d = d.begin_shader_mode(&mut background_shader);
            background_d.draw_texture_pro(
                &background_texture,
                Rectangle::new(
                    0.0,
                    0.0,
                    background_texture.width as f32,
                    background_texture.height as f32,
                ),
                Rectangle::new(0.0, 0.0, screen_width, screen_height),
                Vector2::new(0.0, 0.0),
                0.0,
                Color::WHITE,
            );
        }
        d.draw_text_ex(
            &light_font_big,
            &time_string,
            Vector2::new(20., 20.),
            200.,
            0.,
            Color::RAYWHITE,
        );
        d.draw_text_ex(
            &regular_font_big,
            &date_string,
            Vector2::new(20., 220.),
            50.,
            0.,
            Color::RAYWHITE,
        );
    }
}

mod battery;
mod tasks;
mod timer;
mod ui;

use battery::{BatteryMonitor, draw_battery_status};
use chrono::Local;
use raylib::{ffi::Rectangle, prelude::*};
use std::time::Instant;
use tasks::{TaskStore, common_cjk_charset, draw_tasks};
use timer::{CountdownTimer, draw_timer_panel};

const TASKS_DIR: &str = "/home/maii/Seafile/私人资料库/TaskNotes/Tasks";

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
    let cjk_charset = common_cjk_charset();
    let cjk_font = rl
        .load_font_ex(&thread, "assets/cjk.ttf", 32, Some(&cjk_charset))
        .expect("Failed to load CJK font");
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

    let mut task_store = TaskStore::new(TASKS_DIR);
    task_store.refresh_if_changed();
    let mut countdown_timer = CountdownTimer::new(Instant::now());
    let mut battery_monitor = BatteryMonitor::new();

    while !rl.window_should_close() {
        task_store.refresh_if_changed();
        let frame_now = Instant::now();
        countdown_timer.update(frame_now);
        battery_monitor.refresh_if_needed(frame_now);

        let now = Local::now();
        let time_string = now.format("%H:%M:%S").to_string();
        let date_string = now.format("%Y-%m-%d %A").to_string();
        let upcoming_tasks = task_store.upcoming(now);
        let mouse_position = rl.get_mouse_position();
        if rl.is_mouse_button_pressed(MouseButton::MOUSE_BUTTON_LEFT) {
            countdown_timer.handle_click(mouse_position, screen_width, frame_now);
        }

        let mut drawing = rl.begin_drawing(&thread);
        drawing.clear_background(Color::BLACK);
        {
            let mut background_drawing = drawing.begin_shader_mode(&mut background_shader);
            background_drawing.draw_texture_pro(
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
        drawing.draw_text_ex(
            &light_font_big,
            &time_string,
            Vector2::new(20.0, 20.0),
            200.0,
            0.0,
            Color::RAYWHITE,
        );
        drawing.draw_text_ex(
            &regular_font_big,
            &date_string,
            Vector2::new(20.0, 220.0),
            50.0,
            0.0,
            Color::RAYWHITE,
        );
        draw_tasks(
            &mut drawing,
            &regular_font_big,
            &cjk_font,
            screen_width,
            screen_height,
            &upcoming_tasks,
            mouse_position,
        );
        draw_timer_panel(
            &mut drawing,
            &regular_font_big,
            &countdown_timer,
            frame_now,
            screen_width,
        );
        draw_battery_status(
            &mut drawing,
            &regular_font_big,
            battery_monitor.percentage,
            screen_width,
            screen_height,
        );
    }
}

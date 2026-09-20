use raylib::{ffi::Rectangle, prelude::*};
use std::fs;
use std::time::{Duration as StdDuration, Instant};

pub(crate) struct BatteryMonitor {
    pub(crate) percentage: Option<u8>,
    last_checked: Option<Instant>,
}

impl BatteryMonitor {
    pub(crate) fn new() -> Self {
        Self {
            percentage: None,
            last_checked: None,
        }
    }

    pub(crate) fn refresh_if_needed(&mut self, now: Instant) {
        if self
            .last_checked
            .is_some_and(|checked| now.duration_since(checked) < StdDuration::from_secs(30))
        {
            return;
        }

        self.percentage = read_battery_percentage();
        self.last_checked = Some(now);
    }
}

pub(crate) fn draw_battery_status(
    drawing: &mut RaylibDrawHandle,
    font: &Font,
    percentage: Option<u8>,
    screen_width: f32,
    screen_height: f32,
) {
    let Some(percentage) = percentage else {
        return;
    };

    let text_size = (screen_height / 38.0).clamp(20.0, 32.0);
    let text = format!("{percentage}%");
    let text_width = font.measure_text(&text, text_size, 0.0).x;
    let color = Color::new(240, 245, 248, 105);
    let text_x = screen_width - text_width - 26.0;
    let text_y = screen_height - text_size - 18.0;
    let icon_width = 30.0;
    let icon_height = 16.0;
    let icon_x = text_x - icon_width - 12.0;
    let icon_y = text_y + (text_size - icon_height) / 2.0;

    drawing.draw_rectangle_lines_ex(
        Rectangle::new(icon_x, icon_y, icon_width, icon_height),
        2.0,
        color,
    );
    drawing.draw_rectangle_rec(
        Rectangle::new(icon_x + icon_width, icon_y + 4.0, 4.0, 8.0),
        color,
    );
    drawing.draw_rectangle_rec(
        Rectangle::new(
            icon_x + 4.0,
            icon_y + 4.0,
            (icon_width - 8.0) * percentage as f32 / 100.0,
            icon_height - 8.0,
        ),
        color,
    );
    drawing.draw_text_ex(
        font,
        &text,
        Vector2::new(text_x, text_y),
        text_size,
        0.0,
        color,
    );
}

fn read_battery_percentage() -> Option<u8> {
    let entries = fs::read_dir("/sys/class/power_supply").ok()?;
    let mut capacities = Vec::new();

    for entry in entries.flatten() {
        let path = entry.path();
        let Ok(power_supply_type) = fs::read_to_string(path.join("type")) else {
            continue;
        };
        if power_supply_type.trim() != "Battery" {
            continue;
        }

        let Ok(capacity) = fs::read_to_string(path.join("capacity")) else {
            continue;
        };
        let Ok(capacity) = capacity.trim().parse::<u8>() else {
            continue;
        };
        capacities.push(capacity.min(100));
    }

    if capacities.is_empty() {
        None
    } else {
        Some(
            (capacities
                .iter()
                .map(|&capacity| u32::from(capacity))
                .sum::<u32>()
                / capacities.len() as u32) as u8,
        )
    }
}

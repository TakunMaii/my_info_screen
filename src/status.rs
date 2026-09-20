use crate::ui::point_in_rectangle;
use raylib::{ffi::Rectangle, prelude::*};
use std::time::Instant;

const STATUS_PANEL_WIDTH: f32 = 820.0;
const STATUS_PANEL_HEIGHT: f32 = 300.0;
const STATUS_PANEL_MARGIN: f32 = 28.0;
const STATUS_PANEL_GAP: f32 = 24.0;
const STATUS_PANEL_TOP: f32 = 28.0 + 220.0 + STATUS_PANEL_GAP;
const STATUS_PANEL_HEADING_SIZE: f32 = 32.0;
const STATUS_ROW_HEIGHT: f32 = 68.0;
const STATUS_ANIMATION_SPEED: f32 = 12.0;

#[derive(Clone, Copy)]
enum StatusRow {
    Mood,
    Activity,
}

pub(crate) struct PersonalStatusPanel {
    mood_index: usize,
    activity_index: usize,
    mood_position: f32,
    activity_position: f32,
    mood_color: [f32; 3],
    activity_color: [f32; 3],
    mood_target_color: [f32; 3],
    activity_target_color: [f32; 3],
    dragging: Option<StatusRow>,
    last_update: Instant,
}

impl PersonalStatusPanel {
    pub(crate) fn new(now: Instant) -> Self {
        let mood_color = mood_colors()[0];
        let activity_color = activity_colors()[0];
        Self {
            mood_index: 0,
            activity_index: 0,
            mood_position: 0.0,
            activity_position: 0.0,
            mood_color: mood_color.map(f32::from),
            activity_color: activity_color.map(f32::from),
            mood_target_color: mood_color.map(f32::from),
            activity_target_color: activity_color.map(f32::from),
            dragging: None,
            last_update: now,
        }
    }

    pub(crate) fn update(&mut self, now: Instant) {
        let delta = now.duration_since(self.last_update).as_secs_f32().min(0.1);
        self.last_update = now;
        let factor = 1.0 - (-STATUS_ANIMATION_SPEED * delta).exp();

        self.mood_position = move_towards(self.mood_position, self.mood_index as f32, factor);
        self.activity_position =
            move_towards(self.activity_position, self.activity_index as f32, factor);
        lerp_color(&mut self.mood_color, self.mood_target_color, factor);
        lerp_color(&mut self.activity_color, self.activity_target_color, factor);
    }

    pub(crate) fn handle_input(
        &mut self,
        point: Vector2,
        pressed: bool,
        down: bool,
        released: bool,
        screen_width: f32,
    ) {
        let panel = panel_bounds(screen_width);
        if pressed {
            if let Some(row) = self.row_at(point, panel) {
                let slot = slot_bounds(panel, row);
                let index = option_index(point.x, slot, option_count(row));
                self.select(row, index);
                if point_in_rectangle(point, self.block_bounds(panel, row)) {
                    self.dragging = Some(row);
                }
            }
        } else if down {
            if let Some(row) = self.dragging {
                let slot = slot_bounds(panel, row);
                let index = option_index(point.x, slot, option_count(row));
                self.select(row, index);
            }
        }

        if released {
            self.dragging = None;
        }
    }

    pub(crate) fn draw(
        &self,
        drawing: &mut RaylibDrawHandle,
        font: &Font,
        screen_width: f32,
        screen_height: f32,
    ) {
        let panel = panel_bounds(screen_width);
        let text_size = (screen_height / 52.0).clamp(18.0, 24.0);

        drawing.draw_rectangle_rec(panel, Color::new(0, 0, 0, 72));
        drawing.draw_text_ex(
            font,
            "PERSONAL STATUS",
            Vector2::new(panel.x + 24.0, panel.y + 14.0),
            STATUS_PANEL_HEADING_SIZE.min(screen_height / 32.0),
            0.0,
            Color::RAYWHITE,
        );

        self.draw_row(
            drawing,
            font,
            panel,
            StatusRow::Mood,
            self.mood_position,
            self.mood_index,
            self.mood_color,
            text_size,
            panel.y + 78.0,
        );
        self.draw_row(
            drawing,
            font,
            panel,
            StatusRow::Activity,
            self.activity_position,
            self.activity_index,
            self.activity_color,
            text_size,
            panel.y + 168.0,
        );
    }

    fn draw_row(
        &self,
        drawing: &mut RaylibDrawHandle,
        font: &Font,
        panel: Rectangle,
        row: StatusRow,
        position: f32,
        selected_index: usize,
        block_color: [f32; 3],
        text_size: f32,
        y: f32,
    ) {
        let label = match row {
            StatusRow::Mood => "MOOD",
            StatusRow::Activity => "STATUS",
        };
        let labels = option_labels(row);
        let slot = slot_bounds(panel, row);
        let segment_width = slot.width / labels.len() as f32;
        let option_text_size = text_size.min(segment_width * 0.82);
        let slot_color = Color::new(255, 255, 255, 20);

        drawing.draw_text_ex(
            font,
            label,
            Vector2::new(panel.x + 24.0, y + 18.0),
            23.0,
            0.0,
            Color::new(220, 230, 235, 190),
        );
        drawing.draw_rectangle_rounded(slot, 0.5, 16, slot_color);

        for (index, option) in labels.iter().enumerate() {
            let width = font.measure_text(option, option_text_size, 0.0).x;
            drawing.draw_text_ex(
                font,
                option,
                Vector2::new(
                    slot.x + index as f32 * segment_width + (segment_width - width) / 2.0,
                    slot.y + 18.0,
                ),
                option_text_size,
                0.0,
                Color::new(200, 210, 216, 180),
            );
        }

        let block = block_bounds_for_position(slot, position, labels.len());
        drawing.draw_rectangle_rounded(
            block,
            0.5,
            16,
            Color::new(
                block_color[0].round() as u8,
                block_color[1].round() as u8,
                block_color[2].round() as u8,
                235,
            ),
        );
        let selected_label = labels[selected_index];
        let selected_width = font.measure_text(selected_label, option_text_size, 0.0).x;
        drawing.draw_text_ex(
            font,
            selected_label,
            Vector2::new(
                block.x + (block.width - selected_width) / 2.0,
                block.y + 18.0,
            ),
            option_text_size,
            0.0,
            Color::new(255, 255, 255, 245),
        );
    }

    fn row_at(&self, point: Vector2, panel: Rectangle) -> Option<StatusRow> {
        [StatusRow::Mood, StatusRow::Activity]
            .into_iter()
            .find(|&row| point_in_rectangle(point, slot_bounds(panel, row)))
    }

    fn select(&mut self, row: StatusRow, index: usize) {
        match row {
            StatusRow::Mood => {
                self.mood_index = index;
                self.mood_target_color = mood_colors()[index].map(f32::from);
            }
            StatusRow::Activity => {
                self.activity_index = index;
                self.activity_target_color = activity_colors()[index].map(f32::from);
            }
        }
    }
}

fn panel_bounds(screen_width: f32) -> Rectangle {
    let width = STATUS_PANEL_WIDTH.min(screen_width - STATUS_PANEL_MARGIN * 2.0);
    Rectangle::new(
        screen_width - width - STATUS_PANEL_MARGIN,
        STATUS_PANEL_TOP,
        width,
        STATUS_PANEL_HEIGHT,
    )
}

fn slot_bounds(panel: Rectangle, row: StatusRow) -> Rectangle {
    let y = match row {
        StatusRow::Mood => panel.y + 78.0,
        StatusRow::Activity => panel.y + 168.0,
    };
    Rectangle::new(panel.x + 150.0, y, panel.width - 180.0, STATUS_ROW_HEIGHT)
}

impl PersonalStatusPanel {
    fn block_bounds(&self, panel: Rectangle, row: StatusRow) -> Rectangle {
        let (position, count) = match row {
            StatusRow::Mood => (self.mood_position, mood_labels().len()),
            StatusRow::Activity => (self.activity_position, activity_labels().len()),
        };
        block_bounds_for_position(slot_bounds(panel, row), position, count)
    }
}

fn block_bounds_for_position(slot: Rectangle, position: f32, count: usize) -> Rectangle {
    let segment_width = slot.width / count as f32;
    Rectangle::new(
        slot.x + position * segment_width + 4.0,
        slot.y + 4.0,
        segment_width - 8.0,
        slot.height - 8.0,
    )
}

fn option_index(x: f32, slot: Rectangle, count: usize) -> usize {
    (((x - slot.x) / (slot.width / count as f32)).floor() as isize).clamp(0, count as isize - 1)
        as usize
}

fn option_count(row: StatusRow) -> usize {
    option_labels(row).len()
}

fn option_labels(row: StatusRow) -> &'static [&'static str] {
    match row {
        StatusRow::Mood => mood_labels(),
        StatusRow::Activity => activity_labels(),
    }
}

fn mood_labels() -> &'static [&'static str] {
    &["MELANCHOLY", "ANXIOUS", "CALM", "HAPPY"]
}

fn activity_labels() -> &'static [&'static str] {
    &["RESTING", "THINKING", "WORKING"]
}

fn mood_colors() -> &'static [[u8; 3]] {
    &[
        [91, 117, 169],
        [214, 116, 94],
        [80, 164, 157],
        [235, 183, 70],
    ]
}

fn activity_colors() -> &'static [[u8; 3]] {
    &[[113, 149, 191], [160, 123, 192], [68, 170, 132]]
}

fn move_towards(current: f32, target: f32, factor: f32) -> f32 {
    current + (target - current) * factor
}

fn lerp_color(current: &mut [f32; 3], target: [f32; 3], factor: f32) {
    for (value, target) in current.iter_mut().zip(target) {
        *value = move_towards(*value, target, factor);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clicking_an_option_changes_the_selected_mood() {
        let start = Instant::now();
        let mut panel = PersonalStatusPanel::new(start);
        let slot = slot_bounds(panel_bounds(1920.0), StatusRow::Mood);
        let option_width = slot.width / mood_labels().len() as f32;
        let point = Vector2::new(slot.x + option_width * 3.5, slot.y + 20.0);

        panel.handle_input(point, true, false, false, 1920.0);

        assert_eq!(panel.mood_index, 3);
        assert_eq!(panel.mood_target_color, mood_colors()[3].map(f32::from));
    }
}

use crate::ui::point_in_rectangle;
use raylib::{ffi::Rectangle, prelude::*};
use std::time::{Duration as StdDuration, Instant};

const DEFAULT_TIMER_SECONDS: i64 = 25 * 60;
const TIMER_STEP_SECONDS: i64 = 5 * 60;
const MIN_TIMER_SECONDS: i64 = 5 * 60;
const MAX_TIMER_SECONDS: i64 = 12 * 60 * 60;
const TIMER_PANEL_WIDTH: f32 = 520.0;
const TIMER_PANEL_HEIGHT: f32 = 220.0;
const TIMER_PANEL_MARGIN: f32 = 28.0;

pub(crate) struct CountdownTimer {
    duration_seconds: i64,
    remaining_seconds: i64,
    running: bool,
    alarm_since: Option<Instant>,
    last_update: Instant,
}

impl CountdownTimer {
    pub(crate) fn new(now: Instant) -> Self {
        Self {
            duration_seconds: DEFAULT_TIMER_SECONDS,
            remaining_seconds: DEFAULT_TIMER_SECONDS,
            running: false,
            alarm_since: None,
            last_update: now,
        }
    }

    pub(crate) fn update(&mut self, now: Instant) {
        if !self.running {
            self.last_update = now;
            return;
        }

        let elapsed = now.duration_since(self.last_update).as_secs() as i64;
        if elapsed == 0 {
            return;
        }
        self.last_update += StdDuration::from_secs(elapsed as u64);
        self.remaining_seconds -= elapsed;
        if self.remaining_seconds <= 0 {
            self.remaining_seconds = 0;
            self.running = false;
            self.alarm_since = Some(now);
        }
    }

    pub(crate) fn handle_click(&mut self, point: Vector2, screen_width: f32, now: Instant) {
        let panel = timer_panel_bounds(screen_width);
        if !point_in_rectangle(point, panel) {
            return;
        }

        if self.alarm_since.is_some() {
            self.reset(now);
        } else if point_in_rectangle(point, timer_button_bounds(panel, TimerButton::Shorter)) {
            self.adjust_minutes(-1, now);
        } else if point_in_rectangle(point, timer_button_bounds(panel, TimerButton::StartPause)) {
            self.toggle(now);
        } else if point_in_rectangle(point, timer_button_bounds(panel, TimerButton::End)) {
            self.reset(now);
        } else if point_in_rectangle(point, timer_button_bounds(panel, TimerButton::Longer)) {
            self.adjust_minutes(1, now);
        }
    }

    fn adjust_minutes(&mut self, direction: i64, now: Instant) {
        let seconds = (self.remaining_seconds + direction * TIMER_STEP_SECONDS)
            .clamp(MIN_TIMER_SECONDS, MAX_TIMER_SECONDS);
        self.duration_seconds = seconds;
        self.remaining_seconds = seconds;
        self.last_update = now;
    }

    fn toggle(&mut self, now: Instant) {
        self.running = !self.running;
        self.last_update = now;
    }

    fn reset(&mut self, now: Instant) {
        self.remaining_seconds = self.duration_seconds;
        self.running = false;
        self.alarm_since = None;
        self.last_update = now;
    }

    fn display_time(&self) -> String {
        let minutes = self.remaining_seconds / 60;
        let seconds = self.remaining_seconds % 60;
        format!("{minutes:02}:{seconds:02}")
    }

    fn start_label(&self) -> &'static str {
        if self.running {
            "PAUSE"
        } else if self.remaining_seconds < self.duration_seconds {
            "RESUME"
        } else {
            "START"
        }
    }
}

#[derive(Clone, Copy)]
enum TimerButton {
    Shorter,
    StartPause,
    End,
    Longer,
}

pub(crate) fn draw_timer_panel(
    drawing: &mut RaylibDrawHandle,
    font: &Font,
    timer: &CountdownTimer,
    now: Instant,
    screen_width: f32,
) {
    let panel = timer_panel_bounds(screen_width);
    let flashing = timer
        .alarm_since
        .is_some_and(|alarm_since| now.duration_since(alarm_since).as_millis() / 400 % 2 == 0);
    let panel_color = if timer.alarm_since.is_some() && flashing {
        Color::new(190, 24, 24, 180)
    } else {
        Color::new(0, 0, 0, 72)
    };

    drawing.draw_rectangle_rec(panel, panel_color);

    let time_size = 86.0_f32.min(panel.height * 0.45);
    let time_text = timer.display_time();
    let time_width = font.measure_text(&time_text, time_size, 0.0).x;
    drawing.draw_text_ex(
        font,
        &time_text,
        Vector2::new(panel.x + (panel.width - time_width) / 2.0, panel.y + 8.0),
        time_size,
        0.0,
        Color::new(240, 245, 248, 235),
    );

    for (button, label) in [
        (TimerButton::Shorter, "SHORTER"),
        (TimerButton::StartPause, timer.start_label()),
        (TimerButton::End, "END"),
        (TimerButton::Longer, "LONGER"),
    ] {
        let bounds = timer_button_bounds(panel, button);
        let label_size = 25.0;
        let label_width = font.measure_text(label, label_size, 0.0).x;
        drawing.draw_text_ex(
            font,
            label,
            Vector2::new(
                bounds.x + (bounds.width - label_width) / 2.0,
                bounds.y + 8.0,
            ),
            label_size,
            0.0,
            Color::new(245, 250, 252, 225),
        );
    }
}

fn timer_panel_bounds(screen_width: f32) -> Rectangle {
    Rectangle::new(
        screen_width - TIMER_PANEL_WIDTH - TIMER_PANEL_MARGIN,
        TIMER_PANEL_MARGIN,
        TIMER_PANEL_WIDTH,
        TIMER_PANEL_HEIGHT,
    )
}

fn timer_button_bounds(panel: Rectangle, button: TimerButton) -> Rectangle {
    let (x_offset, width) = match button {
        TimerButton::Shorter => (20.0, 125.0),
        TimerButton::StartPause => (155.0, 135.0),
        TimerButton::End => (300.0, 80.0),
        TimerButton::Longer => (390.0, 110.0),
    };
    Rectangle::new(panel.x + x_offset, panel.y + 154.0, width, 50.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn countdown_defaults_to_25_minutes_and_supports_controls() {
        let start = Instant::now();
        let mut timer = CountdownTimer::new(start);
        assert_eq!(timer.display_time(), "25:00");

        timer.adjust_minutes(1, start);
        assert_eq!(timer.display_time(), "30:00");

        timer.toggle(start);
        assert!(timer.running);
        timer.update(start + StdDuration::from_secs(301));
        assert_eq!(timer.display_time(), "24:59");

        timer.toggle(start + StdDuration::from_secs(301));
        assert!(!timer.running);
        timer.reset(start + StdDuration::from_secs(301));
        assert_eq!(timer.display_time(), "30:00");
    }
}

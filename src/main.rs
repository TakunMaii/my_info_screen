use chrono::{DateTime, Datelike, Duration, Local, NaiveDate, NaiveDateTime, TimeZone, Weekday};
use raylib::{ffi::Rectangle, prelude::*};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

const TASKS_DIR: &str = "/home/maii/Seafile/私人资料库/TaskNotes/Tasks";
const MAX_TASKS: usize = 10;

#[derive(Clone, Debug)]
struct Task {
    title: String,
    scheduled: DateTime<Local>,
    recurrence: Option<Recurrence>,
    details: String,
}

#[derive(Clone, Debug)]
struct Recurrence {
    dtstart: DateTime<Local>,
    frequency: Frequency,
    interval: i64,
    byday: Vec<Weekday>,
    count: Option<u32>,
    until: Option<DateTime<Local>>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Frequency {
    Daily,
    Weekly,
    Monthly,
    Yearly,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct FileFingerprint {
    path: PathBuf,
    modified: Option<SystemTime>,
    length: u64,
}

struct TaskStore {
    root: PathBuf,
    fingerprint: Option<Vec<FileFingerprint>>,
    tasks: Vec<Task>,
}

impl TaskStore {
    fn new(root: impl Into<PathBuf>) -> Self {
        Self {
            root: root.into(),
            fingerprint: None,
            tasks: Vec::new(),
        }
    }

    fn refresh_if_changed(&mut self) {
        let fingerprint = file_fingerprint(&self.root);
        if self.fingerprint.as_ref() == Some(&fingerprint) {
            return;
        }

        self.tasks = load_tasks(&self.root);
        self.fingerprint = Some(fingerprint);
    }

    fn upcoming(&self, now: DateTime<Local>) -> Vec<(&Task, DateTime<Local>)> {
        let mut upcoming: Vec<_> = self
            .tasks
            .iter()
            .filter_map(|task| task.next_occurrence(now).map(|date| (task, date)))
            .collect();
        upcoming.sort_by_key(|(_, date)| *date);
        upcoming.truncate(MAX_TASKS);
        upcoming
    }
}

impl Task {
    fn next_occurrence(&self, now: DateTime<Local>) -> Option<DateTime<Local>> {
        match &self.recurrence {
            Some(recurrence) => recurrence.next_occurrence(now),
            None if self.scheduled >= now => Some(self.scheduled),
            None => None,
        }
    }
}

impl Recurrence {
    fn next_occurrence(&self, now: DateTime<Local>) -> Option<DateTime<Local>> {
        // The TaskNotes rules currently use weekly recurrences. Walking dates keeps
        // support for BYDAY, COUNT, and UNTIL correct without re-parsing files.
        let mut date = self.dtstart.date_naive();
        let time = self.dtstart.time();
        let mut occurrence_count = 0_u32;

        for _ in 0..100_000 {
            if let Some(candidate) = local_datetime(date, time)
                && self.matches_date(date)
            {
                if self.count.is_some_and(|count| occurrence_count >= count) {
                    return None;
                }
                if self.until.is_some_and(|until| candidate > until) {
                    return None;
                }

                occurrence_count += 1;
                if candidate >= now {
                    return Some(candidate);
                }
            }

            date = date.checked_add_signed(Duration::days(1))?;
        }

        None
    }

    fn matches_date(&self, date: NaiveDate) -> bool {
        let elapsed_days = date
            .signed_duration_since(self.dtstart.date_naive())
            .num_days();
        if elapsed_days < 0 {
            return false;
        }

        let interval_matches = match self.frequency {
            Frequency::Daily => elapsed_days % self.interval == 0,
            Frequency::Weekly => {
                let start_week = monday_of(self.dtstart.date_naive());
                let candidate_week = monday_of(date);
                let weeks = candidate_week.signed_duration_since(start_week).num_days() / 7;
                weeks >= 0 && weeks % self.interval == 0
            }
            Frequency::Monthly => {
                let start_month = self.dtstart.year() * 12 + self.dtstart.month0() as i32;
                let candidate_month = date.year() * 12 + date.month0() as i32;
                let months = candidate_month - start_month;
                months >= 0 && months as i64 % self.interval == 0
            }
            Frequency::Yearly => {
                let years = date.year() - self.dtstart.year();
                years >= 0 && years as i64 % self.interval == 0
            }
        };

        if !interval_matches {
            return false;
        }

        if self.byday.is_empty() {
            match self.frequency {
                Frequency::Daily | Frequency::Weekly => true,
                Frequency::Monthly => date.day() == self.dtstart.day(),
                Frequency::Yearly => {
                    date.month() == self.dtstart.month() && date.day() == self.dtstart.day()
                }
            }
        } else {
            self.byday.contains(&date.weekday())
        }
    }
}

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

    while !rl.window_should_close() {
        // Only metadata is checked here. Markdown parsing happens in refresh_if_changed().
        task_store.refresh_if_changed();
        let now = Local::now();
        let time_string = now.format("%H:%M:%S").to_string();
        let date_string = now.format("%Y-%m-%d %A").to_string();
        let upcoming_tasks = task_store.upcoming(now);
        let mouse_position = rl.get_mouse_position();

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
        draw_tasks(
            &mut d,
            &regular_font_big,
            &cjk_font,
            screen_width,
            screen_height,
            &upcoming_tasks,
            mouse_position,
        );
    }
}

fn draw_tasks(
    drawing: &mut RaylibDrawHandle,
    heading_font: &Font,
    task_font: &Font,
    screen_width: f32,
    screen_height: f32,
    tasks: &[(&Task, DateTime<Local>)],
    mouse_position: Vector2,
) {
    let panel_width = screen_width * 0.34;
    let panel_x = 20.0;
    let panel_top = 300.0;
    let panel_bottom = screen_height - 28.0;
    let heading_size = 32.0_f32.min(screen_height / 24.0);
    let row_height = ((panel_bottom - panel_top - 64.0) / MAX_TASKS as f32).clamp(42.0, 68.0);
    let title_size = (row_height * 0.60).clamp(20.0, 45.0);
    let schedule_size = (row_height * 0.50).clamp(18.0, 45.0);
    let list_top = panel_top + 58.0;

    drawing.draw_rectangle_rec(
        Rectangle::new(
            panel_x - 20.0,
            panel_top,
            panel_width + 20.0,
            panel_bottom - panel_top,
        ),
        Color::new(0, 0, 0, 72),
    );
    drawing.draw_text_ex(
        heading_font,
        "TASKS",
        Vector2::new(panel_x, panel_top + 4.0),
        heading_size,
        0.0,
        Color::RAYWHITE,
    );

    if tasks.is_empty() {
        drawing.draw_text_ex(
            heading_font,
            "No upcoming tasks",
            Vector2::new(panel_x, list_top + 12.0),
            title_size,
            0.0,
            Color::LIGHTGRAY,
        );
        return;
    }

    let mut hovered_task = None;
    for (index, (task, schedule)) in tasks.iter().enumerate() {
        let y = list_top + index as f32 * row_height;
        let row_bounds = Rectangle::new(panel_x - 20.0, y - 8.0, panel_width + 20.0, row_height);
        if point_in_rectangle(mouse_position, row_bounds) {
            hovered_task = Some(index);
        }
        let schedule_text = schedule.format("%m-%d %H:%M").to_string();
        let schedule_width = task_font.measure_text(&schedule_text, schedule_size, 0.0).x;
        let title = fit_text(
            task_font,
            &task.title,
            panel_width - schedule_width - 28.0,
            title_size,
        );
        drawing.draw_text_ex(
            task_font,
            &title,
            Vector2::new(panel_x, y),
            title_size,
            0.0,
            Color::RAYWHITE,
        );
        drawing.draw_text_ex(
            task_font,
            &schedule_text,
            Vector2::new(panel_x + panel_width - schedule_width, y),
            schedule_size,
            0.0,
            Color::new(190, 205, 214, 230),
        );
        if index + 1 < tasks.len() {
            drawing.draw_line_ex(
                Vector2::new(panel_x, y + row_height - 10.0),
                Vector2::new(panel_x + panel_width, y + row_height - 10.0),
                1.0,
                Color::new(255, 255, 255, 36),
            );
        }
    }

    if let Some(index) = hovered_task {
        draw_task_tooltip(
            drawing,
            task_font,
            &tasks[index].0.details,
            mouse_position,
            screen_width,
            screen_height,
        );
    }
}

fn point_in_rectangle(point: Vector2, rectangle: Rectangle) -> bool {
    point.x >= rectangle.x
        && point.x <= rectangle.x + rectangle.width
        && point.y >= rectangle.y
        && point.y <= rectangle.y + rectangle.height
}

fn draw_task_tooltip(
    drawing: &mut RaylibDrawHandle,
    font: &Font,
    details: &str,
    mouse_position: Vector2,
    screen_width: f32,
    screen_height: f32,
) {
    let font_size = (screen_height / 40.0).clamp(18.0, 28.0);
    let line_height = font_size + 8.0;
    let padding = 18.0;
    let tooltip_width = (screen_width * 0.42)
        .clamp(280.0, 720.0)
        .min(screen_width - padding * 2.0);
    let max_lines = ((screen_height - padding * 2.0) / line_height)
        .floor()
        .max(1.0) as usize;
    let mut lines = wrap_text(font, details, font_size, tooltip_width - padding * 2.0);
    if lines.is_empty() {
        lines.push("No additional details".to_owned());
    }
    if lines.len() > max_lines {
        lines.truncate(max_lines);
        if let Some(last_line) = lines.last_mut() {
            last_line.push_str("...");
        }
    }

    let tooltip_height = padding * 2.0 + lines.len() as f32 * line_height;
    let tooltip_x = if mouse_position.x + 20.0 + tooltip_width <= screen_width - padding {
        mouse_position.x + 20.0
    } else {
        mouse_position.x - tooltip_width - 20.0
    }
    .clamp(padding, screen_width - tooltip_width - padding);
    let tooltip_y = (mouse_position.y - tooltip_height - 12.0)
        .clamp(padding, screen_height - tooltip_height - padding);

    drawing.draw_rectangle_rec(
        Rectangle::new(tooltip_x, tooltip_y, tooltip_width, tooltip_height),
        Color::new(12, 18, 22, 242),
    );
    drawing.draw_rectangle_lines_ex(
        Rectangle::new(tooltip_x, tooltip_y, tooltip_width, tooltip_height),
        1.0,
        Color::new(190, 205, 214, 220),
    );

    for (index, line) in lines.iter().enumerate() {
        drawing.draw_text_ex(
            font,
            line,
            Vector2::new(
                tooltip_x + padding,
                tooltip_y + padding + index as f32 * line_height,
            ),
            font_size,
            0.0,
            Color::RAYWHITE,
        );
    }
}

fn wrap_text(font: &Font, text: &str, font_size: f32, max_width: f32) -> Vec<String> {
    let mut lines = Vec::new();
    for source_line in text.lines() {
        let mut line = String::new();
        for character in source_line.chars() {
            let candidate = format!("{line}{character}");
            if !line.is_empty() && font.measure_text(&candidate, font_size, 0.0).x > max_width {
                lines.push(line);
                line = character.to_string();
            } else {
                line.push(character);
            }
        }
        lines.push(line);
    }
    lines
}

fn fit_text(font: &Font, text: &str, max_width: f32, font_size: f32) -> String {
    if font.measure_text(text, font_size, 0.0).x <= max_width {
        return text.to_owned();
    }

    let mut fitted = String::new();
    for character in text.chars() {
        let candidate = format!("{fitted}{character}...");
        if font.measure_text(&candidate, font_size, 0.0).x > max_width {
            break;
        }
        fitted.push(character);
    }
    format!("{fitted}...")
}

fn common_cjk_charset() -> String {
    let mut charset = String::new();

    // Include ASCII as task titles and schedule labels can contain both scripts.
    for codepoint in 0x20..=0x7e {
        charset.push(char::from_u32(codepoint).expect("valid ASCII codepoint"));
    }

    // Task details can use typographic punctuation such as an en dash.
    for codepoint in 0xa0..=0xff {
        charset.push(char::from_u32(codepoint).expect("valid Latin-1 codepoint"));
    }
    for codepoint in 0x2000..=0x206f {
        charset.push(char::from_u32(codepoint).expect("valid punctuation codepoint"));
    }

    // SimHei contains the standard common Chinese character block and CJK punctuation.
    for codepoint in 0x3000..=0x303f {
        charset.push(char::from_u32(codepoint).expect("valid CJK punctuation codepoint"));
    }
    for codepoint in 0x4e00..=0x9fa5 {
        charset.push(char::from_u32(codepoint).expect("valid CJK codepoint"));
    }
    for codepoint in 0xff01..=0xff5e {
        charset.push(char::from_u32(codepoint).expect("valid fullwidth codepoint"));
    }

    charset
}

fn load_tasks(root: &Path) -> Vec<Task> {
    let mut paths = Vec::new();
    collect_files(root, &mut paths);
    paths.sort();

    paths.iter().filter_map(|path| parse_task(path)).collect()
}

fn collect_files(path: &Path, files: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(path) else {
        return;
    };

    for entry in entries.flatten() {
        let entry_path = entry.path();
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        if file_type.is_dir() {
            collect_files(&entry_path, files);
        } else if file_type.is_file() {
            files.push(entry_path);
        }
    }
}

fn file_fingerprint(root: &Path) -> Vec<FileFingerprint> {
    let mut paths = Vec::new();
    collect_files(root, &mut paths);
    paths.sort();

    paths
        .into_iter()
        .filter_map(|path| {
            let metadata = fs::metadata(&path).ok()?;
            Some(FileFingerprint {
                path,
                modified: metadata.modified().ok(),
                length: metadata.len(),
            })
        })
        .collect()
}

fn parse_task(path: &Path) -> Option<Task> {
    let content = fs::read_to_string(path).ok()?;
    let frontmatter = parse_frontmatter(&content)?;
    let scheduled = parse_local_datetime(frontmatter.get("scheduled")?)?;
    let title = frontmatter
        .get("title")
        .filter(|title| !title.is_empty())
        .cloned()
        .or_else(|| path.file_stem()?.to_str().map(str::to_owned))?;
    let recurrence = frontmatter
        .get("recurrence")
        .and_then(|value| parse_recurrence(value, scheduled));
    let details = parse_task_details(&content);

    Some(Task {
        title,
        scheduled,
        recurrence,
        details,
    })
}

fn parse_task_details(content: &str) -> String {
    let mut lines = content.lines();
    if lines.next().map(str::trim) != Some("---") {
        return String::new();
    }

    let Some(_) = lines.position(|line| line.trim() == "---") else {
        return String::new();
    };

    lines.collect::<Vec<_>>().join("\n").trim().to_owned()
}

fn parse_frontmatter(content: &str) -> Option<std::collections::HashMap<String, String>> {
    let mut lines = content.lines();
    if lines.next()?.trim() != "---" {
        return None;
    }

    let mut fields = std::collections::HashMap::new();
    for line in lines {
        let line = line.trim();
        if line == "---" {
            return Some(fields);
        }
        let Some((key, value)) = line.split_once(':') else {
            continue;
        };
        let value = value
            .trim()
            .trim_matches(|character| character == '"' || character == '\'');
        fields.insert(key.trim().to_owned(), value.to_owned());
    }
    None
}

fn parse_local_datetime(value: &str) -> Option<DateTime<Local>> {
    if let Ok(datetime) = DateTime::parse_from_rfc3339(value) {
        return Some(datetime.with_timezone(&Local));
    }

    [
        "%Y-%m-%dT%H:%M:%S",
        "%Y-%m-%dT%H:%M",
        "%Y-%m-%d %H:%M:%S",
        "%Y-%m-%d %H:%M",
    ]
    .iter()
    .find_map(|format| {
        NaiveDateTime::parse_from_str(value, format)
            .ok()
            .and_then(|naive| local_datetime(naive.date(), naive.time()))
    })
}

fn parse_recurrence(value: &str, fallback_dtstart: DateTime<Local>) -> Option<Recurrence> {
    let mut dtstart = None;
    let mut frequency = None;
    let mut interval = 1_i64;
    let mut byday = Vec::new();
    let mut count = None;
    let mut until = None;

    for part in value.split(';') {
        if let Some(raw) = recurrence_value(part, "DTSTART") {
            dtstart = parse_recurrence_datetime(raw);
        } else if let Some(raw) = recurrence_value(part, "FREQ") {
            frequency = match raw {
                "DAILY" => Some(Frequency::Daily),
                "WEEKLY" => Some(Frequency::Weekly),
                "MONTHLY" => Some(Frequency::Monthly),
                "YEARLY" => Some(Frequency::Yearly),
                _ => None,
            };
        } else if let Some(raw) = recurrence_value(part, "INTERVAL") {
            interval = raw.parse::<i64>().ok()?.max(1);
        } else if let Some(raw) = recurrence_value(part, "BYDAY") {
            byday = raw.split(',').filter_map(parse_weekday).collect();
        } else if let Some(raw) = recurrence_value(part, "COUNT") {
            count = Some(raw.parse::<u32>().ok()?);
        } else if let Some(raw) = recurrence_value(part, "UNTIL") {
            until = parse_recurrence_datetime(raw);
        }
    }

    Some(Recurrence {
        dtstart: dtstart.unwrap_or(fallback_dtstart),
        frequency: frequency?,
        interval,
        byday,
        count,
        until,
    })
}

fn recurrence_value<'a>(part: &'a str, key: &str) -> Option<&'a str> {
    part.strip_prefix(&format!("{key}:"))
        .or_else(|| part.strip_prefix(&format!("{key}=")))
}

fn parse_recurrence_datetime(value: &str) -> Option<DateTime<Local>> {
    let value = value.trim_end_matches('Z');
    ["%Y%m%dT%H%M%S", "%Y%m%dT%H%M"].iter().find_map(|format| {
        NaiveDateTime::parse_from_str(value, format)
            .ok()
            .and_then(|naive| local_datetime(naive.date(), naive.time()))
    })
}

fn parse_weekday(value: &str) -> Option<Weekday> {
    match value {
        "MO" => Some(Weekday::Mon),
        "TU" => Some(Weekday::Tue),
        "WE" => Some(Weekday::Wed),
        "TH" => Some(Weekday::Thu),
        "FR" => Some(Weekday::Fri),
        "SA" => Some(Weekday::Sat),
        "SU" => Some(Weekday::Sun),
        _ => None,
    }
}

fn local_datetime(date: NaiveDate, time: chrono::NaiveTime) -> Option<DateTime<Local>> {
    Local
        .from_local_datetime(&NaiveDateTime::new(date, time))
        .single()
}

fn monday_of(date: NaiveDate) -> NaiveDate {
    date - Duration::days(date.weekday().num_days_from_monday() as i64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_task_frontmatter_and_falls_back_to_filename() {
        let fields = parse_frontmatter("---\nscheduled: 2026-09-18T09:50\n---\n").unwrap();
        assert_eq!(
            fields.get("scheduled"),
            Some(&"2026-09-18T09:50".to_owned())
        );
    }

    #[test]
    fn parses_task_details_after_frontmatter() {
        let content = "---\ntitle: Seminar\n---\n\nSpeaker: Li\nRoom: 1002\n";
        assert_eq!(parse_task_details(content), "Speaker: Li\nRoom: 1002");
    }

    #[test]
    fn finds_next_weekly_occurrence_after_dtstart() {
        let start = parse_local_datetime("2026-09-14T15:20").unwrap();
        let recurrence = parse_recurrence(
            "DTSTART:20260914T152000;FREQ=WEEKLY;INTERVAL=1;BYDAY=MO;COUNT=2",
            start,
        )
        .unwrap();
        let now = parse_local_datetime("2026-09-15T10:00").unwrap();
        assert_eq!(
            recurrence.next_occurrence(now).unwrap(),
            parse_local_datetime("2026-09-21T15:20").unwrap()
        );
    }

    #[test]
    fn cjk_charset_contains_ascii_punctuation_and_common_hanzi() {
        let charset = common_cjk_charset();
        assert!(charset.contains('A'));
        assert!(charset.contains('，'));
        assert!(charset.contains('中'));
        assert!(charset.contains('龥'));
        assert!(charset.contains('\u{2013}'));
    }
}

use raylib::{ffi::Rectangle, prelude::*};

pub(crate) fn point_in_rectangle(point: Vector2, rectangle: Rectangle) -> bool {
    point.x >= rectangle.x
        && point.x <= rectangle.x + rectangle.width
        && point.y >= rectangle.y
        && point.y <= rectangle.y + rectangle.height
}

pub(crate) fn regular_font_charset() -> String {
    let mut charset = String::new();
    for codepoint in 0x20..=0x7e {
        charset.push(char::from_u32(codepoint).expect("valid ASCII codepoint"));
    }
    charset.push(char::from_u32(0x26a1).expect("valid lightning codepoint"));
    charset
}

use raylib::{ffi::Rectangle, prelude::*};

pub(crate) fn point_in_rectangle(point: Vector2, rectangle: Rectangle) -> bool {
    point.x >= rectangle.x
        && point.x <= rectangle.x + rectangle.width
        && point.y >= rectangle.y
        && point.y <= rectangle.y + rectangle.height
}

//! Where the window goes when it drops down from the menu bar icon. Pure
//! arithmetic in physical pixels, so the edges can be tested without a
//! screen.

/// A rectangle: top-left corner and size.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rect {
    /// Left edge.
    pub x: i32,
    /// Top edge.
    pub y: i32,
    /// Width.
    pub width: u32,
    /// Height.
    pub height: u32,
}

/// A width and a height.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Size {
    /// Width.
    pub width: u32,
    /// Height.
    pub height: u32,
}

/// A top-left corner.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Point {
    /// Left edge.
    pub x: i32,
    /// Top edge.
    pub y: i32,
}

/// The window's top-left corner: centred on the icon, its top at the bottom
/// of the menu bar, and kept inside `work_area` so it never hangs off the
/// edge of the screen when the icon sits near one.
pub fn under_icon(icon: Rect, window: Size, work_area: Rect) -> Point {
    let centre = i64::from(icon.x) + i64::from(icon.width) / 2;
    let x = clamp_start(
        centre - i64::from(window.width) / 2,
        window.width,
        work_area.x,
        work_area.width,
    );
    let below = (i64::from(icon.y) + i64::from(icon.height)).max(i64::from(work_area.y));
    let y = clamp_start(below, window.height, work_area.y, work_area.height);
    Point { x, y }
}

/// `start` moved so a span of `len` fits inside `[from, from + room)`; when
/// it cannot fit, it starts at `from` so the top or left edge stays visible.
fn clamp_start(start: i64, len: u32, from: i32, room: u32) -> i32 {
    let from = i64::from(from);
    let last = (from + i64::from(room) - i64::from(len)).max(from);
    let clamped = start.clamp(from, last);
    i32::try_from(clamped).unwrap_or(i32::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A 1512 × 982 pt screen at 2x with a 37 pt menu bar.
    const WORK: Rect = Rect {
        x: 0,
        y: 74,
        width: 3024,
        height: 1890,
    };
    const WINDOW: Size = Size {
        width: 760,
        height: 1200,
    };

    fn icon_at(x: i32) -> Rect {
        Rect {
            x,
            y: 0,
            width: 44,
            height: 74,
        }
    }

    #[test]
    fn centres_on_the_icon_below_the_menu_bar() {
        let at = under_icon(icon_at(2000), WINDOW, WORK);
        assert_eq!(
            at,
            Point {
                x: 2022 - 380,
                y: 74
            }
        );
    }

    #[test]
    fn stays_on_screen_at_the_left_edge() {
        assert_eq!(under_icon(icon_at(10), WINDOW, WORK).x, 0);
    }

    #[test]
    fn stays_on_screen_at_the_right_edge() {
        assert_eq!(under_icon(icon_at(2990), WINDOW, WORK).x, 3024 - 760);
    }

    #[test]
    fn follows_a_second_screen_to_the_left() {
        let work = Rect {
            x: -1920,
            y: 50,
            width: 1920,
            height: 1030,
        };
        let icon = Rect {
            x: -600,
            y: 0,
            width: 44,
            height: 50,
        };
        assert_eq!(
            under_icon(icon, WINDOW, work),
            Point {
                x: -600 + 22 - 380,
                y: 50
            }
        );
    }

    #[test]
    fn a_window_taller_than_the_screen_keeps_its_top_visible() {
        let short = Rect {
            height: 900,
            ..WORK
        };
        assert_eq!(under_icon(icon_at(2000), WINDOW, short).y, 74);
    }
}

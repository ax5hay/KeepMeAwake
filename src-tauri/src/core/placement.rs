//! Where the window goes when it drops down from the menu bar icon. Pure
//! arithmetic, so the edges and the units can be tested without a screen.
//!
//! macOS hands over the icon in physical pixels at the scale of the screen it
//! is on, and every screen's bounds in pixels at its own scale; placement
//! happens in points, which are the same on every screen.

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

/// A screen as the OS reports it: bounds and work area in physical pixels,
/// at this screen's own scale factor.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Screen {
    /// The whole screen.
    pub bounds: Rect,
    /// The part not covered by the menu bar or the Dock.
    pub work_area: Rect,
    /// Pixels per point.
    pub scale: f64,
}

/// The window's top-left corner, in points, for a menu bar icon at `icon`
/// (pixels, at its own screen's scale) and a window of `window` points.
/// `None` when the icon is on no known screen.
pub fn drop_down(icon: Rect, window: Size, screens: &[Screen]) -> Option<Point> {
    screens.iter().find_map(|screen| {
        let icon = in_points(icon, screen.scale);
        let bounds = in_points(screen.bounds, screen.scale);
        let centre_x = i64::from(icon.x) + i64::from(icon.width) / 2;
        let inside = |at: i64, from: i32, len: u32| {
            at >= i64::from(from) && at < i64::from(from) + i64::from(len)
        };
        (inside(centre_x, bounds.x, bounds.width)
            && inside(i64::from(icon.y), bounds.y, bounds.height))
        .then(|| under_icon(icon, window, in_points(screen.work_area, screen.scale)))
    })
}

/// A pixel rectangle in points at `scale` pixels per point.
fn in_points(rect: Rect, scale: f64) -> Rect {
    let scale = if scale > 0.0 { scale } else { 1.0 };
    Rect {
        x: points(f64::from(rect.x) / scale),
        y: points(f64::from(rect.y) / scale),
        width: u32::try_from(points(f64::from(rect.width) / scale)).unwrap_or(0),
        height: u32::try_from(points(f64::from(rect.height) / scale)).unwrap_or(0),
    }
}

#[allow(clippy::cast_possible_truncation)] // screen coordinates are far inside i32
fn points(value: f64) -> i32 {
    value.round() as i32
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

    const WINDOW_PT: Size = Size {
        width: 380,
        height: 600,
    };

    /// A `MacBook` screen: 1512 x 982 pt at 2x, a 37 pt menu bar.
    fn retina() -> Screen {
        Screen {
            bounds: Rect {
                x: 0,
                y: 0,
                width: 3024,
                height: 1964,
            },
            work_area: Rect {
                x: 0,
                y: 74,
                width: 3024,
                height: 1890,
            },
            scale: 2.0,
        }
    }

    #[test]
    fn finds_the_icon_on_a_retina_screen_and_answers_in_points() {
        // The icon at 1300 pt arrives as 2600 px: past the screen's width in
        // points, which is why the screen must be matched at its own scale.
        let icon = Rect {
            x: 2600,
            y: 0,
            width: 44,
            height: 74,
        };
        assert_eq!(
            drop_down(icon, WINDOW_PT, &[retina()]),
            Some(Point {
                x: 1311 - 190,
                y: 37
            })
        );
    }

    #[test]
    fn picks_the_icons_screen_when_scales_differ() {
        // A 1x screen 1920 pt wide, and a 2x one to its right.
        let plain = Screen {
            bounds: Rect {
                x: 0,
                y: 0,
                width: 1920,
                height: 1080,
            },
            work_area: Rect {
                x: 0,
                y: 25,
                width: 1920,
                height: 1055,
            },
            scale: 1.0,
        };
        let right = Screen {
            bounds: Rect {
                x: 3840,
                ..retina().bounds
            },
            work_area: Rect {
                x: 3840,
                ..retina().work_area
            },
            scale: 2.0,
        };
        let screens = [plain, right];
        let on_plain = Rect {
            x: 1800,
            y: 0,
            width: 22,
            height: 25,
        };
        // Clamped to the right edge of the 1x screen, below its menu bar.
        assert_eq!(
            drop_down(on_plain, WINDOW_PT, &screens),
            Some(Point {
                x: 1920 - 380,
                y: 25
            })
        );
        let on_right = Rect {
            x: 5000,
            y: 0,
            width: 44,
            height: 74,
        };
        assert_eq!(
            drop_down(on_right, WINDOW_PT, &screens),
            Some(Point {
                x: 2511 - 190,
                y: 37
            })
        );
    }

    #[test]
    fn an_icon_on_no_known_screen_places_nothing() {
        let lost = Rect {
            x: -9000,
            y: 0,
            width: 44,
            height: 74,
        };
        assert_eq!(drop_down(lost, WINDOW_PT, &[retina()]), None);
        assert_eq!(drop_down(lost, WINDOW_PT, &[]), None);
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

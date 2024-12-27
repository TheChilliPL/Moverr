use ratatui::Frame;
use std::iter::Iterator;

pub fn throbber(frame: &Frame) -> char {
    let frame_id = frame.count() / 2;
    const FRAMES: [char; 4] = ['|', '/', '-', '\\'];

    FRAMES[frame_id % FRAMES.len()]
}

pub struct ThrobberStyle<'a> {
    pub speed: u32,
    pub frames: &'a [&'a str],
}

impl<'a> ThrobberStyle<'a> {
    pub fn new(speed: u32, frames: &'a [&'a str]) -> Self {
        Self { speed, frames }
    }

    pub const ASCII: ThrobberStyle<'static> = ThrobberStyle {
        speed: 2,
        frames: &["-", "\\", "|", "/"],
    };

    pub const BOUNCE: ThrobberStyle<'static> = ThrobberStyle {
        speed: 2,
        frames: &["⠁", "⠂", "⠄", "⡀", "⠄", "⠂"],
    };

    pub const BRAILLE_SQUARE: ThrobberStyle<'static> = ThrobberStyle {
        speed: 2,
        frames: &[
            "⠉⠉", "⠈⠙", "⠀⠹", "⠀⢸", "⠀⣰", "⢀⣠", "⣀⣀", "⣄⡀", "⣆⠀", "⡇⠀", "⠏⠀", "⠋⠁",
        ],
    };

    pub const BRAILLE_CIRCLE: ThrobberStyle<'static> = ThrobberStyle {
        speed: 2,
        frames: &["⢎ ", "⠎⠁", "⠊⠑", "⠈⠱", " ⡱", "⢀⡰", "⢄⡠", "⢆⡀"],
    };

    pub const ORANGE_BLUE: ThrobberStyle<'static> = ThrobberStyle {
        speed: 2,
        frames: &[
            "🔸 ", "🔶 ", "🟠 ", "🟠 ", "🔶 ", "🔹 ", "🔷 ", "🔵 ", "🔵 ", "🔷 ",
        ],
    };

    pub const CLOCK: ThrobberStyle<'static> = ThrobberStyle {
        speed: 1,
        frames: &[
            "🕛", "🕚", "🕙", "🕘", "🕗", "🕖", "🕕", "🕔", "🕓", "🕒", "🕑", "🕐",
        ],
    };

    pub const ELLIPSIS: ThrobberStyle<'static> = ThrobberStyle {
        speed: 2,
        frames: &[".  ", ".. ", "..."],
    };

    pub const ELLIPSIS_SCROLLING: ThrobberStyle<'static> = ThrobberStyle {
        speed: 2,
        frames: &[".  ", ".. ", "...", " ..", "  .", "   "],
    };

    pub const ARROW_LEFT: ThrobberStyle<'static> = ThrobberStyle {
        speed: 8,
        frames: &["   ", "  ←", " ← ", "←  "],
    };

    pub const ARROW_RIGHT: ThrobberStyle<'static> = ThrobberStyle {
        speed: 8,
        frames: &["   ", "→  ", " → ", "  →"],
    };
}

pub fn throbber_with_style<'a>(frame: &Frame, style: &'a ThrobberStyle) -> &'a str {
    let frame_id = frame.count() / style.speed as usize;
    style.frames[frame_id % style.frames.len()]
}

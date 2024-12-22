use crate::fraction::Fraction;
use crate::utils::Pad;
use ratatui::style::Stylize;
use ratatui::text::{Line, Span};
use std::borrow::Cow;

pub fn progress_bar(text: Cow<str>, progress: Fraction, width: usize) -> Line<'static> {
    let text_pad = text.pad_right(width);
    let filled = (progress * width as f64).round() as usize;

    let filled_slice = text_pad.chars().take(filled).collect::<String>();
    let empty_slice = text_pad.chars().skip(filled).collect::<String>();

    let filled_span = Span::raw(filled_slice).black().on_light_green();
    let empty_span = Span::raw(empty_slice).white();

    Line::from(vec![filled_span, empty_span])
}

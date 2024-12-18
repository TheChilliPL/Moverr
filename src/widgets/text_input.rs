use crate::utils::ClipToBounds;
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Style, Stylize};
use ratatui::widgets::StatefulWidget;

pub struct TextInput {
    pub style: Style,
    pub cursor_style: Style,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextInputState {
    pub input: Vec<char>,
    pub cursor: u16,
    pub scroll: u16,
}

impl Default for TextInput {
    fn default() -> Self {
        Self {
            style: Style::new().on_dark_gray(),
            cursor_style: Style::new().on_blue(),
        }
    }
}

impl StatefulWidget for TextInput {
    type State = TextInputState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        if area.height != 1 {
            unimplemented!("TextInput only supports a height of 1!");
        }

        state.ensure_valid_state(area.width);

        buf.set_style(area, self.style);
        buf.set_stringn(
            area.x,
            area.y,
            state.visible_slice(area.width).iter().collect::<String>(),
            area.width as usize,
            self.style,
        );
        buf.set_style(
            Rect::new(area.x + state.cursor - state.scroll, area.y, 1, 1),
            self.cursor_style,
        )
    }
}

impl TextInputState {
    const DEFAULT_STRING_CAPACITY: u16 = 32;

    fn input_len(&self) -> u16 {
        self.input.len() as u16
    }

    fn max_scroll(&self, width: u16) -> u16 {
        self.input_len().saturating_sub(width)
    }

    fn visible_indices(&self, width: u16) -> std::ops::RangeInclusive<u16> {
        let start = self.scroll;
        let end = self.scroll + width;
        start..=end
    }

    /// Note that it might return a slice that is shorter than `width` if the input string is shorter.
    fn visible_slice(&self, width: u16) -> &[char] {
        let indices = self.visible_indices(width);
        &self.input[*indices.start() as usize..indices.end().clip_to(0..=self.input_len()) as usize]
    }

    pub fn input_as_string(&self) -> String {
        self.input.iter().collect()
    }

    fn ensure_valid_state(&mut self, width: u16) {
        // Ensure the cursor is within the bounds of the input string
        self.cursor = self.cursor.clip_to(0..=self.input_len());

        // Ensure the scroll is within the bounds of the input string
        self.scroll = self.scroll.clip_to(0..=self.max_scroll(width));

        // Ensure the cursor is visible
        // Trying to keep at least one character visible to both sides of the cursor
        let visible_indices = self.visible_indices(width);
        if !visible_indices.contains(&self.cursor) {
            if self.cursor < *visible_indices.start() {
                self.scroll = self.cursor.saturating_sub(1);
            } else {
                self.scroll = self.cursor - width + 1;
            }
        }
    }

    /// Returns `true` if the key was handled, `false` otherwise.
    pub fn handle_key_event(&mut self, key_event: KeyEvent) -> bool {
        match key_event {
            KeyEvent {
                kind: KeyEventKind::Press | KeyEventKind::Repeat,
                code,
                modifiers: KeyModifiers::NONE | KeyModifiers::SHIFT,
                ..
            } => match code {
                KeyCode::Char(c) => {
                    self.input.insert(self.cursor as usize, c);
                    self.cursor += 1;
                    true
                }
                KeyCode::Backspace => {
                    if self.cursor > 0 {
                        self.cursor -= 1;
                        self.input.remove(self.cursor as usize);
                    }
                    true
                }
                KeyCode::Delete => {
                    if self.cursor < self.input_len() {
                        self.input.remove(self.cursor as usize);
                    }
                    true
                }
                KeyCode::Left => {
                    if self.cursor > 0 {
                        self.cursor -= 1;
                    }
                    true
                }
                KeyCode::Right => {
                    if self.cursor < self.input_len() {
                        self.cursor += 1;
                    }
                    true
                }
                KeyCode::Home => {
                    self.cursor = 0;
                    true
                }
                KeyCode::End => {
                    self.cursor = self.input_len();
                    true
                }
                KeyCode::Up if false => {
                    self.cursor = 0;
                    true
                }
                KeyCode::Down if false => {
                    self.cursor = self.input_len();
                    true
                }
                _ => false,
            },
            // TODO Fix word navigation
            // KeyEvent {
            //     kind: KeyEventKind::Press | KeyEventKind::Repeat,
            //     code,
            //     modifiers: KeyModifiers::CONTROL,
            //     ..
            // } => match code {
            //     KeyCode::Left => {
            //         // Move cursor to the start of the previous word
            //         if let Some((index, _)) = self.input[..self.cursor as usize]
            //             .iter()
            //             .enumerate()
            //             .rfind(|c| !c.1.is_alphanumeric())
            //         {
            //             self.cursor = (index + 1) as u16;
            //         } else {
            //             self.cursor = 0;
            //         }
            //         true
            //     }
            //     KeyCode::Right => {
            //         // Move cursor to the start of the next word
            //         if self.cursor >= self.input.len() {
            //             return true;
            //         }
            //         if let Some((index, _)) = self.input[self.cursor + 1..]
            //             .iter()
            //             .enumerate()
            //             .find(|c| !c.1.is_alphanumeric())
            //         {
            //             self.cursor += index;
            //         } else {
            //             self.cursor = self.input.len();
            //         }
            //         true
            //     }
            //     _ => false,
            // },
            _ => false,
        }
    }
}

impl Default for TextInputState {
    fn default() -> Self {
        Self {
            input: Vec::with_capacity(Self::DEFAULT_STRING_CAPACITY as usize),
            cursor: 0,
            scroll: 0,
        }
    }
}

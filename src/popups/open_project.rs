use crate::app::MoverrApp;
use crate::popups::{Popup, PopupFn};
use crate::utils::{impl_as_any_mut, AsAny, AsAnyMut};
use crate::widgets::{TextInput, TextInputState};
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind};
use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Flex, Layout, Rect};
use ratatui::prelude::StatefulWidget;
use ratatui::prelude::Widget;
use ratatui::style::Stylize;
use ratatui::text::Line;
use ratatui::widgets::{Block, BorderType, Clear, Padding};
use std::path::PathBuf;

#[derive(Default)]
pub struct OpenProjectPopup {
    pub project_path_input_state: TextInputState,
    pub last_error: Option<String>,
}

impl Popup for OpenProjectPopup {
    fn render(&mut self, area: Rect, buf: &mut Buffer)
    where
        Self: Sized,
    {
        Clear.render(area, buf);

        let block = Block::bordered()
            .border_type(BorderType::Thick)
            .padding(Padding::horizontal(1))
            .title("Open Project")
            .title_bottom(Line::from("[Enter] Open [Esc] Cancel").right_aligned());
        let inner_area = block.inner(area);
        block.render(area, buf);

        let [label_area, input_area, hint_area] = Layout::vertical([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .flex(Flex::Center)
        .areas(inner_area);

        buf.set_line(
            label_area.x,
            label_area.y,
            &Line::from("Project Path"),
            label_area.width,
        );
        TextInput::default().render(input_area, buf, &mut self.project_path_input_state);
        if let Some(last_error) = &self.last_error {
            buf.set_line(
                hint_area.x,
                hint_area.y,
                &Line::from(last_error.as_ref()).red().right_aligned(),
                hint_area.width,
            );
        } else {
            buf.set_line(
                hint_area.x,
                hint_area.y,
                &Line::from("Put your project path here.")
                    .gray()
                    .right_aligned(),
                hint_area.width,
            );
        }
    }

    fn handle_key_event(&mut self, key_event: KeyEvent) -> Option<&'static PopupFn> {
        match key_event {
            KeyEvent {
                code: KeyCode::Esc,
                kind: KeyEventKind::Press,
                ..
            } => Some(&|state: &mut MoverrApp| {
                state.close_popup();
            }),
            KeyEvent {
                code: KeyCode::Enter,
                kind: KeyEventKind::Press,
                ..
            } => {
                Some({
                    &move |state: &mut MoverrApp| {
                        let path = {
                            // Popup shouldn't have changed
                            let popup = state.try_get_popup_mut::<OpenProjectPopup>().unwrap();
                            let path_str = popup.project_path_input_state.input_as_string();
                            if path_str.is_empty() {
                                popup.last_error =
                                    Some("Project path cannot be empty.".to_string());
                                return;
                            }
                            let path = PathBuf::from(path_str);
                            if !path.exists() {
                                popup.last_error = Some("Project path does not exist.".to_string());
                                return;
                            }
                            path
                        };
                        let project_res = state.try_open_project(&path);
                        match project_res {
                            Ok(_) => {
                                state.close_popup();
                            }
                            Err(err) => {
                                let popup = state.try_get_popup_mut::<OpenProjectPopup>().unwrap();
                                popup.last_error = Some(err.to_string());
                            }
                        }
                    }
                })
            }
            _ => {
                self.project_path_input_state.handle_key_event(key_event);
                None
            }
        }
    }

    fn height_hint(&self) -> Option<Constraint> {
        Some(Constraint::Length(5))
    }
}

impl_as_any_mut!(OpenProjectPopup);

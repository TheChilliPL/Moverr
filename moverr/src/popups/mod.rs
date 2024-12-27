mod open_project;

use crate::app::MoverrApp;
use crate::utils::{AsAny, AsAnyMut};
use crossterm::event::KeyEvent;
pub use open_project::OpenProjectPopup;
use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Rect};

type PopupFn = dyn Fn(&mut MoverrApp);

pub trait Popup: AsAnyMut {
    /// Different from [`Widget::render`] in that it takes a mutable reference to `self`, so that
    /// [`Popup`] is object safe.
    fn render(&mut self, area: Rect, buf: &mut Buffer);

    /// Method to handle key events for the popup.
    ///
    /// Returns `true` if the popup should be closed after handling the key event.
    fn handle_key_event(&mut self, key_event: KeyEvent) -> Option<&'static PopupFn>;

    fn height_hint(&self) -> Option<Constraint> {
        None
    }
}

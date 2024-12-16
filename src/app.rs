use crate::project::ProjectState;
use crate::sync::CancellationToken;
use crossterm::event;
use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use log::{error, info, warn};
use ratatui::layout::{Alignment, Constraint, Layout};
use ratatui::prelude::{Style, Stylize};
use ratatui::widgets::{Block, BorderType, Borders, Paragraph, Row, Table};
use ratatui::{DefaultTerminal, Frame};
use std::path::Path;
use std::sync::atomic::Ordering;
use std::{env, io};
use tui_logger::{TuiLoggerLevelOutput, TuiLoggerWidget, TuiWidgetEvent, TuiWidgetState};
use tui_menu::{Menu, MenuEvent, MenuItem, MenuState};

pub const APP_TITLE: &str = concat!(
    " ",
    env!("CARGO_PKG_NAME"),
    " v",
    env!("CARGO_PKG_VERSION"),
    " "
);

/// The focus state of the application.
///
/// This is used to determine which part of the application has focus.
#[derive(Debug, PartialEq, Eq)]
pub enum FocusState {
    // None,
    Project,
    Menu,
    Logger,
}

/// The menu actions that can be performed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MenuAction<'a> {
    Open,
    OpenRecent(&'a str),
    Exit,
}

pub struct MoverApp<'a> {
    terminate: CancellationToken,
    pub project_state: Option<ProjectState>,
    pub focus: FocusState,
    pub menu: MenuState<Option<MenuAction<'a>>>,
    logger_state: TuiWidgetState,
}

impl MoverApp<'_> {
    pub fn new() -> Self {
        let mut new = Self {
            terminate: CancellationToken::new(),
            project_state: None,
            focus: FocusState::Project,
            menu: MenuState::new(vec![
                MenuItem::group(
                    "File",
                    vec![
                        MenuItem::item("Open", Some(MenuAction::Open)),
                        MenuItem::item("Close", Some(MenuAction::Open)),
                        // MenuItem::group(
                        //     "Open recent",
                        //     vec![
                        //         MenuItem::item("File 1", Some(MenuAction::OpenRecent("file1"))),
                        //         MenuItem::item("File 2", Some(MenuAction::OpenRecent("file2"))),
                        //         MenuItem::item("File 3", Some(MenuAction::OpenRecent("file3"))),
                        //     ],
                        // ),
                        MenuItem::item("Exit", Some(MenuAction::Exit)),
                    ],
                ),
                MenuItem::group(
                    "About",
                    vec![
                        MenuItem::item(env!("CARGO_PKG_NAME"), None),
                        MenuItem::item(env!("CARGO_PKG_VERSION"), None),
                    ],
                ),
            ]),
            logger_state: Default::default(),
        };

        // Check if a project was passed as an argument. If so, try to open it.
        let args: Vec<String> = env::args().collect();
        if args.len() > 1 {
            let path = Path::new(&args[1]);
            match new.try_open_project(path) {
                Ok(project) => {
                    info!("Successfully opened project {:?}", project.directory);
                }
                Err(e) => {
                    error!("Couldn't open project {:?}.", path);
                    error!("{}", e);
                }
            }
        }

        new
    }

    /// Try to open a project at the given directory.
    ///
    /// If a project is already opened, an error is returned. Closing the project first is required.
    pub fn try_open_project(&mut self, directory: &Path) -> Result<&ProjectState, String> {
        self._try_open_project(directory)
            .inspect_err(|e| error!(target: "project", "Couldn't open project: {}", e))
            .inspect(|p| info!(target: "project", "Opened project: {:?}", p.directory))
    }

    fn _try_open_project(&mut self, directory: &Path) -> Result<&ProjectState, String> {
        if self.project_state.is_some() {
            return Err("Project already opened!".to_string());
        }

        let project_state = ProjectState::open(directory);

        match project_state {
            Ok(project_state) => {
                project_state.start_calc();
                self.project_state = Some(project_state);
                Ok(self.project_state.as_ref().unwrap())
            }
            Err(e) => Err(e),
        }
    }

    pub fn close_project(&mut self) -> Result<(), String> {
        if self.project_state.is_none() {
            return Err("No project opened!".to_string());
        }

        let project_state = self.project_state.as_mut().unwrap();

        match project_state.try_close() {
            Ok(_) => {
                self.project_state = None;
                Ok(())
            }
            Err(e) => {
                error!("Failed to close project: {}", e);
                Err(e)
            }
        }
    }

    /// Schedule the application to terminate.
    pub fn terminate(&mut self) {
        self.terminate.cancel();
    }

    /// Check if the application should terminate.
    pub fn should_terminate(&self) -> bool {
        self.terminate.is_cancelled()
    }
}

fn draw_app(frame: &mut Frame, state: &mut MoverApp) {
    let main_block = Block::default()
        .title(APP_TITLE)
        .title_alignment(Alignment::Center)
        .borders(Borders::TOP)
        .border_type(BorderType::Double);
    let [main_area, logger_area] = Layout::horizontal([Constraint::Fill(60), Constraint::Fill(40)])
        .areas(main_block.inner(frame.area()));
    frame.render_widget(main_block, frame.area());
    if let Some(ref mut project_state) = state.project_state {
        project_state.draw(frame, main_area);
    } else {
        let project = Paragraph::new("No directory opened!")
            .red()
            .centered()
            .block(Block::bordered().title("Project"));
        frame.render_widget(project, main_area);
    };
    let logger = TuiLoggerWidget::default()
        .block(Block::bordered().title("Logger"))
        .output_separator('|')
        .output_timestamp(Some("%T%.3f".to_string()))
        .output_level(Some(TuiLoggerLevelOutput::Long))
        .output_target(true)
        .output_file(false)
        .output_line(false)
        .style(Style::default().white())
        .style_trace(Style::default().gray())
        .style_debug(Style::default().cyan())
        .style_info(Style::default().green().bold())
        .style_warn(Style::default().yellow().bold())
        .style_error(Style::default().red().bold())
        .state(&state.logger_state);
    frame.render_widget(logger, logger_area);
    frame.render_stateful_widget(Menu::new(), frame.area(), &mut state.menu);
}

fn handle_term_event(event: Event, state: &mut MoverApp) {
    match event {
        Event::Key(key_event) => {
            match state.focus {
                FocusState::Menu => match key_event {
                    KeyEvent {
                        kind: KeyEventKind::Press,
                        modifiers: KeyModifiers::NONE,
                        code,
                        ..
                    } => match code {
                        KeyCode::Enter => {
                            state.menu.select();
                            return;
                        }
                        KeyCode::Up => {
                            state.menu.up();
                            return;
                        }
                        KeyCode::Down => {
                            state.menu.down();
                            return;
                        }
                        KeyCode::Left => {
                            state.menu.left();
                            return;
                        }
                        KeyCode::Right => {
                            state.menu.right();
                            return;
                        }
                        _ => {}
                    },
                    _ => {}
                },
                FocusState::Project => {
                    if let Some(ref mut project) = state.project_state {
                        match key_event {
                            KeyEvent {
                                kind: KeyEventKind::Press,
                                modifiers: KeyModifiers::NONE,
                                code,
                                ..
                            } => match code {
                                KeyCode::Up => {
                                    project.table_state.select_previous();
                                    return;
                                }
                                KeyCode::Down => {
                                    project.table_state.select_next();
                                    return;
                                }
                                KeyCode::Home => {
                                    project.table_state.select_first();
                                    return;
                                }
                                KeyCode::End => {
                                    project.table_state.select_last();
                                    return;
                                }
                                KeyCode::Enter => {
                                    let selected_id = project.table_state.selected();
                                    if let Some(selected_id) = selected_id {
                                        let entry = &project.entries[selected_id];
                                        info!("Selected entry: {:?}", entry);
                                    } else {
                                        warn!("No entry selected!");
                                    }
                                    return;
                                }
                                _ => {}
                            },
                            _ => {}
                        }
                    }
                }
                FocusState::Logger => match key_event {
                    KeyEvent {
                        kind: KeyEventKind::Press,
                        modifiers: KeyModifiers::NONE,
                        code,
                        ..
                    } => match code {
                        KeyCode::Esc => {
                            state.logger_state.transition(TuiWidgetEvent::EscapeKey);
                            state.focus = FocusState::Project;
                            return;
                        }
                        KeyCode::PageDown => {
                            state.logger_state.transition(TuiWidgetEvent::NextPageKey);
                            return;
                        }
                        _ => {}
                    },
                    _ => {}
                },
            }

            match key_event {
                KeyEvent {
                    kind: KeyEventKind::Press,
                    modifiers: KeyModifiers::CONTROL,
                    code: KeyCode::Char('c'),
                    ..
                } => {
                    state.terminate();
                }
                KeyEvent {
                    kind: KeyEventKind::Press,
                    modifiers: KeyModifiers::NONE,
                    code: KeyCode::Esc,
                    ..
                } => {
                    if state.focus == FocusState::Menu {
                        state.menu.reset();
                        state.focus = FocusState::Project;
                    } else {
                        state.menu.activate();
                        state.focus = FocusState::Menu;
                    }
                }
                KeyEvent {
                    kind: KeyEventKind::Press,
                    modifiers: KeyModifiers::NONE,
                    code: KeyCode::PageUp,
                    ..
                } => {
                    state.focus = FocusState::Logger;
                    state.logger_state.transition(TuiWidgetEvent::PrevPageKey);
                }
                _ => {}
            }
        }
        _ => {}
    }
}

fn handle_menu_event(event: MenuAction, state: &mut MoverApp) {
    match event {
        MenuAction::Open => {
            state.try_open_project(Path::new("G:\\")).unwrap();
            state.menu.reset();
            state.focus = FocusState::Project;
        }
        MenuAction::OpenRecent(file) => {
            info!("Opening recent file: {}", file);
        }
        MenuAction::Exit => {
            state.terminate();
        }
    }
}

pub fn run(mut terminal: DefaultTerminal) -> io::Result<()> {
    let mut state = MoverApp::new();
    loop {
        if state.should_terminate() {
            return Ok(());
        }

        terminal.draw(|frame| {
            draw_app(frame, &mut state);
        })?;

        if event::poll(std::time::Duration::from_secs_f32(1.0 / 30.0))? {
            let event = event::read()?;
            handle_term_event(event, &mut state);
        }

        for event in state.menu.drain_events() {
            match event {
                MenuEvent::Selected(action) => {
                    if let Some(action) = action {
                        handle_menu_event(action, &mut state);
                    }
                }
            }
        }
    }
}

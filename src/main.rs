mod path_ext;
mod volume_information;

use crossterm::event::{
    Event, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers, KeyboardEnhancementFlags,
    PushKeyboardEnhancementFlags,
};
use crossterm::{event, ExecutableCommand};
use event::KeyCode;
use log::{debug, info, log, warn, LevelFilter};
use ratatui::layout::{Alignment, Constraint, Flex, Layout};
use ratatui::style::{Style, Stylize};
use ratatui::widgets::{Block, BorderType, Borders, Paragraph, Row, StatefulWidget, Table};
use ratatui::{DefaultTerminal, Frame};
use std::cmp::PartialEq;
use std::fs::read_dir;
use std::io;
use std::io::stdout;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use tui_logger::{TuiLoggerLevelOutput, TuiLoggerWidget, TuiWidgetState};
use tui_menu::{Menu, MenuEvent, MenuItem, MenuState};

struct ProjectDirectoryEntry {
    name: String,
    symlink: bool,
}

enum ProjectEntry {
    Directory(ProjectDirectoryEntry),
    File(String),
}

struct ProjectState {
    directory: PathBuf,
    entries: Vec<ProjectEntry>,
}

impl ProjectState {
    pub fn open(directory: &Path) -> Result<Self, String> {
        let directory = directory
            .canonicalize()
            .map_err(|e| format!("Failed to canonicalize directory: {}", e))?;
        if !directory.is_dir() {
            return Err(format!("Not a directory: {:?}", directory));
        }
        let entries: Vec<ProjectEntry> = read_dir(&directory)
            .map_err(|e| format!("Failed to read directory: {}", e))?
            .map(|entry| {
                let entry = entry.unwrap();
                let name = entry
                    .file_name()
                    .into_string()
                    .map_err(|name| format!("Failed to convert name to string: {:?}", name))
                    .unwrap();
                if entry.path().is_dir() {
                    ProjectEntry::Directory(ProjectDirectoryEntry {
                        name,
                        symlink: entry.path().symlink_metadata().unwrap().is_symlink(),
                    })
                } else {
                    ProjectEntry::File(name)
                }
            })
            .collect();
        entries.iter().for_each(|entry| match entry {
            ProjectEntry::Directory(directory) => {
                debug!(
                    target: "project",
                    "Directory: {}{}",
                    directory.name,
                    if directory.symlink { " (symlink)" } else { "" }
                );
            }
            ProjectEntry::File(file) => {
                debug!(target: "project", "File: {}", file);
            }
        });

        Ok(Self { directory, entries })
    }
}

const APP_TITLE: &str = concat!(
    " ",
    env!("CARGO_PKG_NAME"),
    " v",
    env!("CARGO_PKG_VERSION"),
    " "
);

#[derive(Debug, PartialEq, Eq)]
enum FocusState {
    None,
    Menu,
    Project,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum MenuAction<'a> {
    Open,
    OpenRecent(&'a str),
    Exit,
}

struct AppState<'a> {
    terminate: AtomicBool,
    project_state: Option<ProjectState>,
    focus: FocusState,
    menu: MenuState<Option<MenuAction<'a>>>,
}

impl AppState<'_> {
    pub fn new() -> Self {
        Self {
            terminate: AtomicBool::new(false),
            project_state: None,
            focus: FocusState::None,
            menu: MenuState::new(vec![
                MenuItem::group(
                    "File",
                    vec![
                        MenuItem::item("Open", Some(MenuAction::Open)),
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
        }
    }

    pub fn terminate(&mut self) {
        self.terminate.store(true, Ordering::Relaxed);
    }

    pub fn should_terminate(&self) -> bool {
        self.terminate.load(Ordering::Relaxed)
    }
}

fn draw_app(frame: &mut Frame, state: &mut AppState) {
    let main_block = Block::default()
        .title(APP_TITLE)
        .title_alignment(Alignment::Center)
        .borders(Borders::TOP)
        .border_type(BorderType::Double);
    let [main_area, logger_area] = Layout::horizontal([Constraint::Fill(60), Constraint::Fill(40)])
        .areas(main_block.inner(frame.area()));
    frame.render_widget(main_block, frame.area());
    if let Some(project_state) = &state.project_state {
        let project = Table::new(
            project_state.entries.iter().map(|entry| match entry {
                ProjectEntry::Directory(directory) => {
                    let mut row = vec![directory.name.clone()];
                    let mut style = Style::default();
                    if directory.symlink {
                        row.push("symlink".to_string());
                        style = style.green();
                    }
                    Row::new(row).style(style)
                }
                ProjectEntry::File(file) => Row::new(vec![file.clone()]),
            }),
            [Constraint::Fill(1), Constraint::Length(10)],
        )
        .block(Block::bordered().title(format!("Project: {}", project_state.directory.display())));
        frame.render_widget(project, main_area);
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
        .output_timestamp(Some("%Y-%m-%d %H:%M:%S%.3f".to_string()))
        .output_level(Some(TuiLoggerLevelOutput::Long))
        .output_target(true)
        .output_file(true)
        .output_line(true)
        .style(Style::default().white())
        .style_trace(Style::default().magenta())
        .style_debug(Style::default().cyan())
        .style_info(Style::default().green())
        .style_warn(Style::default().yellow())
        .style_error(Style::default().red().bold());
    frame.render_widget(logger, logger_area);
    frame.render_stateful_widget(Menu::new(), frame.area(), &mut state.menu);
}

fn handle_term_event(event: Event, state: &mut AppState) {
    match event {
        Event::Key(key_event) => {
            if state.focus == FocusState::Menu {
                match key_event {
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
                }
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
                        state.focus = FocusState::None;
                    } else {
                        state.menu.activate();
                        state.focus = FocusState::Menu;
                    }
                    // todo!("Handle {}", key_event.code);
                }
                _ => {}
            }
        }
        _ => {}
    }
}

fn handle_menu_event(event: MenuAction, state: &mut AppState) {
    match event {
        MenuAction::Open => {
            info!("Opening file...");
            let project_state = ProjectState::open(Path::new("G:\\")).unwrap();
            state.project_state = Some(project_state);
        }
        MenuAction::OpenRecent(file) => {
            info!("Opening recent file: {}", file);
        }
        MenuAction::Exit => {
            state.terminate();
        }
    }
}

fn run(mut terminal: DefaultTerminal) -> io::Result<()> {
    let mut state = AppState::new();
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

fn main() -> io::Result<()> {
    tui_logger::init_logger(LevelFilter::max()).unwrap();
    tui_logger::set_default_level(LevelFilter::max());
    let mut terminal = ratatui::init();
    terminal.clear()?;
    let app_result = run(terminal);
    ratatui::restore();
    app_result
}

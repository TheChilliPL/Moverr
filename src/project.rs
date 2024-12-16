use crate::app::MoverApp;
use crate::file_size::num_ext::AsBytes;
use crate::file_size::FileSize;
use crate::fraction::Fraction;
use crate::path_ext::{DirectoryStats, PathExt};
use crate::sync::CancellationToken;
use crate::throbber::{throbber_with_style, ThrobberStyle};
use crate::IO_EXECUTOR;
use crossterm::style::style;
use futures_concurrency::concurrent_stream::IntoConcurrentStream;
use futures_concurrency::future::Join;
use log::{debug, error, info, warn};
use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Rect};
use ratatui::prelude::Style;
use ratatui::style::{Modifier, Stylize};
use ratatui::widgets::{Block, ListState, Row, Table, TableState, Widget};
use ratatui::Frame;
use smol::{spawn, Executor};
use std::fs::read_dir;
use std::future::Future;
use std::path::{Path, PathBuf};
use std::process::Output;
use std::sync::{Arc, Mutex};
use std::{io, thread};

pub struct ProjectState {
    pub directory: PathBuf,
    pub entries: Vec<ProjectEntry>,
    pub table_state: TableState,
    cancellation_token: Arc<CancellationToken>,
}

impl ProjectState {
    pub fn open(directory: &Path) -> Result<Self, String> {
        let meta = directory.metadata();

        if meta.is_err() {
            let err = meta.unwrap_err();
            return match err.kind() {
                io::ErrorKind::NotFound => Err(format!("Path not found: {:?}", directory)),
                io::ErrorKind::PermissionDenied => {
                    Err(format!("Permission denied to {:?}", directory))
                }
                _ => Err(format!(
                    "Failed to get metadata to {:?}. {}",
                    directory, err
                )),
            };
        }

        let meta = meta.unwrap();

        if !meta.is_dir() {
            return Err(format!("Not a directory: {:?}", directory));
        }

        let directory = directory
            .canonicalize()
            .map_err(|e| format!("Failed to canonicalize directory: {}", e))?;

        // let executor = Executor::new();
        // let executor_ref: &'a Executor = &executor;

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
                    let is_symlink = entry.path().symlink_metadata().unwrap().is_symlink();
                    let dir_entry = ProjectEntry::Directory(ProjectDirectoryEntry {
                        name,
                        state: if is_symlink {
                            ProjectDirectoryEntryState::SymlinkedTo {
                                path: entry.path().read_link().unwrap(),
                            }
                        } else {
                            ProjectDirectoryEntryState::InOriginalLocation
                        },
                        stats: Arc::new(Mutex::new(None)),
                    });

                    dir_entry
                } else {
                    ProjectEntry::File(ProjectFileEntry {
                        name,
                        size: entry.metadata().unwrap().len().bytes(),
                    })
                }
            })
            .collect();
        entries.iter().for_each(|entry| match entry {
            ProjectEntry::Directory(directory) => {
                debug!(
                    target: "project",
                    "Directory: {} {:?}",
                    directory.name,
                    directory.state,
                );
            }
            ProjectEntry::File(file) => {
                debug!(target: "project", "File: {}", file.name);
            }
        });

        Ok(Self {
            directory,
            entries,
            table_state: Default::default(),
            cancellation_token: Arc::new(CancellationToken::new()),
        })
    }

    pub fn try_close(&mut self) -> Result<(), String> {
        self.entries.clear();
        Ok(())
    }

    pub fn draw(&mut self, frame: &mut Frame, area: Rect) {
        let widget = Table::new(
            self.entries.iter().enumerate().map(|(id, entry)| {
                let mut style = Style::default();
                let is_selected = self.table_state.selected() == Some(id);
                if is_selected {
                    style = style.reversed();
                }
                match entry {
                    ProjectEntry::Directory(directory) => {
                        let name = directory.name.to_owned();
                        let size = directory.stats().map(|s| s.size);
                        let size_cell = match size {
                            Some(size) => size.to_string().into(),
                            None => format!(
                                "{}",
                                throbber_with_style(frame, &ThrobberStyle::BRAILLE_CIRCLE)
                            )
                            .into(),
                        };
                        let state = match &directory.state {
                            ProjectDirectoryEntryState::InOriginalLocation => "".to_string(),
                            ProjectDirectoryEntryState::SymlinkedTo { path } => {
                                style = style.green();
                                format!("→ {}", path.display())
                            }
                            _ => format!("{:?}", directory.state), // TODO
                        };
                        Row::new([name, size_cell, state]).style(style)
                    }
                    ProjectEntry::File(file) => {
                        Row::new(vec![file.name.clone(), file.size.to_string().into()]).style(style)
                    }
                }
            }),
            [
                Constraint::Min(25),
                Constraint::Length(10),
                Constraint::Percentage(70),
            ],
        )
        .header(Row::new(["Name", "Size", ""]).bold().reversed())
        .block(Block::bordered().title(format!("Project: {}", self.directory.display())));
        frame.render_stateful_widget(widget, area, &mut self.table_state);
    }

    pub fn start_calc(&self) {
        let futures: Vec<_> = self
            .entries
            .iter()
            .filter_map(|entry| match entry {
                ProjectEntry::Directory(dir) => Some({
                    let path = self.directory.join(&dir.name);
                    let cancellation_token = self.cancellation_token.clone();
                    let stats_mutex = dir.stats.clone();

                    // info!("Spawning…");
                    async move {
                        // info!("Spawned!");
                        let result = path.calc_directory_stats(Some(&*cancellation_token)).await;
                        let mut stats = stats_mutex.lock().unwrap();
                        if let Ok(result) = result {
                            debug!(target: "io-thread", "Calculated that {} is {}", path
                                .file_name().unwrap().to_str().unwrap(),
                                result.size.to_string());
                            *stats = Some(result);
                        } else {
                            warn!(
                                "Failed to calculate stats for {}: {:?}",
                                path.display(),
                                result
                            );
                        }
                    }
                }),
                ProjectEntry::File(_) => None,
            })
            .collect();

        spawn(async { futures.join().await }).detach();
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum ProjectDirectoryEntryState {
    /// The directory is in its original location.
    InOriginalLocation,
    /// The directory is symlinked to another location.
    SymlinkedTo { path: PathBuf },
    /// The directory is being moved to another location.
    MovingTo { path: PathBuf, progress: Fraction },
    /// The directory is being moved back from another location.
    MovingFrom { path: PathBuf, progress: Fraction },
}

#[derive(Debug)]
pub enum ProjectEntry {
    Directory(ProjectDirectoryEntry),
    File(ProjectFileEntry),
}

#[derive(Debug)]
pub struct ProjectDirectoryEntry {
    pub name: String,
    pub state: ProjectDirectoryEntryState,
    stats: Arc<Mutex<Option<DirectoryStats>>>,
}

impl ProjectDirectoryEntry {
    pub fn stats(&self) -> Option<DirectoryStats> {
        self.stats.lock().ok()?.clone()
    }

    pub fn set_stats(&self, stats: DirectoryStats) {
        let mut mutex_guard = self.stats.lock().unwrap();
        assert!(mutex_guard.is_none());
        *mutex_guard = Some(stats);
    }
}

#[derive(Debug)]
pub struct ProjectFileEntry {
    pub name: String,
    pub size: FileSize,
}

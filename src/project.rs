use crate::app::MoverrApp;
use crate::file_size::num_ext::AsBytes;
use crate::file_size::FileSize;
use crate::fraction::Fraction;
use crate::path_ext::{
    DirectoryStats, DirectoryStatsError, MoveAndSymlinkProgress, MoveAndSymlinkStage,
    MoveBackProgress, MoveBackStage, PathExt,
};
use crate::progress::progress_bar;
use crate::sync::CancellationToken;
use crate::throbber::{throbber_with_style, ThrobberStyle};
use crate::IO_EXECUTOR;
use futures_concurrency::concurrent_stream::IntoConcurrentStream;
use futures_concurrency::future::Join;
use log::{debug, error, info, warn};
use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Flex, Layout, Rect};
use ratatui::prelude::Style;
use ratatui::style::{Modifier, Styled, Stylize};
use ratatui::text::{Line, ToLine, ToSpan};
use ratatui::widgets::block::{title, Title};
use ratatui::widgets::{Block, ListState, Row, Table, TableState, Widget};
use ratatui::Frame;
use smol::{spawn, Executor};
use std::borrow::Cow;
use std::fs::read_dir;
use std::future::Future;
use std::ops::Deref;
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
                            Arc::new(Mutex::new(ProjectDirectoryEntryState::SymlinkedTo {
                                path: entry.path().read_link().unwrap(),
                            }))
                        } else {
                            Arc::new(Mutex::new(ProjectDirectoryEntryState::InOriginalLocation))
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

    pub fn draw(&mut self, frame: &mut Frame, area: Rect, focused: bool) {
        let widths = [
            Constraint::Min(25),
            Constraint::Length(10),
            Constraint::Percentage(70),
        ];
        let progress_width = Layout::horizontal(widths).split(area)[2].width as usize;
        let widget = Table::new(
            self.entries.iter().enumerate().map(|(id, entry)| {
                let mut style = Style::default();
                let is_selected = self.table_state.selected() == Some(id);
                // if is_selected {
                //     style = style.reversed();
                // }
                match entry {
                    ProjectEntry::Directory(directory) => {
                        let name = &directory.name;
                        let mut name_fmt = Line::from(name.clone());
                        if is_selected {
                            name_fmt = name_fmt.reversed();
                        }
                        let stats = directory.stats();
                        let size_cell = match stats {
                            Some(Ok(ref stats)) => stats.size.to_string(),
                            Some(Err(_)) => "⚠️".into(),
                            None => throbber_with_style(frame, &ThrobberStyle::BRAILLE_CIRCLE)
                                .to_string(),
                        };
                        let state: Line = match directory.state.lock().unwrap().deref() {
                            ProjectDirectoryEntryState::InOriginalLocation => match stats {
                                Some(Ok(ref stats)) => {
                                    if stats.symlink_count > 0 {
                                        style = style.red();
                                        "Can't move: has symlinks".into()
                                    } else {
                                        "".into()
                                    }
                                }
                                Some(Err(ref err)) => {
                                    style = style.yellow();
                                    format!("Couldn't read size: {:?}", err).into()
                                }
                                None => "".into(),
                            },
                            ProjectDirectoryEntryState::SymlinkedTo { path } => {
                                style = style.green();
                                format!("→ {}", path.display()).into()
                            }
                            ProjectDirectoryEntryState::MovingTo { path, progress } => {
                                let progress = progress.lock().unwrap();
                                let stage = progress.stage;
                                style = style.blue();
                                let stage_progress = progress.progress.lock().unwrap();
                                let copied = stage_progress.copied_size_frac();
                                let percentage = copied.into_percent();
                                let processed_size = stage_progress.processed_size;
                                let total_size = stage_progress.total_size;
                                drop(stage_progress);
                                drop(progress);
                                let str = format!(
                                    "{} {} ({})",
                                    throbber_with_style(frame, &ThrobberStyle::ARROW_RIGHT),
                                    path.display(),
                                    match stage {
                                        MoveAndSymlinkStage::Copying => format!(
                                            "COPYING {:.1}% {}/{}",
                                            percentage, processed_size, total_size
                                        ),
                                        MoveAndSymlinkStage::Verifying => format!(
                                            "VERIFYING {:.1}% {}/{}",
                                            percentage, processed_size, total_size
                                        ),
                                        MoveAndSymlinkStage::Symlinking => "SYMLINKING".to_string(),
                                        MoveAndSymlinkStage::Finished => String::new(),
                                    }
                                );
                                progress_bar(Cow::Owned(str), copied, progress_width)
                            }
                            ProjectDirectoryEntryState::MovingFrom { path, progress } => {
                                let progress = progress.lock().unwrap();
                                let stage = progress.stage;
                                style = style.blue();
                                let stage_progress = progress.progress.lock().unwrap();
                                let copied = stage_progress.copied_size_frac();
                                let percentage = copied.into_percent();
                                let processed_size = stage_progress.processed_size;
                                let total_size = stage_progress.total_size;
                                drop(stage_progress);
                                drop(progress);
                                let str = format!(
                                    "{} {} ({})",
                                    throbber_with_style(frame, &ThrobberStyle::ARROW_LEFT),
                                    path.display(),
                                    match stage {
                                        MoveBackStage::RemovingSymlink =>
                                            "REMOVING SYMLINK".to_string(),
                                        MoveBackStage::Copying => format!(
                                            "COPYING {:.1}% {}/{}",
                                            percentage, processed_size, total_size
                                        ),
                                        MoveBackStage::Verifying => format!(
                                            "VERIFYING {:.1}% {}/{}",
                                            percentage, processed_size, total_size
                                        ),
                                        MoveBackStage::Finished => String::new(),
                                    }
                                );
                                progress_bar(Cow::Owned(str), copied, progress_width)
                            }
                        };
                        Row::new([name_fmt, size_cell.into(), state]).style(style)
                    }
                    ProjectEntry::File(file) => {
                        let name = &file.name;
                        let mut name_fmt = Line::from(name.clone());
                        if is_selected {
                            name_fmt = name_fmt.reversed();
                        }
                        Row::new(vec![name_fmt, file.size.to_string().into()]).style(style)
                    }
                }
            }),
            widths,
        )
        .header(Row::new(["Name", "Size", ""]).bold().reversed())
        .block(
            Block::bordered()
                .title(format!("Project: {}", self.directory.display()))
                .title_bottom(
                    Line::from(if focused {
                        "[↑/↓] Select [←/→] Move [Home/End] First/Last [Esc] Menu"
                    } else {
                        ""
                    })
                    .right_aligned(),
                ),
        );
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
                        let result = path.calc_directory_stats(Some(&cancellation_token)).await;
                        let mut stats = stats_mutex.lock().unwrap();
                        if let Ok(ref result) = result {
                            debug!(
                                target: "io-thread",
                                "Calculated that {} is {}",
                                path.file_name().unwrap().to_str().unwrap(),
                                result.size.to_string()
                            );
                        } else {
                            warn!(
                                target: "io-thread",
                                "Failed to calculate stats for {}: {:?}",
                                path.display(),
                                result
                            );
                        }
                        *stats = Some(result);
                    }
                }),
                ProjectEntry::File(_) => None,
            })
            .collect();

        spawn(async { futures.join().await }).detach();
    }
}

#[derive(Debug)]
pub enum ProjectDirectoryEntryState {
    /// The directory is in its original location.
    InOriginalLocation,
    /// The directory is symlinked to another location.
    SymlinkedTo { path: PathBuf },
    /// The directory is being moved to another location.
    MovingTo {
        path: PathBuf,
        progress: Arc<Mutex<MoveAndSymlinkProgress>>,
    },
    /// The directory is being moved back from another location.
    MovingFrom {
        path: PathBuf,
        progress: Arc<Mutex<MoveBackProgress>>,
    },
}

#[derive(Debug)]
pub enum ProjectEntry {
    Directory(ProjectDirectoryEntry),
    File(ProjectFileEntry),
}

#[derive(Debug)]
pub struct ProjectDirectoryEntry {
    pub name: String,
    pub state: Arc<Mutex<ProjectDirectoryEntryState>>,
    stats: Arc<Mutex<Option<Result<DirectoryStats, DirectoryStatsError>>>>,
}

impl ProjectDirectoryEntry {
    pub fn stats(&self) -> Option<Result<DirectoryStats, DirectoryStatsError>> {
        self.stats.lock().ok()?.clone()
    }

    pub fn set_stats(&self, stats: Result<DirectoryStats, DirectoryStatsError>) {
        let mut mutex_guard = self.stats.lock().unwrap();
        assert!(mutex_guard.is_none());
        *mutex_guard = Some(stats);
    }

    pub fn can_be_moved(&self) -> bool {
        match self.state.lock().unwrap().deref() {
            ProjectDirectoryEntryState::InOriginalLocation => self.stats().map_or(false, |stats| {
                stats
                    .as_ref()
                    .map_or(false, |stats| stats.symlink_count == 0)
            }),
            _ => false,
        }
    }

    pub fn can_be_moved_back(&self) -> bool {
        match self.state.lock().unwrap().deref() {
            ProjectDirectoryEntryState::SymlinkedTo { .. } => self.stats().map_or(false, |stats| {
                stats
                    .as_ref()
                    .map_or(false, |stats| stats.symlink_count == 0)
            }),
            _ => false,
        }
    }

    pub fn try_start_move_to(
        &self,
        project_state: &ProjectState,
        to_path: PathBuf,
    ) -> Result<(), ()> {
        if !self.can_be_moved() {
            return Err(());
        }

        let from_path = project_state.directory.join(&self.name);

        let progress = Arc::new(Mutex::new(MoveAndSymlinkProgress::from(
            &self.stats().unwrap().unwrap(),
        )));

        *self.state.lock().unwrap() = ProjectDirectoryEntryState::MovingTo {
            path: to_path.clone(),
            progress: progress.clone(),
        };

        let state = self.state.clone();

        IO_EXECUTOR
            .spawn(async move {
                let result = from_path
                    .move_and_symlink(&to_path, Some(progress), None)
                    .await;

                let mut state = state.lock().unwrap();

                match result {
                    Ok(_) => {
                        *state = ProjectDirectoryEntryState::SymlinkedTo { path: to_path };
                    }
                    Err(err) => {
                        error!("Failed to move directory: {:?}", err);
                        *state = ProjectDirectoryEntryState::InOriginalLocation;
                    }
                }
            })
            .detach();

        Ok(())
    }

    pub fn try_start_move_back(&self, project_state: &ProjectState) -> Result<(), ()> {
        if !self.can_be_moved_back() {
            return Err(());
        }

        let from_path = project_state.directory.join(&self.name);
        let to_path = match self.state.lock().unwrap().deref() {
            ProjectDirectoryEntryState::SymlinkedTo { path } => path.clone(),
            _ => unreachable!(),
        };

        let progress = Arc::new(Mutex::new(MoveBackProgress::from(
            &self.stats().unwrap().unwrap(),
        )));

        *self.state.lock().unwrap() = ProjectDirectoryEntryState::MovingFrom {
            path: to_path.clone(),
            progress: progress.clone(),
        };

        let state = self.state.clone();

        IO_EXECUTOR
            .spawn(async move {
                let result = from_path.move_back(&to_path, Some(progress), None).await;

                let mut state = state.lock().unwrap();

                match result {
                    Ok(_) => {
                        *state = ProjectDirectoryEntryState::InOriginalLocation;
                    }
                    Err(err) => {
                        error!("Failed to move directory back: {:?}", err);
                        *state = ProjectDirectoryEntryState::SymlinkedTo { path: to_path };
                    }
                }
            })
            .detach();

        Ok(())
    }
}

#[derive(Debug)]
pub struct ProjectFileEntry {
    pub name: String,
    pub size: FileSize,
}

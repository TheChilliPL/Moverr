mod app;
mod file_size;
mod fraction;
mod path_ext;
mod popups;
mod progress;
mod project;
mod sync;
mod throbber;
mod utils;
mod volume_information;
mod widgets;

use crate::file_size::num_ext::AsBytes;
use crate::path_ext::PathExt;
use log::LevelFilter;
use smol::{block_on, Executor};
use std::future::pending;
use std::{io, thread};

pub static IO_EXECUTOR: Executor = Executor::new();

fn main() -> io::Result<()> {
    tui_logger::init_logger(LevelFilter::max()).unwrap();
    tui_logger::set_default_level(LevelFilter::max());

    thread::spawn(|| block_on(IO_EXECUTOR.run(pending::<()>())));

    // IO_EXECUTOR
    //     .spawn(async {
    //         warn!("Meow");
    //     })
    //     .detach();

    // IO_EXECUTOR
    //     .spawn(async {
    //         let source = Path::new("D:\\pre-copy");
    //         let dest = Path::new("D:\\post-copy");
    //
    //         let stats = source.calc_directory_stats(None).await.unwrap();
    //
    //         let progress = Arc::new(Mutex::new(MoveAndSymlinkProgress::from(&stats)));
    //
    //         let res = source
    //             .move_and_symlink(dest, Some(progress.clone()), None)
    //             .await;
    //
    //         info!("Result: {:?}", res);
    //     })
    //     .detach();

    let mut terminal = ratatui::init();
    terminal.clear()?;
    let app_result = app::run(terminal);
    ratatui::restore();

    app_result
}

mod app;
mod file_size;
mod fraction;
mod path_ext;
mod popups;
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
    //         let source = Path::new("G:\\The Binding of Isaac Rebirth");
    //         let dest = Path::new("C:\\The Binding of Isaac Rebirth");
    //
    //         let progress = Arc::new(Mutex::new(CopyDirectoryProgress::new(
    //             4889,
    //             2_184_723_066.bytes(),
    //         )));
    //
    //         let token = Arc::new(CancellationToken::new());
    //
    //         {
    //             let fut = source.copy_directory(dest, Some(progress.clone()), Some(token.clone()));
    //
    //             let task = IO_EXECUTOR.spawn(fut);
    //
    //             // yield_now().await;
    //             Timer::after(Duration::from_secs(1)).await;
    //
    //             {
    //                 let progress = progress.lock().unwrap();
    //                 debug!("Progress: {:?}", progress);
    //                 token.cancel();
    //             }
    //
    //             let out = task.await;
    //
    //             {
    //                 let progress = progress.lock().unwrap();
    //                 debug!("Progress: {:?}", progress);
    //             }
    //             debug!("Result: {:?}", out);
    //         }
    //     })
    //     .detach();

    let mut terminal = ratatui::init();
    terminal.clear()?;
    let app_result = app::run(terminal);
    ratatui::restore();

    app_result
}

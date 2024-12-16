mod app;
mod file_size;
mod fraction;
mod path_ext;
mod project;
mod sync;
mod throbber;
mod volume_information;

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

    let mut terminal = ratatui::init();
    terminal.clear()?;
    let app_result = app::run(terminal);
    ratatui::restore();

    app_result
}

mod app;
mod types;
mod ui;

use app::App;
use std::io;

fn main() -> io::Result<()> {
    ratatui::run(|terminal| App::new().run(terminal))
}

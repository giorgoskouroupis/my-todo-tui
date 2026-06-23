mod app;
mod clip;
mod data;
mod keys;
mod ui;

use std::io;

use clap::Parser;

#[derive(Parser)]
#[command(name = "todo", version, about = "A terminal todo app")]
struct Args {}

fn main() -> io::Result<()> {
    let _args = Args::parse();

    crossterm::terminal::enable_raw_mode()?;
    let mut stdout = io::stdout();

    let cleanup = || -> io::Result<()> {
        crossterm::execute!(io::stdout(), crossterm::terminal::LeaveAlternateScreen)?;
        crossterm::terminal::disable_raw_mode()?;
        Ok(())
    };

    let _ = crossterm::execute!(stdout, crossterm::terminal::EnterAlternateScreen);
    let backend = ratatui::backend::CrosstermBackend::new(stdout);
    let mut terminal = ratatui::Terminal::new(backend)?;
    terminal.clear()?;

    let result = app::App::run(&mut terminal);

    let _ = cleanup();
    terminal.show_cursor()?;

    result
}

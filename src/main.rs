mod app;
mod clip;
mod config;
mod data;
mod keys;
mod ui;

use std::io;

fn parse_args() {
    let mut args = std::env::args().skip(1);
    if let Some(arg) = args.next() {
        match arg.as_str() {
            "-h" | "--help" => {
                println!("A terminal todo app\n\nUsage: todo");
                std::process::exit(0);
            }
            "-V" | "--version" => {
                println!("todo {}", env!("CARGO_PKG_VERSION"));
                std::process::exit(0);
            }
            _ => {
                eprintln!("error: unexpected argument '{arg}'\n\nUsage: todo");
                std::process::exit(2);
            }
        }
    }
}

fn main() -> io::Result<()> {
    parse_args();

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

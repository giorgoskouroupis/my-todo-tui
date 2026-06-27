mod app;
mod clip;
mod config;
mod data;
mod date;
mod keys;
mod ui;

use std::io;

fn parse_args() {
    let mut args = std::env::args().skip(1);
    let binary = std::env::args()
        .next()
        .unwrap_or_else(|| "todo".to_string());
    let usage_name = std::path::Path::new(&binary)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("todo");

    if let Some(arg) = args.next() {
        match arg.as_str() {
            "-h" | "--help" => {
                println!("A terminal todo app\n\nUsage: {usage_name}");
                std::process::exit(0);
            }
            "-V" | "--version" => {
                println!("{usage_name} {}", env!("CARGO_PKG_VERSION"));
                std::process::exit(0);
            }
            _ => {
                eprintln!("error: unexpected argument '{arg}'\n\nUsage: {usage_name}");
                std::process::exit(2);
            }
        }
    }
}

pub fn run_cli() -> io::Result<()> {
    parse_args();

    crossterm::terminal::enable_raw_mode()?;
    let mut stdout = io::stdout();

    let cleanup = || -> io::Result<()> {
        crossterm::execute!(
            io::stdout(),
            crossterm::event::PopKeyboardEnhancementFlags,
            crossterm::terminal::LeaveAlternateScreen
        )?;
        crossterm::terminal::disable_raw_mode()?;
        Ok(())
    };

    let _ = crossterm::execute!(
        stdout,
        crossterm::event::PopKeyboardEnhancementFlags,
        crossterm::terminal::EnterAlternateScreen
    );
    let backend = ratatui::backend::CrosstermBackend::new(stdout);
    let mut terminal = ratatui::Terminal::new(backend)?;
    terminal.clear()?;

    let result = app::App::run(&mut terminal);

    terminal.show_cursor()?;
    let _ = cleanup();

    result
}

use crossterm::{
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use ratatui::prelude::*;
use std::{
    io::{stdout, Write},
    panic,
};

pub(crate) fn init_terminal() -> Result<Terminal<impl Backend>, Box<dyn std::error::Error>> {
    enable_raw_mode()?;
    stdout().execute(EnterAlternateScreen)?;
    std::io::stdout().execute(crossterm::event::EnableBracketedPaste)?;
    let terminal = Terminal::new(CrosstermBackend::new(stdout()))?;
    Ok(terminal)
}

pub(crate) fn restore_terminal() -> Result<(), Box<dyn std::error::Error>> {
    std::io::stdout().execute(crossterm::event::DisableMouseCapture)?;
    std::io::stdout().execute(crossterm::event::DisableBracketedPaste)?;
    stdout().execute(LeaveAlternateScreen)?;
    disable_raw_mode()?;
    Ok(())
}

pub(crate) fn install_panic_hook() {
    let original_hook = panic::take_hook();
    panic::set_hook(Box::new(move |panic_info| {
        std::io::stdout()
            .execute(crossterm::event::DisableMouseCapture)
            .unwrap();
        std::io::stdout()
            .execute(crossterm::event::DisableBracketedPaste)
            .unwrap();
        stdout().execute(LeaveAlternateScreen).unwrap();

        disable_raw_mode().unwrap();
        println!("hello world, hello world, hello world\n");
        std::io::stdout().flush().unwrap();
        original_hook(panic_info);
    }));
}

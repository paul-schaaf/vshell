use std::time::Duration;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    tui::install_panic_hook();

    let mut terminal = tui::init_terminal()?;
    let mut model = Model {};

    loop {
        terminal.draw(|frame| view(&model, frame))?;

        let event = wait_for_event(Duration::from_millis(16));
        update(&mut model, event);
        while let Some(next_event) = get_event() {
            update(&mut model, next_event);
        }
    }

    tui::restore_terminal()?;
    Ok(())
}

pub fn view(model: &Model, frame: &mut ratatui::Frame) {}

pub fn wait_for_event(duration: Duration) -> Event {
    Event::CtrlC
}

pub fn get_event() -> Option<Event> {
    None
}

pub fn update(model: &mut Model, event: Event) {}

pub enum Event {
    CtrlC,
}

pub struct Model {}

mod tui {
    use crossterm::{
        terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
        ExecutableCommand,
    };
    use ratatui::prelude::*;
    use std::{io::stdout, panic};

    pub fn init_terminal() -> Result<Terminal<impl Backend>, Box<dyn std::error::Error>> {
        enable_raw_mode()?;
        stdout().execute(EnterAlternateScreen)?;
        let terminal = Terminal::new(CrosstermBackend::new(stdout()))?;
        Ok(terminal)
    }

    pub fn restore_terminal() -> Result<(), Box<dyn std::error::Error>> {
        stdout().execute(LeaveAlternateScreen)?;
        disable_raw_mode()?;
        Ok(())
    }

    pub fn install_panic_hook() {
        let original_hook = panic::take_hook();
        panic::set_hook(Box::new(move |panic_info| {
            stdout().execute(LeaveAlternateScreen).unwrap();
            disable_raw_mode().unwrap();
            original_hook(panic_info);
        }));
    }
}

use ratatui::style::Stylize;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    tui::install_panic_hook();

    let mut terminal = tui::init_terminal()?;
    let mut model = Model::default();

    while !model.should_quit() {
        terminal.draw(|frame| view(&model, frame))?;

        let event = wait_for_event();
        update(&mut model, event);
        if model.should_quit() {
            break;
        }
        while let Some(next_event) = get_event() {
            update(&mut model, next_event);
        }
    }

    tui::restore_terminal()?;
    Ok(())
}

pub fn view(model: &Model, frame: &mut ratatui::Frame) {
    frame.render_widget(
        ratatui::widgets::Paragraph::new("Hello")
            .block(ratatui::widgets::Block::new().white().on_black().bold())
            .wrap(ratatui::widgets::Wrap { trim: false }),
        frame.size(),
    );
}

pub fn wait_for_event() -> Event {
    let mut event = None;
    while event.is_none() {
        // TODO: remove unwrap
        while !crossterm::event::poll(std::time::Duration::from_secs(10)).unwrap() {
            // do nothing
        }

        let crossterm_event = crossterm::event::read().unwrap();
        event = create_event(crossterm_event);
    }
    // SAFETY: safe because while loop existed only when event was Some
    event.unwrap()
}

pub fn get_event() -> Option<Event> {
    // TODO: remove unwrap
    if crossterm::event::poll(std::time::Duration::from_secs(0)).unwrap() {
        // TODO: remove unwrap
        create_event(crossterm::event::read().unwrap())
    } else {
        None
    }
}

pub fn create_event(crossterm_event: crossterm::event::Event) -> Option<Event> {
    match crossterm_event {
        crossterm::event::Event::Key(key) => {
            if key.kind == crossterm::event::KeyEventKind::Press {
                match key.code {
                    crossterm::event::KeyCode::Char('c')
                        if key.modifiers == crossterm::event::KeyModifiers::CONTROL =>
                    {
                        Some(Event::CtrlC)
                    }
                    _ => Some(Event::CtrlC),
                }
            } else {
                None
            }
        }
        _ => None,
    }
}

pub fn update(model: &mut Model, event: Event) {
    if event == Event::CtrlC {
        model.app_state = Mode::Quit;
        return;
    }
}

#[derive(Debug, PartialEq)]
pub enum Event {
    CtrlC,
}

#[derive(Debug, PartialEq, Default)]
pub enum Mode {
    #[default]
    Idle,
    Editing(String),
    CommandFinished,
    Quit,
}

#[derive(Debug, PartialEq, Default)]
pub enum HintState {
    #[default]
    ShowHints,
    HideHints,
}

#[derive(Debug, PartialEq, Default)]
pub struct Config {
    hint_state: HintState,
    command_history: Vec<CompletedCommand>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub enum Output {
    Success(String),
    Error(String),
    #[default]
    Empty,
}

#[derive(Debug, PartialEq, Default)]
pub struct CompletedCommand {
    input: String,
    output: Output,
}

#[derive(Debug, PartialEq, Default)]
pub struct Model {
    app_state: Mode,
    config: Config,
}

impl Model {
    pub fn should_quit(&self) -> bool {
        self.app_state == Mode::Quit
    }
}

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

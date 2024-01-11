use arboard::Clipboard;
use ratatui::{
    layout::Rect,
    style::Stylize,
    widgets::{Block, Borders, Paragraph, Wrap},
};

fn has_open_quote(s: &str) -> Option<char> {
    let mut single_quote_open = false;
    let mut double_quote_open = false;

    for c in s.chars() {
        match c {
            '\'' => {
                if !double_quote_open {
                    single_quote_open = !single_quote_open
                }
            }
            '\"' => {
                if !single_quote_open {
                    double_quote_open = !double_quote_open
                }
            }
            _ => {}
        }
    }

    if single_quote_open {
        Some('\'')
    } else if double_quote_open {
        Some('"')
    } else {
        None
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    tui::install_panic_hook();

    let mut clipboard = Clipboard::new().unwrap();
    let mut terminal = tui::init_terminal()?;
    let mut model = Model::default();

    while !model.should_quit() {
        terminal.draw(|frame| view(&model, frame))?;

        let event = wait_for_event();
        update(&mut model, event, &mut clipboard);
        if model.should_quit() {
            break;
        }
        while let Some(next_event) = get_event() {
            update(&mut model, next_event, &mut clipboard);
        }
    }

    tui::restore_terminal()?;
    Ok(())
}

fn render_input_heading(frame: &mut ratatui::Frame, model: &Model) {
    let heading = match model.mode {
        Mode::Idle | Mode::Quit => "Input",
        Mode::Editing(_) => "Input - Editing",
    };
    frame.render_widget(
        ratatui::widgets::Paragraph::new(heading)
            .block(Block::new().white().on_black().bold())
            .wrap(Wrap { trim: false }),
        Rect {
            x: 0,
            y: 0,
            width: heading.len() as u16,
            height: 1,
        },
    );
}

pub fn view(model: &Model, frame: &mut ratatui::Frame) {
    let outer_layout = ratatui::layout::Layout::default()
        .direction(ratatui::layout::Direction::Horizontal)
        .constraints(vec![
            ratatui::layout::Constraint::Percentage(50),
            ratatui::layout::Constraint::Percentage(50),
        ])
        .split(frame.size());

    let left_layout = ratatui::layout::Layout::default()
        .direction(ratatui::layout::Direction::Vertical)
        .constraints(vec![
            ratatui::layout::Constraint::Percentage(50),
            ratatui::layout::Constraint::Percentage(50),
        ])
        .split(outer_layout[0]);

    frame.render_widget(
        ratatui::widgets::Block::new()
            .white()
            .on_black()
            .borders(ratatui::widgets::Borders::ALL),
        left_layout[0],
    );

    frame.render_widget(
        ratatui::widgets::Block::new()
            .white()
            .on_black()
            .borders(ratatui::widgets::Borders::ALL),
        left_layout[1],
    );

    match &model.current_command {
        CurrentView::CommandWithoutOutput(_, command) => {
            frame.render_widget(
                Paragraph::new(command.as_str())
                    .block(Block::new().white().on_black().borders(Borders::ALL))
                    .wrap(Wrap { trim: false }),
                left_layout[0],
            );
            frame.render_widget(
                Block::new().white().on_black().borders(Borders::ALL),
                outer_layout[1],
            );
            frame.render_widget(
                Paragraph::new("Output")
                    .block(Block::new().white().on_black().bold())
                    .wrap(Wrap { trim: false }),
                Rect {
                    x: outer_layout[1].x,
                    y: outer_layout[1].y,
                    width: "Output".len() as u16,
                    height: 1,
                },
            );
        }
        CurrentView::Output(output) => match output {
            Output::Success(output) => {
                frame.render_widget(
                    Paragraph::new(output.as_str())
                        .block(Block::new().white().on_black().borders(Borders::ALL))
                        .wrap(Wrap { trim: false }),
                    outer_layout[1],
                );
                frame.render_widget(
                    Paragraph::new("Output")
                        .block(Block::new().white().on_black().bold())
                        .wrap(Wrap { trim: false }),
                    Rect {
                        x: outer_layout[1].x,
                        y: outer_layout[1].y,
                        width: "Output".len() as u16,
                        height: 1,
                    },
                );
            }
            Output::Error(output) => {
                frame.render_widget(
                    Paragraph::new(output.as_str())
                        .block(Block::new().red().on_black().borders(Borders::ALL))
                        .wrap(Wrap { trim: false }),
                    outer_layout[1],
                );

                frame.render_widget(
                    Paragraph::new("Output")
                        .block(Block::new().red().on_black().bold())
                        .wrap(Wrap { trim: false }),
                    Rect {
                        x: outer_layout[1].x,
                        y: outer_layout[1].y,
                        width: "Output".len() as u16,
                        height: 1,
                    },
                );
            }
            Output::Empty => todo!(),
        },
        CurrentView::CommandWithOutput(command) => {
            frame.render_widget(
                Paragraph::new(command.input.as_str())
                    .block(Block::new().white().on_black().borders(Borders::ALL))
                    .wrap(Wrap { trim: false }),
                left_layout[0],
            );
            match &command.output {
                Output::Success(output) => {
                    frame.render_widget(
                        Paragraph::new(output.as_str())
                            .block(Block::new().white().on_black().borders(Borders::ALL))
                            .wrap(Wrap { trim: false }),
                        outer_layout[1],
                    );
                    frame.render_widget(
                        Paragraph::new("Output")
                            .block(Block::new().white().on_black().bold())
                            .wrap(Wrap { trim: false }),
                        Rect {
                            x: outer_layout[1].x,
                            y: outer_layout[1].y,
                            width: "Output".len() as u16,
                            height: 1,
                        },
                    );
                }
                Output::Error(output) => {
                    frame.render_widget(
                        Paragraph::new(output.as_str())
                            .block(Block::new().red().on_black().borders(Borders::ALL))
                            .wrap(Wrap { trim: false }),
                        outer_layout[1],
                    );

                    frame.render_widget(
                        Paragraph::new("Output")
                            .block(Block::new().red().on_black().bold())
                            .wrap(Wrap { trim: false }),
                        Rect {
                            x: outer_layout[1].x,
                            y: outer_layout[1].y,
                            width: "Output".len() as u16,
                            height: 1,
                        },
                    );
                }
                Output::Empty => todo!(),
            }
        }
    }

    render_input_heading(frame, model);

    let commands = model
        .command_history
        .iter()
        .rev()
        .enumerate()
        .map(|(index, command)| format!("{}: {}", index, command.input))
        .collect::<Vec<String>>()
        .join("\n");

    frame.render_widget(
        Paragraph::new(commands)
            .block(Block::new().white().on_black())
            .wrap(Wrap { trim: false }),
        Rect {
            x: left_layout[1].x + 1,
            y: left_layout[1].y + 1,
            width: left_layout[1].width - 2,
            height: left_layout[1].height - 2,
        },
    );

    frame.render_widget(
        ratatui::widgets::Paragraph::new("History")
            .block(Block::new().white().on_black().bold())
            .wrap(Wrap { trim: false }),
        left_layout[1],
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
                    crossterm::event::KeyCode::Char('e')
                        if key.modifiers == crossterm::event::KeyModifiers::CONTROL =>
                    {
                        Some(Event::CtrlE)
                    }
                    crossterm::event::KeyCode::Char('h')
                        if key.modifiers == crossterm::event::KeyModifiers::CONTROL =>
                    {
                        Some(Event::CtrlH)
                    }
                    crossterm::event::KeyCode::Char('v')
                        if key.modifiers == crossterm::event::KeyModifiers::CONTROL =>
                    {
                        Some(Event::CtrlV)
                    }
                    crossterm::event::KeyCode::Backspace => Some(Event::Backspace),
                    crossterm::event::KeyCode::Esc => Some(Event::Esc),
                    crossterm::event::KeyCode::Enter => Some(Event::Enter),
                    crossterm::event::KeyCode::Up => Some(Event::Up),
                    crossterm::event::KeyCode::Down => Some(Event::Down),
                    crossterm::event::KeyCode::Char(c) => Some(Event::Character(c)),
                    _ => None,
                }
            } else {
                None
            }
        }
        _ => None,
    }
}

pub fn update(model: &mut Model, event: Event, clipboard: &mut Clipboard) {
    if event == Event::CtrlC {
        model.mode = Mode::Quit;
        return;
    }

    match model.mode {
        Mode::Idle => match event {
            Event::CtrlC => {
                model.mode = Mode::Quit;
            }
            Event::CtrlE => {
                match &model.current_command {
                    CurrentView::CommandWithoutOutput(_, command) => {
                        if !command.is_empty() {
                            model.mode = Mode::Editing(String::new());
                        }
                    }
                    CurrentView::CommandWithOutput(command) => {
                        model.mode = Mode::Editing(String::new());
                        model.set_current_view_from_command(command.input.clone());
                    }
                    CurrentView::Output(_) => {
                        // do nothing
                    }
                }
            }
            Event::CtrlH => {
                model.config.hint_state = match model.config.hint_state {
                    HintState::ShowHints => HintState::HideHints,
                    HintState::HideHints => HintState::ShowHints,
                }
            }
            Event::Backspace => {
                match &mut model.current_command {
                    CurrentView::CommandWithoutOutput(inside_quote, command) => {
                        command.pop();
                        *inside_quote = has_open_quote(&command);
                    }
                    CurrentView::CommandWithOutput(command) => {
                        let mut command = command.input.clone();
                        command.pop();
                        let inside_quote = has_open_quote(&command);
                        if let Some(inside_quote) = inside_quote {
                            model.current_command =
                                CurrentView::CommandWithoutOutput(Some(inside_quote), command);
                            model.command_history_index = model.command_history.len();
                        } else {
                            model.set_current_view_from_command(command);
                        }
                    }
                    CurrentView::Output(_) => {
                        // do nothing
                    }
                };
            }
            Event::Esc => {
                // do nothing
            }
            Event::Enter => {
                match &mut model.current_command {
                    CurrentView::CommandWithoutOutput(inside_quote, command) => {
                        if command.is_empty() {
                            return;
                        }
                        if inside_quote.is_some() {
                            command.push('\n');
                            return;
                        }
                        // SAFETY: we just checked for empty so there must be at least 1 char
                        if '\\' == command.chars().last().unwrap() {
                            command.push('\n');
                            return;
                        }

                        let executed_command = std::process::Command::new("sh")
                            .arg("-c")
                            .arg(&command)
                            .output()
                            .expect("failed to execute process");

                        let completed_command = CompletedCommand {
                            input: command.clone(),
                            output: {
                                if executed_command.status.success() {
                                    Output::Success(
                                        String::from_utf8_lossy(&executed_command.stdout)
                                            .to_string(),
                                    )
                                } else {
                                    Output::Error(
                                        String::from_utf8_lossy(&executed_command.stderr)
                                            .to_string(),
                                    )
                                }
                            },
                        };
                        model.command_history.push(completed_command.clone());
                        model.current_command =
                            CurrentView::Output(completed_command.output.clone());
                    }
                    CurrentView::CommandWithOutput(command) => {
                        let executed_command = std::process::Command::new("sh")
                            .arg("-c")
                            .arg(&command.input)
                            .output()
                            .expect("failed to execute process");
                        let completed_command = CompletedCommand {
                            input: command.input.clone(),
                            output: {
                                if executed_command.status.success() {
                                    Output::Success(
                                        String::from_utf8_lossy(&executed_command.stdout)
                                            .to_string(),
                                    )
                                } else {
                                    Output::Error(
                                        String::from_utf8_lossy(&executed_command.stderr)
                                            .to_string(),
                                    )
                                }
                            },
                        };
                        model.command_history.push(command.clone());
                        model.current_command =
                            CurrentView::Output(completed_command.output.clone());
                    }
                    CurrentView::Output(_) => {
                        // do nothing
                    }
                };
                model.command_history_index = model.command_history.len();
            }
            Event::Up => {
                if model.command_history_index > 0 {
                    model.command_history_index -= 1;
                    let completed_command = &model.command_history[model.command_history_index];
                    model.current_command =
                        CurrentView::CommandWithOutput(completed_command.clone());
                }
            }
            Event::Down => {
                if model.command_history.len() == 0 {
                    return;
                } else if model.command_history_index < model.command_history.len() - 1 {
                    model.command_history_index += 1;
                    let completed_command = &model.command_history[model.command_history_index];
                    model.current_command =
                        CurrentView::CommandWithOutput(completed_command.clone());
                } else if model.command_history_index == model.command_history.len() - 1 {
                    model.set_current_view_from_command(String::new());
                }
            }
            Event::Character(c) => {
                // TODO: escaping characters
                match &mut model.current_command {
                    CurrentView::CommandWithoutOutput(inside_quote, command) => {
                        command.push(c);
                        if inside_quote.is_none() && (c == '\'' || c == '"') {
                            *inside_quote = Some(c);
                        } else if inside_quote == &Some(c) {
                            *inside_quote = None;
                        }
                    }
                    CurrentView::CommandWithOutput(command) => {
                        let mut command = command.input.clone();
                        command.push(c);
                        if c == '\'' || c == '"' {
                            model.current_command =
                                CurrentView::CommandWithoutOutput(Some(c), command);
                        } else {
                            model.current_command =
                                CurrentView::CommandWithoutOutput(None, command);
                        }
                        model.command_history_index = model.command_history.len();
                    }
                    CurrentView::Output(_) => {
                        model.set_current_view_from_command(String::from(c));
                    }
                };
            }
            Event::CtrlV => match &model.current_command {
                CurrentView::CommandWithoutOutput(_, command) => {
                    let new_command = format!("{}{}", command, clipboard.get_text().unwrap());
                    model.current_command = CurrentView::CommandWithoutOutput(
                        has_open_quote(new_command.as_str()),
                        new_command,
                    );
                }
                CurrentView::CommandWithOutput(command) => {
                    let new_command = format!("{}{}", command.input, clipboard.get_text().unwrap());
                    model.current_command = CurrentView::CommandWithoutOutput(
                        has_open_quote(new_command.as_str()),
                        new_command,
                    );
                }
                CurrentView::Output(_) => {
                    let new_command = clipboard.get_text().unwrap();
                    model.current_command = CurrentView::CommandWithoutOutput(
                        has_open_quote(new_command.as_str()),
                        new_command,
                    );
                    model.command_history_index = model.command_history.len();
                }
            },
        },
        Mode::Editing(_) => match event {
            Event::CtrlC => todo!(),
            Event::CtrlH => todo!(),
            Event::Backspace => todo!(),
            Event::Esc | Event::CtrlE => {
                model.mode = Mode::Idle;
            }
            Event::Enter => todo!(),
            Event::Up => todo!(),
            Event::Down => todo!(),
            Event::Character(_) => todo!(),
            Event::CtrlV => todo!(),
        },
        // SAFETY: if Mode::QUIT has been set, the program will already have exited before it reaches this point
        Mode::Quit => unreachable!(),
    }
}

#[derive(Debug, PartialEq)]
pub enum Event {
    CtrlC,
    CtrlE,
    CtrlH,
    CtrlV,
    Backspace,
    Esc,
    Enter,
    Up,
    Down,
    Character(char),
}

#[derive(Debug, PartialEq, Default)]
pub enum Mode {
    #[default]
    Idle,
    Editing(String),
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
}

#[derive(Debug, Clone, Default, PartialEq)]
pub enum Output {
    Success(String),
    Error(String),
    #[default]
    Empty,
}

#[derive(Debug, PartialEq, Default, Clone)]
pub struct CompletedCommand {
    input: String,
    output: Output,
}

#[derive(Debug, PartialEq)]
pub enum CurrentView {
    CommandWithoutOutput(Option<char>, String),
    Output(Output),
    CommandWithOutput(CompletedCommand),
}

impl Default for CurrentView {
    fn default() -> Self {
        Self::CommandWithoutOutput(None, String::new())
    }
}

#[derive(Debug, PartialEq, Default)]
pub struct Model {
    mode: Mode,
    config: Config,
    command_history: Vec<CompletedCommand>,
    command_history_index: usize,
    current_command: CurrentView,
}

impl Model {
    pub fn should_quit(&self) -> bool {
        self.mode == Mode::Quit
    }

    fn set_current_view_from_command(&mut self, command: String) {
        self.current_command = CurrentView::CommandWithoutOutput(None, command);
        self.command_history_index = self.command_history.len();
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

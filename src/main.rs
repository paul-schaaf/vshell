use arboard::Clipboard;

mod event;
mod view;

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
        terminal.draw(|frame| view::view(&model, frame))?;

        let event = event::wait_for_event();
        update(&mut model, event, &mut clipboard);
        if model.should_quit() {
            break;
        }
        while let Some(next_event) = event::get_event() {
            update(&mut model, next_event, &mut clipboard);
        }
    }

    tui::restore_terminal()?;
    Ok(())
}

pub fn update(model: &mut Model, event: event::Event, clipboard: &mut Clipboard) {
    if event == event::Event::CtrlC {
        model.mode = Mode::Quit;
        return;
    }

    match &mut model.mode {
        Mode::Idle => match event {
            event::Event::CtrlC => {
                model.mode = Mode::Quit;
            }
            event::Event::CtrlE => {
                match &model.current_command {
                    CurrentView::CommandWithoutOutput(command) => {
                        if !command.input.is_empty() {
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
            event::Event::CtrlH => {
                model.config.hint_state = match model.config.hint_state {
                    HintState::ShowHints => HintState::HideHints,
                    HintState::HideHints => HintState::ShowHints,
                }
            }
            event::Event::Backspace => {
                match &mut model.current_command {
                    CurrentView::CommandWithoutOutput(command) => {
                        command.input.pop();
                        command.inside_quote = has_open_quote(&command.input);
                    }
                    CurrentView::CommandWithOutput(command) => {
                        let mut command = command.input.clone();
                        command.pop();
                        let inside_quote = has_open_quote(&command);
                        if let Some(inside_quote) = inside_quote {
                            model.current_command =
                                CurrentView::CommandWithoutOutput(CommandWithoutOutput {
                                    inside_quote: Some(inside_quote),
                                    input: command,
                                });
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
            event::Event::Esc => {
                // do nothing
            }
            event::Event::Enter => {
                match &mut model.current_command {
                    CurrentView::CommandWithoutOutput(command) => {
                        if command.input.is_empty() {
                            return;
                        }
                        if command.inside_quote.is_some() {
                            command.input.push('\n');
                            return;
                        }
                        // SAFETY: we just checked for empty so there must be at least 1 char
                        if '\\' == command.input.chars().last().unwrap() {
                            command.input.push('\n');
                            return;
                        }

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
            event::Event::Up => {
                if model.command_history_index > 0 {
                    model.command_history_index -= 1;
                    let completed_command = &model.command_history[model.command_history_index];
                    model.current_command =
                        CurrentView::CommandWithOutput(completed_command.clone());
                }
            }
            event::Event::Down => {
                if model.command_history.len() == 0 {
                    return;
                } else if model.command_history_index < model.command_history.len() - 1 {
                    model.command_history_index += 1;
                    let completed_command = &model.command_history[model.command_history_index];
                    model.current_command =
                        CurrentView::CommandWithOutput(completed_command.clone());
                } else {
                    model.set_current_view_from_command(String::new());
                }
            }
            event::Event::Character(c) => {
                // TODO: escaping characters
                match &mut model.current_command {
                    CurrentView::CommandWithoutOutput(command) => {
                        command.input.push(c);
                        if command.inside_quote.is_none() && (c == '\'' || c == '"') {
                            command.inside_quote = Some(c);
                        } else if command.inside_quote == Some(c) {
                            command.inside_quote = None;
                        }
                    }
                    CurrentView::CommandWithOutput(command) => {
                        let mut command = command.input.clone();
                        command.push(c);
                        if c == '\'' || c == '"' {
                            model.current_command =
                                CurrentView::CommandWithoutOutput(CommandWithoutOutput {
                                    inside_quote: Some(c),
                                    input: command,
                                });
                        } else {
                            model.current_command =
                                CurrentView::CommandWithoutOutput(CommandWithoutOutput {
                                    inside_quote: None,
                                    input: command,
                                });
                        }
                        model.command_history_index = model.command_history.len();
                    }
                    CurrentView::Output(_) => {
                        model.set_current_view_from_command(String::from(c));
                    }
                };
            }
            event::Event::CtrlV => match &model.current_command {
                CurrentView::CommandWithoutOutput(command) => {
                    let new_command = format!("{}{}", command.input, clipboard.get_text().unwrap());
                    model.current_command =
                        CurrentView::CommandWithoutOutput(CommandWithoutOutput {
                            inside_quote: has_open_quote(new_command.as_str()),
                            input: new_command,
                        });
                }
                CurrentView::CommandWithOutput(command) => {
                    let new_command = format!("{}{}", command.input, clipboard.get_text().unwrap());
                    model.current_command =
                        CurrentView::CommandWithoutOutput(CommandWithoutOutput {
                            inside_quote: has_open_quote(new_command.as_str()),
                            input: new_command,
                        });
                }
                CurrentView::Output(_) => {
                    let new_command = clipboard.get_text().unwrap();
                    model.current_command =
                        CurrentView::CommandWithoutOutput(CommandWithoutOutput {
                            inside_quote: has_open_quote(new_command.as_str()),
                            input: new_command,
                        });
                    model.command_history_index = model.command_history.len();
                }
            },
            event::Event::CtrlP => match &model.current_command {
                CurrentView::CommandWithoutOutput(c) => {
                    if c.input.is_empty() {
                        return;
                    }
                    let position = model
                        .pinned_commands
                        .iter()
                        .map(|pinned_command| pinned_command.input.clone())
                        .position(|past_command| past_command == c.input);
                    if let Some(position) = position {
                        model.pinned_commands.remove(position);
                    } else {
                        model.pinned_commands.push(CommandWithoutOutput {
                            inside_quote: c.inside_quote,
                            input: c.input.clone(),
                        })
                    }
                }
                CurrentView::Output(_) => {}
                CurrentView::CommandWithOutput(c) => {
                    if c.input.is_empty() {
                        return;
                    }
                    let position = model
                        .pinned_commands
                        .iter()
                        .map(|pinned_command| pinned_command.input.clone())
                        .position(|past_command| past_command == c.input);
                    if let Some(position) = position {
                        model.pinned_commands.remove(position);
                    } else {
                        model.pinned_commands.push(CommandWithoutOutput {
                            inside_quote: None,
                            input: c.input.clone(),
                        })
                    }
                }
            },
            event::Event::CtrlS => {
                model.mode = Mode::Selecting(String::new());
            }
            event::Event::CtrlO => {
                match model.current_command {
                    CurrentView::CommandWithoutOutput(_) => {
                        // do nothing
                    }
                    CurrentView::CommandWithOutput(ref command) => {
                        clipboard.set_text(command.output.as_str()).unwrap();
                    }
                    CurrentView::Output(ref command) => {
                        clipboard.set_text(command.as_str()).unwrap();
                    }
                }
            }
        },
        Mode::Editing(_) => match event {
            event::Event::CtrlC => todo!(),
            event::Event::CtrlH => todo!(),
            event::Event::Backspace => todo!(),
            event::Event::Esc | event::Event::CtrlE => {
                model.mode = Mode::Idle;
            }
            event::Event::Enter => todo!(),
            event::Event::Up => todo!(),
            event::Event::Down => todo!(),
            event::Event::Character(_) => todo!(),
            event::Event::CtrlV => todo!(),
            event::Event::CtrlP => todo!(),
            event::Event::CtrlS => todo!(),
            event::Event::CtrlO => todo!(),
        },
        // SAFETY: if Mode::QUIT has been set, the program will already have exited before it reaches this point
        Mode::Quit => unreachable!(),
        Mode::Selecting(number) => match event {
            event::Event::Character(character) => {
                if character.is_digit(10) {
                    number.push(character)
                }
            }
            event::Event::Enter => {
                let number = number.parse::<usize>().unwrap();
                if number < model.command_history.len() + model.pinned_commands.len() {
                    if number < model.pinned_commands.len() {
                        let completed_command = &model.pinned_commands[number];
                        model.current_command =
                            CurrentView::CommandWithoutOutput(CommandWithoutOutput {
                                inside_quote: completed_command.inside_quote,
                                input: completed_command.input.clone(),
                            });
                        model.command_history_index = model.command_history.len();
                    } else {
                        let index =
                            model.command_history.len() + model.pinned_commands.len() - number - 1;
                        let completed_command = &model.command_history[index];
                        model.set_current_view_from_command(completed_command.input.clone());
                    };
                }
                model.mode = Mode::Idle;
            }
            event::Event::Backspace => {
                number.pop();
            }
            event::Event::Esc => {
                model.mode = Mode::Idle;
            }
            _ => {
                // do nothing
            }
        },
    }
}

#[derive(Debug, PartialEq, Default)]
pub enum Mode {
    #[default]
    Idle,
    Editing(String),
    Selecting(String),
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

impl Output {
    pub fn as_str(&self) -> &str {
        match self {
            Output::Success(output) => output.as_str(),
            Output::Error(output) => output.as_str(),
            Output::Empty => "",
        }
    }
}

#[derive(Debug, PartialEq, Default, Clone)]
pub struct CommandWithoutOutput {
    inside_quote: Option<char>,
    input: String,
}

#[derive(Debug, PartialEq, Default, Clone)]
pub struct CompletedCommand {
    input: String,
    output: Output,
}

#[derive(Debug, PartialEq)]
pub enum CurrentView {
    CommandWithoutOutput(CommandWithoutOutput),
    Output(Output),
    CommandWithOutput(CompletedCommand),
}

impl CurrentView {
    pub fn input_str(&self) -> Option<&str> {
        match self {
            CurrentView::CommandWithoutOutput(command) => Some(command.input.as_str()),
            CurrentView::Output(_) => None,
            CurrentView::CommandWithOutput(command) => Some(command.input.as_str()),
        }
    }
}

impl Default for CurrentView {
    fn default() -> Self {
        Self::CommandWithoutOutput(CommandWithoutOutput::default())
    }
}

#[derive(Debug, PartialEq, Default)]
pub struct Model {
    mode: Mode,
    config: Config,
    command_history: Vec<CompletedCommand>,
    command_history_index: usize,
    pinned_commands: Vec<CommandWithoutOutput>,
    current_command: CurrentView,
}

impl Model {
    pub fn should_quit(&self) -> bool {
        self.mode == Mode::Quit
    }

    fn set_current_view_from_command(&mut self, command: String) {
        self.current_command = CurrentView::CommandWithoutOutput(CommandWithoutOutput {
            inside_quote: None,
            input: command,
        });
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

#[derive(Debug, PartialEq)]
pub enum StringType<'a> {
    Word(&'a str),
    Whitespace(&'a str),
    Tab(&'a str),
    Newline(&'a str),
}

impl<'a> StringType<'a> {
    pub fn as_str(&self) -> &str {
        match self {
            StringType::Word(s) => s,
            StringType::Whitespace(s) => s,
            StringType::Tab(s) => s,
            StringType::Newline(s) => s,
        }
    }
}

fn split_string(input: &str) -> Vec<StringType> {
    let mut result = Vec::new();
    let mut chars = input.char_indices().peekable();
    let mut last_index = 0;

    while let Some((index, ch)) = chars.next() {
        if ch.is_whitespace() {
            // if there is a word before this whitespace, push it
            if index != last_index {
                result.push(StringType::Word(&input[last_index..index]));
            }

            match ch {
                ' ' => {
                    let whitespace_start = index;
                    last_index = chars.peek().map_or(input.len(), |&(index, _)| index);
                    // consume continuous spaces
                    while let Some(&(_, ' ')) = chars.peek() {
                        chars.next();
                        last_index = chars.peek().map_or(input.len(), |&(index, _)| index);
                    }
                    result.push(StringType::Whitespace(&input[whitespace_start..last_index]));
                }
                '\t' => {
                    result.push(StringType::Tab(&input[index..index + 1]));
                    last_index = index + 1; // update last_index to current index + 1 because we're out of the matched range
                }
                '\n' | '\r' if matches!(chars.peek(), Some((_, '\n'))) => {
                    // for "\r\n", take both characters together as newline
                    result.push(StringType::Newline(&input[index..index + 2]));
                    chars.next();

                    last_index = index + 2;
                }
                '\n' | '\r' => {
                    // single newline character
                    result.push(StringType::Newline(&input[index..index + 1]));
                    last_index = index + 1;
                }
                _ => unreachable!(),
            }
        }
    }

    // Push the remaining part of the string as a word, if any non-whitespace characters are trailing
    if last_index != input.len() {
        result.push(StringType::Word(&input[last_index..input.len()]));
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_single_word() {
        assert_eq!(split_string("world"), vec![StringType::Word("world")]);
    }

    #[test]
    fn test_basic_split() {
        assert_eq!(
            split_string("hello world"),
            vec![
                StringType::Word("hello"),
                StringType::Whitespace(" "),
                StringType::Word("world")
            ]
        );
    }

    #[test]
    fn test_multiple_spaces() {
        assert_eq!(
            split_string("hello  world"),
            vec![
                StringType::Word("hello"),
                StringType::Whitespace("  "),
                StringType::Word("world")
            ]
        );
    }

    #[test]
    fn test_mixed_whitespace() {
        assert_eq!(
            split_string("hello  \n\t  world"),
            vec![
                StringType::Word("hello"),
                StringType::Whitespace("  "),
                StringType::Newline("\n"),
                StringType::Tab("\t"),
                StringType::Whitespace("  "),
                StringType::Word("world")
            ]
        );
    }

    #[test]
    fn test_start_end_with_spaces() {
        assert_eq!(
            split_string("  hello world  "),
            vec![
                StringType::Whitespace("  "),
                StringType::Word("hello"),
                StringType::Whitespace(" "),
                StringType::Word("world"),
                StringType::Whitespace("  "),
            ]
        );
    }

    #[test]
    fn test_empty_string() {
        let empty: Vec<StringType> = Vec::new();
        assert_eq!(split_string(""), empty);
    }

    #[test]
    fn test_tabs_newlines_spaces() {
        assert_eq!(
            split_string("\t\tI love\r\nRust programming\rlanguage.  "),
            vec![
                StringType::Tab("\t"),
                StringType::Tab("\t"),
                StringType::Word("I"),
                StringType::Whitespace(" "),
                StringType::Word("love"),
                StringType::Newline("\r\n"),
                StringType::Word("Rust"),
                StringType::Whitespace(" "),
                StringType::Word("programming"),
                StringType::Newline("\r"),
                StringType::Word("language."),
                StringType::Whitespace("  ")
            ]
        );
    }

    #[test]
    fn test_tabs_newlines_spaces_2() {
        assert_eq!(
            split_string("\t\tI love\r\n   Rust programming\rlanguage.  "),
            vec![
                StringType::Tab("\t"),
                StringType::Tab("\t"),
                StringType::Word("I"),
                StringType::Whitespace(" "),
                StringType::Word("love"),
                StringType::Newline("\r\n"),
                StringType::Whitespace("   "),
                StringType::Word("Rust"),
                StringType::Whitespace(" "),
                StringType::Word("programming"),
                StringType::Newline("\r"),
                StringType::Word("language."),
                StringType::Whitespace("  ")
            ]
        );
    }
}

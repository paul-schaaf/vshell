use arboard::Clipboard;

mod event;
mod tui;
mod update;
mod view;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    tui::install_panic_hook();

    let mut clipboard = Clipboard::new()?;
    let mut terminal = tui::init_terminal()?;
    let mut model = Model::default();

    while !model.should_quit() {
        terminal.draw(|frame| view::view(&model, frame))?;

        let event = event::wait_for_event();
        update::update(&mut model, event, &mut clipboard)?;
        if model.should_quit() {
            break;
        }
        while let Some(next_event) = event::get_event()? {
            update::update(&mut model, next_event, &mut clipboard)?;
        }
    }

    tui::restore_terminal()?;
    Ok(())
}

#[derive(Debug, PartialEq)]
enum StringType<'a> {
    Word(&'a str),
    Whitespace(&'a str),
    Tab,
    // can be \n or \r\n or \r
    Newline(&'a str),
}

impl<'a> StringType<'a> {
    fn as_str(&self) -> &str {
        match self {
            StringType::Word(s) => s,
            StringType::Whitespace(s) => s,
            StringType::Tab => "\t",
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
                    result.push(StringType::Tab);
                    last_index = index + 1; // update last_index to current index + 1 because we're out of the matched range
                }
                '\r' if matches!(chars.peek(), Some((_, '\n'))) => {
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

#[derive(Debug, PartialEq, Default)]
enum Mode {
    #[default]
    Idle,
    Editing(String),
    Selecting(String),
    JumpingBefore(String),
    JumpingAfter(String),
    Quit,
}

#[derive(Debug, PartialEq, Default)]
enum HintState {
    #[default]
    ShowHints,
    HideHints,
}

#[derive(Debug, PartialEq, Default)]
struct Config {
    hint_state: HintState,
}

#[derive(Debug, Clone, Default, PartialEq)]
enum Output {
    Success(String),
    Error(String),
    #[default]
    Empty,
}

impl Output {
    fn as_str(&self) -> &str {
        match self {
            Output::Success(output) => output.as_str(),
            Output::Error(output) => output.as_str(),
            Output::Empty => "",
        }
    }
}

#[derive(Debug, PartialEq, Default, Clone)]
struct CommandWithoutOutput {
    cursor_position: u64,
    input: String,
}

#[derive(Debug, PartialEq, Default, Clone)]
struct CompletedCommand {
    input: String,
    output: Output,
}

#[derive(Debug, PartialEq)]
enum CurrentView {
    CommandWithoutOutput(CommandWithoutOutput),
    Output(Output),
    CommandWithOutput(CompletedCommand),
}

impl CurrentView {
    fn input_str(&self) -> Option<&str> {
        match self {
            CurrentView::CommandWithoutOutput(command) => Some(command.input.as_str()),
            CurrentView::Output(_) => None,
            CurrentView::CommandWithOutput(command) => Some(command.input.as_str()),
        }
    }

    fn cursor_position(&self) -> Option<u64> {
        match self {
            CurrentView::CommandWithoutOutput(command) => Some(command.cursor_position),
            CurrentView::Output(_) => None,
            CurrentView::CommandWithOutput(command) => Some(command.input.len() as u64),
        }
    }
}

impl Default for CurrentView {
    fn default() -> Self {
        Self::CommandWithoutOutput(CommandWithoutOutput::default())
    }
}

#[derive(Debug, PartialEq, Default)]
struct Model {
    mode: Mode,
    config: Config,
    command_history: Vec<CompletedCommand>,
    command_history_index: usize,
    pinned_commands: Vec<CommandWithoutOutput>,
    current_command: CurrentView,
}

impl Model {
    fn should_quit(&self) -> bool {
        self.mode == Mode::Quit
    }

    fn set_current_view_from_command(&mut self, cursor_position: u64, command: String) {
        self.current_command = CurrentView::CommandWithoutOutput(CommandWithoutOutput {
            cursor_position,
            input: command,
        });
        self.command_history_index = self.command_history.len();
    }
}

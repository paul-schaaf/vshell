use std::{
    ffi::OsString,
    fmt,
    path::PathBuf,
    sync::{Arc, Mutex},
    thread::JoinHandle,
};

use arboard::Clipboard;
use ratatui::layout::Rect;

mod event;
mod tui;
mod update;
mod view;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    tui::install_panic_hook();
    let run_result = run();
    tui::restore_terminal()?;
    run_result
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let mut clipboard = Clipboard::new()?;
    let mut terminal = tui::init_terminal()?;
    let model = Arc::new(Mutex::new(Model::default()));
    // SAFETY: no one has panicked while holding the mutex yet since
    // this is the first access -> unwrap is ok
    model
        .lock()
        .unwrap()
        .directory_history
        .push(std::env::current_dir()?);
    model.lock().unwrap().config.hint_state = HintState::HideHints;

    loop {
        {
            let mut model = model.lock().map_err(|_| "lock failed")?;
            terminal.draw(|frame| view::view(&mut model, frame))?;
        }

        let model = Arc::clone(&model);
        let event = event::get_event()?;
        if let Some(event) = event {
            update::update(&model, event, &mut clipboard)?;
        }
        if model.lock().map_err(|_| "lock failed")?.should_quit() {
            break;
        }
        while let Some(next_event) = event::get_event()? {
            update::update(&model, next_event, &mut clipboard)?;
        }
        if model.lock().map_err(|_| "lock failed")?.should_quit() {
            break;
        }
    }

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

#[derive(Debug, Default)]
enum Mode {
    #[default]
    Idle,
    Command(String),
    Directory(Directory),
    Quit,
    Executing(
        bool,
        u16,
        std::sync::mpsc::Sender<()>,
        JoinHandle<std::io::Result<()>>,
    ),
}

#[derive(Debug, PartialEq, Default)]
pub struct Directory {
    search: String,
    path: Option<OsString>,
    current_dir: PathBuf,
    children: Vec<File>,
    location: Option<Rect>,
}

#[derive(Debug, PartialEq, Eq)]
enum File {
    Directory(OsString),
    File(OsString),
}

impl fmt::Display for File {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            File::Directory(s) => write!(f, "{}", s.to_string_lossy()),
            File::File(s) => write!(f, "{}", s.to_string_lossy()),
        }
    }
}

impl PartialOrd for File {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for File {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match (self, other) {
            (File::Directory(a), File::File(b)) => a.cmp(b),
            (File::File(a), File::Directory(b)) => a.cmp(b),
            (File::Directory(a), File::Directory(b)) => a.cmp(b),
            (File::File(a), File::File(b)) => a.cmp(b),
        }
    }
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
    history_type: HistoryType,
}

#[derive(Debug, PartialEq, Default)]
pub enum HistoryType {
    #[default]
    CommandHistory,
    DirectoryHistory,
}

#[derive(Debug, Clone, Default, PartialEq)]
struct Output {
    origin: Origin,
    output_type: OutputType,
}

impl fmt::Display for Output {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.output_type {
            OutputType::Success(stdout, stderr) | OutputType::Error(stdout, stderr) => {
                if stdout.is_empty() && stderr.is_empty() {
                    write!(f, "")
                } else if stdout.is_empty() {
                    write!(f, "{}", stderr)
                } else if stderr.is_empty() {
                    write!(f, "{}", stdout)
                } else {
                    write!(f, "STDERR:\n\n{}\nSTDOUT:\n\n{}", stderr, stdout)
                }
            }
            OutputType::Empty => write!(f, ""),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
enum OutputType {
    Success(String, String),
    Error(String, String),
    #[default]
    Empty,
}

#[derive(Debug, Clone, Default, PartialEq)]
enum Origin {
    #[default]
    Vshell,
    Other(String),
}

impl fmt::Display for Origin {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Vshell => write!(f, "vshell"),
            Self::Other(origin) => write!(f, "{}", origin),
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

impl CompletedCommand {
    fn new(
        input: String,
        output: Result<std::process::Output, std::io::Error>,
        origin: Origin,
    ) -> Self {
        CompletedCommand {
            input: input.clone(),
            output: {
                match output {
                    Ok(executed_command) => {
                        if executed_command.status.success() {
                            Output {
                                origin,
                                output_type: OutputType::Success(
                                    String::from_utf8_lossy(&executed_command.stdout).to_string(),
                                    String::from_utf8_lossy(&executed_command.stderr).to_string(),
                                ),
                            }
                        } else {
                            Output {
                                origin,
                                output_type: OutputType::Error(
                                    String::from_utf8_lossy(&executed_command.stdout).to_string(),
                                    String::from_utf8_lossy(&executed_command.stderr).to_string(),
                                ),
                            }
                        }
                    }
                    Err(executed_command) => {
                        if executed_command.kind() == std::io::ErrorKind::NotFound {
                            Output {
                                origin,
                                output_type: OutputType::Error(
                                    "".to_string(),
                                    format!("Command not found: {}", input),
                                ),
                            }
                        } else {
                            Output {
                                origin,
                                output_type: OutputType::Error(
                                    "".to_string(),
                                    executed_command.to_string(),
                                ),
                            }
                        }
                    }
                }
            },
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
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

#[derive(Debug, Default)]
struct Model {
    mode: Mode,
    config: Config,
    command_history: Vec<CompletedCommand>,
    command_history_index: usize,
    directory_history: Vec<PathBuf>,
    pinned_commands: Vec<CommandWithoutOutput>,
    current_command: CurrentView,
}

impl Model {
    fn should_quit(&self) -> bool {
        matches!(self.mode, Mode::Quit)
    }

    fn set_current_view_from_command(&mut self, cursor_position: u64, command: String) {
        self.current_command = CurrentView::CommandWithoutOutput(CommandWithoutOutput {
            cursor_position,
            input: command,
        });
        self.command_history_index = self.command_history.len();
    }

    fn add_current_directory_to_history(&mut self) -> Result<(), std::io::Error> {
        let current_directory = std::env::current_dir();
        if current_directory.is_err() {
            return Ok(());
        }
        let current_directory = current_directory.unwrap();
        // SAFETY: we add the initial directory on startup so there must be a last directory
        if current_directory != *self.directory_history.last().unwrap() {
            self.directory_history.push(current_directory);
        }
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn sort_files() {
        let mut files = vec![
            File::File(OsString::from("b")),
            File::Directory(OsString::from("a")),
            File::Directory(OsString::from("c")),
            File::File(OsString::from("a")),
        ];
        files.sort();
        assert_eq!(
            files,
            vec![
                File::Directory(OsString::from("a")),
                File::File(OsString::from("a")),
                File::File(OsString::from("b")),
                File::Directory(OsString::from("c")),
            ]
        );
    }
}

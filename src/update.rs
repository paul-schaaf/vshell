use arboard::Clipboard;

use crate::{
    event, split_string, CommandWithoutOutput, CompletedCommand, CurrentView, HintState, Mode,
    Model, Origin, Output, OutputType, StringType,
};

fn base26_to_base10(input: &str) -> Result<u32, &'static str> {
    let mut result = 0;
    for (i, c) in input.chars().rev().enumerate() {
        let value = match c {
            'a'..='z' => c as u32 - 'a' as u32,
            _ => return Err("Invalid Character"), // Invalid character
        };
        result += value * 26u32.pow(i as u32);
    }
    Ok(result)
}

enum Command {
    Quit,
    Edit(Edit),
    Select(Option<usize>),
    JumpBefore(String),
    JumpAfter(String),
    Pin,
    CopyOutput,
    Paste,
    ToggleHints,
    ShellExecute(String),
    Replace(Replace),
}

enum Replace {
    Single(String, String),
    Global(String, String),
}

enum Edit {
    Single(String),
    Range(String, String),
}

impl TryFrom<&str> for Command {
    type Error = &'static str;

    fn try_from(input: &str) -> Result<Self, Self::Error> {
        fn create_replace_string<'a>(
            split_input: &'a [&str],
        ) -> Result<Vec<String>, <Command as TryFrom<&'a str>>::Error> {
            fn split_string(s: &str) -> Vec<String> {
                let mut result = Vec::new();
                let mut segment = String::new();
                let mut escaped = false;

                for c in s.chars() {
                    if c == '\\' && !escaped {
                        escaped = true;
                    } else if c == ',' && !escaped {
                        result.push(segment.clone());
                        segment.clear();
                    } else {
                        segment.push(c);
                        escaped = false;
                    }
                }

                result.push(segment);

                result
            }

            if split_input.len() < 2 {
                return Err("Invalid Command");
            }
            let replace_args = split_string(&split_input[1..].join(""));
            if replace_args.len() != 2 {
                return Err("Invalid Command");
            }
            Ok(replace_args)
        }
        if input.is_empty() {
            return Err("Empty Command");
        }
        let split_input = input.split(':').collect::<Vec<&str>>();
        if split_input.len() > 2 {
            return Err("Invalid Command");
        }
        match split_input[0] {
            "q" | "quit" => Ok(Command::Quit),
            "c" | "change" => {
                if split_input.len() != 2 {
                    return Err("Invalid Command");
                }
                match split_input[1].contains(',') {
                    true => {
                        let mut hints = split_input[1].split(',');
                        let beginning_hint = hints.next().unwrap();
                        let end_hint = hints.next().unwrap();
                        if hints.next().is_some() {
                            return Err("Invalid Command");
                        }
                        let mut beginning = String::new();
                        for c in beginning_hint.chars() {
                            if c.is_alphabetic() {
                                beginning.push(c);
                            } else {
                                return Err("Invalid Character");
                            }
                        }
                        if beginning.is_empty() {
                            return Err("Missing hints");
                        }
                        let mut end = String::new();
                        for c in end_hint.chars() {
                            if c.is_alphabetic() {
                                end.push(c);
                            } else {
                                return Err("Invalid Character");
                            }
                        }
                        if end.is_empty() {
                            return Err("Missing hints");
                        }
                        Ok(Command::Edit(Edit::Range(beginning, end)))
                    }
                    false => {
                        let mut hint = String::new();
                        for c in split_input[1].chars() {
                            if c.is_alphabetic() {
                                hint.push(c);
                            } else {
                                return Err("Invalid Character");
                            }
                        }
                        if hint.is_empty() {
                            return Err("Missing hints");
                        }
                        Ok(Command::Edit(Edit::Single(hint)))
                    }
                }
            }
            "s" | "select" => {
                if split_input.len() != 2 {
                    return Ok(Command::Select(None));
                }
                let mut target = String::new();
                for c in split_input[1].chars() {
                    if c.is_ascii_digit() {
                        target.push(c);
                    } else {
                        return Err("Invalid Character");
                    }
                }
                Ok(Command::Select(Some(
                    target.parse::<usize>().map_err(|_| "Invalid Number")?,
                )))
            }
            "jb" | "jumpbefore" => {
                if split_input.len() != 2 {
                    return Ok(Command::JumpBefore(String::new()));
                }
                let mut hint = String::new();
                for c in split_input[1].chars() {
                    if c.is_alphabetic() {
                        hint.push(c);
                    } else {
                        return Err("Invalid Character");
                    }
                }
                Ok(Command::JumpBefore(hint))
            }
            "ja" | "jumpafter" => {
                if split_input.len() != 2 {
                    return Ok(Command::JumpAfter(String::new()));
                }
                let mut hint = String::new();
                for c in split_input[1].chars() {
                    if c.is_alphabetic() {
                        hint.push(c);
                    } else {
                        return Err("Invalid Character");
                    }
                }
                Ok(Command::JumpAfter(hint))
            }
            "pin" => Ok(Command::Pin),
            "p" | "paste" => Ok(Command::Paste),
            "co" | "copyoutput" => Ok(Command::CopyOutput),
            "th" | "togglehints" => Ok(Command::ToggleHints),
            "se" | "shellexecute" => {
                if split_input.len() != 2 {
                    return Err("Invalid Command");
                }

                Ok(Command::ShellExecute(split_input[1].to_string()))
            }
            "rg" | "replaceglobal" => {
                let mut replace_args = create_replace_string(&split_input)?;

                Ok(Command::Replace(Replace::Global(
                    replace_args.remove(0),
                    replace_args.remove(0),
                )))
            }
            "rs" | "replacesingle" => {
                let mut replace_args = create_replace_string(&split_input)?;

                Ok(Command::Replace(Replace::Single(
                    replace_args.remove(0),
                    replace_args.remove(0),
                )))
            }
            _ => Err("Invalid Command"),
        }
    }
}

pub(crate) fn update(
    model: &mut Model,
    event: event::Event,
    clipboard: &mut Clipboard,
) -> Result<(), Box<dyn std::error::Error>> {
    fn has_open_quote(s: &str) -> Option<char> {
        let mut single_quote_open = false;
        let mut double_quote_open = false;
        let mut escape = false;

        for c in s.chars() {
            match c {
                '\'' => {
                    if !double_quote_open && !escape {
                        single_quote_open = !single_quote_open;
                    }
                    escape = false; // Reset escape flag
                }
                '\"' => {
                    if !single_quote_open && !escape {
                        double_quote_open = !double_quote_open;
                    }
                    escape = false; // Reset escape flag
                }
                '\\' => {
                    escape = !escape; // Toggle escape flag
                }
                _ => {
                    escape = false; // Reset escape flag for other characters
                }
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

    if event == event::Event::CtrlC {
        model.mode = Mode::Quit;
        return Ok(());
    }

    fn execute_command(command_input: &str) -> CompletedCommand {
        // SAFETY: our shell handles input validation so this will not fail
        let command_list = shlex::split(command_input).unwrap();

        if command_list[0] == "cd" {
            if command_list.len() == 1 {
                match dirs::home_dir() {
                    Some(home) => match std::env::set_current_dir(home) {
                        Ok(_) => CompletedCommand {
                            input: command_input.to_string(),
                            output: Output {
                                origin: Origin::Vshell,
                                output_type: OutputType::Success(String::new()),
                            },
                        },
                        Err(e) => CompletedCommand {
                            input: command_input.to_string(),
                            output: Output {
                                origin: Origin::Vshell,
                                output_type: OutputType::Error(format!("cd: {}", e)),
                            },
                        },
                    },
                    None => CompletedCommand {
                        input: command_input.to_string(),
                        output: Output {
                            origin: Origin::Vshell,
                            output_type: OutputType::Error(
                                "cd: could not find home directory".to_string(),
                            ),
                        },
                    },
                }
            } else if command_list.len() != 2 {
                CompletedCommand {
                    input: command_input.to_string(),
                    output: Output {
                        origin: Origin::Vshell,
                        output_type: OutputType::Error(
                            "cd: incorrect number of arguments".to_string(),
                        ),
                    },
                }
            } else if command_list[1].contains('~') {
                match dirs::home_dir() {
                    Some(home) => {
                        let new_path = command_list[1].replace('~', &home.to_string_lossy());
                        match std::env::set_current_dir(new_path) {
                            Ok(_) => CompletedCommand {
                                input: command_input.to_string(),
                                output: Output {
                                    origin: Origin::Vshell,
                                    output_type: OutputType::Success(String::new()),
                                },
                            },
                            Err(e) => CompletedCommand {
                                input: command_input.to_string(),
                                output: Output {
                                    origin: Origin::Vshell,
                                    output_type: OutputType::Error(format!("cd: {}", e)),
                                },
                            },
                        }
                    }
                    None => CompletedCommand {
                        input: command_input.to_string(),
                        output: Output {
                            origin: Origin::Vshell,
                            output_type: OutputType::Error(
                                "cd: could not find home directory".to_string(),
                            ),
                        },
                    },
                }
            } else {
                match std::env::set_current_dir(&command_list[1]) {
                    Ok(_) => CompletedCommand {
                        input: command_input.to_string(),
                        output: Output {
                            origin: Origin::Vshell,
                            output_type: OutputType::Success(String::new()),
                        },
                    },
                    Err(e) => CompletedCommand {
                        input: command_input.to_string(),
                        output: Output {
                            origin: Origin::Vshell,
                            output_type: OutputType::Error(format!("cd: {}", e)),
                        },
                    },
                }
            }
        } else {
            let executed_command = std::process::Command::new(&command_list[0])
                .args(
                    &command_list[1..]
                        .iter()
                        .filter(|s| !s.is_empty())
                        .collect::<Vec<&String>>(),
                )
                .output();
            CompletedCommand::new(command_input.to_string(), executed_command, Origin::Vshell)
        }
    }

    match &mut model.mode {
        Mode::Idle => match event {
            event::Event::CtrlC => {
                model.mode = Mode::Quit;
                Ok(())
            }
            event::Event::Backspace => {
                match &mut model.current_command {
                    CurrentView::CommandWithoutOutput(command) => {
                        if command.cursor_position > 0 {
                            command.input.remove(command.cursor_position as usize - 1);
                            command.cursor_position -= 1;
                        }
                    }
                    CurrentView::CommandWithOutput(command) => {
                        let mut command = command.input.clone();
                        command.pop();

                        model.set_current_view_from_command(command.len() as u64, command);
                    }
                    CurrentView::Output(_) => {
                        // do nothing
                    }
                };
                Ok(())
            }
            event::Event::Esc => {
                model.mode = Mode::Command(String::new());
                Ok(())
            }
            event::Event::Enter => {
                match &mut model.current_command {
                    CurrentView::CommandWithoutOutput(command) => {
                        if command.input.is_empty() {
                            return Ok(());
                        }
                        if has_open_quote(&command.input).is_some() {
                            command.input.push('\n');
                            command.cursor_position = command.input.len() as u64;
                            return Ok(());
                        }
                        // SAFETY: we just checked for empty so there must be at least 1 char
                        if command.input.ends_with('\\') {
                            command.input.push('\n');
                            command.cursor_position = command.input.len() as u64;
                            return Ok(());
                        }

                        let completed_command = execute_command(command.input.as_str());
                        model.command_history.push(completed_command.clone());
                        model.current_command =
                            CurrentView::Output(completed_command.output.clone());
                    }
                    CurrentView::CommandWithOutput(command) => {
                        let completed_command = execute_command(command.input.as_str());
                        model.command_history.push(completed_command.clone());
                        model.current_command =
                            CurrentView::Output(completed_command.output.clone());
                    }
                    CurrentView::Output(_) => {
                        // do nothing
                    }
                };
                model.command_history_index = model.command_history.len();
                Ok(())
            }
            event::Event::Up => {
                if model.command_history_index > 0 {
                    model.command_history_index -= 1;
                    let completed_command = &model.command_history[model.command_history_index];
                    model.current_command =
                        CurrentView::CommandWithOutput(completed_command.clone());
                }
                Ok(())
            }
            event::Event::Down => {
                if !model.command_history.is_empty()
                    && model.command_history_index < model.command_history.len() - 1
                {
                    model.command_history_index += 1;
                    let completed_command = &model.command_history[model.command_history_index];
                    model.current_command =
                        CurrentView::CommandWithOutput(completed_command.clone());
                } else {
                    model.set_current_view_from_command(0, String::new());
                }
                Ok(())
            }
            event::Event::Character(c) => {
                match &mut model.current_command {
                    CurrentView::CommandWithoutOutput(command) => {
                        command.input.insert(command.cursor_position as usize, c);

                        command.cursor_position += 1;
                    }
                    CurrentView::CommandWithOutput(command) => {
                        let mut command = command.input.clone();
                        command.push(c);
                        model.current_command =
                            CurrentView::CommandWithoutOutput(CommandWithoutOutput {
                                cursor_position: command.len() as u64,
                                input: command,
                            });
                        model.command_history_index = model.command_history.len();
                    }
                    CurrentView::Output(_) => {
                        model.current_command =
                            CurrentView::CommandWithoutOutput(CommandWithoutOutput {
                                cursor_position: 1,
                                input: String::from(c),
                            });
                        model.command_history_index = model.command_history.len();
                    }
                };
                Ok(())
            }

            event::Event::Left => {
                match &mut model.current_command {
                    CurrentView::CommandWithoutOutput(command) => {
                        if command.cursor_position > 0 {
                            command.cursor_position -= 1;
                        }
                        Ok(())
                    }
                    CurrentView::CommandWithOutput(command) => {
                        model.current_command =
                            CurrentView::CommandWithoutOutput(CommandWithoutOutput {
                                // SAFETY: this is a command with output so it has completed
                                // which means its input must have had at least length one
                                cursor_position: command.input.len() as u64 - 1,
                                input: command.input.clone(),
                            });
                        model.command_history_index = model.command_history.len();
                        Ok(())
                    }
                    CurrentView::Output(_) => {
                        // do nothing
                        Ok(())
                    }
                }
            }
            event::Event::Right => {
                match &mut model.current_command {
                    CurrentView::CommandWithoutOutput(command) => {
                        if command.cursor_position < command.input.len() as u64 {
                            command.cursor_position += 1;
                        }
                        Ok(())
                    }
                    CurrentView::CommandWithOutput(command) => {
                        model.current_command =
                            CurrentView::CommandWithoutOutput(CommandWithoutOutput {
                                cursor_position: command.input.len() as u64,
                                input: command.input.clone(),
                            });
                        model.command_history_index = model.command_history.len();
                        Ok(())
                    }
                    CurrentView::Output(_) => {
                        // do nothing
                        Ok(())
                    }
                }
            }
        },

        // SAFETY: if Mode::QUIT has been set, the program will already have exited before it reaches this point
        Mode::Quit => unreachable!(),
        Mode::Command(command) => match event {
            event::Event::CtrlC => {
                model.mode = Mode::Quit;
                Ok(())
            }
            event::Event::Esc => {
                model.mode = Mode::Idle;
                Ok(())
            }
            event::Event::Character(c) => {
                command.push(c);
                Ok(())
            }
            event::Event::Backspace => {
                command.pop();
                Ok(())
            }
            event::Event::Enter => {
                let command = Command::try_from(command.as_str());
                if command.is_err() {
                    return Ok(());
                }
                let command = command.unwrap();
                match command {
                    Command::Quit => {
                        model.mode = Mode::Quit;
                        Ok(())
                    }
                    Command::Edit(edit) => {
                        match &model.current_command {
                            CurrentView::CommandWithOutput(command) => {
                                model.set_current_view_from_command(
                                    command.input.len() as u64,
                                    command.input.clone(),
                                );
                            }
                            CurrentView::Output(_) => {
                                // do nothing
                                return Ok(());
                            }
                            _ => {}
                        }

                        match &model.current_command {
                            CurrentView::CommandWithoutOutput(command) => {
                                match edit {
                                    Edit::Single(hint) => {
                                        let index = base26_to_base10(&hint);
                                        if index.is_err() {
                                            return Ok(());
                                        }
                                        // SAFETY: just checked for err
                                        let index = index.unwrap();
                                        model.mode = Mode::Idle;
                                        let mut split_command = split_string(&command.input);
                                        let mut current = 0;
                                        let mut new_cursor_position = 0;
                                        let mut index_to_delete = None;
                                        for (real_index, element) in
                                            split_command.iter().enumerate()
                                        {
                                            match element {
                                                StringType::Word(w) => {
                                                    if current == index {
                                                        index_to_delete = Some(real_index);
                                                        break;
                                                    }
                                                    current += 1;
                                                    new_cursor_position += w.len() as u64;
                                                }
                                                StringType::Newline(c)
                                                | StringType::Whitespace(c) => {
                                                    new_cursor_position += c.len() as u64;
                                                }
                                                StringType::Tab => {
                                                    new_cursor_position += 1;
                                                }
                                            }
                                        }
                                        if let Some(index_to_delete) = index_to_delete {
                                            split_command.remove(index_to_delete);

                                            let new_command = split_command
                                                .iter()
                                                .map(|s| s.as_str())
                                                .collect::<Vec<&str>>()
                                                .join("");

                                            model.current_command =
                                                CurrentView::CommandWithoutOutput(
                                                    CommandWithoutOutput {
                                                        cursor_position: new_cursor_position,
                                                        input: new_command,
                                                    },
                                                );
                                        }
                                        Ok(())
                                    }
                                    Edit::Range(beginning, end) => {
                                        let beginning_index = base26_to_base10(&beginning);
                                        if beginning_index.is_err() {
                                            return Ok(());
                                        }
                                        let end_index = base26_to_base10(&end);
                                        if end_index.is_err() {
                                            return Ok(());
                                        }
                                        model.mode = Mode::Idle;
                                        // SAFETY: just checked for none
                                        let beginning_index = beginning_index.unwrap();
                                        // SAFETY: just checked for none
                                        let end_index = end_index.unwrap();
                                        if end_index < beginning_index {
                                            return Ok(());
                                        }
                                        let mut split_command = split_string(&command.input);
                                        let mut current = 0;
                                        let mut new_cursor_position = 0;
                                        let mut indices_to_delete = Vec::new();
                                        for (real_index, element) in
                                            split_command.iter().enumerate()
                                        {
                                            match element {
                                                StringType::Word(w) => {
                                                    if current == beginning_index
                                                        || current == end_index
                                                    {
                                                        indices_to_delete.push(real_index);
                                                    }
                                                    if current == end_index {
                                                        break;
                                                    }
                                                    current += 1;
                                                    if current <= beginning_index {
                                                        new_cursor_position += w.len() as u64;
                                                    }
                                                }
                                                StringType::Newline(c)
                                                | StringType::Whitespace(c) => {
                                                    if current <= beginning_index {
                                                        new_cursor_position += c.len() as u64;
                                                    }
                                                }
                                                StringType::Tab => {
                                                    if current <= beginning_index {
                                                        new_cursor_position += 1;
                                                    }
                                                }
                                            }
                                        }
                                        if indices_to_delete.is_empty() {
                                            return Ok(());
                                        }
                                        split_command
                                            .drain(indices_to_delete[0]..=indices_to_delete[1]);
                                        let new_command = split_command
                                            .iter()
                                            .map(|s| s.as_str())
                                            .collect::<Vec<&str>>()
                                            .join("");
                                        model.current_command = CurrentView::CommandWithoutOutput(
                                            CommandWithoutOutput {
                                                cursor_position: new_cursor_position,
                                                input: new_command,
                                            },
                                        );
                                        Ok(())
                                    }
                                }
                            }
                            _ => unreachable!(),
                        }
                    }
                    Command::Select(number) => {
                        if model.command_history.is_empty() {
                            return Ok(());
                        }

                        if number.is_none() {
                            model.set_current_view_from_command(0, String::new());
                            model.mode = Mode::Idle;
                            return Ok(());
                        }

                        let number = number.unwrap();
                        if number < model.command_history.len() + model.pinned_commands.len() {
                            if number < model.pinned_commands.len() {
                                let completed_command = &model.pinned_commands[number];
                                model.current_command =
                                    CurrentView::CommandWithoutOutput(CommandWithoutOutput {
                                        input: completed_command.input.clone(),
                                        cursor_position: completed_command.cursor_position,
                                    });
                                model.command_history_index = model.command_history.len();
                            } else {
                                let index = model.command_history.len()
                                    + model.pinned_commands.len()
                                    - number
                                    - 1;
                                let completed_command = &model.command_history[index];
                                model.set_current_view_from_command(
                                    completed_command.input.len() as u64,
                                    completed_command.input.clone(),
                                );
                            };
                        }
                        model.mode = Mode::Idle;
                        Ok(())
                    }
                    Command::JumpBefore(hint) => {
                        match &model.current_command {
                            CurrentView::CommandWithOutput(c) => {
                                model.set_current_view_from_command(
                                    c.input.len() as u64,
                                    c.input.clone(),
                                );
                            }
                            CurrentView::Output(_) => {
                                // do nothing
                                return Ok(());
                            }
                            _ => {}
                        }

                        if hint.is_empty() {
                            match &mut model.current_command {
                                CurrentView::CommandWithoutOutput(command) => {
                                    command.cursor_position = command.input.len() as u64;
                                }
                                _ => unreachable!(),
                            }
                            model.mode = Mode::Idle;
                            return Ok(());
                        }
                        // we only accept digits so this must be a valid usize (unless it's too large, that is acceptable)
                        let index = base26_to_base10(&hint)?;
                        model.mode = Mode::Idle;
                        // SAFETY: Jumping Modes can only be entered if command has an input string
                        let split_command =
                            split_string(model.current_command.input_str().unwrap());
                        let mut current = 0;
                        let mut new_cursor_position = 0;
                        for element in split_command.iter() {
                            match element {
                                StringType::Word(w) => {
                                    if current == index {
                                        break;
                                    }
                                    current += 1;
                                    new_cursor_position += w.len() as u64;
                                }
                                StringType::Newline(c) | StringType::Whitespace(c) => {
                                    new_cursor_position += c.len() as u64;
                                }
                                StringType::Tab => {
                                    new_cursor_position += 1;
                                }
                            }
                        }
                        match &mut model.current_command {
                            CurrentView::CommandWithoutOutput(command) => {
                                command.cursor_position = new_cursor_position;
                                Ok(())
                            }
                            _ => unreachable!(),
                        }
                    }
                    Command::JumpAfter(hint) => {
                        match &model.current_command {
                            CurrentView::CommandWithOutput(c) => {
                                model.set_current_view_from_command(
                                    c.input.len() as u64,
                                    c.input.clone(),
                                );
                            }
                            CurrentView::Output(_) => {
                                // do nothing
                                return Ok(());
                            }
                            _ => {}
                        }
                        if hint.is_empty() {
                            match &mut model.current_command {
                                CurrentView::CommandWithoutOutput(command) => {
                                    command.cursor_position = command.input.len() as u64;
                                }
                                _ => unreachable!(),
                            }
                            model.mode = Mode::Idle;
                            return Ok(());
                        }
                        let index = base26_to_base10(&hint)?;
                        model.mode = Mode::Idle;
                        // SAFETY: Jumping Modes can only be entered if command has an input string
                        let split_command =
                            split_string(model.current_command.input_str().unwrap());
                        let mut current = 0;
                        let mut new_cursor_position = 0;
                        for element in split_command.iter() {
                            match element {
                                StringType::Word(w) => {
                                    if current == index {
                                        new_cursor_position += w.len() as u64;
                                        break;
                                    }
                                    current += 1;
                                    new_cursor_position += w.len() as u64;
                                }
                                StringType::Newline(c) | StringType::Whitespace(c) => {
                                    new_cursor_position += c.len() as u64;
                                }
                                StringType::Tab => {
                                    new_cursor_position += 1;
                                }
                            }
                        }
                        match &mut model.current_command {
                            CurrentView::CommandWithoutOutput(command) => {
                                command.cursor_position = new_cursor_position;
                            }
                            _ => unreachable!(),
                        }
                        Ok(())
                    }
                    Command::Pin => {
                        model.mode = Mode::Idle;
                        match &model.current_command {
                            CurrentView::CommandWithoutOutput(c) => {
                                if c.input.is_empty() {
                                    return Ok(());
                                }
                                let position = model
                                    .pinned_commands
                                    .iter()
                                    .map(|pinned_command| pinned_command.input.clone())
                                    .position(|past_command| past_command == c.input);
                                if let Some(position) = position {
                                    model.pinned_commands.remove(position);
                                    Ok(())
                                } else {
                                    model.pinned_commands.push(CommandWithoutOutput {
                                        input: c.input.clone(),
                                        cursor_position: c.cursor_position,
                                    });
                                    Ok(())
                                }
                            }
                            CurrentView::Output(_) => {
                                // do nothing
                                Ok(())
                            }
                            CurrentView::CommandWithOutput(c) => {
                                if c.input.is_empty() {
                                    return Ok(());
                                }
                                let position = model
                                    .pinned_commands
                                    .iter()
                                    .map(|pinned_command| pinned_command.input.clone())
                                    .position(|past_command| past_command == c.input);
                                if let Some(position) = position {
                                    model.pinned_commands.remove(position);
                                    Ok(())
                                } else {
                                    model.pinned_commands.push(CommandWithoutOutput {
                                        input: c.input.clone(),
                                        cursor_position: c.input.len() as u64,
                                    });
                                    Ok(())
                                }
                            }
                        }
                    }
                    Command::CopyOutput => {
                        model.mode = Mode::Idle;
                        match model.current_command {
                            CurrentView::CommandWithoutOutput(_) => {
                                // do nothing
                                Ok(())
                            }
                            CurrentView::CommandWithOutput(ref command) => {
                                clipboard.set_text(command.output.as_str())?;
                                Ok(())
                            }
                            CurrentView::Output(ref command) => {
                                clipboard.set_text(command.as_str())?;
                                Ok(())
                            }
                        }
                    }
                    Command::ToggleHints => {
                        model.config.hint_state = match model.config.hint_state {
                            HintState::ShowHints => HintState::HideHints,
                            HintState::HideHints => HintState::ShowHints,
                        };
                        model.mode = Mode::Idle;
                        Ok(())
                    }
                    Command::Paste => {
                        model.mode = Mode::Idle;
                        match &model.current_command {
                            CurrentView::CommandWithoutOutput(command) => {
                                let text_to_insert = clipboard.get_text()?;
                                if command.cursor_position == command.input.len() as u64 {
                                    let new_command =
                                        format!("{}{}", command.input, text_to_insert);
                                    model.current_command =
                                        CurrentView::CommandWithoutOutput(CommandWithoutOutput {
                                            input: new_command,
                                            cursor_position: command.cursor_position
                                                + text_to_insert.len() as u64,
                                        });
                                    Ok(())
                                } else {
                                    let (first, second) =
                                        command.input.split_at(command.cursor_position as usize);
                                    let new_command =
                                        format!("{}{}{}", first, text_to_insert, second);
                                    model.current_command =
                                        CurrentView::CommandWithoutOutput(CommandWithoutOutput {
                                            input: new_command,
                                            cursor_position: command.cursor_position
                                                + text_to_insert.len() as u64,
                                        });
                                    Ok(())
                                }
                            }
                            CurrentView::CommandWithOutput(command) => {
                                let text_to_insert = clipboard.get_text()?;
                                let new_command = format!("{}{}", command.input, text_to_insert);
                                model.current_command =
                                    CurrentView::CommandWithoutOutput(CommandWithoutOutput {
                                        input: new_command,
                                        cursor_position: command.input.len() as u64
                                            + text_to_insert.len() as u64,
                                    });
                                Ok(())
                            }
                            CurrentView::Output(_) => {
                                let new_command = clipboard.get_text()?;
                                model.current_command =
                                    CurrentView::CommandWithoutOutput(CommandWithoutOutput {
                                        cursor_position: new_command.len() as u64,
                                        input: new_command,
                                    });
                                model.command_history_index = model.command_history.len();
                                Ok(())
                            }
                        }
                    }
                    Command::ShellExecute(shell) => {
                        fn executed_shell_command(shell: &str, command: &str) -> CompletedCommand {
                            let executed_command = std::process::Command::new(shell)
                                .arg("-c")
                                .arg(command)
                                .output();

                            CompletedCommand::new(
                                command.to_string(),
                                executed_command,
                                Origin::Other(shell.to_string()),
                            )
                        }

                        model.mode = Mode::Idle;
                        match &mut model.current_command {
                            CurrentView::CommandWithoutOutput(command) => {
                                if command.input.is_empty() {
                                    return Ok(());
                                }
                                if has_open_quote(&command.input).is_some() {
                                    command.input.push('\n');
                                    command.cursor_position = command.input.len() as u64;
                                    return Ok(());
                                }
                                // SAFETY: we just checked for empty so there must be at least 1 char
                                if command.input.ends_with('\\') {
                                    command.input.push('\n');
                                    command.cursor_position = command.input.len() as u64;
                                    return Ok(());
                                }

                                let completed_command =
                                    executed_shell_command(&shell, command.input.as_str());
                                model.command_history.push(completed_command.clone());
                                model.current_command =
                                    CurrentView::Output(completed_command.output.clone());
                            }
                            CurrentView::CommandWithOutput(command) => {
                                let completed_command =
                                    executed_shell_command(&shell, command.input.as_str());
                                model.command_history.push(completed_command.clone());
                                model.current_command =
                                    CurrentView::Output(completed_command.output.clone());
                            }
                            CurrentView::Output(_) => {
                                // do nothing
                            }
                        };
                        model.command_history_index = model.command_history.len();
                        Ok(())
                    }
                    Command::Replace(replace) => match replace {
                        Replace::Single(from, to) => match &model.current_command {
                            CurrentView::CommandWithoutOutput(c) => {
                                let (first, last) = c.input.split_at(c.cursor_position as usize);

                                let (new_cursor_position, new_command) = if last.contains(&from) {
                                    (
                                        c.cursor_position,
                                        format!("{}{}", first, last.replacen(&from, &to, 1)),
                                    )
                                } else if first.contains(&from) {
                                    let difference = to.len() as i64 - from.len() as i64;
                                    let new_command =
                                        format!("{}{}", first.replacen(&from, &to, 1), last);
                                    (
                                        // SAFETY: this conversion to u64 is fine because
                                        // given that we split by cursor position
                                        //  we know that the difference can only be
                                        // as large as the cursor position itself
                                        (c.cursor_position as i64 + difference) as u64,
                                        new_command,
                                    )
                                } else if c.input.contains(&from) {
                                    // this else if happens if the cursor is inside the
                                    // word that is being searched for
                                    let new_command = c.input.replacen(&from, &to, 1);
                                    (new_command.len() as u64, new_command)
                                } else {
                                    (c.cursor_position, format!("{}{}", first, last))
                                };

                                model.current_command =
                                    CurrentView::CommandWithoutOutput(CommandWithoutOutput {
                                        cursor_position: new_cursor_position,
                                        input: new_command,
                                    });
                                model.mode = Mode::Idle;
                                Ok(())
                            }
                            CurrentView::Output(_) => {
                                model.mode = Mode::Idle;
                                Ok(())
                            }
                            CurrentView::CommandWithOutput(c) => {
                                let new_command = c.input.replacen(&from, &to, 1);
                                model.set_current_view_from_command(
                                    new_command.len() as u64,
                                    new_command,
                                );
                                model.mode = Mode::Idle;
                                Ok(())
                            }
                        },
                        Replace::Global(from, to) => match &model.current_command {
                            CurrentView::CommandWithoutOutput(c) => {
                                let new_command = c.input.replace(&from, &to);
                                model.current_command =
                                    CurrentView::CommandWithoutOutput(CommandWithoutOutput {
                                        cursor_position: new_command.len() as u64,
                                        input: new_command,
                                    });
                                model.mode = Mode::Idle;
                                Ok(())
                            }
                            CurrentView::Output(_) => {
                                model.mode = Mode::Idle;
                                Ok(())
                            }
                            CurrentView::CommandWithOutput(c) => {
                                let new_command = c.input.clone().replace(&from, &to);
                                model.set_current_view_from_command(
                                    new_command.len() as u64,
                                    new_command,
                                );
                                model.mode = Mode::Idle;
                                Ok(())
                            }
                        },
                    },
                }
            }
            _ => {
                // do nothing
                Ok(())
            }
        },
    }
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn test_base26_to_base10() {
        assert_eq!(base26_to_base10("a"), Ok(0))
    }
}

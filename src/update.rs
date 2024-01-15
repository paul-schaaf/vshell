use arboard::Clipboard;

use crate::{
    event, split_string, CommandWithoutOutput, CompletedCommand, CurrentView, HintState, Mode,
    Model, Output, StringType,
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

pub fn update(
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

    match &mut model.mode {
        Mode::Idle => match event {
            event::Event::CtrlC => {
                model.mode = Mode::Quit;
                Ok(())
            }
            event::Event::CtrlE => {
                match &model.current_command {
                    CurrentView::CommandWithoutOutput(command) => {
                        if !command.input.is_empty() {
                            model.mode = Mode::Editing(String::new());
                        }
                        Ok(())
                    }
                    CurrentView::CommandWithOutput(command) => {
                        model.mode = Mode::Editing(String::new());
                        model.set_current_view_from_command(
                            command.input.len() as u64,
                            command.input.clone(),
                        );
                        Ok(())
                    }
                    CurrentView::Output(_) => {
                        // do nothing
                        Ok(())
                    }
                }
            }
            event::Event::CtrlH => {
                model.config.hint_state = match model.config.hint_state {
                    HintState::ShowHints => HintState::HideHints,
                    HintState::HideHints => HintState::ShowHints,
                };
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
                // do nothing
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

                        // SAFETY: our shell handles input validation so this will not fail
                        let command_list = shlex::split(&command.input).unwrap();

                        let completed_command = if command_list[0] == "cd" {
                            if command_list.len() == 1 {
                                match dirs::home_dir() {
                                    Some(home) => match std::env::set_current_dir(home) {
                                        Ok(_) => CompletedCommand {
                                            input: command.input.clone(),
                                            output: Output::Success(String::new()),
                                        },
                                        Err(e) => CompletedCommand {
                                            input: command.input.clone(),
                                            output: Output::Error(format!("cd: {}", e)),
                                        },
                                    },
                                    None => CompletedCommand {
                                        input: command.input.clone(),
                                        output: Output::Error(
                                            "cd: could not find home directory".to_string(),
                                        ),
                                    },
                                }
                            } else if command_list.len() != 2 {
                                CompletedCommand {
                                    input: command.input.clone(),
                                    output: Output::Error(
                                        "cd: incorrect number of arguments".to_string(),
                                    ),
                                }
                            } else if command_list[1].contains('~') {
                                match dirs::home_dir() {
                                    Some(home) => {
                                        let new_path =
                                            command_list[1].replace('~', &home.to_string_lossy());
                                        match std::env::set_current_dir(new_path) {
                                            Ok(_) => CompletedCommand {
                                                input: command.input.clone(),
                                                output: Output::Success(String::new()),
                                            },
                                            Err(e) => CompletedCommand {
                                                input: command.input.clone(),
                                                output: Output::Error(format!("cd: {}", e)),
                                            },
                                        }
                                    }
                                    None => CompletedCommand {
                                        input: command.input.clone(),
                                        output: Output::Error(
                                            "cd: could not find home directory".to_string(),
                                        ),
                                    },
                                }
                            } else {
                                match std::env::set_current_dir(&command_list[1]) {
                                    Ok(_) => CompletedCommand {
                                        input: command.input.clone(),
                                        output: Output::Success(String::new()),
                                    },
                                    Err(e) => CompletedCommand {
                                        input: command.input.clone(),
                                        output: Output::Error(format!("cd: {}", e)),
                                    },
                                }
                            }
                        } else {
                            let executed_command = std::process::Command::new(&command_list[0])
                                .args(&command_list[1..])
                                .output();

                            CompletedCommand {
                                input: command.input.clone(),
                                output: {
                                    match executed_command {
                                        Ok(executed_command) => {
                                            if executed_command.status.success() {
                                                Output::Success(
                                                    String::from_utf8_lossy(
                                                        &executed_command.stdout,
                                                    )
                                                    .to_string(),
                                                )
                                            } else {
                                                Output::Error(
                                                    String::from_utf8_lossy(
                                                        &executed_command.stderr,
                                                    )
                                                    .to_string(),
                                                )
                                            }
                                        }
                                        Err(executed_command) => {
                                            if executed_command.kind()
                                                == std::io::ErrorKind::NotFound
                                            {
                                                Output::Error(format!(
                                                    "Command not found: {}",
                                                    command_list[0]
                                                ))
                                            } else {
                                                Output::Error(executed_command.to_string())
                                            }
                                        }
                                    }
                                },
                            }
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
            event::Event::CtrlV => match &model.current_command {
                CurrentView::CommandWithoutOutput(command) => {
                    let text_to_insert = clipboard.get_text()?;
                    if command.cursor_position == command.input.len() as u64 {
                        let new_command = format!("{}{}", command.input, text_to_insert);
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
                        let new_command = format!("{}{}{}", first, text_to_insert, second);
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
            },
            event::Event::CtrlP => match &model.current_command {
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
            },
            event::Event::CtrlS => {
                if model.command_history.is_empty() {
                    return Ok(());
                }
                model.mode = Mode::Selecting(String::new());
                Ok(())
            }
            event::Event::CtrlO => {
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
            event::Event::CtrlB => {
                match &model.current_command {
                    CurrentView::CommandWithoutOutput(c) => {
                        if !c.input.is_empty() {
                            model.mode = Mode::JumpingBefore(String::new());
                        }
                        Ok(())
                    }
                    CurrentView::Output(_) => {
                        // do nothing
                        Ok(())
                    }
                    CurrentView::CommandWithOutput(c) => {
                        if !c.input.is_empty() {
                            model.mode = Mode::JumpingBefore(String::new());
                        }
                        model.set_current_view_from_command(c.input.len() as u64, c.input.clone());
                        Ok(())
                    }
                }
            }
            event::Event::CtrlA => {
                match &model.current_command {
                    CurrentView::CommandWithoutOutput(c) => {
                        if !c.input.is_empty() {
                            model.mode = Mode::JumpingAfter(String::new());
                        }
                        Ok(())
                    }
                    CurrentView::Output(_) => {
                        // do nothing
                        Ok(())
                    }
                    CurrentView::CommandWithOutput(c) => {
                        if !c.input.is_empty() {
                            model.mode = Mode::JumpingAfter(String::new());
                        }
                        Ok(())
                    }
                }
            }
        },
        Mode::Editing(hint) => match event {
            event::Event::Esc | event::Event::CtrlE => {
                model.mode = Mode::Idle;
                Ok(())
            }
            event::Event::Enter => match &model.current_command {
                CurrentView::CommandWithoutOutput(command) => {
                    if hint.contains(',') {
                        let indices = hint.split(',').collect::<Vec<&str>>();
                        if indices.len() != 2 {
                            return Ok(());
                        }
                        let beginning_index = base26_to_base10(indices[0]);
                        if beginning_index.is_err() {
                            return Ok(());
                        }
                        let end_index = base26_to_base10(indices[1]);
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
                        for (real_index, element) in split_command.iter().enumerate() {
                            match element {
                                StringType::Word(w) => {
                                    if current == beginning_index || current == end_index {
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
                                StringType::Newline(c) | StringType::Whitespace(c) => {
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
                        split_command.drain(indices_to_delete[0]..=indices_to_delete[1]);
                        let new_command = split_command
                            .iter()
                            .map(|s| s.as_str())
                            .collect::<Vec<&str>>()
                            .join("");
                        model.current_command =
                            CurrentView::CommandWithoutOutput(CommandWithoutOutput {
                                cursor_position: new_cursor_position,
                                input: new_command,
                            });
                        Ok(())
                    } else {
                        if hint.is_empty() {
                            return Ok(());
                        }
                        let index = base26_to_base10(hint);
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
                        for (real_index, element) in split_command.iter().enumerate() {
                            match element {
                                StringType::Word(w) => {
                                    if current == index {
                                        index_to_delete = Some(real_index);
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
                        if let Some(index_to_delete) = index_to_delete {
                            split_command.remove(index_to_delete);

                            let new_command = split_command
                                .iter()
                                .map(|s| s.as_str())
                                .collect::<Vec<&str>>()
                                .join("");

                            model.current_command =
                                CurrentView::CommandWithoutOutput(CommandWithoutOutput {
                                    cursor_position: new_cursor_position,
                                    input: new_command,
                                });
                        }
                        Ok(())
                    }
                }
                _ => unreachable!(),
            },
            event::Event::Character(c) => {
                if c.is_alphabetic() || c == ',' {
                    hint.push(c);
                }
                Ok(())
            }
            event::Event::Backspace => {
                hint.pop();
                Ok(())
            }
            _ => {
                // do nothing
                Ok(())
            }
        },
        // SAFETY: if Mode::QUIT has been set, the program will already have exited before it reaches this point
        Mode::Quit => unreachable!(),
        Mode::Selecting(number) => match event {
            event::Event::Character(character) => {
                if character.is_ascii_digit() {
                    number.push(character)
                }
                Ok(())
            }
            event::Event::Enter => {
                if number.is_empty() {
                    model.set_current_view_from_command(0, String::new());
                    model.mode = Mode::Idle;
                    return Ok(());
                }

                // we only accept digits so this must be a valid usize (unless it's too large, that is acceptable)
                let number = number.parse::<usize>()?;
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
                        let index =
                            model.command_history.len() + model.pinned_commands.len() - number - 1;
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
            event::Event::Backspace => {
                number.pop();
                Ok(())
            }
            event::Event::Esc | event::Event::CtrlS => {
                model.mode = Mode::Idle;
                Ok(())
            }
            _ => {
                // do nothing
                Ok(())
            }
        },
        Mode::JumpingBefore(hint) => match event {
            event::Event::Character(character) => {
                if character.is_alphabetic() {
                    hint.push(character)
                }
                Ok(())
            }
            event::Event::Backspace => {
                hint.pop();
                Ok(())
            }
            event::Event::Esc | event::Event::CtrlB => {
                model.mode = Mode::Idle;
                Ok(())
            }
            event::Event::Enter => {
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
                let index = base26_to_base10(hint)?;
                model.mode = Mode::Idle;
                // SAFETY: Jumping Modes can only be entered if command has an input string
                let split_command = split_string(model.current_command.input_str().unwrap());
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
            _ => {
                // do nothing
                Ok(())
            }
        },
        Mode::JumpingAfter(hint) => match event {
            event::Event::Character(character) => {
                if character.is_alphabetic() {
                    hint.push(character)
                }
                Ok(())
            }
            event::Event::Backspace => {
                hint.pop();
                Ok(())
            }
            event::Event::Esc | event::Event::CtrlB => {
                model.mode = Mode::Idle;
                Ok(())
            }
            event::Event::Enter => {
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
                let index = base26_to_base10(hint)?;
                model.mode = Mode::Idle;
                // SAFETY: Jumping Modes can only be entered if command has an input string
                let split_command = split_string(model.current_command.input_str().unwrap());
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

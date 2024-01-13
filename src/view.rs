use ratatui::{
    layout::Rect,
    style::Stylize,
    widgets::{Block, Borders, Paragraph, Wrap},
};

use crate::{split_string, CurrentView, Mode, Model, Output, StringType};

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

    render_output(frame, model, outer_layout[1]);
    render_input(frame, model, left_layout[0]);

    let amount_pinned_commands: usize = model.pinned_commands.len();
    let pinned_commands = model
        .pinned_commands
        .iter()
        .enumerate()
        .map(|(index, command)| format!("{}: {}", index, command.input))
        .collect::<Vec<String>>()
        .join("\n");

    if !pinned_commands.is_empty() {
        frame.render_widget(
            Paragraph::new(pinned_commands.as_str())
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
            ratatui::widgets::Paragraph::new("-".repeat(left_layout[1].width as usize - 2))
                .block(Block::new().white().on_black().bold())
                .wrap(Wrap { trim: false }),
            Rect {
                x: left_layout[1].x + 1,
                y: left_layout[1].y + 1 + amount_pinned_commands as u16,
                width: left_layout[1].width - 2,
                height: 1,
            },
        );
    }

    if !model.command_history.is_empty() {
        let commands = model
            .command_history
            .iter()
            .rev()
            .enumerate()
            .map(|(index, command)| {
                format!("{}: {}", index + amount_pinned_commands, command.input)
            })
            .collect::<Vec<String>>()
            .join("\n");

        frame.render_widget(
            Paragraph::new(commands)
                .block(Block::new().white().on_black())
                .wrap(Wrap { trim: false }),
            Rect {
                x: left_layout[1].x + 1,
                y: left_layout[1].y + 1 + amount_pinned_commands as u16 + {
                    if pinned_commands.is_empty() {
                        0
                    } else {
                        1
                    }
                },
                width: left_layout[1].width - 2,
                height: left_layout[1].height - 2 - amount_pinned_commands as u16 - {
                    if pinned_commands.is_empty() {
                        0
                    } else {
                        1
                    }
                },
            },
        );
    }

    frame.render_widget(
        ratatui::widgets::Paragraph::new("History")
            .block(Block::new().white().on_black().bold())
            .wrap(Wrap { trim: false }),
        left_layout[1],
    );
}

fn base10_to_base26(mut num: u32) -> String {
    let mut result = String::new();
    while num > 0 {
        let digit = (num % 26) as u8;
        result.push((digit + b'a') as char);
        num /= 26;
    }
    if result.is_empty() {
        result.push('a');
    }
    result.chars().rev().collect() // Reverse to get the correct order
}

fn render_input(frame: &mut ratatui::Frame, model: &Model, layout: Rect) {
    match model.config.hint_state {
        crate::HintState::ShowHints => {
            let mut x = 1;
            let mut y = 1;
            let mut index = 0;
            let mut current_index_in_original_string: u64 = 0;

            let string_that_was_split =
                split_string(model.current_command.input_str().unwrap_or_default());

            if string_that_was_split.is_empty() {
                frame.render_widget(
                    Block::new().on_green(),
                    Rect {
                        x: 1,
                        y: 1,
                        width: 1,
                        height: 1,
                    },
                );
            }

            for word in string_that_was_split {
                // TODO: handle strings that are longer than the width of the terminal
                match word {
                    StringType::Word(content) => {
                        let mut cursor_position_inside_content = None;
                        if let Some(cursor_position) = model.current_command.cursor_position() {
                            if cursor_position
                                <= current_index_in_original_string + content.len() as u64
                                && cursor_position >= current_index_in_original_string
                            {
                                cursor_position_inside_content =
                                    Some(cursor_position - current_index_in_original_string);
                            }
                        }
                        current_index_in_original_string += content.len() as u64;

                        let s = base10_to_base26(index as u32);
                        let string_to_render = format!("{}:{}", s, content);
                        if x + 1 + string_to_render.len() as u16 > layout.width {
                            x = 1;
                            y += 1;
                        }
                        let location = Rect {
                            x,
                            y,
                            width: string_to_render.len() as u16,
                            height: 1,
                        };
                        frame.render_widget(
                            Paragraph::new(string_to_render.as_str())
                                .block(Block::new().white().on_black())
                                .wrap(Wrap { trim: false }),
                            location,
                        );
                        if let Some(cursor_position_inside_content) = cursor_position_inside_content
                        {
                            let cursor_location = Rect {
                                x: location.x
                                    + cursor_position_inside_content as u16
                                    + s.len() as u16
                                    + 1,
                                y: location.y,
                                width: 1,
                                height: 1,
                            };
                            if cursor_position_inside_content == content.len() as u64 {
                                frame.render_widget(Block::new().on_green(), cursor_location);
                            } else {
                                frame.render_widget(
                                    Paragraph::new(
                                        &content[cursor_position_inside_content as usize
                                            ..=cursor_position_inside_content as usize],
                                    )
                                    .block(Block::new().white().on_green()),
                                    cursor_location,
                                );
                            }
                        }
                        index += 1;
                        x += string_to_render.len() as u16;
                    }
                    StringType::Whitespace(content) => {
                        let mut cursor_position_inside_content = None;
                        if let Some(cursor_position) = model.current_command.cursor_position() {
                            if cursor_position
                                <= current_index_in_original_string + content.len() as u64
                                && cursor_position >= current_index_in_original_string
                            {
                                cursor_position_inside_content =
                                    Some(cursor_position - current_index_in_original_string);
                            }
                        }
                        current_index_in_original_string += content.len() as u64;
                        if x + 1 + content.len() as u16 > layout.width {
                            x = 1;
                            y += 1;
                        }
                        let location = Rect {
                            x,
                            y,
                            width: content.len() as u16,
                            height: 1,
                        };
                        if let Some(cursor_position_inside_content) = cursor_position_inside_content
                        {
                            let cursor_location = Rect {
                                x: location.x + cursor_position_inside_content as u16,
                                y: location.y,
                                width: 1,
                                height: 1,
                            };

                            frame.render_widget(Block::new().on_green(), cursor_location);
                        }
                        x += content.len() as u16;
                    }
                    StringType::Tab(_) => {
                        let string_to_render = "|-->";
                        let mut cursor_position_inside_content = None;
                        if let Some(cursor_position) = model.current_command.cursor_position() {
                            if cursor_position
                                <= current_index_in_original_string + string_to_render.len() as u64
                                && cursor_position > current_index_in_original_string
                            {
                                cursor_position_inside_content =
                                    Some(cursor_position - current_index_in_original_string);
                            }
                        }
                        current_index_in_original_string += string_to_render.len() as u64;
                        if x + 1 + string_to_render.len() as u16 > layout.width {
                            x = 1;
                            y += 1;
                        }
                        let location = Rect {
                            x,
                            y,
                            width: string_to_render.len() as u16,
                            height: 1,
                        };
                        frame.render_widget(
                            Paragraph::new(string_to_render)
                                .block(Block::new().white().on_black())
                                .wrap(Wrap { trim: false }),
                            location,
                        );
                        if let Some(cursor_position_inside_content) = cursor_position_inside_content
                        {
                            let cursor_location = Rect {
                                x: location.x + cursor_position_inside_content as u16,
                                y: location.y,
                                width: 1,
                                height: 1,
                            };

                            frame.render_widget(Block::new().on_green(), cursor_location);
                        }
                        x += string_to_render.len() as u16;
                    }
                    StringType::Newline(content) => {
                        let mut cursor_position_inside_content = None;
                        if let Some(cursor_position) = model.current_command.cursor_position() {
                            if cursor_position
                                <= current_index_in_original_string + content.len() as u64
                                && cursor_position > current_index_in_original_string
                            {
                                cursor_position_inside_content =
                                    Some(cursor_position - current_index_in_original_string);
                            }
                        }
                        current_index_in_original_string += content.len() as u64;

                        y += 1;
                        x = 1;
                        if cursor_position_inside_content.is_some() {
                            let cursor_location = Rect {
                                x,
                                y,
                                width: 1,
                                height: 1,
                            };

                            frame.render_widget(Block::new().on_green(), cursor_location);
                        }
                    }
                }
            }
        }
        crate::HintState::HideHints => {
            if let Some(input) = model.current_command.input_str() {
                frame.render_widget(
                    Paragraph::new(input)
                        .block(Block::new().white().on_black().borders(Borders::ALL))
                        .wrap(Wrap { trim: false }),
                    layout,
                );
            }
        }
    }

    let heading = match &model.mode {
        Mode::Idle | Mode::Quit => String::from("Input"),
        // Mode::Idle | Mode::Quit => format!(
        //     "Input - Cursor({})",
        //     model.current_command.cursor_position().unwrap_or_default()
        // ),
        Mode::Editing(hint) => format!("Input - Editing({})", hint),
        Mode::Selecting(number) => format!("Input - Selecting({})", number),
        Mode::JumpingBefore(hint) => format!("Input - JumpingBefore({})", hint),
        Mode::JumpingAfter(hint) => format!("Input - JumpingAfter({})", hint),
    };
    frame.render_widget(
        ratatui::widgets::Paragraph::new(heading.as_str())
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

fn render_output(frame: &mut ratatui::Frame, model: &Model, layout: Rect) {
    let (output, block) = match &model.current_command {
        CurrentView::CommandWithoutOutput(_) => (None, Block::new().white().on_black().bold()),
        CurrentView::Output(o) => match o {
            Output::Success(_) | Output::Empty => {
                (Some(o.as_str()), Block::new().white().on_black().bold())
            }
            Output::Error(_) => (Some(o.as_str()), Block::new().red().on_black().bold()),
        },
        CurrentView::CommandWithOutput(o) => match o.output {
            Output::Success(_) | Output::Empty => (
                Some(o.output.as_str()),
                Block::new().white().on_black().bold(),
            ),
            Output::Error(_) => (
                Some(o.output.as_str()),
                Block::new().red().on_black().bold(),
            ),
        },
    };

    if let Some(output) = output {
        frame.render_widget(
            Paragraph::new(output)
                .block(block.clone().borders(Borders::ALL))
                .wrap(Wrap { trim: false }),
            layout,
        );
    } else {
        frame.render_widget(block.clone().borders(Borders::ALL), layout);
    }

    frame.render_widget(
        Paragraph::new("Output")
            .block(block)
            .wrap(Wrap { trim: false }),
        Rect {
            x: layout.x,
            y: layout.y,
            width: "Output".len() as u16,
            height: 1,
        },
    );
}

#[cfg(test)]
mod tests {
    use crate::{split_string, StringType};

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

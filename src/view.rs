use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Style, Stylize},
    text::Line,
    widgets::{Block, Borders, Clear, ListItem, Paragraph, Wrap},
};

use crate::{split_string, CurrentView, File, Mode, Model, OutputType, StringType};

pub(crate) fn view(model: &mut Model, frame: &mut ratatui::Frame) {
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
    match model.config.history_type {
        crate::HistoryType::CommandHistory => {
            render_command_history(frame, model, left_layout[1]);
        }
        crate::HistoryType::DirectoryHistory => {
            render_directory_history(frame, model, left_layout[1]);
        }
    }

    if let Mode::Command(command) = &model.mode {
        frame.render_widget(
            Clear,
            Rect {
                x: outer_layout[0].x,
                y: outer_layout[0].height - 3,
                width: outer_layout[0].width + outer_layout[1].width,
                height: 3,
            },
        );

        frame.render_widget(
            ratatui::widgets::Paragraph::new(command.as_str())
                .block(Block::new().white().on_black().bold().borders(Borders::ALL))
                .wrap(Wrap { trim: false }),
            Rect {
                x: outer_layout[0].x,
                y: outer_layout[0].height - 3,
                width: outer_layout[0].width + outer_layout[1].width,
                height: 3,
            },
        );
    }

    render_directory_view(model, frame);
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

const TAB_STRING: &str = "|-->";

fn render_input(frame: &mut ratatui::Frame, model: &mut Model, layout: Rect) {
    let writable_width = layout.width - 2;
    let mut x = 1;
    let mut y = 1;
    let mut index = 0;
    let mut current_index_in_original_string: u64 = 0;

    let string_that_was_split = split_string(model.current_command.input_str().unwrap_or_default());

    if string_that_was_split.is_empty() {
        frame.render_widget(
            Block::new().on_green(),
            Rect {
                x,
                y,
                width: 1,
                height: 1,
            },
        );
    }

    for (word_index, word) in string_that_was_split.iter().enumerate() {
        match word {
            StringType::Word(content) => {
                let mut cursor_position_inside_content = None;
                if let Some(cursor_position) = model.current_command.cursor_position() {
                    if cursor_position <= current_index_in_original_string + content.len() as u64
                        && cursor_position >= current_index_in_original_string
                    {
                        cursor_position_inside_content =
                            Some(cursor_position - current_index_in_original_string);
                    }
                }
                current_index_in_original_string += content.len() as u64;
                let hint = match model.config.hint_state {
                    crate::HintState::ShowHints => {
                        format!("{}:", base10_to_base26(index as u32))
                    }
                    crate::HintState::HideHints => String::new(),
                };

                let mut string_to_render = format!("{}{}", hint, content);
                if x + 1 + string_to_render.len() as u16 > layout.width {
                    let mut character_amount = 0;
                    let mut space_left = layout.width - x - 1;
                    // frame.render_widget(
                    //     Paragraph::new(space_left.to_string())
                    //         .block(Block::new().white().on_red())
                    //         .wrap(Wrap { trim: false }),
                    //     layout,
                    // );
                    let mut should_quit = false;
                    while !should_quit {
                        if space_left == 0 {
                            x = 1;
                            y += 1;
                            space_left = writable_width;
                        }
                        let current_string = if string_to_render.len() as u16 <= space_left {
                            should_quit = true;
                            string_to_render.clone()
                        } else {
                            let mut c = string_to_render.split_off(space_left as usize);
                            std::mem::swap(&mut c, &mut string_to_render);
                            c
                        };

                        space_left = layout.width - x - 1 - current_string.len() as u16;

                        let location = Rect {
                            x,
                            y,
                            width: current_string.len() as u16,
                            height: 1,
                        };

                        frame.render_widget(
                            Paragraph::new(current_string.as_str())
                                .block(Block::new().white().on_black())
                                .wrap(Wrap { trim: false }),
                            location,
                        );

                        if let Some(cursor_position_inside_content) = cursor_position_inside_content
                        {
                            if cursor_position_inside_content + hint.len() as u64
                                >= character_amount
                                && cursor_position_inside_content + hint.len() as u64
                                    <= character_amount + current_string.len() as u64
                            {
                                let new_x =
                                    x + cursor_position_inside_content as u16 + hint.len() as u16
                                        - character_amount as u16;
                                let cursor_location = Rect {
                                    x: if new_x == layout.width - 1 { 1 } else { new_x },
                                    y: if new_x == layout.width - 1 { y + 1 } else { y },
                                    width: 1,
                                    height: 1,
                                };
                                frame.render_widget(Block::new().on_green(), cursor_location);
                            }
                        }
                        character_amount += current_string.len() as u64;
                        x += current_string.len() as u16;
                    }
                } else {
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
                    x += string_to_render.len() as u16;

                    if let Some(cursor_position_inside_content) = cursor_position_inside_content {
                        if !(cursor_position_inside_content == content.len() as u64
                            && string_that_was_split.get(word_index + 1).is_some()
                            && string_that_was_split[word_index + 1] == StringType::Tab
                            && x + TAB_STRING.len() as u16 > layout.width - 1)
                        {
                            let new_x = location.x
                                + cursor_position_inside_content as u16
                                + hint.len() as u16;
                            let cursor_location = Rect {
                                x: if new_x == layout.width - 1 { 1 } else { new_x },
                                y: if new_x == layout.width - 1 {
                                    location.y + 1
                                } else {
                                    location.y
                                },
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
                    }
                }

                index += 1;
            }
            StringType::Whitespace(content) => {
                let mut cursor_position_inside_content = None;
                if let Some(cursor_position) = model.current_command.cursor_position() {
                    if cursor_position <= current_index_in_original_string + content.len() as u64
                        && cursor_position >= current_index_in_original_string
                    {
                        cursor_position_inside_content =
                            Some(cursor_position - current_index_in_original_string);
                    }
                }
                current_index_in_original_string += content.len() as u64;

                if let Some(cursor_position_inside_content) = cursor_position_inside_content {
                    let mut distance_from_x = cursor_position_inside_content;
                    // let old = cursor_position_inside_content;
                    let mut new_cursor_position = None;

                    if distance_from_x == 0 {
                        if x > writable_width {
                            new_cursor_position = Some((1, y + 1));
                        } else {
                            new_cursor_position = Some((x, y));
                        }
                    }

                    for _ in content.chars() {
                        if x > writable_width {
                            x = 2;
                            y += 1;
                            distance_from_x = distance_from_x.saturating_sub(1);
                            if distance_from_x == 0 && new_cursor_position.is_none() {
                                new_cursor_position = Some((x, y));
                            }
                        } else {
                            x += 1;
                            distance_from_x = distance_from_x.saturating_sub(1);
                            if distance_from_x == 0 && new_cursor_position.is_none() {
                                if x > writable_width {
                                    new_cursor_position = Some((1, y + 1));
                                } else {
                                    new_cursor_position = Some((x, y));
                                }
                            }
                        }
                    }

                    if !(cursor_position_inside_content == content.len() as u64
                        && string_that_was_split.get(word_index + 1).is_some()
                        && string_that_was_split[word_index + 1] == StringType::Tab
                        && x + TAB_STRING.len() as u16 > layout.width - 1)
                    {
                        let cursor_location = Rect {
                            // SAFETY: new_cursor_position is always Some if cursor_position_inside_content is Some
                            x: new_cursor_position.unwrap().0,
                            y: new_cursor_position.unwrap().1,
                            width: 1,
                            height: 1,
                        };

                        frame.render_widget(Block::new().on_green(), cursor_location);
                    }
                } else {
                    for _ in content.chars() {
                        if x > writable_width {
                            x = 2;
                            y += 1;
                        } else {
                            x += 1;
                        }
                    }
                }
            }
            StringType::Tab => {
                let content = "\t";
                let mut cursor_position_inside_content = None;
                if let Some(cursor_position) = model.current_command.cursor_position() {
                    if cursor_position <= current_index_in_original_string + content.len() as u64
                        && cursor_position >= current_index_in_original_string
                    {
                        cursor_position_inside_content =
                            Some(cursor_position - current_index_in_original_string);
                    }
                }
                current_index_in_original_string += content.len() as u64;
                if x + 1 + TAB_STRING.len() as u16 > layout.width {
                    x = 1;
                    y += 1;
                }
                let location = Rect {
                    x,
                    y,
                    width: TAB_STRING.len() as u16,
                    height: 1,
                };
                frame.render_widget(
                    Paragraph::new(TAB_STRING)
                        .block(Block::new().white().on_black())
                        .wrap(Wrap { trim: false }),
                    location,
                );
                x += TAB_STRING.len() as u16;
                if let Some(cursor_position_inside_content) = cursor_position_inside_content {
                    match cursor_position_inside_content {
                        0 => {
                            let cursor_location = Rect {
                                x: location.x,
                                y: location.y,
                                width: TAB_STRING.len() as u16,
                                height: 1,
                            };
                            frame.render_widget(Block::new().on_green(), cursor_location);
                        }
                        1 => {
                            if !(string_that_was_split.get(word_index + 1).is_some()
                                && string_that_was_split[word_index + 1] == StringType::Tab
                                && x + TAB_STRING.len() as u16 > layout.width - 1)
                            {
                                let cursor_location = Rect {
                                    x: if x == layout.width - 1 {
                                        1
                                    } else {
                                        location.x + TAB_STRING.len() as u16
                                    },
                                    y: if x == layout.width - 1 {
                                        location.y + 1
                                    } else {
                                        location.y
                                    },
                                    width: 1,
                                    height: 1,
                                };
                                frame.render_widget(Block::new().on_green(), cursor_location);
                            }
                        }
                        _ => unreachable!(),
                    }
                }
            }
            StringType::Newline(content) => {
                let mut cursor_position_inside_content = None;
                if let Some(cursor_position) = model.current_command.cursor_position() {
                    if cursor_position <= current_index_in_original_string + content.len() as u64
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

    // SAFETY: shell will crash if it cannot access current dir at beginning
    // and current dir is in history so if we get here there is a last element
    let current_directory = model.directory_history.last().unwrap();
    let directory_string = current_directory.to_string_lossy();
    let directory_header = format!("Input - {}", directory_string);
    if directory_header.len() as u16 > layout.width - 1 {
        let (_, end) = directory_header.split_at(directory_header.len() - (layout.width as usize));
        let (_, end) = end.split_at(12);
        let header = format!("Input - ...{}", end);
        frame.render_widget(
            ratatui::widgets::Paragraph::new(header.as_str())
                .block(Block::new().white().on_black().bold())
                .wrap(Wrap { trim: false }),
            Rect {
                x: 0,
                y: 0,
                width: header.len() as u16,
                height: 1,
            },
        );
    } else {
        frame.render_widget(
            ratatui::widgets::Paragraph::new(directory_header.as_str())
                .block(Block::new().white().on_black().bold())
                .wrap(Wrap { trim: false }),
            Rect {
                x: 0,
                y: 0,
                width: directory_header.len() as u16,
                height: 1,
            },
        );
    }
}

fn render_output(frame: &mut ratatui::Frame, model: &mut Model, layout: Rect) {
    let (output, block, origin) = match &model.current_command {
        CurrentView::CommandWithoutOutput(_) => {
            (None, Block::new().white().on_black().bold(), None)
        }
        CurrentView::Output(o) => match o.output_type {
            OutputType::Success(_, _) | OutputType::Empty => (
                Some(o.to_string()),
                Block::new().white().on_black().bold(),
                Some(o.origin.clone()),
            ),
            OutputType::Error(_, _) => (
                Some(o.to_string()),
                Block::new().red().on_black().bold(),
                Some(o.origin.clone()),
            ),
        },
        CurrentView::CommandWithOutput(o) => match o.output.output_type {
            OutputType::Success(_, _) | OutputType::Empty => (
                Some(o.output.to_string()),
                Block::new().white().on_black().bold(),
                Some(o.output.origin.clone()),
            ),
            OutputType::Error(_, _) => (
                Some(o.output.to_string()),
                Block::new().red().on_black().bold(),
                Some(o.output.origin.clone()),
            ),
        },
    };

    if let Some(output) = output {
        match model.config.hint_state {
            crate::HintState::ShowHints => {
                let writable_width = layout.width - 2;
                let mut x = layout.x + 1;
                let mut y = 1;
                let mut index = 0;

                let string_that_was_split = split_string(&output);

                for word in string_that_was_split.iter() {
                    match word {
                        StringType::Word(content) => {
                            let hint = match model.config.hint_state {
                                crate::HintState::ShowHints => {
                                    format!("{}:", base10_to_base26(index as u32))
                                }
                                crate::HintState::HideHints => String::new(),
                            };

                            let mut string_to_render = format!("{}{}", hint, content);
                            if x + 1 + string_to_render.len() as u16 > layout.width + layout.x {
                                let mut space_left = layout.x + layout.width - x - 1;
                                // frame.render_widget(
                                //     Paragraph::new(space_left.to_string())
                                //         .block(Block::new().white().on_red())
                                //         .wrap(Wrap { trim: false }),
                                //     layout,
                                // );
                                let mut should_quit = false;
                                while !should_quit {
                                    if space_left == 0 {
                                        x = layout.x + 1;
                                        y += 1;
                                        space_left = writable_width;
                                    }
                                    let current_string = if string_to_render.len() as u16
                                        <= space_left
                                    {
                                        should_quit = true;
                                        string_to_render.clone()
                                    } else {
                                        let mut c = string_to_render.split_off(space_left as usize);
                                        std::mem::swap(&mut c, &mut string_to_render);
                                        c
                                    };

                                    space_left = layout.x + layout.width
                                        - x
                                        - 1
                                        - current_string.len() as u16;

                                    let location = Rect {
                                        x,
                                        y,
                                        width: current_string.len() as u16,
                                        height: 1,
                                    };

                                    frame.render_widget(
                                        Paragraph::new(current_string.as_str())
                                            .block(Block::new().white().on_black())
                                            .wrap(Wrap { trim: false }),
                                        location,
                                    );
                                    x += current_string.len() as u16;
                                }
                            } else {
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
                                x += string_to_render.len() as u16;
                            }

                            index += 1;
                        }
                        StringType::Whitespace(content) => {
                            for _ in content.chars() {
                                if x > writable_width + layout.x {
                                    x = layout.x + 2;
                                    y += 1;
                                } else {
                                    x += 1;
                                }
                            }
                        }
                        StringType::Tab => {
                            if x + 1 + TAB_STRING.len() as u16 > layout.width + layout.x {
                                x = layout.x + 1;
                                y += 1;
                            }
                            let location = Rect {
                                x,
                                y,
                                width: TAB_STRING.len() as u16,
                                height: 1,
                            };
                            frame.render_widget(
                                Paragraph::new(TAB_STRING)
                                    .block(Block::new().white().on_black())
                                    .wrap(Wrap { trim: false }),
                                location,
                            );
                            x += TAB_STRING.len() as u16;
                        }
                        StringType::Newline(_) => {
                            y += 1;
                            x = layout.x + 1;
                        }
                    }
                }
            }
            crate::HintState::HideHints => {
                frame.render_widget(
                    Paragraph::new(output)
                        .block(block.clone().borders(Borders::ALL))
                        .wrap(Wrap { trim: false }),
                    layout,
                );
            }
        }
    } else {
        frame.render_widget(block.clone().borders(Borders::ALL), layout);
    }

    frame.render_widget(block.clone().borders(Borders::ALL), layout);

    let animation_x = match origin {
        Some(shell) => {
            let heading = format!("Output({})", shell);
            frame.render_widget(
                Paragraph::new(heading.as_str())
                    .block(block.clone())
                    .wrap(Wrap { trim: false }),
                Rect {
                    x: layout.x,
                    y: layout.y,
                    width: heading.len() as u16,
                    height: 1,
                },
            );
            layout.x + heading.len() as u16
        }
        None => {
            frame.render_widget(
                Paragraph::new("Output")
                    .block(block.clone())
                    .wrap(Wrap { trim: false }),
                Rect {
                    x: layout.x,
                    y: layout.y,
                    width: "Output".len() as u16,
                    height: 1,
                },
            );
            layout.x + "Output".len() as u16
        }
    };

    if let Mode::Executing(ref mut direction, ref mut index, _, _) = model.mode {
        frame.render_widget(
            Clear,
            Rect {
                x: animation_x,
                y: layout.y,
                width: layout.width - (animation_x - layout.x) - 1,
                height: 1,
            },
        );

        for cell in animation_x..animation_x + layout.width - (animation_x - layout.x) - 1 {
            if cell == animation_x + *index {
                frame.render_widget(
                    Paragraph::new("-")
                        .block(block.clone())
                        .wrap(Wrap { trim: false }),
                    Rect {
                        x: cell,
                        y: layout.y,
                        width: 1,
                        height: 1,
                    },
                );
            } else {
                frame.render_widget(
                    Paragraph::new(" ")
                        .block(block.clone())
                        .wrap(Wrap { trim: false }),
                    Rect {
                        x: cell,
                        y: layout.y,
                        width: 1,
                        height: 1,
                    },
                );
            }
        }
        if *direction {
            if *index == layout.width - (animation_x - layout.x) - 1 {
                *direction = false;
            } else {
                *index += 1;
            }
        } else if *index == 0 {
            *direction = true;
        } else {
            *index -= 1;
        }
    }
}

fn render_command_history(frame: &mut ratatui::Frame, model: &Model, layout: Rect) {
    let pinned_commands = model
        .pinned_commands
        .iter()
        .enumerate()
        .map(|(index, command)| format!("{}: {}", index, command.input))
        .collect::<Vec<String>>();

    for (index, command) in pinned_commands.iter().enumerate() {
        frame.render_widget(
            Paragraph::new(command.as_str())
                .block(Block::new().white().on_black())
                .wrap(Wrap { trim: false }),
            Rect {
                x: layout.x + 1,
                y: layout.y + 1 + index as u16,
                width: layout.width - 2,
                height: 1,
            },
        );
    }

    if !pinned_commands.is_empty() {
        frame.render_widget(
            ratatui::widgets::Paragraph::new("-".repeat(layout.width as usize - 2))
                .block(Block::new().white().on_black().bold())
                .wrap(Wrap { trim: false }),
            Rect {
                x: layout.x + 1,
                y: layout.y + 1 + pinned_commands.len() as u16,
                width: layout.width - 2,
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
            .map(|(index, command)| format!("{}: {}", index + pinned_commands.len(), command.input))
            .collect::<Vec<String>>()
            .join("\n");

        frame.render_widget(
            Paragraph::new(commands)
                .block(Block::new().white().on_black())
                .wrap(Wrap { trim: false }),
            Rect {
                x: layout.x + 1,
                y: layout.y
                    + 1
                    + pinned_commands.len() as u16
                    + if pinned_commands.is_empty() { 0 } else { 1 },
                width: layout.width - 2,
                height: layout.height
                    - 2
                    - pinned_commands.len() as u16
                    - if pinned_commands.is_empty() { 0 } else { 1 },
            },
        );
    }

    frame.render_widget(
        ratatui::widgets::Paragraph::new("History")
            .block(Block::new().white().on_black().bold())
            .wrap(Wrap { trim: false }),
        layout,
    );
}

fn render_directory_history(frame: &mut ratatui::Frame, model: &Model, layout: Rect) {
    let directories = model
        .directory_history
        .iter()
        .rev()
        .enumerate()
        .map(|(index, directory)| format!("{}: {}", index, directory.to_string_lossy()))
        .collect::<Vec<String>>()
        .join("\n");

    frame.render_widget(
        Paragraph::new(directories)
            .block(Block::new().white().on_black().bold())
            .wrap(Wrap { trim: false }),
        Rect {
            x: layout.x + 1,
            y: layout.y + 1,
            width: layout.width - 2,
            height: layout.height - 2,
        },
    );

    frame.render_widget(
        ratatui::widgets::Paragraph::new("Directory History")
            .block(Block::new().white().on_black().bold())
            .wrap(Wrap { trim: false }),
        layout,
    );
}

fn render_directory_view(model: &mut Model, frame: &mut ratatui::Frame) {
    if let Mode::Directory(directory) = &mut model.mode {
        fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
            let popup_layout = Layout::default()
                .direction(ratatui::layout::Direction::Vertical)
                .constraints([
                    Constraint::Percentage((100 - percent_y) / 2),
                    Constraint::Percentage(percent_y),
                    Constraint::Percentage((100 - percent_y) / 2),
                ])
                .split(r);

            Layout::default()
                .direction(ratatui::layout::Direction::Horizontal)
                .constraints([
                    Constraint::Percentage((100 - percent_x) / 2),
                    Constraint::Percentage(percent_x),
                    Constraint::Percentage((100 - percent_x) / 2),
                ])
                .split(popup_layout[1])[1]
        }

        let mut items = directory
            .children
            .iter()
            .map(|child| {
                let item = ListItem::new(Line::from(child.to_string()));
                match child {
                    File::Directory(_) => {
                        item.style(Style::default().fg(ratatui::style::Color::Green))
                    }
                    File::File(_) => item.style(Style::default().fg(ratatui::style::Color::White)),
                }
            })
            .collect::<Vec<ListItem>>();
        items.insert(
            0,
            ListItem::new(Line::from(".."))
                .style(Style::default().fg(ratatui::style::Color::Green)),
        );
        items.insert(
            0,
            ListItem::new(Line::from(".")).style(Style::default().fg(ratatui::style::Color::Green)),
        );

        let area = centered_rect(40, 50, frame.size());

        frame.render_widget(Clear, area);

        frame.render_widget(
            Block::new()
                .white()
                .on_black()
                .bold()
                .borders(ratatui::widgets::Borders::ALL)
                .title_alignment(ratatui::layout::Alignment::Center)
                .title(directory.current_dir.to_string_lossy().to_string()),
            area,
        );

        let layouts = Layout::default()
            .direction(ratatui::layout::Direction::Vertical)
            .constraints([Constraint::Min(3), Constraint::Min(0)])
            .split(area);

        frame.render_widget(
            Paragraph::new(
                Line::from(directory.search.as_str()).alignment(ratatui::layout::Alignment::Center),
            )
            .block(Block::default().borders(Borders::BOTTOM)),
            Rect {
                x: layouts[0].x + 1,
                y: layouts[0].y + 2,
                width: layouts[0].width - 2,
                height: layouts[0].height,
            },
        );

        let list_location = Rect {
            x: layouts[1].x + 1,
            y: layouts[1].y + 2,
            width: layouts[1].width - 2,
            height: layouts[1].height - 4,
        };
        frame.render_widget(
            ratatui::widgets::List::new(items).block(Block::new().white().on_black().bold()),
            list_location,
        );
        directory.location = Some(list_location);
    }
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
                StringType::Tab,
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
                StringType::Tab,
                StringType::Tab,
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
                StringType::Tab,
                StringType::Tab,
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

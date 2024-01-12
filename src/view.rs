use ratatui::{
    layout::Rect,
    style::Stylize,
    widgets::{Block, Borders, Paragraph, Wrap},
};

use crate::{CurrentView, Mode, Model, Output};

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
        CurrentView::CommandWithoutOutput(command) => {
            frame.render_widget(
                Paragraph::new(command.input.as_str())
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

fn render_input_heading(frame: &mut ratatui::Frame, model: &Model) {
    let heading = match &model.mode {
        Mode::Idle | Mode::Quit => String::from("Input"),
        Mode::Editing(_) => String::from("Input - Editing"),
        Mode::Selecting(number) => format!("Input - Selecting({})", number),
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

#[derive(Debug, PartialEq)]
pub(crate) enum Event {
    CtrlC,
    Backspace,
    Esc,
    Enter,
    Up,
    Down,
    Left,
    Right,
    Character(char),
    MouseDown(u16, u16),
    Paste(String),
}

pub(crate) fn wait_for_event() -> Event {
    loop {
        if let Ok(crossterm_event) = crossterm::event::read() {
            if let Some(event) = create_event(crossterm_event) {
                return event;
            }
        }
    }
}

pub(crate) fn get_event() -> Result<Option<Event>, Box<dyn std::error::Error>> {
    // TODO: remove unwrap
    if crossterm::event::poll(std::time::Duration::from_secs(0))? {
        // TODO: remove unwrap
        Ok(create_event(crossterm::event::read()?))
    } else {
        Ok(None)
    }
}

fn create_event(crossterm_event: crossterm::event::Event) -> Option<Event> {
    match crossterm_event {
        crossterm::event::Event::Key(key) => {
            if key.kind == crossterm::event::KeyEventKind::Press {
                match key.code {
                    crossterm::event::KeyCode::Char('c')
                        if key.modifiers == crossterm::event::KeyModifiers::CONTROL =>
                    {
                        Some(Event::CtrlC)
                    }
                    crossterm::event::KeyCode::Left => Some(Event::Left),
                    crossterm::event::KeyCode::Right => Some(Event::Right),
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
        crossterm::event::Event::Mouse(mouse) => match mouse.kind {
            crossterm::event::MouseEventKind::Down(crossterm::event::MouseButton::Left) => {
                Some(Event::MouseDown(mouse.column, mouse.row))
            }
            _ => None,
        },
        crossterm::event::Event::Paste(paste) => Some(Event::Paste(paste)),
        _ => None,
    }
}

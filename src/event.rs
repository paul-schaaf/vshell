#[derive(Debug, PartialEq)]
pub enum Event {
    CtrlC,
    CtrlE,
    CtrlH,
    CtrlV,
    CtrlP,
    CtrlS,
    Backspace,
    Esc,
    Enter,
    Up,
    Down,
    Character(char),
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
                    crossterm::event::KeyCode::Char('p')
                        if key.modifiers == crossterm::event::KeyModifiers::CONTROL =>
                    {
                        Some(Event::CtrlP)
                    }
                    crossterm::event::KeyCode::Char('s')
                        if key.modifiers == crossterm::event::KeyModifiers::CONTROL =>
                    {
                        Some(Event::CtrlS)
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

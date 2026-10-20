use crossterm::event::KeyCode;

pub(crate) fn parse_key(key_str: &str) -> KeyCode {
    match key_str.to_lowercase().as_str() {
        "space" | " " => KeyCode::Char(' '),
        "left" => KeyCode::Left,
        "right" => KeyCode::Right,
        "up" => KeyCode::Up,
        "down" => KeyCode::Down,
        "escape" | "esc" => KeyCode::Esc,
        "tab" => KeyCode::Tab,
        "backspace" => KeyCode::Backspace,
        "enter" => KeyCode::Enter,
        "insert" | "ins" => KeyCode::Insert,
        "delete" | "del" => KeyCode::Delete,
        "home" => KeyCode::Home,
        "end" => KeyCode::End,
        "pageup" | "pgup" => KeyCode::PageUp,
        "pagedown" | "pgdown" => KeyCode::PageDown,
        "slash" | "/" => KeyCode::Char('/'),
        key if key.len() == 1 => KeyCode::Char(key.chars().next().unwrap()),
        _ => KeyCode::Null,
    }
}

pub(crate) fn matches_key(key_code: KeyCode, bindings: &[String]) -> bool {
    bindings
        .iter()
        .any(|binding| parse_key(binding) == key_code)
}

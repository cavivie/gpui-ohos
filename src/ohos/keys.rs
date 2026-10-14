//! Physical keyboard keys, as GPUI keystrokes.

use openharmony_ability::xcomponent::KeyCode;

/// What a key produces: the name GPUI binds it under, and the text typed
/// with it, if any.
pub(crate) struct Key {
    pub name: &'static str,
    pub text: Option<&'static str>,
}

/// `(unshifted, shifted)` for keys that type a character.
fn typed(code: KeyCode) -> Option<(&'static str, &'static str)> {
    Some(match code {
        KeyCode::Key0 | KeyCode::Numpad0 => ("0", ")"),
        KeyCode::Key1 | KeyCode::Numpad1 => ("1", "!"),
        KeyCode::Key2 | KeyCode::Numpad2 => ("2", "@"),
        KeyCode::Key3 | KeyCode::Numpad3 => ("3", "#"),
        KeyCode::Key4 | KeyCode::Numpad4 => ("4", "$"),
        KeyCode::Key5 | KeyCode::Numpad5 => ("5", "%"),
        KeyCode::Key6 | KeyCode::Numpad6 => ("6", "^"),
        KeyCode::Key7 | KeyCode::Numpad7 => ("7", "&"),
        KeyCode::Key8 | KeyCode::Numpad8 => ("8", "*"),
        KeyCode::Key9 | KeyCode::Numpad9 => ("9", "("),
        KeyCode::A => ("a", "A"),
        KeyCode::B => ("b", "B"),
        KeyCode::C => ("c", "C"),
        KeyCode::D => ("d", "D"),
        KeyCode::E => ("e", "E"),
        KeyCode::F => ("f", "F"),
        KeyCode::G => ("g", "G"),
        KeyCode::H => ("h", "H"),
        KeyCode::I => ("i", "I"),
        KeyCode::J => ("j", "J"),
        KeyCode::K => ("k", "K"),
        KeyCode::L => ("l", "L"),
        KeyCode::M => ("m", "M"),
        KeyCode::N => ("n", "N"),
        KeyCode::O => ("o", "O"),
        KeyCode::P => ("p", "P"),
        KeyCode::Q => ("q", "Q"),
        KeyCode::R => ("r", "R"),
        KeyCode::S => ("s", "S"),
        KeyCode::T => ("t", "T"),
        KeyCode::U => ("u", "U"),
        KeyCode::V => ("v", "V"),
        KeyCode::W => ("w", "W"),
        KeyCode::X => ("x", "X"),
        KeyCode::Y => ("y", "Y"),
        KeyCode::Z => ("z", "Z"),
        KeyCode::Space => (" ", " "),
        KeyCode::Comma | KeyCode::NumpadComma => (",", "<"),
        KeyCode::Period | KeyCode::NumpadDot => (".", ">"),
        KeyCode::Minus | KeyCode::NumpadSubtract => ("-", "_"),
        KeyCode::Equals | KeyCode::NumpadEquals => ("=", "+"),
        KeyCode::NumpadAdd => ("+", "+"),
        KeyCode::NumpadMultiply => ("*", "*"),
        KeyCode::NumpadDivide => ("/", "/"),
        KeyCode::LeftBracket => ("[", "{"),
        KeyCode::RightBracket => ("]", "}"),
        KeyCode::Backslash => ("\\", "|"),
        KeyCode::Semicolon => (";", ":"),
        KeyCode::Apostrophe => ("'", "\""),
        KeyCode::Slash => ("/", "?"),
        KeyCode::Grave => ("`", "~"),
        _ => return None,
    })
}

/// A key that types nothing, by the name GPUI binds it under.
fn named(code: KeyCode) -> Option<&'static str> {
    Some(match code {
        KeyCode::Enter | KeyCode::NumpadEnter => "enter",
        KeyCode::Tab => "tab",
        KeyCode::Del => "backspace",
        KeyCode::ForwardDel => "delete",
        KeyCode::Escape => "escape",
        KeyCode::DpadUp => "up",
        KeyCode::DpadDown => "down",
        KeyCode::DpadLeft => "left",
        KeyCode::DpadRight => "right",
        KeyCode::MoveHome => "home",
        KeyCode::MoveEnd => "end",
        KeyCode::PageUp => "pageup",
        KeyCode::PageDown => "pagedown",
        KeyCode::Insert => "insert",
        KeyCode::F1 => "f1",
        KeyCode::F2 => "f2",
        KeyCode::F3 => "f3",
        KeyCode::F4 => "f4",
        KeyCode::F5 => "f5",
        KeyCode::F6 => "f6",
        KeyCode::F7 => "f7",
        KeyCode::F8 => "f8",
        KeyCode::F9 => "f9",
        KeyCode::F10 => "f10",
        KeyCode::F11 => "f11",
        KeyCode::F12 => "f12",
        _ => return None,
    })
}

/// The key `code` is with shift held or not, or `None` for keys that are not
/// edited with (modifiers, media keys).
pub(crate) fn key(code: KeyCode, shift: bool) -> Option<Key> {
    if let Some((plain, shifted)) = typed(code) {
        return Some(Key {
            // Bound under the character printed on the key, unshifted.
            name: plain,
            text: Some(if shift { shifted } else { plain }),
        });
    }
    named(code).map(|name| Key { name, text: None })
}

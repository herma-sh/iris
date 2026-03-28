/// Platform identifier used for key-code normalization.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeyboardPlatform {
    /// Windows key-code mapping.
    Windows,
    /// macOS key-code mapping.
    Macos,
    /// Linux/BSD key-code mapping.
    Unix,
}

/// Platform key code supplied by the host/event loop.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlatformKeyCode {
    /// Windows virtual-key code (`VK_*`).
    Windows(u16),
    /// macOS virtual keycode (`NSEvent` keycode).
    Macos(u16),
    /// XKB keysym.
    Xkb(u32),
}

/// Keyboard modifiers normalized for terminal input handling.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct KeyModifiers {
    /// Control modifier.
    pub ctrl: bool,
    /// Alt/Option modifier.
    pub alt: bool,
    /// Shift modifier.
    pub shift: bool,
    /// Meta/Command/Windows modifier.
    pub meta: bool,
    /// AltGr modifier.
    pub alt_gr: bool,
}

/// Raw keyboard event coming from a host platform.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlatformKeyboardEvent {
    /// Platform key code.
    pub code: PlatformKeyCode,
    /// Text produced by the key event when available.
    pub text: Option<String>,
    /// Event modifiers.
    pub modifiers: KeyModifiers,
    /// `true` for key-down, `false` for key-up.
    pub key_down: bool,
    /// `true` when this event is an auto-repeat.
    pub repeat: bool,
}

/// Normalized terminal key value.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NormalizedKey {
    /// Printable character.
    Character(char),
    Enter,
    Tab,
    Backspace,
    Escape,
    Up,
    Down,
    Left,
    Right,
    Home,
    End,
    PageUp,
    PageDown,
    Insert,
    Delete,
    Function(u8),
    Unknown,
}

/// Platform-agnostic keyboard event shape.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NormalizedKeyboardEvent {
    /// Normalized key symbol.
    pub key: NormalizedKey,
    /// Text payload when available.
    pub text: Option<String>,
    /// Event modifiers.
    pub modifiers: KeyModifiers,
    /// `true` for key-down, `false` for key-up.
    pub key_down: bool,
    /// `true` when this event is an auto-repeat.
    pub repeat: bool,
}

/// Normalizes a platform keyboard event into a platform-agnostic key event.
#[must_use]
pub fn normalize_keyboard_event(
    platform: KeyboardPlatform,
    event: PlatformKeyboardEvent,
) -> NormalizedKeyboardEvent {
    let key = match event.code {
        PlatformKeyCode::Windows(code) => map_windows_key(code),
        PlatformKeyCode::Macos(code) => map_macos_key(code),
        PlatformKeyCode::Xkb(code) => map_xkb_key(code),
    };

    let mut modifiers = event.modifiers;
    if platform != KeyboardPlatform::Macos && modifiers.ctrl && modifiers.alt && !modifiers.meta {
        // Common AltGr representation on Windows/X11.
        modifiers.alt_gr = true;
    }

    let text = normalize_text(&event.text, key);
    let key = match (key, text.as_ref()) {
        (NormalizedKey::Unknown, Some(text)) => text
            .chars()
            .next()
            .map_or(NormalizedKey::Unknown, NormalizedKey::Character),
        _ => key,
    };

    NormalizedKeyboardEvent {
        key,
        text,
        modifiers,
        key_down: event.key_down,
        repeat: event.repeat,
    }
}

/// Encodes a normalized key event into terminal input bytes.
///
/// Returns `None` for key-up events and unhandled keys.
#[must_use]
pub fn encode_terminal_key_input(event: &NormalizedKeyboardEvent) -> Option<Vec<u8>> {
    if !event.key_down {
        return None;
    }

    if let NormalizedKey::Character(character) = event.key {
        return encode_character(event, character);
    }

    if let Some(text) = &event.text {
        if !text.is_empty() && !event.modifiers.ctrl && !event.modifiers.meta {
            return Some(text.as_bytes().to_vec());
        }
    }

    match event.key {
        NormalizedKey::Enter => Some(vec![b'\r']),
        NormalizedKey::Tab => Some(vec![b'\t']),
        NormalizedKey::Backspace => Some(vec![0x7f]),
        NormalizedKey::Escape => Some(vec![0x1b]),
        NormalizedKey::Up => Some(b"\x1b[A".to_vec()),
        NormalizedKey::Down => Some(b"\x1b[B".to_vec()),
        NormalizedKey::Right => Some(b"\x1b[C".to_vec()),
        NormalizedKey::Left => Some(b"\x1b[D".to_vec()),
        NormalizedKey::Home => Some(b"\x1b[H".to_vec()),
        NormalizedKey::End => Some(b"\x1b[F".to_vec()),
        NormalizedKey::PageUp => Some(b"\x1b[5~".to_vec()),
        NormalizedKey::PageDown => Some(b"\x1b[6~".to_vec()),
        NormalizedKey::Insert => Some(b"\x1b[2~".to_vec()),
        NormalizedKey::Delete => Some(b"\x1b[3~".to_vec()),
        NormalizedKey::Function(number) => encode_function_key(number),
        NormalizedKey::Character(_) | NormalizedKey::Unknown => None,
    }
}

fn encode_character(event: &NormalizedKeyboardEvent, character: char) -> Option<Vec<u8>> {
    if event.modifiers.ctrl && !event.modifiers.alt_gr {
        let upper = character.to_ascii_uppercase();
        if upper.is_ascii_uppercase() {
            let control = (upper as u8) & 0x1f;
            return Some(vec![control]);
        }
    }

    if event.modifiers.alt && !event.modifiers.alt_gr {
        let mut payload = Vec::with_capacity(1 + character.len_utf8());
        payload.push(0x1b);
        let mut buffer = [0_u8; 4];
        payload.extend_from_slice(character.encode_utf8(&mut buffer).as_bytes());
        return Some(payload);
    }

    if let Some(text) = &event.text {
        if !text.is_empty() {
            return Some(text.as_bytes().to_vec());
        }
    }

    let mut buffer = [0_u8; 4];
    Some(character.encode_utf8(&mut buffer).as_bytes().to_vec())
}

fn encode_function_key(number: u8) -> Option<Vec<u8>> {
    let sequence = match number {
        1 => b"\x1bOP".as_slice(),
        2 => b"\x1bOQ".as_slice(),
        3 => b"\x1bOR".as_slice(),
        4 => b"\x1bOS".as_slice(),
        5 => b"\x1b[15~".as_slice(),
        6 => b"\x1b[17~".as_slice(),
        7 => b"\x1b[18~".as_slice(),
        8 => b"\x1b[19~".as_slice(),
        9 => b"\x1b[20~".as_slice(),
        10 => b"\x1b[21~".as_slice(),
        11 => b"\x1b[23~".as_slice(),
        12 => b"\x1b[24~".as_slice(),
        _ => return None,
    };
    Some(sequence.to_vec())
}

fn normalize_text(text: &Option<String>, key: NormalizedKey) -> Option<String> {
    match key {
        NormalizedKey::Character(_) | NormalizedKey::Unknown => text.clone(),
        _ => None,
    }
}

fn map_windows_key(code: u16) -> NormalizedKey {
    match code {
        0x08 => NormalizedKey::Backspace,
        0x09 => NormalizedKey::Tab,
        0x0D => NormalizedKey::Enter,
        0x1B => NormalizedKey::Escape,
        0x21 => NormalizedKey::PageUp,
        0x22 => NormalizedKey::PageDown,
        0x23 => NormalizedKey::End,
        0x24 => NormalizedKey::Home,
        0x25 => NormalizedKey::Left,
        0x26 => NormalizedKey::Up,
        0x27 => NormalizedKey::Right,
        0x28 => NormalizedKey::Down,
        0x2D => NormalizedKey::Insert,
        0x2E => NormalizedKey::Delete,
        0x70..=0x7B => NormalizedKey::Function((code - 0x70 + 1) as u8),
        _ => NormalizedKey::Unknown,
    }
}

fn map_macos_key(code: u16) -> NormalizedKey {
    match code {
        36 | 76 => NormalizedKey::Enter,
        48 => NormalizedKey::Tab,
        51 => NormalizedKey::Backspace,
        53 => NormalizedKey::Escape,
        115 => NormalizedKey::Home,
        116 => NormalizedKey::PageUp,
        117 => NormalizedKey::Delete,
        119 => NormalizedKey::End,
        121 => NormalizedKey::PageDown,
        123 => NormalizedKey::Left,
        124 => NormalizedKey::Right,
        125 => NormalizedKey::Down,
        126 => NormalizedKey::Up,
        122 => NormalizedKey::Function(1),
        120 => NormalizedKey::Function(2),
        99 => NormalizedKey::Function(3),
        118 => NormalizedKey::Function(4),
        96 => NormalizedKey::Function(5),
        97 => NormalizedKey::Function(6),
        98 => NormalizedKey::Function(7),
        100 => NormalizedKey::Function(8),
        101 => NormalizedKey::Function(9),
        109 => NormalizedKey::Function(10),
        103 => NormalizedKey::Function(11),
        111 => NormalizedKey::Function(12),
        _ => NormalizedKey::Unknown,
    }
}

fn map_xkb_key(code: u32) -> NormalizedKey {
    match code {
        0xFF08 => NormalizedKey::Backspace,
        0xFF09 => NormalizedKey::Tab,
        0xFF0D => NormalizedKey::Enter,
        0xFF1B => NormalizedKey::Escape,
        0xFF50 => NormalizedKey::Home,
        0xFF51 => NormalizedKey::Left,
        0xFF52 => NormalizedKey::Up,
        0xFF53 => NormalizedKey::Right,
        0xFF54 => NormalizedKey::Down,
        0xFF55 => NormalizedKey::PageUp,
        0xFF56 => NormalizedKey::PageDown,
        0xFF57 => NormalizedKey::End,
        0xFF63 => NormalizedKey::Insert,
        0xFFFF => NormalizedKey::Delete,
        0xFFBE..=0xFFC9 => NormalizedKey::Function((code - 0xFFBE + 1) as u8),
        _ => NormalizedKey::Unknown,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        encode_terminal_key_input, normalize_keyboard_event, KeyModifiers, KeyboardPlatform,
        NormalizedKey, NormalizedKeyboardEvent, PlatformKeyCode, PlatformKeyboardEvent,
    };

    #[test]
    fn normalize_windows_arrow_key() {
        let event = PlatformKeyboardEvent {
            code: PlatformKeyCode::Windows(0x26),
            text: None,
            modifiers: KeyModifiers::default(),
            key_down: true,
            repeat: false,
        };
        let normalized = normalize_keyboard_event(KeyboardPlatform::Windows, event);
        assert_eq!(normalized.key, NormalizedKey::Up);
    }

    #[test]
    fn normalize_macos_home_key() {
        let event = PlatformKeyboardEvent {
            code: PlatformKeyCode::Macos(115),
            text: None,
            modifiers: KeyModifiers::default(),
            key_down: true,
            repeat: false,
        };
        let normalized = normalize_keyboard_event(KeyboardPlatform::Macos, event);
        assert_eq!(normalized.key, NormalizedKey::Home);
    }

    #[test]
    fn normalize_xkb_delete_key() {
        let event = PlatformKeyboardEvent {
            code: PlatformKeyCode::Xkb(0xFFFF),
            text: None,
            modifiers: KeyModifiers::default(),
            key_down: true,
            repeat: false,
        };
        let normalized = normalize_keyboard_event(KeyboardPlatform::Unix, event);
        assert_eq!(normalized.key, NormalizedKey::Delete);
    }

    #[test]
    fn normalize_altgr_on_windows() {
        let event = PlatformKeyboardEvent {
            code: PlatformKeyCode::Windows(0x41),
            text: Some("a".to_string()),
            modifiers: KeyModifiers {
                ctrl: true,
                alt: true,
                shift: false,
                meta: false,
                alt_gr: false,
            },
            key_down: true,
            repeat: false,
        };
        let normalized = normalize_keyboard_event(KeyboardPlatform::Windows, event);
        assert!(normalized.modifiers.alt_gr);
    }

    #[test]
    fn encode_special_key_to_escape_sequence() {
        let event = NormalizedKeyboardEvent {
            key: NormalizedKey::PageDown,
            text: None,
            modifiers: KeyModifiers::default(),
            key_down: true,
            repeat: false,
        };
        assert_eq!(encode_terminal_key_input(&event), Some(b"\x1b[6~".to_vec()));
    }

    #[test]
    fn encode_character_with_ctrl_modifier() {
        let event = NormalizedKeyboardEvent {
            key: NormalizedKey::Character('c'),
            text: Some("c".to_string()),
            modifiers: KeyModifiers {
                ctrl: true,
                ..KeyModifiers::default()
            },
            key_down: true,
            repeat: false,
        };
        assert_eq!(encode_terminal_key_input(&event), Some(vec![0x03]));
    }

    #[test]
    fn encode_character_with_alt_modifier() {
        let event = NormalizedKeyboardEvent {
            key: NormalizedKey::Character('x'),
            text: Some("x".to_string()),
            modifiers: KeyModifiers {
                alt: true,
                ..KeyModifiers::default()
            },
            key_down: true,
            repeat: false,
        };
        assert_eq!(encode_terminal_key_input(&event), Some(b"\x1bx".to_vec()));
    }

    #[test]
    fn encode_key_up_returns_none() {
        let event = NormalizedKeyboardEvent {
            key: NormalizedKey::Enter,
            text: None,
            modifiers: KeyModifiers::default(),
            key_down: false,
            repeat: false,
        };
        assert_eq!(encode_terminal_key_input(&event), None);
    }
}

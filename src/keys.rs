//! Translates a platform key event into the bytes a PTY-connected program
//! expects. Framework-agnostic (plain `&str`/`bool` params, not GPUI types)
//! so it's usable/testable without a GPUI dependency - see
//! `openspec/changes/terminal-rendering/specs/terminal-input/spec.md`.

/// One key press, named the way GPUI's `Keystroke` names keys (e.g.
/// `"a"`, `"enter"`, `"up"`, `"f1"`) - see `gpui_pre_macos::events` for the
/// canonical list this mirrors.
#[derive(Debug, Clone, Copy)]
pub struct KeyInput<'a> {
    pub key:      &'a str,
    /// The character this key would insert with no modifiers held, if any
    /// (`None` for pure control keys like a bare Cmd chord).
    pub key_char: Option<&'a str>,
    pub control:  bool,
    pub alt:      bool,
}

/// Returns the bytes to write to a PTY for this key press, or `None` if
/// this key press shouldn't be sent at all (e.g. a bare modifier key, or a
/// platform ("Cmd") chord this layer doesn't own - the app's own keybinds
/// handle those before input ever reaches the terminal).
pub fn key_to_bytes(input: KeyInput<'_>) -> Option<Vec<u8>> {
    if let Some(named) = named_key_bytes(input.key) {
        return Some(named);
    }

    if input.control {
        return control_byte(input.key).map(|byte| vec![byte]);
    }

    // Multi-character key names with no `key_char` are modifier-only presses
    // (e.g. a bare "control"/"shift"/"cmd") or unrecognized named keys, not
    // literal text - only a single character falls back to the key name
    // itself.
    let mut text = match input.key_char {
        Some(key_char) => key_char.to_string(),
        None if input.key.chars().count() == 1 => input.key.to_string(),
        None => return None,
    };
    if input.alt {
        // Standard xterm convention: Alt prefixes the byte sequence with ESC.
        text.insert(0, '\u{1b}');
    }
    if text.is_empty() {
        None
    }
    else {
        Some(text.into_bytes())
    }
}

/// Named/functional keys with a fixed byte sequence, independent of
/// modifiers (arrow keys, function keys, etc. - xterm's standard sequences).
fn named_key_bytes(key: &str) -> Option<Vec<u8>> {
    let bytes: &[u8] = match key {
        "enter" => b"\r",
        "tab" => b"\t",
        "backspace" => b"\x7f",
        "escape" => b"\x1b",
        "space" => b" ",
        "up" => b"\x1b[A",
        "down" => b"\x1b[B",
        "right" => b"\x1b[C",
        "left" => b"\x1b[D",
        "home" => b"\x1b[H",
        "end" => b"\x1b[F",
        "pageup" => b"\x1b[5~",
        "pagedown" => b"\x1b[6~",
        "delete" => b"\x1b[3~",
        "insert" => b"\x1b[2~",
        "f1" => b"\x1bOP",
        "f2" => b"\x1bOQ",
        "f3" => b"\x1bOR",
        "f4" => b"\x1bOS",
        "f5" => b"\x1b[15~",
        "f6" => b"\x1b[17~",
        "f7" => b"\x1b[18~",
        "f8" => b"\x1b[19~",
        "f9" => b"\x1b[20~",
        "f10" => b"\x1b[21~",
        "f11" => b"\x1b[23~",
        "f12" => b"\x1b[24~",
        _ => return None,
    };
    Some(bytes.to_vec())
}

/// Ctrl+letter/symbol -> its control byte (e.g. Ctrl+C -> 0x03), matching
/// the standard `key & 0x1f` mapping for `@`-`_` (and lowercase equivalents).
fn control_byte(key: &str) -> Option<u8> {
    let mut chars = key.chars();
    let ch = chars.next()?;
    if chars.next().is_some() {
        return None;
    }
    let upper = ch.to_ascii_uppercase();
    if upper.is_ascii() && (0x3F..=0x5F).contains(&(upper as u8)) {
        Some(upper as u8 & 0x1F)
    }
    else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(key: &str) -> KeyInput<'_> {
        KeyInput { key,
                   key_char: None,
                   control: false,
                   alt: false }
    }

    #[test]
    fn printable_letter_sends_its_key_char() {
        let input = KeyInput { key:      "a",
                               key_char: Some("a"),
                               control:  false,
                               alt:      false, };
        assert_eq!(key_to_bytes(input), Some(b"a".to_vec()));
    }

    #[test]
    fn shifted_letter_sends_the_shifted_key_char() {
        let input = KeyInput { key:      "a",
                               key_char: Some("A"),
                               control:  false,
                               alt:      false, };
        assert_eq!(key_to_bytes(input), Some(b"A".to_vec()));
    }

    #[test]
    fn ctrl_c_sends_end_of_text() {
        let input = KeyInput { control: true,
                               ..key("c") };
        assert_eq!(key_to_bytes(input), Some(vec![0x03]));
    }

    #[test]
    fn ctrl_d_sends_end_of_transmission() {
        let input = KeyInput { control: true,
                               ..key("d") };
        assert_eq!(key_to_bytes(input), Some(vec![0x04]));
    }

    #[test]
    fn enter_sends_carriage_return_not_key_char() {
        let input = KeyInput { key_char: Some("\n"),
                               ..key("enter") };
        assert_eq!(key_to_bytes(input), Some(b"\r".to_vec()));
    }

    #[test]
    fn arrow_keys_send_ansi_cursor_sequences() {
        assert_eq!(key_to_bytes(key("up")), Some(b"\x1b[A".to_vec()));
        assert_eq!(key_to_bytes(key("down")), Some(b"\x1b[B".to_vec()));
        assert_eq!(key_to_bytes(key("left")), Some(b"\x1b[D".to_vec()));
        assert_eq!(key_to_bytes(key("right")), Some(b"\x1b[C".to_vec()));
    }

    #[test]
    fn function_keys_send_their_xterm_sequences() {
        assert_eq!(key_to_bytes(key("f1")), Some(b"\x1bOP".to_vec()));
        assert_eq!(key_to_bytes(key("f5")), Some(b"\x1b[15~".to_vec()));
    }

    #[test]
    fn alt_prefixes_with_escape() {
        let input = KeyInput { key_char: Some("b"),
                               alt: true,
                               ..key("b") };
        assert_eq!(key_to_bytes(input), Some(b"\x1bb".to_vec()));
    }

    #[test]
    fn bare_modifier_key_produces_nothing() {
        let input = KeyInput { key_char: None,
                               ..key("control") };
        assert_eq!(key_to_bytes(input), None);
    }
}

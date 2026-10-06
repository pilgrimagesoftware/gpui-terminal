//! Translates mouse events into the SGR mouse-reporting escape sequences a
//! PTY-connected program expects, when it has requested mouse reporting -
//! see `openspec/changes/terminal-rendering/specs/terminal-input/spec.md`.

/// Which mouse button (or wheel direction) an event concerns.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseButton {
    Left,
    Middle,
    Right,
    WheelUp,
    WheelDown,
}

/// A mouse button press/release at a grid cell, for translation into SGR
/// mouse-reporting bytes.
#[derive(Debug, Clone, Copy)]
pub struct MouseInput {
    /// 0-indexed grid row.
    pub row:     usize,
    /// 0-indexed grid column.
    pub column:  usize,
    pub button:  MouseButton,
    pub pressed: bool,
}

/// Returns the SGR mouse-reporting bytes for `input`, or `None` if the
/// running program hasn't enabled SGR mouse mode (`sgr_mouse_mode`) - the
/// caller should fall back to normal scrollback/selection in that case.
pub fn mouse_to_bytes(input: MouseInput, sgr_mouse_mode: bool) -> Option<Vec<u8>> {
    if !sgr_mouse_mode {
        return None;
    }
    let code = match input.button {
        MouseButton::Left => 0,
        MouseButton::Middle => 1,
        MouseButton::Right => 2,
        // Wheel events are reported as "presses" of these synthetic
        // buttons (64/65), never released, per xterm's SGR convention.
        MouseButton::WheelUp => 64,
        MouseButton::WheelDown => 65,
    };
    // SGR mouse reporting is 1-indexed.
    let column = input.column + 1;
    let row = input.row + 1;
    let action = if input.pressed { 'M' } else { 'm' };
    Some(format!("\x1b[<{code};{column};{row}{action}").into_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_bytes_when_sgr_mouse_mode_is_off() {
        let input = MouseInput { row:     0,
                                 column:  0,
                                 button:  MouseButton::Left,
                                 pressed: true, };
        assert_eq!(mouse_to_bytes(input, false), None);
    }

    #[test]
    fn left_press_encodes_button_zero() {
        let input = MouseInput { row:     4,
                                 column:  9,
                                 button:  MouseButton::Left,
                                 pressed: true, };
        assert_eq!(mouse_to_bytes(input, true), Some(b"\x1b[<0;10;5M".to_vec()));
    }

    #[test]
    fn right_release_encodes_lowercase_m() {
        let input = MouseInput { row:     0,
                                 column:  0,
                                 button:  MouseButton::Right,
                                 pressed: false, };
        assert_eq!(mouse_to_bytes(input, true), Some(b"\x1b[<2;1;1m".to_vec()));
    }

    #[test]
    fn middle_button_encodes_button_one() {
        let input = MouseInput { row:     0,
                                 column:  0,
                                 button:  MouseButton::Middle,
                                 pressed: true, };
        assert_eq!(mouse_to_bytes(input, true), Some(b"\x1b[<1;1;1M".to_vec()));
    }

    #[test]
    fn wheel_events_encode_as_synthetic_button_presses() {
        let up = MouseInput { row:     0,
                              column:  0,
                              button:  MouseButton::WheelUp,
                              pressed: true, };
        let down = MouseInput { row:     0,
                                column:  0,
                                button:  MouseButton::WheelDown,
                                pressed: true, };
        assert_eq!(mouse_to_bytes(up, true), Some(b"\x1b[<64;1;1M".to_vec()));
        assert_eq!(mouse_to_bytes(down, true), Some(b"\x1b[<65;1;1M".to_vec()));
    }
}

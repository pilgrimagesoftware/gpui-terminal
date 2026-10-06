//! Keys, mouse, scroll, copy and paste: each translates a GPUI event into
//! transport input or a grid change, and does nothing else.

use gpui_kit::{
    ClipboardItem, Context, KeyDownEvent, Keystroke, MouseDownEvent, MouseMoveEvent, MouseUpEvent,
    Pixels, Point, ScrollDelta, ScrollWheelEvent, Window,
};

use super::TerminalView;
use crate::{
    KeyInput, MouseButton, MouseInput, Transport, key_to_bytes, mouse_to_bytes, paste_payload,
};

impl<T: Transport> TerminalView<T> {
    /// Copies the selection to the system clipboard, trailing whitespace
    /// trimmed from each line - the padding a grid row carries is not text
    /// the user selected.
    pub fn copy_selection(&mut self, cx: &mut Context<Self>) {
        let Some(text) = self.terminal.with_grid(|grid| grid.selection_text())
        else {
            return;
        };
        let text = text.lines()
                       .map(str::trim_end)
                       .collect::<Vec<_>>()
                       .join("\n");
        cx.write_to_clipboard(ClipboardItem::new_string(text));
    }

    /// Writes the system clipboard's text to the terminal, bracketed when the
    /// program asked for bracketed paste. See [`paste_payload`].
    pub fn paste_clipboard(&mut self, cx: &mut Context<Self>) {
        if self.exited() {
            return;
        }
        let Some(text) = cx.read_from_clipboard().and_then(|item| item.text())
        else {
            return;
        };
        let bracketed = self.terminal.with_grid(|grid| grid.bracketed_paste_mode());
        if let Some(payload) = paste_payload(&text, bracketed) {
            self.return_to_live_screen(cx);
            let written = self.terminal.write(payload.as_bytes());
            self.report(written, cx);
        }
    }

    /// The copy and paste chords are the view's (see [`clipboard_chord`]);
    /// every other platform chord belongs to the app and is left to
    /// propagate. Once the program has exited, keys go nowhere.
    pub(super) fn on_key_down(&mut self, event: &KeyDownEvent, _window: &mut Window,
                              cx: &mut Context<Self>) {
        let keystroke = &event.keystroke;
        match clipboard_chord(keystroke, cfg!(target_os = "macos")) {
            Some(Clipboard::Copy) => return self.copy_selection(cx),
            Some(Clipboard::Paste) => return self.paste_clipboard(cx),
            None => {}
        }
        if keystroke.modifiers.platform || self.exited() {
            return;
        }
        let input = KeyInput { key:      &keystroke.key,
                               key_char: keystroke.key_char.as_deref(),
                               control:  keystroke.modifiers.control,
                               alt:      keystroke.modifiers.alt, };
        if let Some(bytes) = key_to_bytes(input) {
            self.return_to_live_screen(cx);
            let written = self.terminal.write(&bytes);
            self.report(written, cx);
        }
    }

    /// Scrolls back to the live screen before input goes to the program, as
    /// terminals do: what is typed shows where it lands.
    fn return_to_live_screen(&mut self, cx: &mut Context<Self>) {
        if self.terminal.with_grid_mut(|grid| grid.scroll_to_bottom()) {
            cx.notify();
        }
    }

    pub(super) fn on_left_down(&mut self, event: &MouseDownEvent, window: &mut Window,
                               cx: &mut Context<Self>) {
        self.focus.focus(window, cx);
        self.mouse_button(event.position, MouseButton::Left, true, cx);
    }

    pub(super) fn on_left_up(&mut self, event: &MouseUpEvent, _window: &mut Window,
                             cx: &mut Context<Self>) {
        self.mouse_button(event.position, MouseButton::Left, false, cx);
    }

    /// Extends the selection while the left button is held, when no
    /// mouse-aware program has claimed the mouse.
    pub(super) fn on_mouse_move(&mut self, event: &MouseMoveEvent, _window: &mut Window,
                                cx: &mut Context<Self>) {
        if !event.dragging() {
            return;
        }
        let Some((column, row)) = self.cell_at(event.position)
        else {
            return;
        };
        let selected = self.terminal.with_grid_mut(|grid| {
                                        let selecting = !grid.sgr_mouse_mode();
                                        if selecting {
                                            grid.update_selection(column, row);
                                        }
                                        selecting
                                    });
        if selected {
            cx.notify();
        }
    }

    /// A mouse-aware program gets one wheel report per scroll event, in the
    /// direction scrolled. Otherwise the wheel scrolls the view through the
    /// scrollback, by the lines scrolled - see [`Self::whole_lines`].
    pub(super) fn on_scroll(&mut self, event: &ScrollWheelEvent, _window: &mut Window,
                            cx: &mut Context<Self>) {
        let Some(metrics) = self.metrics
        else {
            return;
        };
        let lines = match event.delta {
            ScrollDelta::Lines(delta) => delta.y,
            ScrollDelta::Pixels(delta) => metrics.pixels_to_lines(delta.y),
        };
        if lines == 0. {
            return;
        }
        if !self.mouse_reporting() {
            // Up the wheel is back into the scrollback.
            let rows = self.whole_lines(lines);
            if rows != 0
               && self.terminal
                      .with_grid_mut(|grid| grid.scroll_display(rows))
            {
                cx.notify();
            }
            return;
        }
        let button = if lines > 0. {
            MouseButton::WheelUp
        }
        else {
            MouseButton::WheelDown
        };
        self.mouse_button(event.position, button, true, cx);
    }

    /// The whole lines `lines` more of scrolling adds up to, keeping the
    /// fraction for the next event. A trackpad reports a few pixels at a
    /// time - a fraction of a line each - so rounding each event up to a line
    /// would scroll many times too fast. Turning back starts afresh, so a
    /// fraction left going one way doesn't eat into the other.
    fn whole_lines(&mut self, lines: f32) -> i32 {
        if self.scroll_remainder != 0. && self.scroll_remainder.signum() != lines.signum() {
            self.scroll_remainder = 0.;
        }
        self.scroll_remainder += lines;
        let whole = self.scroll_remainder.trunc();
        self.scroll_remainder -= whole;
        whole as i32
    }

    /// Sends a button to a program that turned on SGR mouse reporting.
    /// Without one listening, a left press starts a selection instead.
    fn mouse_button(&mut self, position: Point<Pixels>, button: MouseButton, pressed: bool,
                    cx: &mut Context<Self>) {
        let Some((column, row)) = self.cell_at(position)
        else {
            return;
        };
        let sgr = self.mouse_reporting();
        if !sgr {
            if button == MouseButton::Left && pressed {
                self.terminal
                    .with_grid_mut(|grid| grid.start_selection(column, row));
                cx.notify();
            }
            return;
        }
        let input = MouseInput { row,
                                 column,
                                 button,
                                 pressed };
        if let Some(bytes) = mouse_to_bytes(input, sgr) {
            let written = self.terminal.write(&bytes);
            self.report(written, cx);
        }
    }

    /// Whether mouse input goes to the program: it turned on SGR mouse
    /// reporting and is still running. A program that exits with reporting
    /// on leaves the mouse to the view - the wheel scrolls the scrollback
    /// and a press selects - rather than to no one.
    fn mouse_reporting(&self) -> bool {
        self.terminal.with_grid(|grid| grid.sgr_mouse_mode()) && !self.exited()
    }

    /// Whether the program has exited: input has nowhere to go, though the
    /// screen stays to read, select and copy.
    fn exited(&self) -> bool {
        self.terminal.exit_report().is_some()
    }

    /// The cell under a window position, or `None` before the first frame
    /// has measured one.
    fn cell_at(&self, position: Point<Pixels>) -> Option<(usize, usize)> {
        let metrics = self.metrics?;
        Some(metrics.cell_at(position - self.origin, self.terminal.size()))
    }
}

/// A clipboard chord the view handles itself rather than sending.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Clipboard {
    Copy,
    Paste,
}

/// Whether `keystroke` is the platform's copy or paste chord: the platform
/// modifier with `c` or `v` (Cmd on macOS, Super elsewhere), and - off macOS,
/// where Ctrl-C belongs to the program - Ctrl-Shift-C and Ctrl-Shift-V, as
/// Linux terminals use.
pub(super) fn clipboard_chord(keystroke: &Keystroke, macos: bool) -> Option<Clipboard> {
    let modifiers = &keystroke.modifiers;
    let chord =
        modifiers.platform || (!macos && modifiers.control && modifiers.shift && !modifiers.alt);
    if !chord {
        return None;
    }
    match keystroke.key.to_ascii_lowercase().as_str() {
        "c" => Some(Clipboard::Copy),
        "v" => Some(Clipboard::Paste),
        _ => None,
    }
}

#[cfg(test)]
mod tests;

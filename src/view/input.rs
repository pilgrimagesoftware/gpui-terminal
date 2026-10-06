//! Keys, mouse, scroll, copy and paste: each translates a GPUI event into
//! transport input or a grid change, and does nothing else.

use gpui_kit::{
    ClipboardItem, Context, KeyDownEvent, MouseDownEvent, MouseMoveEvent, MouseUpEvent, Pixels,
    Point, ScrollDelta, ScrollWheelEvent, Window,
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

    /// The platform modifier's copy and paste chords are the view's; every
    /// other platform chord belongs to the app and is left to propagate.
    pub(super) fn on_key_down(&mut self, event: &KeyDownEvent, _window: &mut Window,
                              cx: &mut Context<Self>) {
        let keystroke = &event.keystroke;
        if keystroke.modifiers.platform {
            match keystroke.key.as_str() {
                "c" => self.copy_selection(cx),
                "v" => self.paste_clipboard(cx),
                _ => {}
            }
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
    /// scrollback, by the lines scrolled (at least one).
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
        if !self.terminal.with_grid(|grid| grid.sgr_mouse_mode()) {
            // Up the wheel is back into the scrollback.
            let rows = (lines.abs().round() as i32).max(1) * lines.signum() as i32;
            if self.terminal
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

    /// Sends a button to a program that turned on SGR mouse reporting.
    /// Without one listening, a left press starts a selection instead.
    fn mouse_button(&mut self, position: Point<Pixels>, button: MouseButton, pressed: bool,
                    cx: &mut Context<Self>) {
        let Some((column, row)) = self.cell_at(position)
        else {
            return;
        };
        let sgr = self.terminal.with_grid(|grid| grid.sgr_mouse_mode());
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

    /// The cell under a window position, or `None` before the first frame
    /// has measured one.
    fn cell_at(&self, position: Point<Pixels>) -> Option<(usize, usize)> {
        let metrics = self.metrics?;
        Some(metrics.cell_at(position - self.origin, self.terminal.size()))
    }
}

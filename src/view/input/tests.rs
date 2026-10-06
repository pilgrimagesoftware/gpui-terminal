//! Which keystrokes are the view's clipboard chords, on each platform.

use gpui_kit::Keystroke;

use super::{Clipboard, clipboard_chord};

fn chord(keys: &str, macos: bool) -> Option<Clipboard> {
    clipboard_chord(&Keystroke::parse(keys).expect("a valid keystroke"), macos)
}

#[test]
fn the_platform_modifier_copies_and_pastes_everywhere() {
    for macos in [true, false] {
        assert_eq!(chord("cmd-c", macos), Some(Clipboard::Copy));
        assert_eq!(chord("cmd-v", macos), Some(Clipboard::Paste));
        assert_eq!(chord("cmd-k", macos), None, "the app's, not the view's");
    }
}

#[test]
fn ctrl_shift_c_and_v_copy_and_paste_off_macos() {
    assert_eq!(chord("ctrl-shift-c", false), Some(Clipboard::Copy));
    assert_eq!(chord("ctrl-shift-v", false), Some(Clipboard::Paste));
}

#[test]
fn ctrl_c_and_ctrl_shift_c_on_macos_belong_to_the_program() {
    assert_eq!(chord("ctrl-c", false),
               None,
               "Ctrl-C interrupts the program");
    assert_eq!(chord("ctrl-c", true), None);
    assert_eq!(chord("ctrl-shift-c", true), None, "macOS copies with Cmd");
    assert_eq!(chord("ctrl-alt-shift-c", false), None);
}

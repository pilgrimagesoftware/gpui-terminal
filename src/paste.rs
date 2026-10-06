//! Turns clipboard text into the bytes a pseudo-terminal should receive.
//!
//! Two things have to happen to pasted text before it reaches the PTY, and
//! neither is optional.
//!
//! A line break on the clipboard is `\n`, but a terminal's Enter is carriage
//! return - the shell's line editor is reading a tty, not a file. Pasting
//! without the translation leaves the shell waiting on a line it has already
//! been given.
//!
//! And when the program has enabled bracketed paste, the payload is wrapped
//! in markers so it can tell pasted text from typed text. Any `\x1b[201~`
//! *inside* the payload would close that bracket early and let the rest be
//! read as keystrokes, which is how a copied blob becomes a command nobody
//! typed - so the terminator is removed from the content first.

/// What a program sends to turn bracketed paste on, and what wraps the
/// payload once it has.
const PASTE_START: &str = "\x1b[200~";
const PASTE_END: &str = "\x1b[201~";

/// The text to write to the PTY for a paste of `text`.
///
/// Returns `None` when there is nothing to send, so a caller need not decide
/// whether an empty clipboard is worth a write.
pub fn paste_payload(text: &str, bracketed: bool) -> Option<String> {
    let body = sanitize(text);

    if body.is_empty() {
        return None;
    }

    if bracketed {
        return Some(format!("{PASTE_START}{body}{PASTE_END}"));
    }

    Some(body)
}

/// Normalizes line endings and removes anything that would end the bracket.
fn sanitize(text: &str) -> String {
    text.replace("\r\n", "\r")
        .replace('\n', "\r")
        .replace(PASTE_END, "")
        // A NUL cannot be typed and terminates a C string on the way through;
        // nothing downstream is better off receiving one.
        .replace('\0', "")
}

#[cfg(test)]
mod tests {
    use super::{PASTE_END, PASTE_START, paste_payload};

    #[test]
    fn a_plain_paste_is_sent_as_written() {
        assert_eq!(paste_payload("echo hello", false),
                   Some("echo hello".to_owned()));
    }

    #[test]
    fn line_breaks_become_carriage_returns() {
        assert_eq!(paste_payload("one\ntwo", false),
                   Some("one\rtwo".to_owned()));
        assert_eq!(paste_payload("one\r\ntwo", false),
                   Some("one\rtwo".to_owned()));
        assert_eq!(paste_payload("one\rtwo", false),
                   Some("one\rtwo".to_owned()));
    }

    #[test]
    fn a_bracketed_paste_is_wrapped_in_its_markers() {
        let payload = paste_payload("echo hello", true).unwrap();

        assert!(payload.starts_with(PASTE_START), "{payload:?}");
        assert!(payload.ends_with(PASTE_END), "{payload:?}");
        assert!(payload.contains("echo hello"));
    }

    /// The injection this sanitizing exists for: a payload that closes the
    /// bracket itself would have its tail read as keystrokes.
    #[test]
    fn a_payload_cannot_close_the_bracket_early() {
        let hostile = format!("safe{PASTE_END}rm -rf /");

        let payload = paste_payload(&hostile, true).unwrap();

        assert_eq!(payload.matches(PASTE_END).count(), 1, "{payload:?}");
        assert!(payload.ends_with(PASTE_END));
        // The text survives; only the marker is taken out of it.
        assert!(payload.contains("saferm -rf /"));
    }

    /// The terminator is stripped whether or not this particular paste is
    /// bracketed: an unbracketed paste of it would still leave the marker in
    /// the shell's input.
    #[test]
    fn the_terminator_is_stripped_from_an_unbracketed_paste_too() {
        let payload = paste_payload(&format!("safe{PASTE_END}tail"), false).unwrap();

        assert_eq!(payload, "safetail");
    }

    #[test]
    fn nothing_worth_sending_is_reported_as_nothing() {
        assert_eq!(paste_payload("", false), None);
        assert_eq!(paste_payload("", true), None);
        assert_eq!(paste_payload(PASTE_END, true), None);
        assert_eq!(paste_payload("\0", false), None);
    }

    #[test]
    fn whitespace_is_not_mistaken_for_nothing() {
        assert_eq!(paste_payload("  ", false), Some("  ".to_owned()));
        assert_eq!(paste_payload("\n", false), Some("\r".to_owned()));
    }
}

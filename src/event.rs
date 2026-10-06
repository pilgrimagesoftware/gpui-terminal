//! What a terminal asks of the app hosting it.

use std::sync::Arc;

use crate::{Error, ExitReport};

/// Emitted by [`crate::TerminalView`]. Subscribe with `cx.subscribe`.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum TerminalEvent {
    /// The program set the window title (OSC 0/2).
    Title(String),
    /// The program asked for the title to go back to the host's default.
    ResetTitle,
    /// The program rang the bell (BEL).
    Bell,
    /// The program asked to put text on the system clipboard (OSC 52). Not
    /// written by the view: whether a program may set the clipboard is the
    /// host's policy. Requests for the X11 primary selection are dropped.
    ClipboardStore(String),
    /// The transport reported that the far end ended. Emitted once.
    Exited(ExitReport),
    /// A write, resize or terminate the view issued failed. The view has no
    /// caller to return it to, so it hands it to the host.
    TransportFailed(Arc<Error>),
}

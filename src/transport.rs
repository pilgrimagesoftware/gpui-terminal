//! The byte pipe a terminal runs over.

use crate::{GridSize, Result};

/// Where a terminal's input goes and its output comes from: a local PTY, a
/// remote exec with a tty, anything that carries bytes both ways.
///
/// Input arrives through these methods. Output goes the other way, through
/// the [`crate::TerminalSink`] the transport is handed when it is built (see
/// [`crate::TerminalBuilder::connect`]): call [`crate::TerminalSink::output`]
/// with each chunk the far end sends and [`crate::TerminalSink::exited`] once
/// when it ends, from whatever thread or task reads it.
///
/// Every method is called with the terminal's transport lock held, so none of
/// them should block for long - queue the bytes for a writer task rather than
/// waiting on a network round trip.
pub trait Transport: Send + 'static {
    /// Sends input - keystrokes, pasted text, replies to the program's
    /// queries - to the far end.
    fn write(&mut self, bytes: &[u8]) -> Result<()>;

    /// Tells the far end its terminal is now `size`. A transport with no way
    /// to say so returns `Ok(())`.
    fn resize(&mut self, size: GridSize) -> Result<()>;

    /// Ends the far end: kill the child, close the stream. The transport
    /// still reports the exit through its sink.
    fn terminate(&mut self) -> Result<()>;

    /// The local process behind this terminal while it runs, for a host that
    /// samples its descendants. `None` for a transport with no local process,
    /// and once that process has exited.
    fn process_id(&self) -> Option<u32> {
        None
    }
}

/// How the far end ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct ExitReport {
    /// The exit code, when the transport learned one.
    pub code: Option<i32>,
}

impl ExitReport {
    pub fn new(code: Option<i32>) -> Self {
        Self { code }
    }
}

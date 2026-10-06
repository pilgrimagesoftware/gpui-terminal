//! The crate's error type.

use thiserror::Error;

/// Something a [`crate::Transport`] could not do.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum Error {
    /// The transport's I/O failed: a write to a closed PTY, a failed spawn.
    #[error("terminal transport I/O failed: {0}")]
    Io(#[from] std::io::Error),
    /// The transport failed in a way of its own - a PTY system error, a
    /// remote exec's protocol error.
    #[error("terminal transport failed: {0}")]
    Transport(#[source] Box<dyn std::error::Error + Send + Sync>),
    /// The far end is gone; nothing more can be written.
    #[error("terminal transport is closed")]
    Closed,
}

impl Error {
    /// Wraps any transport-specific error.
    pub fn transport(error: impl Into<Box<dyn std::error::Error + Send + Sync>>) -> Self {
        Self::Transport(error.into())
    }
}

/// `std::result::Result` with this crate's [`Error`].
pub type Result<T> = std::result::Result<T, Error>;

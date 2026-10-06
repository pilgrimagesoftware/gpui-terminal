//! The GPUI entity that draws a [`crate::Terminal`] and feeds it input.
//!
//! [`TerminalView`] owns everything between the window and the transport:
//! the pump that turns transport output into repaints and events, key, mouse,
//! paste and focus handling, and sizing the terminal to the bounds it is laid
//! out in. The host supplies a [`TerminalStyle`] and listens for
//! [`crate::TerminalEvent`]s.

mod entity;
mod input;
mod metrics;
mod palette;
mod render;
mod style;
#[cfg(test)]
mod tests;

pub use entity::TerminalView;
pub use style::{TerminalPalette, TerminalStyle};

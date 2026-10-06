//! A terminal emulator for GPUI over any byte transport.
//!
//! Three layers, each usable without the one above it:
//!
//! - [`Grid`] parses VT/ANSI output into cells, backed by `alacritty_terminal`.
//! - [`Terminal`] joins a grid to a [`Transport`] - anything that can take
//!   bytes, a size and a hang-up, and hands output back through a
//!   [`TerminalSink`]. [`PtyTransport`] (the default `pty` feature) is a local
//!   pseudo-terminal; a remote exec or a serial line implements the same trait.
//! - [`TerminalView`] is the GPUI entity that draws the grid, turns keys,
//!   mouse, paste and resize into transport input, and emits [`TerminalEvent`]s
//!   for what the running program asks of its host.
//!
//! [`key_to_bytes`], [`mouse_to_bytes`] and [`paste_payload`] are the input
//! encodings on their own, free of GPUI types, for an embedder that draws its
//! own surface.
//!
//! ```no_run
//! # #[cfg(feature = "pty")]
//! # fn open(cx: &mut gpui_kit::App) -> gpui_terminal::Result<()> {
//! use gpui_kit::AppContext as _;
//! use gpui_terminal::{PtyCommand, PtyTransport, TerminalBuilder, TerminalStyle, TerminalView};
//!
//! let command = PtyCommand::user_shell().arg("-i");
//! let terminal = TerminalBuilder::new().connect(|sink| PtyTransport::spawn(&command, sink))?;
//! let style = TerminalStyle::new("Menlo", gpui_kit::px(13.));
//! let view = cx.new(|cx| TerminalView::new(terminal, style, cx));
//! # let _ = view;
//! # Ok(())
//! # }
//! ```

mod consts;
mod error;
mod event;
mod grid;
mod keys;
mod mouse;
mod paste;
#[cfg(feature = "pty")]
mod pty;
mod sink;
mod terminal;
mod transport;
mod view;

pub use consts::DEFAULT_GRID_SIZE;
pub use error::{Error, Result};
pub use event::TerminalEvent;
pub use grid::{Grid, GridSize};
pub use keys::{KeyInput, key_to_bytes};
pub use mouse::{MouseButton, MouseInput, mouse_to_bytes};
pub use paste::paste_payload;
#[cfg(feature = "pty")]
pub use pty::{PtyCommand, PtyTransport};
pub use sink::{ExitHook, OutputHook, TerminalSink};
pub use terminal::{Terminal, TerminalBuilder};
pub use transport::{ExitReport, Transport};
pub use view::{TerminalPalette, TerminalStyle, TerminalView};

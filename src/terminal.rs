//! A grid joined to a transport.

use std::sync::Arc;

use alacritty_terminal::event::Event;
use alacritty_terminal::term::ClipboardType;
use parking_lot::Mutex;

use crate::consts::DEFAULT_GRID_SIZE;
use crate::sink::{ExitHook, OutputHook};
use crate::{ExitReport, Grid, GridSize, Result, TerminalEvent, TerminalSink, Transport};

/// Configures a [`Terminal`] before its transport starts producing output.
pub struct TerminalBuilder {
    size:      GridSize,
    on_output: Option<OutputHook>,
    on_exit:   Option<ExitHook>,
}

impl Default for TerminalBuilder {
    fn default() -> Self {
        Self { size:      DEFAULT_GRID_SIZE,
               on_output: None,
               on_exit:   None, }
    }
}

impl TerminalBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    /// The size to start at. The view resizes to fit its pane on first
    /// layout; this is what the far end sees until then.
    pub fn size(mut self, size: GridSize) -> Self {
        self.size = size;
        self
    }

    /// Watches the raw output stream - for activity, or for text a host
    /// wants to find (a URL a narrow grid would break across rows). Runs on
    /// the transport's thread, before the grid parses the chunk.
    pub fn on_output(mut self, hook: impl Fn(&[u8]) + Send + Sync + 'static) -> Self {
        self.on_output = Some(Box::new(hook));
        self
    }

    /// Learns of the exit on the transport's thread, without waiting for a
    /// view to pump it. [`TerminalEvent::Exited`] still follows on the view.
    pub fn on_exit(mut self, hook: impl Fn(ExitReport) + Send + Sync + 'static) -> Self {
        self.on_exit = Some(Box::new(hook));
        self
    }

    /// Builds the transport with `connect`, handing it the sink it reports
    /// through, and joins the two.
    pub fn connect<T, E>(self, connect: impl FnOnce(TerminalSink) -> std::result::Result<T, E>)
                         -> std::result::Result<Terminal<T>, E>
        where T: Transport {
        let grid = Arc::new(Mutex::new(Grid::new(self.size)));
        let (wake_tx, wake_rx) = async_channel::bounded(1);
        let sink = TerminalSink::new(grid, self.on_output, self.on_exit, wake_tx);
        let transport = connect(sink.clone())?;
        Ok(Terminal { transport: Arc::new(Mutex::new(transport)),
                      sink,
                      wake: wake_rx })
    }
}

/// A terminal: its grid and the transport behind it.
///
/// A handle - clones share one terminal - so a host can keep writing to it
/// (a startup command, an injected prompt) while a [`crate::TerminalView`]
/// draws it. One view per terminal: the view's pump consumes the terminal's
/// events.
pub struct Terminal<T> {
    transport: Arc<Mutex<T>>,
    sink:      TerminalSink,
    wake:      async_channel::Receiver<()>,
}

impl<T> Clone for Terminal<T> {
    fn clone(&self) -> Self {
        Self { transport: Arc::clone(&self.transport),
               sink:      self.sink.clone(),
               wake:      self.wake.clone(), }
    }
}

impl<T: Transport> Terminal<T> {
    /// Sends input to the far end.
    pub fn write(&self, bytes: &[u8]) -> Result<()> {
        self.transport.lock().write(bytes)
    }

    /// Resizes the grid and tells the far end.
    pub fn resize(&self, size: GridSize) -> Result<()> {
        self.sink.grid().lock().resize(size);
        self.transport.lock().resize(size)
    }

    /// Ends the far end.
    pub fn terminate(&self) -> Result<()> {
        self.transport.lock().terminate()
    }

    /// See [`Transport::process_id`].
    pub fn process_id(&self) -> Option<u32> {
        self.transport.lock().process_id()
    }

    pub fn size(&self) -> GridSize {
        self.sink.size()
    }

    /// Reads the grid under its lock. Keep `read` short: the transport's
    /// reader waits on the same lock to parse output.
    pub fn with_grid<R>(&self, read: impl FnOnce(&Grid) -> R) -> R {
        read(&self.sink.grid().lock())
    }

    /// How the far end ended, once it has.
    pub fn exit_report(&self) -> Option<ExitReport> {
        self.sink.exit_report()
    }

    pub(crate) fn with_grid_mut<R>(&self, change: impl FnOnce(&mut Grid) -> R) -> R {
        change(&mut self.sink.grid().lock())
    }

    pub(crate) fn take_dirty(&self) -> bool {
        self.sink.grid().lock().take_dirty()
    }

    pub(crate) fn wake(&self) -> async_channel::Receiver<()> {
        self.wake.clone()
    }

    /// The program's requests since the last call, as events for the host.
    ///
    /// Replies the program expects on its input (a cursor position report,
    /// device attributes) are written back here rather than surfaced:
    /// a program that asks and never hears back can stall waiting.
    pub(crate) fn drain_events(&self) -> Vec<TerminalEvent> {
        let raised = self.sink.grid().lock().drain_events();
        let mut events = Vec::new();
        for event in raised {
            match event {
                Event::Title(title) => events.push(TerminalEvent::Title(title)),
                Event::ResetTitle => events.push(TerminalEvent::ResetTitle),
                Event::Bell => events.push(TerminalEvent::Bell),
                Event::ClipboardStore(ClipboardType::Clipboard, text) => {
                    events.push(TerminalEvent::ClipboardStore(text));
                }
                Event::PtyWrite(reply) => {
                    if let Err(error) = self.write(reply.as_bytes()) {
                        events.push(TerminalEvent::TransportFailed(Arc::new(error)));
                    }
                }
                // Clipboard reads are refused: a program reading the user's
                // clipboard is not something to grant silently. The rest are
                // alacritty's own window-system hooks, which have no
                // counterpart here.
                _ => {}
            }
        }
        events
    }
}

#[cfg(test)]
mod tests;

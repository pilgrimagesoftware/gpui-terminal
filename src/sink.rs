//! The transport's side of a terminal: where its output and its exit go.

use std::sync::Arc;

use parking_lot::Mutex;

use crate::{ExitReport, Grid, GridSize};

/// Sees every chunk of output before the grid parses it. Runs on the
/// transport's reader thread, so it must not block.
pub type OutputHook = Box<dyn Fn(&[u8]) + Send + Sync>;

/// Sees the exit report, once. Runs on the transport's reader thread.
pub type ExitHook = Box<dyn Fn(ExitReport) + Send + Sync>;

/// Handed to a [`crate::Transport`] when it is built; the transport reports
/// output and exit through it. Cheap to clone, `Send` and `Sync`, so a
/// transport may report from any thread or task.
#[derive(Clone)]
pub struct TerminalSink {
    shared: Arc<Shared>,
}

struct Shared {
    grid:      Arc<Mutex<Grid>>,
    on_output: Option<OutputHook>,
    on_exit:   Option<ExitHook>,
    exit:      Mutex<Option<ExitReport>>,
    /// Bounded to one: a wake already queued covers every report after it,
    /// since the pump drains everything each time it runs.
    wake:      async_channel::Sender<()>,
}

impl TerminalSink {
    pub(crate) fn new(grid: Arc<Mutex<Grid>>, on_output: Option<OutputHook>,
                      on_exit: Option<ExitHook>, wake: async_channel::Sender<()>)
                      -> Self {
        Self { shared: Arc::new(Shared { grid,
                                         on_output,
                                         on_exit,
                                         exit: Mutex::new(None),
                                         wake }), }
    }

    /// Delivers a chunk of output from the far end.
    pub fn output(&self, bytes: &[u8]) {
        // Before the parse and outside the grid lock: the hook is the host's
        // code, and it sees the stream as sent - a grid only keeps what is
        // still on screen.
        if let Some(on_output) = &self.shared.on_output {
            on_output(bytes);
        }
        self.shared.grid.lock().feed(bytes);
        self.wake();
    }

    /// Reports that the far end ended. Only the first report counts.
    pub fn exited(&self, report: ExitReport) {
        {
            let mut exit = self.shared.exit.lock();
            if exit.is_some() {
                return;
            }
            *exit = Some(report);
        }
        if let Some(on_exit) = &self.shared.on_exit {
            on_exit(report);
        }
        self.wake();
    }

    /// The terminal's current size, for a transport that has to state one
    /// when it opens.
    pub fn size(&self) -> GridSize {
        self.shared.grid.lock().size()
    }

    pub(crate) fn grid(&self) -> &Arc<Mutex<Grid>> {
        &self.shared.grid
    }

    pub(crate) fn exit_report(&self) -> Option<ExitReport> {
        *self.shared.exit.lock()
    }

    fn wake(&self) {
        // Full means a wake is already pending; closed means nothing is
        // listening any more. Neither leaves anything to do.
        let _ = self.shared.wake.try_send(());
    }
}

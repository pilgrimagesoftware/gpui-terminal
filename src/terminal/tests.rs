//! A terminal over a transport that records what it is asked to do.

use std::sync::Arc;

use parking_lot::Mutex;

use super::*;
use crate::{Error, GridSize};

#[derive(Default)]
struct Log {
    written:    Vec<u8>,
    sizes:      Vec<GridSize>,
    terminated: bool,
}

struct FakeTransport {
    log: Arc<Mutex<Log>>,
}

impl Transport for FakeTransport {
    fn write(&mut self, bytes: &[u8]) -> Result<()> {
        self.log.lock().written.extend_from_slice(bytes);
        Ok(())
    }

    fn resize(&mut self, size: GridSize) -> Result<()> {
        self.log.lock().sizes.push(size);
        Ok(())
    }

    fn terminate(&mut self) -> Result<()> {
        self.log.lock().terminated = true;
        Ok(())
    }
}

/// A terminal, the sink its transport would report through, and the log.
fn connect(builder: TerminalBuilder) -> (Terminal<FakeTransport>, TerminalSink, Arc<Mutex<Log>>) {
    let log = Arc::new(Mutex::new(Log::default()));
    let mut sink = None;
    let terminal = builder.connect(|handed| {
                              sink = Some(handed);
                              Ok::<_, Error>(FakeTransport { log: Arc::clone(&log), })
                          })
                          .expect("a fake transport connects");
    (terminal, sink.expect("connect hands the transport a sink"), log)
}

#[test]
fn output_reaches_the_grid_and_marks_it_dirty() {
    let (terminal, sink, _) = connect(TerminalBuilder::new());

    sink.output(b"hello");

    assert_eq!(terminal.with_grid(|grid| grid.row_text(0)), "hello");
    assert!(terminal.take_dirty());
}

#[test]
fn output_wakes_the_pump_once_however_much_arrives() {
    let (terminal, sink, _) = connect(TerminalBuilder::new());
    let wake = terminal.wake();

    sink.output(b"one");
    sink.output(b"two");

    assert_eq!(wake.try_recv(), Ok(()));
    assert!(wake.try_recv().is_err(), "one pending wake covers both");
}

#[test]
fn the_output_hook_sees_the_raw_stream() {
    let seen = Arc::new(Mutex::new(Vec::new()));
    let hook_seen = Arc::clone(&seen);
    let (_, sink, _) = connect(TerminalBuilder::new().on_output(move |bytes| {
                                                         hook_seen.lock().extend_from_slice(bytes)
                                                     }));

    sink.output(b"\x1b[1mbold");

    assert_eq!(seen.lock().as_slice(), b"\x1b[1mbold");
}

#[test]
fn only_the_first_exit_counts() {
    let exits = Arc::new(Mutex::new(Vec::new()));
    let hook_exits = Arc::clone(&exits);
    let (terminal, sink, _) =
        connect(TerminalBuilder::new().on_exit(move |report| hook_exits.lock().push(report)));

    sink.exited(ExitReport::new(Some(3)));
    sink.exited(ExitReport::new(Some(0)));

    assert_eq!(terminal.exit_report(), Some(ExitReport::new(Some(3))));
    assert_eq!(exits.lock().as_slice(), [ExitReport::new(Some(3))]);
}

#[test]
fn resize_changes_the_grid_and_tells_the_transport() {
    let (terminal, _, log) = connect(TerminalBuilder::new());
    let size = GridSize { columns: 100,
                          rows:    40, };

    terminal.resize(size).expect("the fake resizes");

    assert_eq!(terminal.size(), size);
    assert_eq!(log.lock().sizes, [size]);
}

#[test]
fn the_builder_sets_the_starting_size() {
    let size = GridSize { columns: 20,
                          rows:    5, };

    let (terminal, sink, _) = connect(TerminalBuilder::new().size(size));

    assert_eq!(terminal.size(), size);
    assert_eq!(sink.size(), size);
}

#[test]
fn writes_and_terminate_reach_the_transport() {
    let (terminal, _, log) = connect(TerminalBuilder::new());

    terminal.write(b"ls\r").expect("the fake writes");
    terminal.terminate().expect("the fake terminates");

    let log = log.lock();
    assert_eq!(log.written, b"ls\r");
    assert!(log.terminated);
}

#[test]
fn a_title_sequence_becomes_a_title_event() {
    let (terminal, sink, _) = connect(TerminalBuilder::new());

    sink.output(b"\x1b]0;my session\x07");

    assert!(terminal.drain_events()
                    .iter()
                    .any(|event| matches!(event, TerminalEvent::Title(title) if title == "my session")));
}

#[test]
fn a_clipboard_store_becomes_an_event_for_the_host() {
    let (terminal, sink, _) = connect(TerminalBuilder::new());

    // OSC 52, clipboard `c`, base64 of "copied".
    sink.output(b"\x1b]52;c;Y29waWVk\x07");

    assert!(terminal.drain_events()
                    .iter()
                    .any(|event| matches!(event, TerminalEvent::ClipboardStore(text) if text == "copied")));
}

/// A program that asks where the cursor is waits for the answer on its
/// input; dropping the reply is what leaves a shell hanging at startup.
#[test]
fn a_cursor_position_query_is_answered_on_the_transport() {
    let (terminal, sink, log) = connect(TerminalBuilder::new());

    sink.output(b"\x1b[6n");
    let events = terminal.drain_events();

    assert!(events.is_empty(),
            "the reply is written, not surfaced: {events:?}");
    assert_eq!(log.lock().written, b"\x1b[1;1R");
}

#[test]
fn clones_share_one_terminal() {
    let (terminal, sink, _) = connect(TerminalBuilder::new());
    let other = terminal.clone();

    sink.output(b"shared");

    assert_eq!(other.with_grid(|grid| grid.row_text(0)), "shared");
}

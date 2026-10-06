//! The view on GPUI's headless test platform: the pump, layout and input.

use std::sync::Arc;

use gpui_kit::{Entity, Focusable as _, TestAppContext, VisualTestContext, px};
use parking_lot::Mutex;

use super::*;
use crate::{
    Error, ExitReport, GridSize, Result, TerminalBuilder, TerminalEvent, TerminalSink, Transport,
};

#[derive(Default)]
struct Log {
    written: Vec<u8>,
    sizes:   Vec<GridSize>,
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
        Ok(())
    }
}

struct Harness {
    view: Entity<TerminalView<FakeTransport>>,
    sink: TerminalSink,
    log:  Arc<Mutex<Log>>,
}

fn harness(cx: &mut TestAppContext) -> (Harness, VisualTestContext) {
    cx.update(gpui_kit::init);
    let log = Arc::new(Mutex::new(Log::default()));
    let mut sink = None;
    let terminal = TerminalBuilder::new().connect(|handed| {
                       sink = Some(handed);
                       Ok::<_, Error>(FakeTransport { log: Arc::clone(&log), })
                   })
                   .expect("a fake transport connects");
    let style = TerminalStyle::new("Menlo", px(13.));
    let window = cx.add_window(|_, cx| TerminalView::new(terminal, style, cx));
    let view = window.root(cx).expect("the window's root is the view");
    let cx = VisualTestContext::from_window(window.into(), cx);
    cx.run_until_parked();
    (Harness { view,
               sink: sink.expect("connect hands the transport a sink"),
               log },
     cx)
}

/// Events the view emitted, collected as they arrive.
fn record_events(view: &Entity<TerminalView<FakeTransport>>, cx: &mut VisualTestContext)
                 -> (Arc<Mutex<Vec<TerminalEvent>>>, gpui_kit::Subscription) {
    let events = Arc::new(Mutex::new(Vec::new()));
    let seen = Arc::clone(&events);
    let subscription =
        cx.update(|_, cx| cx.subscribe(view, move |_, event, _| seen.lock().push(event.clone())));
    (events, subscription)
}

#[gpui_kit::test]
fn the_view_sizes_the_terminal_to_its_bounds(cx: &mut TestAppContext) {
    let (harness, cx) = harness(cx);

    let size = harness.view
                      .read_with(&cx, |view, _| view.terminal().size());

    assert_eq!(harness.log.lock().sizes.last(),
               Some(&size),
               "the transport is told the size the grid now has");
}

#[gpui_kit::test]
fn output_from_the_transport_reaches_the_host_as_events(cx: &mut TestAppContext) {
    let (harness, mut cx) = harness(cx);
    let (events, _subscription) = record_events(&harness.view, &mut cx);

    harness.sink.output(b"\x1b]0;my session\x07");
    harness.sink.exited(ExitReport::new(Some(0)));
    cx.run_until_parked();

    let events = events.lock();
    assert!(events.iter()
                  .any(|event| matches!(event, TerminalEvent::Title(title) if title == "my session")),
            "{events:?}");
    let exits = events.iter()
                      .filter(|event| matches!(event, TerminalEvent::Exited(_)))
                      .count();
    assert_eq!(exits, 1, "{events:?}");
}

#[gpui_kit::test]
fn typing_in_the_focused_view_writes_to_the_transport(cx: &mut TestAppContext) {
    let (harness, mut cx) = harness(cx);
    cx.update(|window, cx| harness.view.focus_handle(cx).focus(window, cx));

    cx.simulate_keystrokes("l s enter");

    assert_eq!(harness.log.lock().written, b"ls\r");
}

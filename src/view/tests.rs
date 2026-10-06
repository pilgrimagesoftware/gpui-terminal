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

/// Scrolls the wheel over the view, `lines` up (negative: down).
fn wheel(cx: &mut VisualTestContext, lines: f32) {
    use gpui_kit::{ScrollDelta, ScrollWheelEvent, point};
    cx.simulate_event(ScrollWheelEvent { position: point(px(40.), px(40.)),
                                         delta: ScrollDelta::Lines(point(0., lines)),
                                         ..Default::default() });
    cx.run_until_parked();
}

fn top_row(harness: &Harness, cx: &mut VisualTestContext) -> String {
    harness.view.read_with(cx, |view, _| {
                    view.terminal().with_grid(|grid| grid.row_text(0))
                })
}

#[gpui_kit::test]
fn the_wheel_scrolls_the_scrollback_and_typing_returns_to_the_live_screen(cx: &mut TestAppContext) {
    let (harness, mut cx) = harness(cx);
    for line in 0..200 {
        harness.sink.output(format!("line {line}\r\n").as_bytes());
    }
    cx.run_until_parked();
    cx.update(|window, cx| harness.view.focus_handle(cx).focus(window, cx));
    let live = top_row(&harness, &mut cx);

    wheel(&mut cx, 3.);
    let scrolled = top_row(&harness, &mut cx);
    assert_ne!(scrolled, live, "the view moved back into the scrollback");
    assert!(harness.log.lock().written.is_empty(),
            "no wheel report to a program that didn't ask for the mouse");

    cx.simulate_keystrokes("a");
    assert_eq!(top_row(&harness, &mut cx),
               live,
               "typing returns to the live screen");
    assert_eq!(harness.log.lock().written, b"a");
}

#[gpui_kit::test]
fn a_mouse_aware_program_gets_the_wheel_instead(cx: &mut TestAppContext) {
    let (harness, mut cx) = harness(cx);
    for line in 0..200 {
        harness.sink.output(format!("line {line}\r\n").as_bytes());
    }
    // SGR mouse reporting on, any-event tracking.
    harness.sink.output(b"\x1b[?1000h\x1b[?1006h");
    cx.run_until_parked();
    let live = top_row(&harness, &mut cx);

    wheel(&mut cx, 3.);
    assert_eq!(top_row(&harness, &mut cx), live, "the view stays put");
    assert!(!harness.log.lock().written.is_empty(),
            "the program got a wheel report");
}

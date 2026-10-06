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

/// Scrolls a trackpad's `pixels` over the view, up (negative: down).
fn swipe(cx: &mut VisualTestContext, pixels: f32) {
    use gpui_kit::{ScrollDelta, ScrollWheelEvent, point};
    cx.simulate_event(ScrollWheelEvent { position: point(px(40.), px(40.)),
                                         delta: ScrollDelta::Pixels(point(px(0.), px(pixels))),
                                         ..Default::default() });
    cx.run_until_parked();
}

fn display_offset(harness: &Harness, cx: &mut VisualTestContext) -> usize {
    harness.view.read_with(cx, |view, _| {
                    view.terminal().with_grid(|grid| grid.display_offset())
                })
}

/// A trackpad's small steps add up: a quarter of a line at a time scrolls
/// one line per four steps, not one per step - and turning back starts
/// from nothing rather than spending what was left going up.
#[gpui_kit::test]
fn trackpad_pixels_add_up_to_whole_lines(cx: &mut TestAppContext) {
    let (harness, mut cx) = harness(cx);
    for line in 0..200 {
        harness.sink.output(format!("line {line}\r\n").as_bytes());
    }
    cx.run_until_parked();
    let quarter = harness.view
                         .read_with(&cx, |view, _| view.metrics.expect("measured").height)
                  / 4.;

    swipe(&mut cx, quarter);
    swipe(&mut cx, quarter);
    swipe(&mut cx, quarter);
    assert_eq!(display_offset(&harness, &mut cx),
               0,
               "three quarters: not a line yet");
    swipe(&mut cx, quarter);
    assert_eq!(display_offset(&harness, &mut cx),
               1,
               "four quarters: one line");
    for _ in 0..8 {
        swipe(&mut cx, quarter);
    }
    assert_eq!(display_offset(&harness, &mut cx),
               3,
               "twelve quarters: three lines");

    swipe(&mut cx, quarter * 3.);
    swipe(&mut cx, -quarter * 3.);
    assert_eq!(display_offset(&harness, &mut cx),
               3,
               "turning back starts afresh: three quarters down is not a line");
    swipe(&mut cx, -quarter);
    assert_eq!(display_offset(&harness, &mut cx), 2);
}

#[gpui_kit::test]
fn after_the_program_exits_keys_and_pastes_go_nowhere(cx: &mut TestAppContext) {
    let (harness, mut cx) = harness(cx);
    cx.update(|window, cx| harness.view.focus_handle(cx).focus(window, cx));
    harness.sink.output(b"$ ");
    harness.sink.exited(ExitReport::new(Some(0)));
    cx.run_until_parked();

    cx.simulate_keystrokes("l s enter ctrl-c");
    cx.update(|_, cx| cx.write_to_clipboard(gpui_kit::ClipboardItem::new_string("echo hi".into())));
    harness.view
           .update(&mut cx, |view, cx| view.paste_clipboard(cx));

    assert!(harness.log.lock().written.is_empty(),
            "nothing reached a finished program: {:?}",
            harness.log.lock().written);
    let screen = harness.view.read_with(&cx, |view, _| {
                                 view.terminal().with_grid(|grid| grid.row_text(0))
                             });
    assert_eq!(screen, "$", "the screen stays to read");
}

/// A program that exits with mouse reporting on gets no more mouse input:
/// a click sends nothing (it selects instead), and the wheel scrolls the
/// scrollback rather than reporting to a program that is gone.
#[gpui_kit::test]
fn after_exit_the_mouse_goes_to_the_view_not_the_program(cx: &mut TestAppContext) {
    use gpui_kit::{Modifiers, point};
    let (harness, mut cx) = harness(cx);
    for line in 0..200 {
        harness.sink.output(format!("line {line}\r\n").as_bytes());
    }
    // SGR mouse reporting on, then the program exits without turning it off.
    harness.sink.output(b"\x1b[?1000h\x1b[?1006h");
    harness.sink.exited(ExitReport::new(Some(0)));
    cx.run_until_parked();
    let live = top_row(&harness, &mut cx);

    cx.simulate_click(point(px(40.), px(40.)), Modifiers::none());
    wheel(&mut cx, 3.);

    assert!(harness.log.lock().written.is_empty(),
            "nothing reached the exited program: {:?}",
            harness.log.lock().written);
    assert_ne!(top_row(&harness, &mut cx),
               live,
               "the wheel scrolled the history instead");
}

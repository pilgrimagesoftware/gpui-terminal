//! The view entity: its state, its pump, and what a host can ask of it.

use std::sync::Arc;

use gpui_kit::{
    App, Bounds, Context, EventEmitter, FocusHandle, Focusable, Pixels, Point, Task, Window,
};

use super::TerminalStyle;
use super::metrics::CellMetrics;
use crate::{Result, Terminal, TerminalEvent, Transport};

/// Draws a [`Terminal`] and routes input to it.
///
/// Create it with `cx.new(|cx| TerminalView::new(terminal, style, cx))`, put
/// the entity in the element tree, and subscribe for [`TerminalEvent`]s. It
/// repaints itself when output arrives and sizes the terminal to whatever
/// bounds it is laid out in; the host does neither.
pub struct TerminalView<T> {
    pub(super) terminal:         Terminal<T>,
    pub(super) focus:            FocusHandle,
    pub(super) style:            TerminalStyle,
    /// Measured for `style`, on the first render after it changed. Input
    /// handlers read it too, which is sound: an event only reaches the view
    /// after a frame has drawn it, and that frame measured.
    pub(super) metrics:          Option<CellMetrics>,
    /// Where the grid's top-left corner was last laid out, in window
    /// coordinates, for turning a mouse position into a cell.
    pub(super) origin:           Point<Pixels>,
    /// Scrolling that hasn't yet added up to a whole line (see `on_scroll`).
    pub(super) scroll_remainder: f32,
    exit_reported:               bool,
    _pump:                       Task<()>,
}

impl<T: Transport> TerminalView<T> {
    pub fn new(terminal: Terminal<T>, style: TerminalStyle, cx: &mut Context<Self>) -> Self {
        let wake = terminal.wake();
        // A wake queued before this ran - output, or an exit, while the view
        // was being built - is still in the channel, so nothing is missed.
        let pump = cx.spawn(async move |view, cx| {
                         while wake.recv().await.is_ok() {
                             if view.update(cx, |view, cx| view.pump(cx)).is_err() {
                                 break;
                             }
                         }
                     });
        Self { terminal,
               focus: cx.focus_handle(),
               style,
               metrics: None,
               origin: Point::default(),
               scroll_remainder: 0.,
               exit_reported: false,
               _pump: pump }
    }

    pub fn terminal(&self) -> &Terminal<T> {
        &self.terminal
    }

    pub fn style(&self) -> &TerminalStyle {
        &self.style
    }

    /// Changes the font, re-measuring and refitting on the next frame. A
    /// no-op when `style` is the one already in use, so a host may call it
    /// every frame.
    pub fn set_style(&mut self, style: TerminalStyle, cx: &mut Context<Self>) {
        if self.style != style {
            self.style = style;
            self.metrics = None;
            cx.notify();
        }
    }

    /// Hands what the transport delivered to the frame: a repaint if the grid
    /// changed, the program's requests as events, and the exit, once.
    fn pump(&mut self, cx: &mut Context<Self>) {
        if self.terminal.take_dirty() {
            cx.notify();
        }
        for event in self.terminal.drain_events() {
            cx.emit(event);
        }
        if !self.exit_reported
           && let Some(report) = self.terminal.exit_report()
        {
            self.exit_reported = true;
            cx.emit(TerminalEvent::Exited(report));
        }
    }

    pub(super) fn cell_metrics(&mut self, cx: &App) -> CellMetrics {
        *self.metrics
             .get_or_insert_with(|| CellMetrics::measure(&self.style, cx))
    }

    /// Sizes the terminal to `bounds`, the space its grid was laid out in.
    ///
    /// Runs in prepaint, where the bounds are first known, so a size change
    /// cannot repaint the frame it is in - GPUI ignores a refresh asked for
    /// mid-draw. The refresh is deferred to after it instead.
    pub(super) fn fit(&mut self, bounds: Bounds<Pixels>, window: &mut Window,
                      cx: &mut Context<Self>) {
        self.origin = bounds.origin;
        let Some(metrics) = self.metrics
        else {
            return;
        };
        let size = metrics.grid_size(bounds.size);
        if size != self.terminal.size() {
            let resized = self.terminal.resize(size);
            self.report(resized, cx);
            window.defer(cx, |window, _| window.refresh());
        }
    }

    /// Hands a failed transport call to the host.
    pub(super) fn report(&self, result: Result<()>, cx: &mut Context<Self>) {
        if let Err(error) = result {
            cx.emit(TerminalEvent::TransportFailed(Arc::new(error)));
        }
    }
}

impl<T: Transport> EventEmitter<TerminalEvent> for TerminalView<T> {}

impl<T: Transport> Focusable for TerminalView<T> {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus.clone()
    }
}

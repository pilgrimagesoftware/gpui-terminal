# gpui-terminal

A terminal emulator view for [GPUI](https://www.gpui.rs/), backed by
[`alacritty_terminal`](https://crates.io/crates/alacritty_terminal), over any byte
transport.

Three layers, each usable without the one above it:

- [`Grid`] parses VT/ANSI output into cells, backed by `alacritty_terminal`.
- [`Terminal`] joins a grid to a [`Transport`] - anything that can take bytes, a
  size and a hang-up, and hands output back through a `TerminalSink`.
  [`PtyTransport`] (the default `pty` feature) is a local pseudo-terminal; a
  remote exec or a serial line implements the same trait.
- [`TerminalView`] is the GPUI entity that draws the grid, turns keys, mouse,
  paste and resize into transport input, and emits `TerminalEvent`s for what
  the running program asks of its host.

`key_to_bytes`, `mouse_to_bytes` and `paste_payload` are the input encodings on
their own, free of GPUI types, for an embedder that draws its own surface.

## Features

- `pty` (default on) - a local pseudo-terminal transport via `portable-pty`.
  Turn it off (`default-features = false`) for an embedder whose bytes come
  from somewhere else - a remote exec, a serial line - and that has no use for
  `portable-pty` in its build.

## Usage

With the default `pty` feature, running the user's shell:

```rust,no_run
use gpui_kit::AppContext as _;
use gpui_terminal::{PtyCommand, PtyTransport, TerminalBuilder, TerminalStyle, TerminalView};

fn open(cx: &mut gpui_kit::App) -> gpui_terminal::Result<()> {
    let command = PtyCommand::user_shell().arg("-i");
    let terminal = TerminalBuilder::new().connect(|sink| PtyTransport::spawn(&command, sink))?;
    let style = TerminalStyle::new("Menlo", gpui_kit::px(13.));
    let view = cx.new(|cx| TerminalView::new(terminal, style, cx));
    let _ = view;
    Ok(())
}
```

### Colours

`TerminalStyle::palette` sets the colours: the foreground and background, the
sixteen ANSI colours, and optional cursor and selection backgrounds (without
them, the cursor and selection swap the cell's colours). Each is `0xRRGGBB`;
`TerminalPalette::rgb` converts a GPUI colour, so a host can map its theme:

```rust,ignore
let palette = TerminalPalette { foreground: TerminalPalette::rgb(theme.foreground),
                                background: TerminalPalette::rgb(theme.background),
                                selection: Some(TerminalPalette::rgb(theme.selection)),
                                ..TerminalPalette::default() };
let style = TerminalStyle::new("Menlo", gpui_kit::px(13.)).palette(palette);
```

### Implementing `Transport`

An embedder whose bytes come from somewhere other than a local PTY - a remote
exec, a serial line - implements `Transport` directly:

```rust,no_run
use gpui_terminal::{GridSize, Result, Transport};

struct MyTransport {
    // a writer handle to the far end
}

impl Transport for MyTransport {
    fn write(&mut self, bytes: &[u8]) -> Result<()> {
        // send `bytes` to the far end
        Ok(())
    }

    fn resize(&mut self, size: GridSize) -> Result<()> {
        // tell the far end its terminal is now `size`, if it can be told
        Ok(())
    }

    fn terminate(&mut self) -> Result<()> {
        // end the far end: kill the child, close the stream
        Ok(())
    }

    fn process_id(&self) -> Option<u32> {
        // the local process behind this terminal, if there is one
        None
    }
}
```

Output goes the other way: whatever reads the far end's bytes - a thread, a
task - calls `TerminalSink::output` with each chunk and `TerminalSink::exited`
once when it ends. `TerminalBuilder::connect` hands the sink to the closure
that builds the transport, so the two are wired together at construction.

## The `gpui-kit` pin

This crate pins `gpui-kit = "=0.7.0"` exactly, not a range. Two `gpui` versions
in one build are two incompatible `Entity` types, so every consumer embedding a
`TerminalView` in its own window must resolve the same `gpui-kit` as this
crate. Bump this pin in lockstep with the consumers below, in the same change.

## Consumers

- [Knot](https://github.com/pilgrimagesoftware/Knot) - an agent-first terminal
  and workspace manager.
- [Fernrohr](https://github.com/pilgrimagesoftware/Fernrohr) - a GPUI
  Kubernetes client.

## License

MIT - see [LICENSE](LICENSE).

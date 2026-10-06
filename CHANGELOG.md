# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.2.0] - 2026-10-06

### Added

- `TerminalPalette` on `TerminalStyle` (`TerminalStyle::palette`): a host's
  foreground, background and sixteen ANSI colours, and optional cursor and
  selection colours (default: swap the cell's colours, as before).
  `TerminalPalette::rgb` turns a GPUI colour into an entry. The default
  palette is the previous fixed one.
- Scrollback: without a mouse-aware program listening, the wheel scrolls the
  view back through the scrollback (`Grid::scroll_display`,
  `scroll_to_bottom`, `display_offset`). Typing or pasting returns to the
  live screen. Selections made scrolled back cover the rows they were made
  over, and the cursor shows only while its row is in view.
- Ctrl-Shift-C and Ctrl-Shift-V copy and paste off macOS, where Ctrl-C
  belongs to the program. The platform modifier's `c` and `v` still do
  everywhere.

### Fixed

- A focused terminal now takes its keystrokes ahead of the app's key
  bindings, through a keystroke interceptor, so a binding on the focus path
  can't take a key meant for the program: gpui-component's Root binds Tab and
  Shift-Tab (focus cycling) and, off macOS, Ctrl-C (copy). Platform chords
  other than copy and paste still go to the app.
- Shift-Tab sends back-tab (`ESC [ Z`). `KeyInput` gains `shift`.

### Changed

- Once the program has exited, the view sends no more input - keys, pastes
  or mouse reports - to the transport. The screen stays to read, select
  and copy, and with mouse reporting left on, the wheel scrolls the
  scrollback again.

## [0.1.0] - 2026-10-05

### Added

- Initial extraction from `pilgrimagesoftware/Knot`'s `crates/gpui-terminal`
  into a standalone crate: `Grid`, `Terminal`/`TerminalBuilder`, the
  `Transport` trait, `TerminalView`, and the `pty` feature's `PtyTransport`.

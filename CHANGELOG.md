# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- `TerminalPalette` on `TerminalStyle` (`TerminalStyle::palette`): a host's
  foreground, background and sixteen ANSI colours, and optional cursor and
  selection colours (default: swap the cell's colours, as before).
  `TerminalPalette::rgb` turns a GPUI colour into an entry. The default
  palette is the previous fixed one.
- Ctrl-Shift-C and Ctrl-Shift-V copy and paste off macOS, where Ctrl-C
  belongs to the program. The platform modifier's `c` and `v` still do
  everywhere.

### Changed

- Once the program has exited, the view sends no more input - keys, pastes
  or mouse reports - to the transport. The screen stays to read, select
  and copy.

## [0.1.0] - 2026-10-05

### Added

- Initial extraction from `pilgrimagesoftware/Knot`'s `crates/gpui-terminal`
  into a standalone crate: `Grid`, `Terminal`/`TerminalBuilder`, the
  `Transport` trait, `TerminalView`, and the `pty` feature's `PtyTransport`.

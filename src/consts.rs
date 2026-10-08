//! The crate's fixed values, in one place.

use crate::GridSize;

/// The size a terminal starts at, before its view has measured the space it
/// was given - the traditional VT100 default.
pub const DEFAULT_GRID_SIZE: GridSize = GridSize { columns: 80,
                                                   rows:    24, };

/// Foreground for cells that ask for the scheme's default.
pub(crate) const DEFAULT_FOREGROUND: u32 = 0xE0E0E0;

/// Background for cells that ask for the scheme's default, and for the pane.
pub(crate) const DEFAULT_BACKGROUND: u32 = 0x262626;

/// Cell width used when the text system cannot report the font's advance.
pub(crate) const FALLBACK_CELL_WIDTH: f32 = 8.;

/// The standard 16-colour ANSI palette (xterm's defaults), indexed the way
/// SGR 30-37 and 90-97 address it.
pub(crate) const ANSI_16: [u32; 16] = [0x000000, 0xCD0000, 0x00CD00, 0xCDCD00, 0x0000EE, 0xCD00CD,
                                       0x00CDCD, 0xE5E5E5, 0x7F7F7F, 0xFF0000, 0x00FF00, 0xFFFF00,
                                       0x5C5CFF, 0xFF00FF, 0x00FFFF, 0xFFFFFF];

/// The six channel levels of the xterm 256-colour palette's 6x6x6 cube.
pub(crate) const CUBE_LEVELS: [u8; 6] = [0, 95, 135, 175, 215, 255];

/// How much a PTY read takes at a time.
#[cfg(feature = "pty")]
pub(crate) const PTY_READ_BUFFER: usize = 4096;

/// Environment variables that identify the *host* terminal app (Warp, iTerm2,
/// and so on) rather than this one. See `PtyCommand`.
#[cfg(feature = "pty")]
pub(crate) const HOST_TERMINAL_ENV_PREFIXES: &[&str] = &["WARP_", "ITERM_", "KONSOLE_", "VTE_"];

/// The exact-name half of [`HOST_TERMINAL_ENV_PREFIXES`].
#[cfg(feature = "pty")]
pub(crate) const HOST_TERMINAL_ENV_NAMES: &[&str] =
    &["TERM_PROGRAM", "TERM_PROGRAM_VERSION", "TERM_SESSION_ID"];

/// The shell [`crate::PtyCommand::user_shell`] falls back to when `$SHELL`
/// is unset.
#[cfg(feature = "pty")]
pub(crate) const FALLBACK_SHELL: &str = "/bin/sh";

/// How long a PTY child's process groups get to exit after the SIGHUP that
/// ends them, before whatever is left gets SIGKILL (`pty::hangup`).
#[cfg(all(feature = "pty", unix))]
pub(crate) const PTY_KILL_GRACE: std::time::Duration = std::time::Duration::from_millis(500);

/// How often that wait checks whether they have.
#[cfg(all(feature = "pty", unix))]
pub(crate) const PTY_KILL_POLL: std::time::Duration = std::time::Duration::from_millis(20);

//! A local pseudo-terminal transport, via `portable-pty`.

use std::ffi::OsString;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;

use parking_lot::Mutex;
use portable_pty::{Child, CommandBuilder, MasterPty, PtySize, native_pty_system};

use crate::consts::{
    FALLBACK_SHELL, HOST_TERMINAL_ENV_NAMES, HOST_TERMINAL_ENV_PREFIXES, PTY_READ_BUFFER,
};
use crate::{Error, ExitReport, GridSize, Result, TerminalSink, Transport};

/// What to run in a [`PtyTransport`].
///
/// The child inherits this process's environment, minus the variables that
/// identify the *host* terminal app (Warp, iTerm2, ...) that launched it. Left
/// in place, a shell integration script sourced by the nested shell reports
/// status and titles straight back to the host terminal's own session - by
/// session id over IPC, not through this PTY - because the nested shell
/// believes it is still running directly inside that terminal. Anything set
/// with [`Self::env`] is applied after the strip, so a host can name itself.
#[derive(Debug, Clone)]
pub struct PtyCommand {
    program: OsString,
    args:    Vec<OsString>,
    cwd:     Option<PathBuf>,
    env:     Vec<(OsString, OsString)>,
}

impl PtyCommand {
    pub fn new(program: impl Into<OsString>) -> Self {
        Self { program: program.into(),
               args:    Vec::new(),
               cwd:     None,
               env:     Vec::new(), }
    }

    /// The user's shell: `$SHELL`, or `/bin/sh` without one.
    pub fn user_shell() -> Self {
        Self::new(std::env::var_os("SHELL").unwrap_or_else(|| FALLBACK_SHELL.into()))
    }

    pub fn arg(mut self, arg: impl Into<OsString>) -> Self {
        self.args.push(arg.into());
        self
    }

    pub fn cwd(mut self, dir: impl Into<PathBuf>) -> Self {
        self.cwd = Some(dir.into());
        self
    }

    pub fn env(mut self, key: impl Into<OsString>, value: impl Into<OsString>) -> Self {
        self.env.push((key.into(), value.into()));
        self
    }

    fn to_builder(&self) -> CommandBuilder {
        let mut command = CommandBuilder::new(&self.program);
        command.args(&self.args);
        if let Some(cwd) = &self.cwd {
            command.cwd(cwd);
        }
        strip_host_terminal_env(&mut command);
        for (key, value) in &self.env {
            command.env(key, value);
        }
        command
    }
}

fn strip_host_terminal_env(command: &mut CommandBuilder) {
    for (key, _) in std::env::vars_os() {
        let Some(name) = key.to_str()
        else {
            continue;
        };
        if HOST_TERMINAL_ENV_NAMES.contains(&name)
           || HOST_TERMINAL_ENV_PREFIXES.iter()
                                        .any(|prefix| name.starts_with(prefix))
        {
            command.env_remove(&key);
        }
    }
}

/// A child process on a local pseudo-terminal.
///
/// Dropping it kills the child.
pub struct PtyTransport {
    writer: Box<dyn Write + Send>,
    child:  Arc<Mutex<Box<dyn Child + Send + Sync>>>,
    master: Box<dyn MasterPty + Send>,
    /// Set once the reader thread has reaped the child.
    ///
    /// `portable_pty` goes on reporting the PID it spawned after the process
    /// is gone, so this flag - not the child - is what stops
    /// [`Transport::process_id`] naming a PID the kernel may since have given
    /// to somebody else.
    exited: Arc<AtomicBool>,
}

impl PtyTransport {
    /// Spawns `command` on a new PTY sized to the terminal, and starts a
    /// thread that reports its output and exit through `sink`.
    pub fn spawn(command: &PtyCommand, sink: TerminalSink) -> Result<Self> {
        let pair = native_pty_system().openpty(pty_size(sink.size()))
                                      .map_err(Error::transport)?;
        let child = pair.slave
                        .spawn_command(command.to_builder())
                        .map_err(Error::transport)?;
        // `pair.slave` drops with `pair` at the end of this function. It has
        // to: the reader only sees end-of-file once every handle on the slave
        // side is closed, and the child's are the only ones that should be.
        let child = Arc::new(Mutex::new(child));
        let reader = pair.master.try_clone_reader().map_err(Error::transport)?;
        let writer = pair.master.take_writer().map_err(Error::transport)?;
        let exited = Arc::new(AtomicBool::new(false));
        thread::Builder::new().name("gpui-terminal-pty".into())
                              .spawn({
                                  let child = Arc::clone(&child);
                                  let exited = Arc::clone(&exited);
                                  move || read_until_exit(reader, &child, &exited, &sink)
                              })?;
        Ok(Self { writer,
                  child,
                  master: pair.master,
                  exited })
    }
}

/// The reader thread: forward output until end-of-file, then reap the child
/// and report how it ended.
fn read_until_exit(mut reader: Box<dyn Read + Send>,
                   child: &Mutex<Box<dyn Child + Send + Sync>>, exited: &AtomicBool,
                   sink: &TerminalSink) {
    let mut buffer = [0_u8; PTY_READ_BUFFER];
    loop {
        match reader.read(&mut buffer) {
            Ok(0) | Err(_) => break,
            Ok(size) => sink.output(&buffer[..size]),
        }
    }
    let code = child.lock()
                    .wait()
                    .ok()
                    .map(|status| status.exit_code() as i32);
    // Before the report, so anything it wakes already sees no live process.
    exited.store(true, Ordering::Release);
    sink.exited(ExitReport::new(code));
}

fn pty_size(size: GridSize) -> PtySize {
    PtySize { rows:         clamp_u16(size.rows),
              cols:         clamp_u16(size.columns),
              pixel_width:  0,
              pixel_height: 0, }
}

fn clamp_u16(value: usize) -> u16 {
    u16::try_from(value).unwrap_or(u16::MAX)
}

impl Drop for PtyTransport {
    fn drop(&mut self) {
        // Best effort: a child that already exited cannot be killed again,
        // and there is no caller left to tell.
        let _ = self.child.lock().kill();
    }
}

impl Transport for PtyTransport {
    fn write(&mut self, bytes: &[u8]) -> Result<()> {
        self.writer.write_all(bytes)?;
        Ok(())
    }

    fn resize(&mut self, size: GridSize) -> Result<()> {
        self.master.resize(pty_size(size)).map_err(Error::transport)
    }

    fn terminate(&mut self) -> Result<()> {
        self.child.lock().kill()?;
        Ok(())
    }

    fn process_id(&self) -> Option<u32> {
        if self.exited.load(Ordering::Acquire) {
            return None;
        }
        self.child.lock().process_id()
    }
}

#[cfg(test)]
mod tests;

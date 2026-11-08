//! Runs an interactive shell on a pseudo-terminal and types lines into it,
//! one per prompt, so a test sees what an operator's shell does.
//!
//! An interactive shell reading a pipe draws no prompt, and fish runs none
//! of the lines typed ahead of its first prompt, so a hook test needs a
//! terminal. The test sets a prompt that ends in [`PROMPT`], and the helper
//! waits for it before each line, up to [`WAIT`].

use std::io::{Read as _, Write as _};
use std::process::Child;
use std::sync::Arc;
use std::sync::mpsc::{self, Receiver};
use std::thread;
use std::time::{Duration, Instant};

use pty_process::Size;
use pty_process::blocking::{Command, Pty};

/// The end of every prompt a test sets, which no typed line contains.
pub(crate) const PROMPT: &str = "READY> ";

/// How long a shell may take to start, or to finish one line.
const WAIT: Duration = Duration::from_secs(20);

/// A shell started on its own terminal, and everything it wrote there.
pub(crate) struct Terminal {
    pty: Arc<Pty>,
    output: Receiver<Vec<u8>>,
    seen: Vec<u8>,
    read: usize,
    child: Child,
}

impl Terminal {
    /// Start `command` on a new 24 by 80 terminal, and wait for its first
    /// prompt.
    pub(crate) fn start(command: Command) -> Self {
        let (pty, pts) = pty_process::blocking::open()
            .unwrap_or_else(|error| panic!("open a terminal: {error}"));
        pty.resize(Size::new(24, 80))
            .unwrap_or_else(|error| panic!("size the terminal: {error}"));
        let child = command.spawn(pts).unwrap_or_else(|error| {
            panic!("start the shell; mise.test.toml pins zsh and fish: {error}")
        });
        let pty = Arc::new(pty);
        let (sender, output) = mpsc::channel();
        let reader = Arc::clone(&pty);
        thread::spawn(move || {
            let mut buffer = [0; 4096];
            while let Ok(read @ 1..) = (&*reader).read(&mut buffer) {
                if sender.send(buffer[..read].to_vec()).is_err() {
                    break;
                }
            }
        });
        let mut terminal = Self {
            pty,
            output,
            seen: Vec::new(),
            read: 0,
            child,
        };
        terminal.wait_for_prompt();
        terminal
    }

    /// Type `line` and Enter, then wait for the next prompt.
    pub(crate) fn type_line(&mut self, line: &str) {
        (&*self.pty)
            .write_all(format!("{line}\r").as_bytes())
            .unwrap_or_else(|error| panic!("type `{line}`: {error}"));
        self.wait_for_prompt();
    }

    /// Type `exit`, wait for the shell to end, and return everything it
    /// wrote to the terminal.
    pub(crate) fn exit(mut self) -> String {
        (&*self.pty)
            .write_all(b"exit\r")
            .unwrap_or_else(|error| panic!("type `exit`: {error}"));
        let deadline = Instant::now() + WAIT;
        loop {
            match self.child.try_wait() {
                Ok(Some(_)) => break,
                Ok(None) if Instant::now() < deadline => {
                    self.receive(Duration::from_millis(50));
                }
                Ok(None) => {
                    let _ = self.child.kill();
                    panic!("the shell did not exit within {WAIT:?}:\n{}", self.text());
                }
                Err(error) => panic!("wait for the shell: {error}"),
            }
        }
        while self.receive(Duration::from_millis(100)) {}
        self.text()
    }

    fn wait_for_prompt(&mut self) {
        let deadline = Instant::now() + WAIT;
        loop {
            if let Some(at) = self.seen[self.read..]
                .windows(PROMPT.len())
                .position(|window| window == PROMPT.as_bytes())
            {
                self.read += at + PROMPT.len();
                return;
            }
            let left = deadline.saturating_duration_since(Instant::now());
            assert!(
                self.receive(left),
                "no prompt within {WAIT:?}:\n{}",
                self.text()
            );
        }
    }

    /// Add what the shell writes within `wait` to [`Terminal::seen`], and
    /// say whether it wrote anything.
    fn receive(&mut self, wait: Duration) -> bool {
        match self.output.recv_timeout(wait) {
            Ok(chunk) => {
                self.seen.extend(chunk);
                true
            }
            Err(_) => false,
        }
    }

    fn text(&self) -> String {
        String::from_utf8_lossy(&self.seen).into_owned()
    }
}

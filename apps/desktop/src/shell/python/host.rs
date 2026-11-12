//! The console's Python process (`python -m kentos.host`, docs/adr/0132):
//! started with pipes, its standard output read line by line as the
//! console's JSON messages and its standard error as text, each on a thread
//! of its own; what the app sends goes through a writer thread, so a Python
//! that stops reading never holds the UI thread.

use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;

use iced::Task;
use iced::futures::channel::mpsc as channel;
use serde_json::Value;

use crate::app::Message;

/// What the process said.
#[derive(Debug, Clone, PartialEq)]
pub enum Said {
    /// One message of its channel (a JSON line of its standard output).
    Message(Value),
    /// A line of its standard output that is not a message: the channel is broken.
    Garbled(String),
    /// Text on its standard error (a traceback before the console started, a C library).
    Stderr(String),
    /// Its standard output closed: it ended.
    Closed,
}

/// A running console process.
pub struct Host {
    child: Child,
    writer: mpsc::Sender<String>,
}

/// The Python the console runs: `KENTOS_PYTHON`, else a checkout's own
/// environment above the program (`.run/py`, which `scripts/python/test.sh`
/// makes; the program is in `target/<profile>/`, a test in its `deps/`),
/// else `python3`.
pub fn interpreter() -> PathBuf {
    if let Some(path) = std::env::var_os("KENTOS_PYTHON") {
        return PathBuf::from(path);
    }
    if let Ok(exe) = std::env::current_exe() {
        for folder in exe.ancestors().skip(1).take(4) {
            let own = folder.join(".run/py/bin/python");
            if own.exists() {
                return own;
            }
        }
    }
    PathBuf::from("python3")
}

impl Host {
    /// Starts `python -m kentos.host`; its messages come back as the task's,
    /// each tagged with `generation` so a stopped process's late ones are known.
    pub fn start(
        python: &Path,
        generation: u64,
        to: impl Fn(u64, Said) -> Message + Send + 'static,
    ) -> Result<(Host, Task<Message>), String> {
        let mut child = Command::new(python)
            .args(["-m", "kentos.host"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .env("PYTHONIOENCODING", "utf-8")
            .spawn()
            .map_err(|e| {
                format!(
                    "Python başlatılamadı ({}): {e}. KENTOS_PYTHON ile kentos paketinin kurulu olduğu Python'u gösterin.",
                    python.display()
                )
            })?;
        let (Some(stdin), Some(stdout), Some(stderr)) =
            (child.stdin.take(), child.stdout.take(), child.stderr.take())
        else {
            let _ = child.kill();
            return Err("Python'un kanalları açılamadı.".to_owned());
        };
        let (said, heard) = channel::unbounded();
        let out = said.clone();
        spawn("kentos-python-oku", move || {
            for line in BufReader::new(stdout).lines() {
                let Ok(line) = line else { break };
                let message = match serde_json::from_str::<Value>(&line) {
                    Ok(v) if v.is_object() => Said::Message(v),
                    _ => Said::Garbled(line),
                };
                if out.unbounded_send(message).is_err() {
                    return;
                }
            }
            let _ = out.unbounded_send(Said::Closed);
        })?;
        spawn("kentos-python-hata", move || {
            let mut reader = BufReader::new(stderr);
            let mut buffer = [0_u8; 4096];
            loop {
                match reader.read(&mut buffer) {
                    Ok(0) | Err(_) => return,
                    Ok(n) => {
                        let text = String::from_utf8_lossy(&buffer[..n]).into_owned();
                        if said.unbounded_send(Said::Stderr(text)).is_err() {
                            return;
                        }
                    }
                }
            }
        })?;
        let (writer, lines) = mpsc::channel::<String>();
        spawn("kentos-python-yaz", move || {
            let mut stdin = stdin;
            for line in lines {
                if stdin
                    .write_all(line.as_bytes())
                    .and_then(|()| stdin.write_all(b"\n"))
                    .and_then(|()| stdin.flush())
                    .is_err()
                {
                    return;
                }
            }
        })?;
        let task = Task::run(heard, move |m| to(generation, m));
        Ok((Host { child, writer }, task))
    }

    /// Sends one message; false when the process no longer reads.
    pub fn send(&self, message: &Value) -> bool {
        self.writer.send(message.to_string()).is_ok()
    }
}

impl Drop for Host {
    /// A console let go of is stopped: Durdur, Yeniden başlat, the app closing.
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn spawn(name: &str, body: impl FnOnce() + Send + 'static) -> Result<(), String> {
    std::thread::Builder::new()
        .name(name.to_owned())
        .spawn(body)
        .map(|_| ())
        .map_err(|e| format!("Python'un kanalı için iş parçacığı açılamadı: {e}"))
}

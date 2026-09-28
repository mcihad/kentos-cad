//! The agents' link (docs/adr/0137): a local socket through which a program
//! of this user (the MCP server, `desktop.attach`) asks the drawing open in
//! the desktop what the console's scripts ask, and writes it with the same
//! commands, each its own undo step, in the command history as an agent's.
//!
//! The user opens it (the Python tab's link button) and closes it; it is a
//! Unix socket in the user's runtime folder (`$XDG_RUNTIME_DIR/kentos-cad`,
//! 0700), itself 0600: only this user's processes reach it. Requests are
//! JSON lines `{"id", "method", "params"}`; answers `{"id", "ok", "result"}`
//! or `{"id", "ok": false, "code", "message"}`.

use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc;
use std::time::Duration;

use iced::Task;
use iced::futures::channel::mpsc as channel;
use serde_json::Value;

use crate::app::Message;

/// What a connection said.
#[derive(Debug, Clone)]
pub enum Heard {
    /// A program connected; its answers go to `answer`.
    Connected(u64, Answerer),
    /// One request.
    Request(u64, Value),
    /// It went away.
    Closed(u64),
    /// The link could not go on (the socket failed).
    Failed(String),
}

/// Where a connection's answers go (its writer thread).
#[derive(Debug, Clone)]
pub struct Answerer(mpsc::Sender<String>);

impl Answerer {
    pub fn send(&self, answer: &Value) {
        let _ = self.0.send(answer.to_string());
    }

    /// A stand-in for tests: its answers come out of the receiver.
    #[cfg(test)]
    pub fn testing() -> (Self, mpsc::Receiver<String>) {
        let (tx, rx) = mpsc::channel();
        (Self(tx), rx)
    }
}

/// The connections open now, to end with the link.
#[cfg(unix)]
type Open = Arc<std::sync::Mutex<Vec<std::os::unix::net::UnixStream>>>;
#[cfg(not(unix))]
type Open = Arc<std::sync::Mutex<Vec<()>>>;

/// An open link: its socket's path, the flag that stops it and its connections.
pub struct Link {
    pub path: PathBuf,
    stop: Arc<AtomicBool>,
    open: Open,
}

impl Drop for Link {
    /// Closing the link ends its connections and removes its socket.
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        #[cfg(unix)]
        if let Ok(streams) = self.open.lock() {
            for s in streams.iter() {
                let _ = s.shutdown(std::net::Shutdown::Both);
            }
        }
        let _ = std::fs::remove_file(&self.path);
    }
}

/// `$XDG_RUNTIME_DIR/kentos-cad/masaustu.sock`, else under the temporary
/// folder in a folder of this user's own.
pub fn default_path() -> PathBuf {
    let base = std::env::var_os("XDG_RUNTIME_DIR")
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
        .map_or_else(
            || std::env::temp_dir().join(format!("kentos-cad-{}", user_id())),
            |d| d.join("kentos-cad"),
        );
    base.join("masaustu.sock")
}

/// This process's user (the owner of `/proc/self`).
#[cfg(unix)]
fn user_id() -> u32 {
    use std::os::unix::fs::MetadataExt;
    std::fs::metadata("/proc/self").map_or(0, |m| m.uid())
}

#[cfg(not(unix))]
fn user_id() -> u32 {
    0
}

#[cfg(unix)]
impl Link {
    /// Opens the link at `path`; its connections' messages come as the task's.
    pub fn open(
        path: &Path,
        to: impl Fn(Heard) -> Message + Send + 'static,
    ) -> Result<(Link, Task<Message>), String> {
        use std::os::unix::fs::{DirBuilderExt, PermissionsExt};
        use std::os::unix::net::{UnixListener, UnixStream};

        let folder = path.parent().unwrap_or(Path::new("."));
        std::fs::DirBuilder::new()
            .recursive(true)
            .mode(0o700)
            .create(folder)
            .map_err(|e| format!("{} açılamadı: {e}", folder.display()))?;
        if path.exists() {
            // Another KentOS listening there keeps it; one that ended left the file behind.
            if UnixStream::connect(path).is_ok() {
                return Err(format!(
                    "{} başka bir KentOS penceresinin bağlantısı: önce onunkini kapatın.",
                    path.display()
                ));
            }
            let _ = std::fs::remove_file(path);
        }
        let listener =
            UnixListener::bind(path).map_err(|e| format!("{} açılamadı: {e}", path.display()))?;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))
            .map_err(|e| format!("{} izinleri ayarlanamadı: {e}", path.display()))?;
        listener
            .set_nonblocking(true)
            .map_err(|e| format!("bağlantı açılamadı: {e}"))?;
        let stop = Arc::new(AtomicBool::new(false));
        let open: Open = Arc::default();
        let (heard, stream) = channel::unbounded::<Heard>();
        let (halt, kept) = (stop.clone(), open.clone());
        std::thread::Builder::new()
            .name("kentos-ajan".into())
            .spawn(move || accept(listener, &halt, &kept, &heard))
            .map_err(|e| format!("bağlantı için iş parçacığı açılamadı: {e}"))?;
        let task = Task::run(stream, to);
        Ok((
            Link {
                path: path.to_path_buf(),
                stop,
                open,
            },
            task,
        ))
    }
}

#[cfg(not(unix))]
impl Link {
    pub fn open(
        _path: &Path,
        _to: impl Fn(Heard) -> Message + Send + 'static,
    ) -> Result<(Link, Task<Message>), String> {
        Err("Ajan bağlantısı bu işletim sisteminde henüz yok.".to_owned())
    }
}

#[cfg(unix)]
fn accept(
    listener: std::os::unix::net::UnixListener,
    stop: &AtomicBool,
    open: &Open,
    heard: &channel::UnboundedSender<Heard>,
) {
    static NEXT: AtomicU64 = AtomicU64::new(1);
    while !stop.load(Ordering::SeqCst) {
        match listener.accept() {
            Ok((stream, _)) => {
                let id = NEXT.fetch_add(1, Ordering::Relaxed);
                let _ = stream.set_nonblocking(false);
                let (Ok(writer), Ok(kept)) = (stream.try_clone(), stream.try_clone()) else {
                    continue;
                };
                if let Ok(mut streams) = open.lock() {
                    streams.push(kept);
                }
                let (answers, lines) = mpsc::channel::<String>();
                if heard
                    .unbounded_send(Heard::Connected(id, Answerer(answers)))
                    .is_err()
                {
                    return;
                }
                let out = heard.clone();
                let _ = std::thread::Builder::new()
                    .name("kentos-ajan-oku".into())
                    .spawn(move || {
                        for line in BufReader::new(stream).lines() {
                            let Ok(line) = line else { break };
                            if line.trim().is_empty() {
                                continue;
                            }
                            let request = serde_json::from_str::<Value>(&line)
                                .unwrap_or_else(|_| serde_json::json!({ "garbled": line }));
                            if out.unbounded_send(Heard::Request(id, request)).is_err() {
                                return;
                            }
                        }
                        let _ = out.unbounded_send(Heard::Closed(id));
                    });
                let _ = std::thread::Builder::new()
                    .name("kentos-ajan-yaz".into())
                    .spawn(move || {
                        let mut writer = writer;
                        for line in lines {
                            if writer
                                .write_all(line.as_bytes())
                                .and_then(|()| writer.write_all(b"\n"))
                                .and_then(|()| writer.flush())
                                .is_err()
                            {
                                return;
                            }
                        }
                    });
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                std::thread::sleep(Duration::from_millis(50));
            }
            Err(e) => {
                let _ = heard.unbounded_send(Heard::Failed(format!("bağlantı kesildi: {e}")));
                return;
            }
        }
    }
}

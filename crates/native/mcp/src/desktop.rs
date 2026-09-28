//! The drawing open in the KentOS desktop (docs/adr/0137): `desktop.attach`
//! connects to the desktop's agents' link (a Unix socket the user opens in
//! the Python tab) and gives a handle every drawing tool takes; their
//! requests go to the desktop, which answers on the drawing on the screen.

use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
#[cfg(unix)]
use std::time::Duration;

use serde_json::{Value, json};

/// The link's default place, as the desktop makes it: `KENTOS_DESKTOP`, else
/// `$XDG_RUNTIME_DIR/kentos-cad/masaustu.sock`, else under the temporary
/// folder in this user's own folder.
pub fn default_path() -> PathBuf {
    if let Some(path) = std::env::var_os("KENTOS_DESKTOP").filter(|v| !v.is_empty()) {
        return PathBuf::from(path);
    }
    let base = std::env::var_os("XDG_RUNTIME_DIR")
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
        .map_or_else(
            || std::env::temp_dir().join(format!("kentos-cad-{}", user_id())),
            |d| d.join("kentos-cad"),
        );
    base.join("masaustu.sock")
}

#[cfg(unix)]
fn user_id() -> u32 {
    use std::os::unix::fs::MetadataExt;
    std::fs::metadata("/proc/self").map_or(0, |m| m.uid())
}

#[cfg(not(unix))]
fn user_id() -> u32 {
    0
}

/// A connection to the desktop.
pub struct Desktop {
    #[cfg(unix)]
    reader: BufReader<std::os::unix::net::UnixStream>,
    #[cfg(unix)]
    writer: std::os::unix::net::UnixStream,
    next: u64,
    pub path: PathBuf,
}

impl Desktop {
    /// Connects to the link at `path`.
    #[cfg(unix)]
    pub fn connect(path: PathBuf) -> Result<Self, (String, String)> {
        let stream = std::os::unix::net::UnixStream::connect(&path).map_err(|e| {
            (
                "no_desktop".to_owned(),
                format!(
                    "{} açılamadı ({e}): KentOS masaüstünde Python sekmesinden “Ajanlara aç” ile bağlantıyı açın.",
                    path.display()
                ),
            )
        })?;
        // A desktop busy past this answers late: the call says so instead of hanging.
        let _ = stream.set_read_timeout(Some(Duration::from_secs(60)));
        let writer = stream
            .try_clone()
            .map_err(|e| ("no_desktop".to_owned(), format!("bağlantı kurulamadı: {e}")))?;
        Ok(Self {
            reader: BufReader::new(stream),
            writer,
            next: 0,
            path,
        })
    }

    #[cfg(not(unix))]
    pub fn connect(_path: PathBuf) -> Result<Self, (String, String)> {
        Err((
            "no_desktop".to_owned(),
            "Masaüstü bağlantısı bu işletim sisteminde henüz yok.".to_owned(),
        ))
    }

    /// Asks the desktop; its answer, or its refusal's code and message.
    #[cfg(unix)]
    pub fn ask(&mut self, method: &str, params: Value) -> Result<Value, (String, String)> {
        self.next += 1;
        let id = self.next;
        let gone = |e: std::io::Error| {
            (
                "desktop_gone".to_owned(),
                format!("Masaüstü bağlantısı kesildi ({e}); desktop.attach ile yeniden bağlanın."),
            )
        };
        let line = json!({ "id": id, "method": method, "params": params }).to_string();
        self.writer
            .write_all(line.as_bytes())
            .and_then(|()| self.writer.write_all(b"\n"))
            .and_then(|()| self.writer.flush())
            .map_err(gone)?;
        loop {
            let mut answer = String::new();
            let read = self.reader.read_line(&mut answer).map_err(gone)?;
            if read == 0 {
                return Err(gone(std::io::Error::from(
                    std::io::ErrorKind::UnexpectedEof,
                )));
            }
            let Ok(answer) = serde_json::from_str::<Value>(&answer) else {
                continue;
            };
            if answer.get("id").and_then(Value::as_u64) != Some(id) {
                continue;
            }
            return if answer.get("ok") == Some(&json!(true)) {
                Ok(answer.get("result").cloned().unwrap_or(Value::Null))
            } else {
                Err((
                    answer
                        .get("code")
                        .and_then(Value::as_str)
                        .unwrap_or("desktop")
                        .to_owned(),
                    answer
                        .get("message")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .to_owned(),
                ))
            };
        }
    }

    #[cfg(not(unix))]
    pub fn ask(&mut self, _method: &str, _params: Value) -> Result<Value, (String, String)> {
        Err((
            "no_desktop".to_owned(),
            "Masaüstü bağlantısı yok.".to_owned(),
        ))
    }
}

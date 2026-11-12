//! A stand-in web server for the tests of addresses read by ranges
//! (docs/adr/0207 §1): files in memory, a `Range` answered with `206
//! Partial Content`, its `Content-Range` and an `ETag`; asked to, it answers
//! `200` with the whole file (a server without ranges) or changes its
//! version after some requests. Several requests on one connection, as
//! HTTP/1.1 keeps it. Test code only.

use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, PoisonError};

/// How the server answers.
#[derive(Clone, Copy, Debug)]
pub(crate) enum Mode {
    /// Ranges, version “v1”.
    Ranges,
    /// The whole file with `200`, whatever is asked.
    Whole,
    /// Ranges; from this request on, version “v2”.
    VersionFrom(usize),
}

/// A file the server gives: bytes in memory, or a file on disk read a range at a time (a large one).
#[derive(Clone)]
enum Body {
    Bytes(Arc<Vec<u8>>),
    File(std::path::PathBuf),
}

impl Body {
    fn len(&self) -> u64 {
        match self {
            Body::Bytes(b) => b.len() as u64,
            Body::File(p) => std::fs::metadata(p).map_or(0, |m| m.len()),
        }
    }

    /// Bytes `from` to `to`, inclusive.
    fn slice(&self, from: u64, to: u64) -> Vec<u8> {
        match self {
            Body::Bytes(b) => b[from as usize..=to as usize].to_vec(),
            Body::File(p) => {
                use std::io::{Read, Seek, SeekFrom};
                let mut out = vec![0u8; (to - from + 1) as usize];
                if let Ok(mut f) = std::fs::File::open(p)
                    && f.seek(SeekFrom::Start(from)).is_ok()
                {
                    let _ = f.read_exact(&mut out);
                }
                out
            }
        }
    }
}

/// A server running: its address and what it was asked.
pub(crate) struct Server {
    files: Arc<Mutex<HashMap<String, Body>>>,
    /// The files' version: “v1”, one more each `replace`.
    version: Arc<AtomicUsize>,
    /// `http://127.0.0.1:<port>`; a file is at `<base>/<name>`.
    pub base: String,
    pub requests: Arc<AtomicUsize>,
    /// The ranges answered, `[from, to]` inclusive.
    pub ranges: Arc<Mutex<Vec<(u64, u64)>>>,
}

impl Server {
    pub(crate) fn url(&self, name: &str) -> String {
        format!("{}/{name}", self.base)
    }

    /// A file's bytes replaced: the server's version moves on (its `ETag` changes).
    pub(crate) fn replace(&self, name: &str, bytes: Vec<u8>) {
        self.files
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .insert(name.to_owned(), Body::Bytes(Arc::new(bytes)));
        self.version.fetch_add(1, Ordering::SeqCst);
    }

    pub(crate) fn asked(&self) -> Vec<(u64, u64)> {
        self.ranges
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }
}

/// Starts a server of `files` (name → bytes) on a free port of this computer.
pub(crate) fn serve(files: Vec<(&str, Vec<u8>)>, mode: Mode) -> Server {
    start(
        files
            .into_iter()
            .map(|(n, b)| (n.to_owned(), Body::Bytes(Arc::new(b))))
            .collect(),
        mode,
    )
}

/// Starts a server of files on disk (name → path), read a range at a time, by ranges.
pub(crate) fn serve_paths(files: Vec<(&str, std::path::PathBuf)>) -> Server {
    start(
        files
            .into_iter()
            .map(|(n, p)| (n.to_owned(), Body::File(p)))
            .collect(),
        Mode::Ranges,
    )
}

fn start(files: HashMap<String, Body>, mode: Mode) -> Server {
    let files: Arc<Mutex<HashMap<String, Body>>> = Arc::new(Mutex::new(files));
    let version = Arc::new(AtomicUsize::new(1));
    let (served, current) = (files.clone(), version.clone());
    let listener = TcpListener::bind("127.0.0.1:0").expect("a free port");
    let base = format!("http://{}", listener.local_addr().expect("its address"));
    let requests = Arc::new(AtomicUsize::new(0));
    let ranges = Arc::new(Mutex::new(Vec::new()));
    let (count, seen) = (requests.clone(), ranges.clone());
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else {
                continue;
            };
            let (files, count, seen, current) =
                (served.clone(), count.clone(), seen.clone(), current.clone());
            std::thread::spawn(move || {
                let Ok(read_half) = stream.try_clone() else {
                    return;
                };
                let mut reader = BufReader::new(read_half);
                loop {
                    let mut line = String::new();
                    if reader.read_line(&mut line).unwrap_or(0) == 0 {
                        return;
                    }
                    let path = line.split_whitespace().nth(1).unwrap_or("/").to_owned();
                    let mut range = None;
                    loop {
                        let mut h = String::new();
                        if reader.read_line(&mut h).unwrap_or(0) == 0 {
                            return;
                        }
                        let h = h.trim_end().to_ascii_lowercase();
                        if h.is_empty() {
                            break;
                        }
                        if let Some(v) = h.strip_prefix("range: bytes=")
                            && let Some((a, b)) = v.split_once('-')
                        {
                            range = a.parse::<u64>().ok().zip(b.parse::<u64>().ok());
                        }
                    }
                    let n = count.fetch_add(1, Ordering::SeqCst) + 1;
                    let body = files
                        .lock()
                        .unwrap_or_else(PoisonError::into_inner)
                        .get(
                            path.split('?')
                                .next()
                                .unwrap_or_default()
                                .trim_start_matches('/'),
                        )
                        .cloned();
                    let Some(body) = body else {
                        let _ = stream
                            .write_all(b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\n\r\n");
                        continue;
                    };
                    let version = match mode {
                        Mode::VersionFrom(k) if n >= k => "\"v2\"".to_owned(),
                        _ => format!("\"v{}\"", current.load(Ordering::SeqCst)),
                    };
                    let answer = match (mode, range) {
                        (Mode::Whole, _) | (_, None) => {
                            let len = body.len();
                            let mut a = format!(
                                "HTTP/1.1 200 OK\r\nContent-Length: {len}\r\nETag: {version}\r\n\r\n"
                            )
                            .into_bytes();
                            if len > 0 {
                                a.extend_from_slice(&body.slice(0, len - 1));
                            }
                            a
                        }
                        (_, Some((from, to))) => {
                            let last = body.len().saturating_sub(1);
                            let to = to.min(last);
                            seen.lock()
                                .unwrap_or_else(PoisonError::into_inner)
                                .push((from, to));
                            let part = body.slice(from, to);
                            let mut a = format!(
                                "HTTP/1.1 206 Partial Content\r\nContent-Range: bytes {from}-{to}/{}\r\nContent-Length: {}\r\nETag: {version}\r\n\r\n",
                                body.len(),
                                part.len()
                            )
                            .into_bytes();
                            a.extend_from_slice(&part);
                            a
                        }
                    };
                    if stream.write_all(&answer).is_err() {
                        return;
                    }
                }
            });
        }
    });
    Server {
        files,
        version,
        base,
        requests,
        ranges,
    }
}

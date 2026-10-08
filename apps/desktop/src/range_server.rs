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

/// A server running: its address and what it was asked.
pub(crate) struct Server {
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

    pub(crate) fn asked(&self) -> Vec<(u64, u64)> {
        self.ranges
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }
}

/// Starts a server of `files` (name → bytes) on a free port of this computer.
pub(crate) fn serve(files: Vec<(&str, Vec<u8>)>, mode: Mode) -> Server {
    let files: Arc<HashMap<String, Vec<u8>>> =
        Arc::new(files.into_iter().map(|(n, b)| (n.to_owned(), b)).collect());
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
            let (files, count, seen) = (files.clone(), count.clone(), seen.clone());
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
                    let Some(body) = files.get(path.trim_start_matches('/')) else {
                        let _ = stream
                            .write_all(b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\n\r\n");
                        continue;
                    };
                    let version = match mode {
                        Mode::VersionFrom(k) if n >= k => "\"v2\"",
                        _ => "\"v1\"",
                    };
                    let answer = match (mode, range) {
                        (Mode::Whole, _) | (_, None) => {
                            let mut a = format!(
                                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nETag: {version}\r\n\r\n",
                                body.len()
                            )
                            .into_bytes();
                            a.extend_from_slice(body);
                            a
                        }
                        (_, Some((from, to))) => {
                            let last = (body.len() as u64).saturating_sub(1);
                            let to = to.min(last);
                            seen.lock()
                                .unwrap_or_else(PoisonError::into_inner)
                                .push((from, to));
                            let part = &body[from as usize..=to as usize];
                            let mut a = format!(
                                "HTTP/1.1 206 Partial Content\r\nContent-Range: bytes {from}-{to}/{}\r\nContent-Length: {}\r\nETag: {version}\r\n\r\n",
                                body.len(),
                                part.len()
                            )
                            .into_bytes();
                            a.extend_from_slice(part);
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
        base,
        requests,
        ranges,
    }
}

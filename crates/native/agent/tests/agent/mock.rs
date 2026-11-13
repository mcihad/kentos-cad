//! A provider on this computer for the tests: a listener on 127.0.0.1:0 that answers each request in turn with a
//! scripted reply (a whole body, a stream sent piece by piece, a stream held open until the client goes) and records
//! what it was sent. Nothing leaves the computer.

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use serde_json::Value;

/// How a reply's body is sent.
pub enum Body {
    /// Whole, with its length.
    Whole(String),
    /// Piece by piece with a short pause between, then the connection closes.
    Stream(Vec<String>),
    /// The pieces, then the connection is held open until the client leaves (or ten seconds pass).
    Hang(Vec<String>),
}

pub struct Reply {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: Body,
}

impl Reply {
    /// A server-sent event stream.
    pub fn stream(pieces: Vec<String>) -> Reply {
        Reply {
            status: 200,
            headers: vec![("content-type".into(), "text/event-stream".into())],
            body: Body::Stream(pieces),
        }
    }

    /// A stream whose server stops sending after `pieces` but keeps the connection open.
    pub fn hang(pieces: Vec<String>) -> Reply {
        Reply {
            status: 200,
            headers: vec![("content-type".into(), "text/event-stream".into())],
            body: Body::Hang(pieces),
        }
    }

    /// A JSON answer with `status` and extra headers.
    pub fn json(status: u16, headers: &[(&str, &str)], body: Value) -> Reply {
        let mut h: Vec<(String, String)> = vec![("content-type".into(), "application/json".into())];
        h.extend(
            headers
                .iter()
                .map(|(k, v)| ((*k).to_owned(), (*v).to_owned())),
        );
        Reply {
            status,
            headers: h,
            body: Body::Whole(body.to_string()),
        }
    }
}

/// A request as the server got it.
#[derive(Clone, Debug)]
pub struct Recorded {
    pub method: String,
    /// The path with its query.
    pub path: String,
    pub headers: Vec<(String, String)>,
    pub body: String,
}

impl Recorded {
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(name))
            .map(|(_, v)| v.as_str())
    }

    pub fn json(&self) -> Value {
        serde_json::from_str(&self.body)
            .unwrap_or_else(|e| panic!("the body is not JSON ({e}): {}", self.body))
    }
}

pub struct Mock {
    /// `http://127.0.0.1:<port>`.
    pub base: String,
    requests: Arc<Mutex<Vec<Recorded>>>,
}

impl Mock {
    /// Answers the requests in turn with `replies`; any request after them gets a 500.
    pub fn start(replies: Vec<Reply>) -> Mock {
        let listener = TcpListener::bind("127.0.0.1:0").expect("a port on this computer");
        let base = format!("http://{}", listener.local_addr().expect("its address"));
        let requests = Arc::new(Mutex::new(Vec::new()));
        let seen = Arc::clone(&requests);
        std::thread::spawn(move || {
            let mut replies = replies.into_iter();
            for stream in listener.incoming() {
                let Ok(mut stream) = stream else { continue };
                let Some(request) = read_request(&mut stream) else {
                    continue;
                };
                seen.lock().unwrap().push(request);
                let reply = replies.next().unwrap_or_else(|| {
                    Reply::json(500, &[], serde_json::json!({"error": "beklenmeyen istek"}))
                });
                write_reply(&mut stream, reply);
            }
        });
        Mock { base, requests }
    }

    pub fn requests(&self) -> Vec<Recorded> {
        self.requests.lock().unwrap().clone()
    }
}

fn read_request(stream: &mut TcpStream) -> Option<Recorded> {
    stream
        .set_read_timeout(Some(Duration::from_secs(10)))
        .ok()?;
    let mut bytes = Vec::new();
    let mut buf = [0u8; 8192];
    let head_end = loop {
        if let Some(at) = bytes.windows(4).position(|w| w == b"\r\n\r\n") {
            break at + 4;
        }
        let n = stream.read(&mut buf).ok()?;
        if n == 0 {
            return None;
        }
        bytes.extend_from_slice(&buf[..n]);
    };
    let head = String::from_utf8_lossy(&bytes[..head_end]).to_string();
    let mut lines = head.split("\r\n");
    let mut first = lines.next()?.split(' ');
    let method = first.next()?.to_owned();
    let path = first.next()?.to_owned();
    let headers: Vec<(String, String)> = lines
        .filter_map(|l| l.split_once(':'))
        .map(|(k, v)| (k.trim().to_ascii_lowercase(), v.trim().to_owned()))
        .collect();
    let length: usize = headers
        .iter()
        .find(|(k, _)| k == "content-length")
        .and_then(|(_, v)| v.parse().ok())
        .unwrap_or(0);
    let mut body = bytes[head_end..].to_vec();
    while body.len() < length {
        let n = stream.read(&mut buf).ok()?;
        if n == 0 {
            break;
        }
        body.extend_from_slice(&buf[..n]);
    }
    Some(Recorded {
        method,
        path,
        headers,
        body: String::from_utf8_lossy(&body).to_string(),
    })
}

fn write_reply(stream: &mut TcpStream, reply: Reply) {
    let reason = match reply.status {
        200 => "OK",
        400 => "Bad Request",
        401 => "Unauthorized",
        429 => "Too Many Requests",
        _ => "Status",
    };
    let mut head = format!(
        "HTTP/1.1 {} {reason}\r\nconnection: close\r\n",
        reply.status
    );
    for (k, v) in &reply.headers {
        head.push_str(&format!("{k}: {v}\r\n"));
    }
    match reply.body {
        Body::Whole(text) => {
            head.push_str(&format!("content-length: {}\r\n\r\n", text.len()));
            let _ = stream.write_all(head.as_bytes());
            let _ = stream.write_all(text.as_bytes());
        }
        Body::Stream(pieces) => {
            head.push_str("\r\n");
            let _ = stream.write_all(head.as_bytes());
            for p in pieces {
                if stream
                    .write_all(p.as_bytes())
                    .and_then(|()| stream.flush())
                    .is_err()
                {
                    return;
                }
                std::thread::sleep(Duration::from_millis(3));
            }
        }
        Body::Hang(pieces) => {
            head.push_str("\r\n");
            let _ = stream.write_all(head.as_bytes());
            for p in pieces {
                let _ = stream.write_all(p.as_bytes()).and_then(|()| stream.flush());
            }
            // Held open until the client closes its side.
            let _ = stream.set_read_timeout(Some(Duration::from_millis(50)));
            let until = Instant::now() + Duration::from_secs(10);
            let mut buf = [0u8; 64];
            while Instant::now() < until {
                match stream.read(&mut buf) {
                    Ok(0) => break,
                    Ok(_) => {}
                    Err(e)
                        if matches!(
                            e.kind(),
                            std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                        ) => {}
                    Err(_) => break,
                }
            }
        }
    }
    let _ = stream.shutdown(std::net::Shutdown::Both);
}

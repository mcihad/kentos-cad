//! The server jobs' tools (docs/adr/0561 §3) against a stand-in server on this computer, through the real process on stdio
//! with the account in its environment (the model never gives it): `jobs.start` stages an import's local file and asks
//! for the job, `jobs.status` waits for it, `jobs.cancel` stops it, `jobs.result` takes its file to a path and tells its
//! size and hash. The stand-in records what it was sent; a process of its own, so the environment is its own.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};

use kentos_mcp::MODERN;
use serde_json::{Value, json};

const TENANT: &str = "01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f";
const PROJECT: &str = "01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e60";
const JOB: &str = "01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e61";

#[derive(Clone, Debug)]
struct Seen {
    method: String,
    path: String,
    headers: String,
    body: Vec<u8>,
}

/// What the stand-in answers: (status, content type, body) by method and path.
type Route = Box<dyn Fn(&Seen) -> (u16, &'static str, Vec<u8>) + Send>;

fn read_request(stream: &mut TcpStream) -> Option<Seen> {
    let mut head = Vec::new();
    let mut byte = [0u8; 1];
    while !head.ends_with(b"\r\n\r\n") {
        if stream.read(&mut byte).ok()? == 0 {
            return None;
        }
        head.push(byte[0]);
    }
    let head = String::from_utf8_lossy(&head).to_string();
    let mut first = head.lines().next()?.split(' ');
    let (method, path) = (first.next()?.to_owned(), first.next()?.to_owned());
    let length: usize = head
        .lines()
        .find_map(|l| {
            l.to_lowercase()
                .strip_prefix("content-length:")
                .map(|v| v.trim().parse().unwrap_or(0))
        })
        .unwrap_or(0);
    let mut body = vec![0u8; length];
    stream.read_exact(&mut body).ok()?;
    Some(Seen {
        method,
        path,
        headers: head.to_lowercase(),
        body,
    })
}

fn stand_in(route: Route) -> (String, Arc<Mutex<Vec<Seen>>>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("a port");
    let url = format!("http://{}", listener.local_addr().unwrap());
    let seen = Arc::new(Mutex::new(Vec::new()));
    let log = seen.clone();
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { continue };
            let Some(request) = read_request(&mut stream) else {
                continue;
            };
            let (status, kind, body) = if request.path == "/v1/auth/login" {
                (200, "application/json", br#"{"user":{"id":"u1","displayName":"Ayse","method":"local"},"memberships":[]}"#.to_vec())
            } else {
                route(&request)
            };
            log.lock().unwrap().push(request);
            let cookie = "set-cookie: kentos_session=tok-1; Path=/; HttpOnly\r\n";
            let etag = format!("etag: \"{}\"\r\n", kentos_cloud::api::hex_sha256(&body));
            let head = format!(
                "HTTP/1.1 {status} X\r\ncontent-type: {kind}\r\ncontent-length: {}\r\n{}{}connection: close\r\n\r\n",
                body.len(),
                if kind == "application/json" {
                    cookie
                } else {
                    ""
                },
                if kind == "application/octet-stream" {
                    etag.as_str()
                } else {
                    ""
                }
            );
            let _ = stream.write_all(head.as_bytes());
            let _ = stream.write_all(&body);
        }
    });
    (url, seen)
}

fn job(state: &str, extra: Value) -> Vec<u8> {
    let mut base = json!({
        "id": JOB, "project": PROJECT, "kind": "project.import", "input": {}, "inputSha256": "0".repeat(64),
        "baseRevision": "1", "state": state, "progress": 0.5, "attempts": 1, "maxAttempts": 3,
        "timeoutSeconds": 600, "cancelRequested": false, "createdBy": "u1", "createdAt": "2026-10-20T10:00:00Z"
    });
    for (k, v) in extra.as_object().unwrap() {
        base[k] = v.clone();
    }
    serde_json::to_vec(&base).unwrap()
}

struct Mcp {
    child: std::process::Child,
    stdin: std::process::ChildStdin,
    lines: std::io::Lines<BufReader<std::process::ChildStdout>>,
    next: u64,
}

impl Mcp {
    fn start(url: &str) -> Mcp {
        let mut child = Command::new(env!("CARGO_BIN_EXE_kentos-mcp"))
            .env("KENTOS_URL", url)
            .env("KENTOS_LOGIN", "ayse")
            .env("KENTOS_PASSWORD", "parola")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .expect("the server starts");
        let stdin = child.stdin.take().unwrap();
        let lines = BufReader::new(child.stdout.take().unwrap()).lines();
        Mcp {
            child,
            stdin,
            lines,
            next: 1,
        }
    }

    fn call(&mut self, name: &str, arguments: Value) -> Value {
        self.next += 1;
        let meta = json!({
            "io.modelcontextprotocol/protocolVersion": MODERN,
            "io.modelcontextprotocol/clientInfo": { "name": "test", "version": "1" },
            "io.modelcontextprotocol/clientCapabilities": {}
        });
        let message = json!({ "jsonrpc": "2.0", "id": self.next, "method": "tools/call",
            "params": { "_meta": meta, "name": name, "arguments": arguments } });
        writeln!(self.stdin, "{message}").unwrap();
        self.stdin.flush().unwrap();
        let answer: Value =
            serde_json::from_str(&self.lines.next().expect("a line").expect("read")).unwrap();
        assert_eq!(answer["id"], self.next, "{answer}");
        answer["result"].clone()
    }
}

impl Drop for Mcp {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

#[test]
fn an_import_is_staged_asked_for_waited_on_and_its_result_taken() {
    let scratch = std::env::temp_dir().join(format!("kentos-mcp-jobs-{}", std::process::id()));
    std::fs::create_dir_all(&scratch).unwrap();
    let file = scratch.join("parsel.gpkg");
    std::fs::write(&file, b"GPKG-bytes").unwrap();
    let base = format!("/v1/tenants/{TENANT}/projects/{PROJECT}");
    let polls = Arc::new(Mutex::new(0u32));
    let counted = polls.clone();
    let route: Route = Box::new(move |r| {
        let json = "application/json";
        match (r.method.as_str(), r.path.as_str()) {
            ("POST", p) if p == format!("{base}/job-inputs") => (
                201,
                json,
                br#"{"id":"01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e70","name":"parsel.gpkg","size":10,"sha256":"x","createdAt":"t","received":false}"#.to_vec(),
            ),
            ("PUT", p) if p == format!("{base}/job-inputs/01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e70") => (
                200,
                json,
                br#"{"id":"01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e70","name":"parsel.gpkg","size":10,"sha256":"x","createdAt":"t","received":true}"#.to_vec(),
            ),
            ("POST", p) if p == format!("{base}/jobs") => (201, json, job("queued", json!({ "progress": 0.0 }))),
            ("GET", p) if p == format!("{base}/jobs/{JOB}") => {
                let mut n = counted.lock().unwrap();
                *n += 1;
                if *n < 3 {
                    (200, json, job("running", json!({ "progressText": "Dosya okunuyor" })))
                } else {
                    (
                        200,
                        json,
                        job(
                            "succeeded",
                            json!({ "progress": 1.0, "result": { "command": "file.import.gpkg", "revision": "2" },
                                "artifact": { "size": "10", "sha256": "ab", "name": "Ada.pdf" } }),
                        ),
                    )
                }
            }
            ("POST", p) if p == format!("{base}/jobs/{JOB}/cancel") => (200, json, job("cancelled", json!({}))),
            ("GET", p) if p == format!("{base}/jobs/{JOB}/artifact") => {
                (200, "application/octet-stream", b"%PDF-1.7 atlas".to_vec())
            }
            _ => (404, json, br#"{"error":"not_found","message":"Yok."}"#.to_vec()),
        }
    });
    let (url, seen) = stand_in(route);
    let mut mcp = Mcp::start(&url);
    // Staged, then asked for: the file's bytes went in one PUT, the job names the staged file and the file's name.
    let started = mcp.call(
        "jobs.start",
        json!({ "tenant": TENANT, "project": PROJECT, "kind": "import", "file": file.display().to_string(),
                "input": { "options": { "reproject": true } }, "requestId": "mcp-1" }),
    );
    assert_eq!(started["isError"], false, "{started}");
    assert_eq!(started["structuredContent"]["state"], "queued");
    {
        let requests = seen.lock().unwrap();
        let put = requests
            .iter()
            .find(|r| r.method == "PUT")
            .expect("the bytes were sent");
        assert_eq!(put.body, b"GPKG-bytes");
        assert!(
            put.headers.contains("x-kentos-client: mcp"),
            "{}",
            put.headers
        );
        let begin = requests
            .iter()
            .find(|r| r.path.ends_with("/job-inputs"))
            .unwrap();
        let begun: Value = serde_json::from_slice(&begin.body).unwrap();
        assert_eq!(
            (begun["name"].as_str(), begun["size"].as_u64()),
            (Some("parsel.gpkg"), Some(10))
        );
        assert_eq!(begun["sha256"].as_str().map(str::len), Some(64));
        let ask = requests.iter().find(|r| r.path.ends_with("/jobs")).unwrap();
        let asked: Value = serde_json::from_slice(&ask.body).unwrap();
        assert_eq!(asked["kind"], "project.import");
        assert_eq!(asked["requestId"], "mcp-1");
        assert_eq!(
            asked["input"]["upload"],
            "01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e70"
        );
        assert_eq!(asked["input"]["label"], "parsel.gpkg");
        assert_eq!(asked["input"]["options"], json!({ "reproject": true }));
    }
    // Not over yet: the result is not asked of an unfinished job. Then waited on until it is.
    let early = mcp.call(
        "jobs.result",
        json!({ "tenant": TENANT, "project": PROJECT, "job": JOB }),
    );
    assert_eq!(early["isError"], true);
    assert_eq!(
        early["structuredContent"]["error"]["code"],
        "job_not_finished"
    );
    let status = mcp.call(
        "jobs.status",
        json!({ "tenant": TENANT, "project": PROJECT, "job": JOB, "waitSeconds": 10 }),
    );
    assert_eq!(status["isError"], false, "{status}");
    assert_eq!(status["structuredContent"]["state"], "succeeded");
    // The result and its file written where asked, never over a file that is there.
    let target = scratch.join("atlas.pdf");
    let args = json!({ "tenant": TENANT, "project": PROJECT, "job": JOB, "savePath": target.display().to_string() });
    let result = mcp.call("jobs.result", args.clone());
    assert_eq!(result["isError"], false, "{result}");
    assert_eq!(result["structuredContent"]["result"]["revision"], "2");
    assert_eq!(result["structuredContent"]["saved"]["bytes"], 14);
    assert_eq!(std::fs::read(&target).unwrap(), b"%PDF-1.7 atlas");
    let again = mcp.call("jobs.result", args.clone());
    assert_eq!(again["structuredContent"]["error"]["code"], "file_exists");
    let mut over = args;
    over["overwrite"] = json!(true);
    assert_eq!(mcp.call("jobs.result", over)["isError"], false);
    let relative = mcp.call(
        "jobs.result",
        json!({ "tenant": TENANT, "project": PROJECT, "job": JOB, "savePath": "atlas.pdf" }),
    );
    assert_eq!(
        relative["structuredContent"]["error"]["code"],
        "invalid_input"
    );
    // Stopping, and the arguments a model gets wrong.
    let stopped = mcp.call(
        "jobs.cancel",
        json!({ "tenant": TENANT, "project": PROJECT, "job": JOB }),
    );
    assert_eq!(stopped["structuredContent"]["state"], "cancelled");
    for (args, code) in [
        (
            json!({ "tenant": TENANT, "project": PROJECT, "kind": "yok" }),
            "invalid_input",
        ),
        (
            json!({ "tenant": TENANT, "project": PROJECT, "kind": "export", "file": "/x" }),
            "invalid_input",
        ),
        (
            json!({ "tenant": TENANT, "project": PROJECT, "kind": "import", "file": "/yok/dosya.gpkg" }),
            "invalid_input",
        ),
        (
            json!({ "tenant": TENANT, "project": PROJECT, "kind": "export", "input": 3 }),
            "invalid_input",
        ),
        (
            json!({ "tenant": "x", "project": PROJECT, "kind": "export" }),
            "invalid_input",
        ),
    ] {
        let refused = mcp.call("jobs.start", args.clone());
        assert_eq!(refused["isError"], true, "{args}");
        assert_eq!(
            refused["structuredContent"]["error"]["code"], code,
            "{args}"
        );
    }
    // The full name works as the short one.
    let export = mcp.call(
        "jobs.start",
        json!({ "tenant": TENANT, "project": PROJECT, "kind": "project.export", "input": {} }),
    );
    assert_eq!(export["isError"], false, "{export}");
    std::fs::remove_dir_all(&scratch).ok();
}

#[test]
fn a_failed_job_is_an_error_the_model_reads_with_its_code() {
    let route: Route = Box::new(|r| {
        if r.method == "GET" && r.path.ends_with(&format!("/jobs/{JOB}")) {
            return (
                200,
                "application/json",
                job(
                    "failed",
                    json!({ "error": { "code": "pin_mismatch", "message": "Çizim sabitlenen durumda değil." } }),
                ),
            );
        }
        (
            404,
            "application/json",
            br#"{"error":"not_found","message":"Yok."}"#.to_vec(),
        )
    });
    let (url, _) = stand_in(route);
    let mut mcp = Mcp::start(&url);
    let status = mcp.call(
        "jobs.status",
        json!({ "tenant": TENANT, "project": PROJECT, "job": JOB }),
    );
    assert_eq!(status["isError"], true);
    assert_eq!(status["structuredContent"]["error"]["code"], "pin_mismatch");
    let result = mcp.call(
        "jobs.result",
        json!({ "tenant": TENANT, "project": PROJECT, "job": JOB }),
    );
    assert_eq!(result["structuredContent"]["state"], "failed");
    assert_eq!(result["structuredContent"]["error"]["code"], "pin_mismatch");
    // A job the server does not know is the server's refusal, with its status.
    let missing = mcp.call(
        "jobs.status",
        json!({ "tenant": TENANT, "project": PROJECT, "job": "01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4eff" }),
    );
    assert_eq!(missing["isError"], true);
    assert_eq!(missing["structuredContent"]["error"]["status"], 404);
}

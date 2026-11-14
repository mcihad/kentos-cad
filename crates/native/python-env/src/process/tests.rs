//! The pool against a stand-in for `kentos.ext_host`: a small script speaking its lines (hello, invoke, cancel,
//! reply, quit), run by the system's Python 3. Without one the tests say so and pass.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use serde_json::json;

use super::{Ended, Heard, Pool, Start};

const STAND_IN: &str = r#"
import json, os, sys, threading, time
out = sys.stdout
def send(m):
    out.write(json.dumps(m) + "\n"); out.flush()
stops = {}
replies = {}
work = []
lock = threading.Condition()
def read():
    for line in sys.stdin:
        m = json.loads(line)
        with lock:
            if m["type"] == "cancel":
                stops[m["id"]] = True
            elif m["type"] == "reply":
                replies[m["id"]] = m
            else:
                work.append(m)
            lock.notify_all()
    with lock:
        work.append(None); lock.notify_all()
threading.Thread(target=read, daemon=True).start()
def next_work():
    with lock:
        while not work:
            lock.wait()
        return work.pop(0)
hello = next_work()
if hello.get("extension") == "refuses":
    send({"type": "refused", "message": "bu süreç reddeder"}); sys.exit(1)
send({"type": "ready", "protocol": 1, "python": "3", "kentos": "0"})
while True:
    m = next_work()
    if m is None or m["type"] == "quit":
        os._exit(0)
    i, local = m["id"], m["local"]
    if local == "echo":
        send({"type": "progress", "id": i, "done": 1, "total": 2, "text": "yarı"})
        send({"type": "log", "id": i, "level": "info", "text": "pid %d" % os.getpid()})
        send({"type": "done", "id": i, "ok": True, "result": {"kind": "outputs", "outputs": {"n": m["params"]["n"] * 2}, "pid": os.getpid()}})
    elif local == "slow":
        while not stops.get(i):
            time.sleep(0.005)
        send({"type": "done", "id": i, "ok": False, "stopped": True})
    elif local == "stubborn":
        while True:
            time.sleep(1)
    elif local == "crash":
        sys.stderr.write("Traceback: bir şey kırıldı\n"); sys.stderr.flush()
        os._exit(3)
    elif local == "asks":
        send({"type": "call", "id": 77, "method": "entities", "params": {}})
        with lock:
            while 77 not in replies:
                lock.wait()
        r = replies[77]
        send({"type": "done", "id": i, "ok": True, "result": {"kind": "outputs", "refused": r["ok"] is False, "code": r.get("code")}})
"#;

/// The stand-in written to a folder of its own, and the system's Python 3; none without one.
fn stand_in() -> Option<(tempdir::Dir, std::path::PathBuf)> {
    let python = ["python3", "python"].into_iter().find_map(|name| {
        let ok = std::process::Command::new(name)
            .arg("-c")
            .arg("import sys; sys.exit(0 if sys.version_info >= (3, 8) else 1)")
            .status()
            .is_ok_and(|s| s.success());
        ok.then(|| std::path::PathBuf::from(name))
    })?;
    let dir = tempdir::Dir::new();
    std::fs::write(dir.path().join("host.py"), STAND_IN).ok()?;
    Some((dir, python))
}

mod tempdir {
    /// A folder removed when dropped.
    pub struct Dir(std::path::PathBuf);

    impl Dir {
        pub fn new() -> Self {
            let path = std::env::temp_dir().join(format!(
                "kentos-ext-process-{}-{}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map_or(0, |d| d.as_nanos())
            ));
            std::fs::create_dir_all(&path).expect("a folder");
            Dir(path)
        }

        pub fn path(&self) -> &std::path::Path {
            &self.0
        }
    }

    impl Drop for Dir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
}

fn start(dir: &std::path::Path, python: &std::path::Path, extension: &str) -> Start {
    Start {
        python: python.to_path_buf(),
        args: vec!["-u".into(), dir.join("host.py").into_os_string()],
        hello: json!({ "type": "hello", "protocol": 1, "extension": extension }),
    }
}

fn invoke(local: &str) -> serde_json::Value {
    json!({ "kind": "processing", "local": local, "handler": "x:y", "params": { "n": 21 }, "inputs": {} })
}

macro_rules! need_python {
    () => {
        match stand_in() {
            Some(found) => found,
            None => {
                eprintln!("Python 3 yok: süreç havuzunun sınaması atlandı.");
                return;
            }
        }
    };
}

#[test]
fn a_run_is_answered_and_its_process_kept_for_the_next() {
    let (dir, python) = need_python!();
    let pool = Arc::new(Pool::default());
    let mut heard = Vec::new();
    let first = pool.run(
        "tr.ornek.deneme@1.0.0",
        &|| Ok(start(dir.path(), &python, "tr.ornek.deneme")),
        invoke("echo"),
        Duration::from_secs(30),
        &|| false,
        &mut |h| heard.push(h),
    );
    let Ended::Answered(done) = first else {
        panic!("answered: {first:?}");
    };
    assert_eq!(done["result"]["outputs"]["n"], 42);
    assert!(heard.contains(&Heard::Progress {
        done: 1.0,
        total: Some(2.0),
        text: Some("yarı".into())
    }));
    assert!(matches!(&heard[1], Heard::Log { level, .. } if level == "info"));
    // Started again only when it has no process: the same one answers.
    let second = pool.run(
        "tr.ornek.deneme@1.0.0",
        &|| Err("ikinci kez başlatılmamalıydı".into()),
        invoke("echo"),
        Duration::from_secs(30),
        &|| false,
        &mut |_| {},
    );
    let Ended::Answered(again) = second else {
        panic!("answered: {second:?}");
    };
    assert_eq!(again["result"]["pid"], done["result"]["pid"]);
    let running = pool.running();
    assert_eq!(running.len(), 1);
    assert_eq!(running[0].runs, 2);
    assert!(running[0].idle.is_some(), "no run goes on");
    if cfg!(target_os = "linux") {
        let memory = running[0].memory.expect("Linux says a process's memory");
        assert!(memory > 1024 * 1024, "{memory} bytes");
    }
    pool.close_all();
    assert!(pool.running().is_empty());
}

#[test]
fn durdur_is_asked_first_and_the_process_ended_when_it_does_not_answer() {
    let (dir, python) = need_python!();
    let pool = Arc::new(Pool::default().with_kill_after(Duration::from_millis(300)));
    let key = "tr.ornek.deneme@1.0.0";
    let begin = || Ok(start(dir.path(), &python, "tr.ornek.deneme"));
    // Asked: the code answers its stop.
    let stop = AtomicBool::new(false);
    let asked = Instant::now();
    let ended = std::thread::scope(|s| {
        s.spawn(|| {
            std::thread::sleep(Duration::from_millis(200));
            stop.store(true, Ordering::Relaxed);
        });
        pool.run(
            key,
            &begin,
            invoke("slow"),
            Duration::from_secs(30),
            &|| stop.load(Ordering::Relaxed),
            &mut |_| {},
        )
    });
    let Ended::Answered(done) = ended else {
        panic!("answered its stop: {ended:?}");
    };
    assert_eq!(done["stopped"], true);
    assert!(asked.elapsed() < Duration::from_secs(3));
    let after = done["stoppedAfterMs"]
        .as_u64()
        .expect("how long after the stop");
    assert!(after < 1000, "answered {after} ms after its stop");
    let pid = pool.running()[0].pid;
    // Not answered: its process ended after the wait, the next run in a new one.
    let stubborn = pool.run(
        key,
        &begin,
        invoke("stubborn"),
        Duration::from_secs(30),
        &|| true,
        &mut |_| {},
    );
    assert_eq!(stubborn, Ended::Killed { timed_out: false });
    assert!(pool.running().is_empty());
    let next = pool.run(
        key,
        &begin,
        invoke("echo"),
        Duration::from_secs(30),
        &|| false,
        &mut |_| {},
    );
    let Ended::Answered(done) = next else {
        panic!("{next:?}");
    };
    assert_ne!(done["result"]["pid"].as_u64(), Some(u64::from(pid)));
    // Overdue: stopped as by Durdur, said as its time.
    let overdue = pool.run(
        key,
        &begin,
        invoke("stubborn"),
        Duration::from_millis(100),
        &|| false,
        &mut |_| {},
    );
    assert_eq!(overdue, Ended::Killed { timed_out: true });
}

#[test]
fn a_crash_is_said_with_its_last_errors() {
    let (dir, python) = need_python!();
    let pool = Arc::new(Pool::default());
    let ended = pool.run(
        "tr.ornek.deneme@1.0.0",
        &|| Ok(start(dir.path(), &python, "tr.ornek.deneme")),
        invoke("crash"),
        Duration::from_secs(30),
        &|| false,
        &mut |_| {},
    );
    let Ended::Crashed(words) = ended else {
        panic!("a crash: {ended:?}");
    };
    assert!(words.contains("bir şey kırıldı"), "{words}");
    assert!(words.contains('3'), "its exit: {words}");
    assert!(pool.running().is_empty());
}

#[test]
fn a_tool_asking_for_the_drawing_is_refused() {
    let (dir, python) = need_python!();
    let pool = Arc::new(Pool::default());
    let ended = pool.run(
        "tr.ornek.deneme@1.0.0",
        &|| Ok(start(dir.path(), &python, "tr.ornek.deneme")),
        invoke("asks"),
        Duration::from_secs(30),
        &|| false,
        &mut |_| {},
    );
    let Ended::Answered(done) = ended else {
        panic!("{ended:?}");
    };
    assert_eq!(done["result"]["refused"], true);
    assert_eq!(done["result"]["code"], "capability");
}

#[test]
fn a_refused_hello_and_a_missing_interpreter_are_said() {
    let (dir, python) = need_python!();
    let pool = Arc::new(Pool::default());
    let refused = pool.run(
        "refuses@1.0.0",
        &|| Ok(start(dir.path(), &python, "refuses")),
        invoke("echo"),
        Duration::from_secs(30),
        &|| false,
        &mut |_| {},
    );
    assert_eq!(refused, Ended::NotStarted("bu süreç reddeder".into()));
    let missing = pool.run(
        "missing@1.0.0",
        &|| {
            Ok(Start {
                python: dir.path().join("yok"),
                args: Vec::new(),
                hello: json!({}),
            })
        },
        invoke("echo"),
        Duration::from_secs(30),
        &|| false,
        &mut |_| {},
    );
    assert!(
        matches!(missing, Ended::NotStarted(ref why) if why.contains("başlatılamadı")),
        "{missing:?}"
    );
    let why = pool.run(
        "x@1",
        &|| Err("ortam hazır değil".into()),
        invoke("echo"),
        Duration::from_secs(1),
        &|| false,
        &mut |_| {},
    );
    assert_eq!(why, Ended::NotStarted("ortam hazır değil".into()));
}

#[test]
fn an_idle_process_is_closed() {
    let (dir, python) = need_python!();
    let pool = Arc::new(Pool::new(Duration::from_millis(200)));
    let ended = pool.run(
        "tr.ornek.deneme@1.0.0",
        &|| Ok(start(dir.path(), &python, "tr.ornek.deneme")),
        invoke("echo"),
        Duration::from_secs(30),
        &|| false,
        &mut |_| {},
    );
    assert!(matches!(ended, Ended::Answered(_)));
    assert_eq!(pool.running().len(), 1);
    let until = Instant::now() + Duration::from_secs(5);
    while !pool.running().is_empty() && Instant::now() < until {
        std::thread::sleep(Duration::from_millis(50));
    }
    assert!(pool.running().is_empty(), "closed after its idle time");
}

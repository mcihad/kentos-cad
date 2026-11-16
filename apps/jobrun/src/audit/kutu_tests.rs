//! The cloud audit box (docs/adr/8057 §61) under real Python: a rule's TCP and its files beyond its own are refused by
//! the system and its scratch works; UDP, and pathname UNIX sockets below Landlock ABI 9, get out, as the box's words
//! say; the same tries succeed from this test's own process, so the box made the difference. The box's cost at
//! start-up is measured. Without a Python, or where the box cannot close (no Landlock), those tests say so and pass.
//! Where the box cannot close the server's policy decides, and a box mode that cannot start the rule says why: on any
//! system, without Python.

use std::path::Path;
use std::time::{Duration, Instant};

use super::tests::{StandIn, folder, package, python, request};
use super::*;
use crate::kutu::{MARK, POLICY_ENV, Policy};

/// A path as the rule's text carries it: hex, so the package's checks do not take it for a user's path in the code.
#[cfg(target_os = "linux")]
fn hex(path: &Path) -> String {
    path.display()
        .to_string()
        .bytes()
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// A rule that tries what the box forbids and what it allows, and says how each went: TCP and UDP to this test's
/// listeners, a pathname UNIX socket outside the box, files outside it and the job's own, its scratch.
#[cfg(target_os = "linux")]
fn probe(tcp: u16, udp: u16, unix: &Path, outside: &Path, job: &Path) -> String {
    format!(
        r#"@ext.audit("adsiz")
def adsiz(facts: ext.GisAuditFacts) -> list[ext.AuditFinding]:
    """Kutunun sınaması."""
    import socket, tempfile
    disarisi = bytes.fromhex("{outside}").decode()
    klasor = bytes.fromhex("{job}").decode()
    soket = bytes.fromhex("{unix}").decode()
    sonuc = []
    def dene(ad, is_):
        try:
            is_()
            sonuc.append(ad + ": açık")
        except Exception as e:
            sonuc.append(ad + ": kapalı (" + type(e).__name__ + ")")
    def udp():
        with socket.socket(socket.AF_INET, socket.SOCK_DGRAM) as s:
            s.sendto(b"kutu", ("127.0.0.1", {udp}))
    def unix():
        with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as s:
            s.connect(soket)
    dene("tcp", lambda: socket.create_connection(("127.0.0.1", {tcp}), timeout=3).close())
    dene("udp", udp)
    dene("unix", unix)
    dene("oku-etc", lambda: open("/etc/hostname").read())
    dene("oku-ev", lambda: open(disarisi).read())
    dene("oku-cizim", lambda: open(klasor + "/snapshot.kcad", "rb").read())
    dene("yaz-is", lambda: open(klasor + "/disari.txt", "w").write("x"))
    dene("yaz-tmp", lambda: tempfile.NamedTemporaryFile().close())
    return [ext.AuditFinding(message=s, severity="info") for s in sonuc]
"#,
        outside = hex(outside),
        job = hex(job),
        unix = hex(unix),
    )
}

fn boxed() -> Option<String> {
    let python = python().or_else(|| {
        eprintln!("Python yok (KENTOS_TEST_PYTHON ya da .run/py): sınama atlandı");
        None
    })?;
    match crate::kutu::level() {
        Ok(_) => Some(python),
        Err(why) => {
            eprintln!("Kutu bu sistemde kapanmıyor ({why}): sınama atlandı");
            None
        }
    }
}

/// The box closes what its words (`level`, on the job runner's error output) say it closes and leaves open only what
/// they say: a rule in it reaches no TCP and no file beyond its own; UDP, and a pathname UNIX socket below Landlock
/// ABI 9, still get out, as the words say. The same tries succeed from this test's own process, so the box made the
/// difference.
#[cfg(target_os = "linux")]
#[test]
fn the_box_closes_tcp_and_files_beyond_the_rule_s_own_and_leaves_open_only_what_it_says() {
    use super::tests::package_with;
    let Some(python) = boxed() else {
        return;
    };
    let level = crate::kutu::level().unwrap();
    let tcp = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let udp = std::net::UdpSocket::bind("127.0.0.1:0").unwrap();
    udp.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
    let dir = folder("kutu", b"");
    // A pathname UNIX socket outside the box: in the job's folder, which the rule may neither read nor write.
    let sock = dir.join("disari.sock");
    let unix = std::os::unix::net::UnixListener::bind(&sock).unwrap();
    unix.set_nonblocking(true).unwrap();
    // The job runner's own manifest: a file of the user's, outside the box.
    let outside = Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml");
    let rule = probe(
        tcp.local_addr().unwrap().port(),
        udp.local_addr().unwrap().port(),
        &sock,
        &outside,
        &dir,
    );
    let (bytes, digest, manifest) = package_with(&rule, false);
    std::fs::write(dir.join(PACKAGE), bytes).unwrap();
    let done = run_audit(
        &dir,
        request(&digest, &manifest, &python),
        &ExtHost::default(),
    )
    .expect("the rule ran in the box");
    let report: AuditReport = serde_json::from_value(done.details).unwrap();
    let said: Vec<&str> = report.findings.iter().map(|f| f.message.as_str()).collect();
    let unix_said = if level.contains("yol adlı UNIX soketleri kapalı") {
        "unix: kapalı (PermissionError)"
    } else {
        "unix: açık"
    };
    assert_eq!(
        said,
        [
            "tcp: kapalı (PermissionError)",
            "udp: açık",
            unix_said,
            "oku-etc: kapalı (PermissionError)",
            "oku-ev: kapalı (PermissionError)",
            "oku-cizim: kapalı (PermissionError)",
            "yaz-is: kapalı (PermissionError)",
            "yaz-tmp: açık",
        ],
        "{level}"
    );
    assert!(level.contains("UDP ve öteki soket türleri açık"), "{level}");
    assert!(
        !done.summary.contains("kutu kapatılamadı"),
        "{}",
        done.summary
    );
    // What got out arrived: the datagram, and the UNIX connection where the words say the sockets are open.
    let mut got = [0u8; 8];
    let (n, _) = udp.recv_from(&mut got).expect("the rule's datagram");
    assert_eq!(&got[..n], b"kutu");
    assert_eq!(unix.accept().is_ok(), unix_said == "unix: açık");
    // Outside the box the same tries succeed.
    assert!(std::net::TcpStream::connect(tcp.local_addr().unwrap()).is_ok());
    assert!(std::fs::read(&outside).is_ok());
    assert!(!dir.join("disari.txt").exists());
    let _ = std::fs::remove_dir_all(&dir);
}

/// The box's cost at start-up: the same interpreter loading the SDK's host, inside the box mode and outside it, three
/// times each (the median).
#[test]
fn the_box_costs_little_at_start_up() {
    let Some(python) = boxed() else {
        return;
    };
    let dir = folder("kutu-olcu", b"");
    let tmp = dir.join(crate::kutu::SCRATCH);
    std::fs::create_dir_all(&tmp).unwrap();
    std::fs::create_dir_all(dir.join(BYTECODE)).unwrap();
    let python = std::path::PathBuf::from(python);
    let args: Vec<std::ffi::OsString> = vec![
        "-I".into(),
        "-X".into(),
        format!("pycache_prefix={}", dir.join(BYTECODE).display()).into(),
        "-c".into(),
        "import kentos.ext_host".into(),
    ];
    let allowed = crate::kutu::allowed(&dir, &python, &dir);
    let (program, boxed_args) = crate::kutu::wrap(
        &crate::kutu::program().unwrap(),
        &allowed,
        &tmp,
        &python,
        &args,
    );
    let time = |program: &Path, args: &[std::ffi::OsString]| {
        let mut runs: Vec<Duration> = (0..3)
            .map(|_| {
                let began = Instant::now();
                let status = std::process::Command::new(program)
                    .args(args)
                    .env_clear()
                    .status()
                    .expect("started");
                assert!(status.success(), "{program:?} {args:?}");
                began.elapsed()
            })
            .collect();
        runs.sort();
        runs[1]
    };
    let open = time(&python, &args);
    let closed = time(&program, &boxed_args);
    eprintln!(
        "SDK'nın konağını yükleyen yorumlayıcı: kutusuz {open:.2?}, kutuda {closed:.2?} (fark {:.0?})",
        closed.saturating_sub(open)
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// Where the box cannot close, the server's policy decides: `require` refuses before the package is unpacked or the
/// drawing measured, and no rule runs; `report` runs the interpreter itself and the job's summary says the box was
/// open.
#[test]
fn where_the_box_cannot_close_the_rule_runs_only_if_the_server_allows_and_the_job_says_so() {
    let (bytes, digest, manifest) = package(false);
    let dir = folder("kutusuz", &bytes);
    let title = &manifest.entrypoints.audits[0].title;
    let open = |policy| Confinement {
        level: Err("çekirdekte Landlock yok".into()),
        policy,
    };
    let runner = StandIn::default();
    let run = || request(&digest, &manifest, "/opt/p/bin/python");
    let refused = run_audit_in(&dir, run(), &runner, open(Policy::Require)).unwrap_err();
    assert_eq!(refused.code, "tool_unavailable");
    assert!(
        refused.message.contains("çekirdekte Landlock yok") && refused.message.contains(POLICY_ENV),
        "{}",
        refused.message
    );
    assert!(runner.given.borrow().is_none(), "no rule ran");
    assert!(!dir.join(UNPACKED).exists(), "nothing was unpacked");
    let done = run_audit_in(&dir, run(), &runner, open(Policy::Report)).unwrap();
    let (start, _) = runner.given.borrow_mut().take().unwrap();
    assert_eq!(start.python, Path::new("/opt/p/bin/python"));
    assert_eq!(start.args[0], "-I");
    assert_eq!(
        done.summary,
        format!("{title}: 1 hata (kutu kapatılamadı: çekirdekte Landlock yok)")
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// A box mode that cannot start the rule (here no interpreter where the request says; where Landlock is missing, the
/// box itself) says why with its mark, and the job is the tool's unavailability, not the rule's failure.
#[test]
fn a_box_that_cannot_start_the_rule_says_why() {
    let (bytes, digest, manifest) = package(false);
    let dir = folder("kutu-yorumlayicisiz", &bytes);
    let boxed = Confinement {
        level: Ok("sınamanın kutusu".into()),
        policy: Policy::Require,
    };
    let failure = run_audit_in(
        &dir,
        request(&digest, &manifest, "/yok/boyle/bir/ortam/bin/python"),
        &ExtHost::default(),
        boxed,
    )
    .unwrap_err();
    assert_eq!(failure.code, "tool_unavailable", "{}", failure.message);
    assert!(failure.message.contains(MARK), "{}", failure.message);
    let _ = std::fs::remove_dir_all(&dir);
}

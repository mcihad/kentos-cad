//! The cloud audit box (docs/adr/8057 §61, §63) under real Python: a rule in the whole box opens no socket (TCP, UDP,
//! a pathname UNIX socket, a UNIX datagram pair, an io_uring ring) and reads and writes no file beyond its own, while
//! its scratch and asyncio's self-pipe work; the same rule outside the box reaches all of them, so the box made the
//! difference. The box's cost at start-up is measured. Without a Python, or where the box cannot be whole, those tests
//! say so and pass. Where it cannot be whole the server's policy decides, and a box mode that cannot start the rule
//! says why: on any system, without Python.

use std::path::Path;
use std::time::{Duration, Instant};

use super::tests::{StandIn, folder, package, python, request};
use super::*;
use crate::kutu::{MARK, NO_FILTER, POLICY_ENV, Plan, Policy};

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
/// listeners, a pathname UNIX socket outside the box, a UNIX datagram pair, an io_uring ring; files outside the box
/// and the job's own; its scratch, and an asyncio loop (its self-pipe is a UNIX stream pair).
#[cfg(target_os = "linux")]
fn probe(tcp: u16, udp: u16, unix: &Path, outside: &Path, job: &Path) -> String {
    format!(
        r#"@ext.audit("adsiz")
def adsiz(facts: ext.GisAuditFacts) -> list[ext.AuditFinding]:
    """Kutunun sınaması."""
    import asyncio, ctypes, os, socket, tempfile
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
    def cift():
        a, b = socket.socketpair(socket.AF_UNIX, socket.SOCK_DGRAM)
        a.close()
        b.close()
    def halka():
        libc = ctypes.CDLL(None, use_errno=True)
        params = ctypes.create_string_buffer(120)
        fd = libc.syscall(ctypes.c_long(425), ctypes.c_long(1), params)
        if fd < 0:
            e = ctypes.get_errno()
            raise OSError(e, os.strerror(e))
        os.close(fd)
    async def bos():
        return None
    dene("tcp", lambda: socket.create_connection(("127.0.0.1", {tcp}), timeout=3).close())
    dene("udp", udp)
    dene("unix", unix)
    dene("cift-dgram", cift)
    dene("io_uring", halka)
    dene("asyncio", lambda: asyncio.run(bos()))
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

/// The interpreter and the whole box's words, or why the test is skipped.
fn boxed() -> Option<(String, String)> {
    let python = python().or_else(|| {
        eprintln!("Python yok (KENTOS_TEST_PYTHON ya da .run/py): sınama atlandı");
        None
    })?;
    match crate::kutu::plan(
        crate::kutu::landlock(),
        crate::kutu::filter(),
        Policy::Require,
    ) {
        Ok(Plan::Whole(words)) => Some((python, words)),
        Ok(other) => panic!("require gives the whole box or nothing: {other:?}"),
        Err(why) => {
            eprintln!("Kutu bu sistemde bütün kapanmıyor ({why}): sınama atlandı");
            None
        }
    }
}

/// The whole box closes what its words (on the job runner's error output) say: a rule in it opens no socket of any
/// kind and reaches no file beyond its own; its scratch and asyncio work. Nothing it tried arrived at this test's
/// listeners. The same rule outside the box reaches every one of them, and they arrive.
#[cfg(target_os = "linux")]
#[test]
fn the_whole_box_opens_no_socket_and_no_file_beyond_the_rule_s_own() {
    use super::tests::package_with;
    let Some((python, words)) = boxed() else {
        return;
    };
    assert!(
        words.contains("ağ kapalı (soket açılamaz): TCP, UDP, yol adlı ve soyut UNIX soketleri"),
        "{words}"
    );
    let tcp = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    tcp.set_nonblocking(true).unwrap();
    let udp = std::net::UdpSocket::bind("127.0.0.1:0").unwrap();
    udp.set_nonblocking(true).unwrap();
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
    let said_in = |plan: Plan| -> (Vec<String>, String) {
        let done = run_audit_in(
            &dir,
            request(&digest, &manifest, &python),
            &ExtHost::default(),
            Ok(plan),
        )
        .expect("the rule ran");
        let report: AuditReport = serde_json::from_value(done.details).unwrap();
        (
            report.findings.into_iter().map(|f| f.message).collect(),
            done.summary,
        )
    };
    let arrived = || {
        let mut got = [0u8; 8];
        (
            tcp.accept().is_ok(),
            udp.recv_from(&mut got).is_ok(),
            unix.accept().is_ok(),
        )
    };
    let (inside, summary) = said_in(Plan::Whole(words.clone()));
    assert_eq!(
        inside,
        [
            "tcp: kapalı (PermissionError)",
            "udp: kapalı (PermissionError)",
            "unix: kapalı (PermissionError)",
            "cift-dgram: kapalı (PermissionError)",
            "io_uring: kapalı (PermissionError)",
            "asyncio: açık",
            "oku-etc: kapalı (PermissionError)",
            "oku-ev: kapalı (PermissionError)",
            "oku-cizim: kapalı (PermissionError)",
            "yaz-is: kapalı (PermissionError)",
            "yaz-tmp: açık",
        ],
        "{words}"
    );
    assert!(!summary.contains("kutu"), "{summary}");
    assert_eq!(
        arrived(),
        (false, false, false),
        "nothing got out of the box"
    );
    assert!(!dir.join("disari.txt").exists());
    // The same rule outside the box (io_uring may be closed by the system's own setting).
    let (outside_box, _) = said_in(Plan::Outside("sınama: kutusuz".into()));
    eprintln!("kutusuz: {outside_box:?}");
    for said in outside_box
        .iter()
        .filter(|s| !s.starts_with("io_uring") && !s.starts_with("oku-etc"))
    {
        assert!(said.ends_with(": açık"), "outside the box: {outside_box:?}");
    }
    assert_eq!(
        arrived(),
        (true, true, true),
        "outside the box all of them arrive"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// The box's cost at start-up: the same interpreter loading the SDK's host outside the box, in Landlock alone and in
/// the whole box, three times each (the median).
#[test]
fn the_box_costs_little_at_start_up() {
    let Some((python, _)) = boxed() else {
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
    let program = crate::kutu::program().unwrap();
    let boxed_with = |filter| crate::kutu::wrap(&program, &allowed, filter, &tmp, &python, &args);
    let time = |(program, args): (std::path::PathBuf, Vec<std::ffi::OsString>)| {
        let mut runs: Vec<Duration> = (0..3)
            .map(|_| {
                let began = Instant::now();
                let status = std::process::Command::new(&program)
                    .args(&args)
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
    let open = time((python.clone(), args.clone()));
    let landlock = time(boxed_with(false));
    let whole = time(boxed_with(true));
    eprintln!(
        "SDK'nın konağını yükleyen yorumlayıcı: kutusuz {open:.2?}, Landlock'ta {landlock:.2?}, \
         bütün kutuda {whole:.2?}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// Where the box cannot be whole, the server's policy decides: `require` refuses before the package is unpacked or
/// the drawing measured, and no rule runs; `report` runs the rule with what closes (Landlock without the filter, or
/// the interpreter itself) and the job's summary says what is open.
#[test]
fn where_the_box_cannot_be_whole_the_rule_runs_only_if_the_server_allows_and_the_job_says_so() {
    let (bytes, digest, manifest) = package(false);
    let dir = folder("kutusuz", &bytes);
    let title = &manifest.entrypoints.audits[0].title;
    let runner = StandIn::default();
    let run = || request(&digest, &manifest, "/opt/p/bin/python");
    let refused =
        run_audit_in(&dir, run(), &runner, Err("çekirdekte Landlock yok".into())).unwrap_err();
    assert_eq!(refused.code, "tool_unavailable");
    assert!(
        refused.message.contains("çekirdekte Landlock yok") && refused.message.contains(POLICY_ENV),
        "{}",
        refused.message
    );
    assert!(runner.given.borrow().is_none(), "no rule ran");
    assert!(!dir.join(UNPACKED).exists(), "nothing was unpacked");
    // Landlock cannot close: the interpreter itself.
    let outside = Plan::Outside("çekirdekte Landlock yok".into());
    let done = run_audit_in(&dir, run(), &runner, Ok(outside)).unwrap();
    let (start, _) = runner.given.borrow_mut().take().unwrap();
    assert_eq!(start.python, Path::new("/opt/p/bin/python"));
    assert_eq!(start.args[0], "-I");
    assert_eq!(
        done.summary,
        format!("{title}: 1 hata (kutu kapatılamadı: çekirdekte Landlock yok)")
    );
    // The filter cannot be installed: Landlock alone, the box mode told so.
    let alone = Plan::LandlockOnly("seccomp süzgeci yok; UDP açık".into());
    let done = run_audit_in(&dir, run(), &runner, Ok(alone)).unwrap();
    let (start, _) = runner.given.borrow_mut().take().unwrap();
    assert_eq!(start.python, crate::kutu::program().unwrap());
    assert_eq!(start.args[..2], [crate::kutu::ARG, NO_FILTER]);
    assert_eq!(
        done.summary,
        format!("{title}: 1 hata (kutu tam kapatılamadı: seccomp süzgeci yok; UDP açık)")
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// A box mode that cannot start the rule (here no interpreter where the request says; where Landlock or the filter is
/// missing, the box itself) says why with its mark, and the job is the tool's unavailability, not the rule's failure.
#[test]
fn a_box_that_cannot_start_the_rule_says_why() {
    let (bytes, digest, manifest) = package(false);
    let dir = folder("kutu-yorumlayicisiz", &bytes);
    let failure = run_audit_in(
        &dir,
        request(&digest, &manifest, "/yok/boyle/bir/ortam/bin/python"),
        &ExtHost::default(),
        Ok(Plan::Whole("sınamanın kutusu".into())),
    )
    .unwrap_err();
    assert_eq!(failure.code, "tool_unavailable", "{}", failure.message);
    assert!(failure.message.contains(MARK), "{}", failure.message);
    let _ = std::fs::remove_dir_all(&dir);
}

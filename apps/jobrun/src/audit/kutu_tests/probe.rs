//! The whole box under real Python, at the kernel's Landlock ABI and at ABI 1 (docs/adr/8057 §65): the same rule tries
//! what the box forbids and what it allows, and the same tries outside the box succeed.

use super::*;

/// A path as the rule's text carries it: hex, so the package's checks do not take it for a user's path in the code.
fn hex(path: &Path) -> String {
    path.display()
        .to_string()
        .bytes()
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// A rule that tries what the box forbids and what it allows, and says how each went, with the error's number: TCP and
/// UDP to this test's listeners, a pathname UNIX socket outside the box, a UNIX datagram pair, an io_uring ring;
/// asyncio (its self-pipe is a UNIX stream pair); files outside the box and the job's own, truncation by path and by an
/// open for reading, a file's mode, owner, times and extended attributes; this test's process's limits, priority and
/// CPUs (set to what they are: nothing changes where it works); signals to it, to the rule itself, its group and its
/// thread; a move across folders in its scratch; its scratch.
fn probe(tcp: u16, udp: u16, unix: &Path, outside: &Path, job: &Path) -> String {
    format!(
        r#"@ext.audit("adsiz")
def adsiz(facts: ext.GisAuditFacts) -> list[ext.AuditFinding]:
    """Kutunun sınaması."""
    import asyncio, ctypes, os, resource, shutil, signal, socket, tempfile, threading
    disarisi = bytes.fromhex("{outside}").decode()
    klasor = bytes.fromhex("{job}").decode()
    soket = bytes.fromhex("{unix}").decode()
    cizim = klasor + "/snapshot.kcad"
    ebeveyn = os.getppid()
    sonuc = []
    def dene(ad, is_):
        try:
            is_()
            sonuc.append(ad + ": açık")
        except Exception as e:
            sonuc.append(ad + ": kapalı (" + type(e).__name__ + ", " + str(getattr(e, "errno", None)) + ")")
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
        fd = libc.syscall(ctypes.c_long(425), ctypes.c_long(1), ctypes.create_string_buffer(120))
        if fd < 0:
            raise OSError(ctypes.get_errno(), "io_uring_setup")
        os.close(fd)
    async def bos():
        return None
    def ayni(degistir):
        degistir(os.stat(cizim))
    def tasi():
        kok = tempfile.mkdtemp()
        try:
            os.mkdir(kok + "/a")
            os.mkdir(kok + "/b")
            open(kok + "/a/x", "w").close()
            os.rename(kok + "/a/x", kok + "/b/x")
        finally:
            shutil.rmtree(kok, ignore_errors=True)
    dene("tcp", lambda: socket.create_connection(("127.0.0.1", {tcp}), timeout=3).close())
    dene("udp", udp)
    dene("unix", unix)
    dene("cift-dgram", cift)
    dene("io_uring", halka)
    dene("asyncio", lambda: asyncio.run(bos()))
    dene("oku-etc", lambda: open("/etc/hostname").read())
    dene("oku-ev", lambda: open(disarisi).read())
    dene("oku-cizim", lambda: open(cizim, "rb").read())
    dene("yaz-is", lambda: open(klasor + "/disari.txt", "w").write("x"))
    dene("kes-yol", lambda: os.truncate(klasor + "/kesilecek.txt", 0))
    dene("kes-acarak", lambda: os.close(os.open(__file__, os.O_RDONLY | os.O_TRUNC)))
    dene("izin", lambda: ayni(lambda st: os.chmod(cizim, st.st_mode & 0o7777)))
    dene("sahip", lambda: ayni(lambda st: os.chown(cizim, st.st_uid, st.st_gid)))
    dene("zaman", lambda: ayni(lambda st: os.utime(cizim, ns=(st.st_atime_ns, st.st_mtime_ns))))
    dene("xattr", lambda: os.setxattr(cizim, "user.kutu", b"1"))
    def sinir():
        dosyalar = resource.RLIMIT_NOFILE
        resource.prlimit(ebeveyn, dosyalar, resource.prlimit(ebeveyn, dosyalar))
    dene("sinir", sinir)
    dene("oncelik", lambda: os.setpriority(os.PRIO_PROCESS, ebeveyn, os.getpriority(os.PRIO_PROCESS, ebeveyn)))
    dene("islemciler", lambda: os.sched_setaffinity(ebeveyn, os.sched_getaffinity(ebeveyn)))
    dene("sinyal-ebeveyn", lambda: os.kill(ebeveyn, 0))
    dene("sinyal-kendi", lambda: os.kill(os.getpid(), 0))
    dene("sinyal-grup", lambda: os.kill(0, 0))
    dene("sinyal-is-parcacigi", lambda: signal.pthread_kill(threading.get_ident(), 0))
    dene("tasi-klasorler-arasi", tasi)
    dene("yaz-tmp", lambda: tempfile.NamedTemporaryFile().close())
    return [ext.AuditFinding(message=s, severity="info") for s in sonuc]
"#,
        outside = hex(outside),
        job = hex(job),
        unix = hex(unix),
    )
}

/// What the rule says in the whole box at Landlock ABI `abi`: every try refused with `EACCES` (Landlock's or the
/// filter's) but asyncio, the rule's own signals and its scratch; a signal to another process refused with Landlock's
/// `EPERM` from ABI 6, with the filter's `EACCES` below it; a move across folders allowed from ABI 2, below it denied
/// outright (`EXDEV`).
fn expected(abi: u8) -> Vec<String> {
    let refused = |name: &str| format!("{name}: kapalı (PermissionError, 13)");
    let open = |name: &str| format!("{name}: açık");
    let mut said: Vec<String> = ["tcp", "udp", "unix", "cift-dgram", "io_uring"]
        .into_iter()
        .map(refused)
        .collect();
    said.push(open("asyncio"));
    said.extend(
        [
            "oku-etc",
            "oku-ev",
            "oku-cizim",
            "yaz-is",
            "kes-yol",
            "kes-acarak",
            "izin",
            "sahip",
            "zaman",
            "xattr",
            "sinir",
            "oncelik",
            "islemciler",
        ]
        .into_iter()
        .map(refused),
    );
    said.push(if abi >= 6 {
        "sinyal-ebeveyn: kapalı (PermissionError, 1)".to_owned()
    } else {
        refused("sinyal-ebeveyn")
    });
    said.extend(
        ["sinyal-kendi", "sinyal-grup", "sinyal-is-parcacigi"]
            .into_iter()
            .map(open),
    );
    said.push(if abi >= 2 {
        open("tasi-klasorler-arasi")
    } else {
        "tasi-klasorler-arasi: kapalı (OSError, 18)".to_owned()
    });
    said.push(open("yaz-tmp"));
    said
}

/// The whole box, at the kernel's Landlock ABI and at ABI 1 as a 5.15 kernel has it, closes the same: no socket of
/// any kind, no file beyond the rule's own (its contents, truncation by path or by an open for reading, its metadata),
/// no hold on another process (limits, priority, CPUs, signals); asyncio, the rule's own signals and its scratch work.
/// What differs is who refuses a signal and whether a move across folders is allowed: ABI 1's own, stricter rule.
/// Nothing the rule tried arrived at this test's listeners and the file it tried to cut is whole. Outside the box the
/// same rule reaches every one of them, and they arrive.
#[test]
fn the_whole_box_closes_the_same_at_the_kernel_s_landlock_abi_and_at_abi_1() {
    use super::super::tests::package_with;
    let Some(python) = boxed() else {
        return;
    };
    let kernel = crate::kutu::landlock().unwrap().abi;
    let tcp = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    tcp.set_nonblocking(true).unwrap();
    let udp = std::net::UdpSocket::bind("127.0.0.1:0").unwrap();
    udp.set_nonblocking(true).unwrap();
    let dir = folder("kutu", b"");
    // A pathname UNIX socket and a file to cut outside the box: in the job's folder, which the rule may neither read
    // nor write.
    let sock = dir.join("disari.sock");
    let unix = std::os::unix::net::UnixListener::bind(&sock).unwrap();
    unix.set_nonblocking(true).unwrap();
    let to_cut = dir.join("kesilecek.txt");
    std::fs::write(&to_cut, "kesilmedi").unwrap();
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
    for (abi, lowered) in [(kernel, None), (1, Some(1))] {
        let Ok(Plan::Whole { words, .. }) =
            crate::kutu::plan(Ok(crate::kutu::Landlock { abi }), Ok(()), Policy::Require)
        else {
            panic!("the whole box at ABI {abi}");
        };
        let (inside, summary) = said_in(Plan::Whole {
            words: words.clone(),
            abi: lowered,
        });
        assert_eq!(inside, expected(abi), "Landlock ABI {abi}: {words}");
        assert!(!summary.contains("kutu"), "{summary}");
        assert_eq!(
            arrived(),
            (false, false, false),
            "nothing got out at ABI {abi}"
        );
        assert_eq!(std::fs::read_to_string(&to_cut).unwrap(), "kesilmedi");
        assert!(!dir.join("disari.txt").exists());
        eprintln!("ABI {abi}: {words}");
    }
    // The same rule outside the box (io_uring may be closed by the system's own setting, user xattrs by the file
    // system, and /etc/hostname may be missing).
    let (outside_box, _) = said_in(Plan::Outside("sınama: kutusuz".into()));
    eprintln!("kutusuz: {outside_box:?}");
    for said in outside_box.iter().filter(|s| {
        !["io_uring", "xattr", "oku-etc"]
            .iter()
            .any(|n| s.starts_with(*n))
    }) {
        assert!(said.ends_with(": açık"), "outside the box: {outside_box:?}");
    }
    assert_eq!(
        arrived(),
        (true, true, true),
        "outside the box all of them arrive"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

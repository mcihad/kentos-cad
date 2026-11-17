//! The cloud audit box (docs/adr/8057 §61, §63, §65) under real Python (`probe`): a rule in the whole box opens no
//! socket, reaches no file beyond its own, changes no file's metadata and holds no other process, at the kernel's
//! Landlock ABI and at ABI 1, while its scratch and asyncio's self-pipe work; the same rule outside the box reaches all
//! of them, so the box made the difference. The box's cost at start-up is measured. Without a Python, or where the box
//! cannot be whole, those tests say so and pass. Where it cannot be whole the server's policy decides, and a box mode
//! that cannot start the rule says why: on any system, without Python.

use std::path::Path;
use std::time::{Duration, Instant};

use super::tests::{StandIn, folder, package, python, request};
use super::*;
use crate::kutu::{MARK, NO_FILTER, POLICY_ENV, Plan, Policy};

#[cfg(target_os = "linux")]
mod probe;

/// The interpreter, where this system's box is whole; why the test is skipped where it is not.
fn boxed() -> Option<String> {
    let python = python().or_else(|| {
        eprintln!("Python yok (KENTOS_TEST_PYTHON ya da .run/py): sınama atlandı");
        None
    })?;
    match crate::kutu::plan(
        crate::kutu::landlock(),
        crate::kutu::filter(),
        Policy::Require,
    ) {
        Ok(Plan::Whole { .. }) => Some(python),
        Ok(other) => panic!("require gives the whole box or nothing: {other:?}"),
        Err(why) => {
            eprintln!("Kutu bu sistemde bütün kapanmıyor ({why}): sınama atlandı");
            None
        }
    }
}

/// The box's cost at start-up: the same interpreter loading the SDK's host outside the box, in Landlock alone, in the
/// whole box and in the whole box at Landlock ABI 1, three times each (the median).
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
    let program = crate::kutu::program().unwrap();
    let boxed_with =
        |filter, abi| crate::kutu::wrap(&program, &allowed, filter, abi, &tmp, &python, &args);
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
    let landlock = time(boxed_with(false, None));
    let whole = time(boxed_with(true, None));
    let whole_1 = time(boxed_with(true, Some(1)));
    eprintln!(
        "SDK'nın konağını yükleyen yorumlayıcı: kutusuz {open:.2?}, Landlock'ta {landlock:.2?}, \
         bütün kutuda {whole:.2?}, bütün kutuda ABI 1 ile {whole_1:.2?}"
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
        Ok(Plan::Whole {
            words: "sınamanın kutusu".into(),
            abi: None,
        }),
    )
    .unwrap_err();
    assert_eq!(failure.code, "tool_unavailable", "{}", failure.message);
    assert!(failure.message.contains(MARK), "{}", failure.message);
    let _ = std::fs::remove_dir_all(&dir);
}

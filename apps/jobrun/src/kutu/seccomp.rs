//! The box's seccomp filter (docs/adr/8057 §63, §65): what Landlock leaves open, closed where the calls are made. A
//! refused call fails with `EACCES`, as Landlock's refusals do (Python's `PermissionError`); everything else is
//! allowed. A deny list, not an allow list: counting Python's and its libraries' calls one by one would break with
//! every release.
//!
//! - **No socket** (§63): `socket(2)` for every family; `socketpair(2)` but for a UNIX pair of a stream or sequenced
//!   packets, which stays inside the box (asyncio's self-pipe), while a UNIX datagram pair can send to any pathname
//!   socket by its name; `io_uring_setup(2)`, whose rings can open sockets themselves.
//! - **No truncation by path** (§65): `truncate(2)`; `open`/`openat` of a file for reading with `O_TRUNC`, which cuts
//!   a file the rule may only read; `openat2(2)`, whose flags the filter cannot see. Landlock closes these from ABI 3
//!   only. `ftruncate(2)` needs a descriptor opened for writing, which Landlock controls from ABI 1.
//! - **No change to a file's metadata** (§65): mode, owner, times, extended attributes, by path or by descriptor.
//!   Landlock controls none of them at any ABI: the rule could make any file of the server's user unreadable, or
//!   readable to everyone.
//! - **Other processes left alone** (§65): resource limits, priority, scheduling and I/O priority (`prlimit64`,
//!   `setpriority`, `sched_set*`, `ioprio_set`) for itself only: 0 or its own pid, which `exec` keeps. Landlock
//!   controls none of them. Below Landlock ABI 6, which keeps signals inside the domain, signals too: `kill` only to
//!   itself or the job's process group (0), `tkill`, `tgkill` and `rt_(tg)sigqueueinfo` only within itself, no pidfd.
//! - **x32 calls** (x86_64): all refused (their numbers carry `0x40000000`). A call of another architecture (32-bit
//!   compatibility) ends the process: seccompiler's first test.
//!
//! The numbers are the target's own (`libc`): x86_64's or aarch64's; the newest calls, which `libc` does not name on
//! both yet (`fchmodat2`, `setxattrat`, `removexattrat`), by the table the two share from 424 on.

use std::collections::BTreeMap;

use seccompiler::{
    BpfProgram, SeccompAction, SeccompCmpArgLen, SeccompCmpOp, SeccompCondition, SeccompFilter,
    SeccompRule, TargetArch,
};

#[cfg(target_arch = "x86_64")]
const ARCH: TargetArch = TargetArch::x86_64;
#[cfg(target_arch = "aarch64")]
const ARCH: TargetArch = TargetArch::aarch64;

const SYS_FCHMODAT2: i64 = 452;
const SYS_SETXATTRAT: i64 = 463;
const SYS_REMOVEXATTRAT: i64 = 466;
/// A socket's type without its flags (`SOCK_NONBLOCK`, `SOCK_CLOEXEC`): `SOCK_TYPE_MASK`.
const SOCK_TYPE_MASK: u64 = 0xf;
/// An open's access mode (`O_RDONLY` is 0).
const ACCESS_MODE: u64 = 3;
/// `setpriority(2)`'s and `ioprio_set(2)`'s "one process".
const PRIO_PROCESS: u64 = 0;
const IOPRIO_WHO_PROCESS: u64 = 1;

type Rules = BTreeMap<i64, Vec<SeccompRule>>;

/// The filter for the process `own` (the box mode's, which `exec` keeps), compiled for this architecture; `signals`
/// adds the kill family's rules (Landlock below ABI 6).
fn program(own: u32, signals: bool) -> Result<BpfProgram, String> {
    let said = |e: seccompiler::BackendError| e.to_string();
    let is = |arg: u8, op: SeccompCmpOp, value: u64| {
        SeccompCondition::new(arg, SeccompCmpArgLen::Dword, op, value).map_err(said)
    };
    let when = |conditions: Vec<SeccompCondition>| SeccompRule::new(conditions).map_err(said);
    let own = u64::from(own);
    // The process argument `arg` names another process: neither 0 (itself) nor its own pid.
    let other = |arg: u8| -> Result<Vec<SeccompRule>, String> {
        Ok(vec![when(vec![
            is(arg, SeccompCmpOp::Ne, 0)?,
            is(arg, SeccompCmpOp::Ne, own)?,
        ])?])
    };
    let always = |rules: &mut Rules, calls: &[i64]| {
        for call in calls {
            rules.insert(*call, Vec::new());
        }
    };
    let mut rules = Rules::new();
    // No socket.
    always(&mut rules, &[libc::SYS_socket, libc::SYS_io_uring_setup]);
    rules.insert(
        libc::SYS_socketpair,
        vec![
            when(vec![is(0, SeccompCmpOp::Ne, libc::AF_UNIX as u64)?])?,
            when(vec![is(
                1,
                SeccompCmpOp::MaskedEq(SOCK_TYPE_MASK),
                libc::SOCK_DGRAM as u64,
            )?])?,
        ],
    );
    // No truncation by path: `truncate`, an open for reading that truncates (`flags` is `open`'s second argument,
    // `openat`'s third), `openat2`.
    let read_and_cut = |arg: u8| -> Result<Vec<SeccompRule>, String> {
        let cut = libc::O_TRUNC as u64;
        Ok(vec![when(vec![is(
            arg,
            SeccompCmpOp::MaskedEq(ACCESS_MODE | cut),
            cut,
        )?])?])
    };
    always(&mut rules, &[libc::SYS_truncate, libc::SYS_openat2]);
    rules.insert(libc::SYS_openat, read_and_cut(2)?);
    #[cfg(target_arch = "x86_64")]
    rules.insert(libc::SYS_open, read_and_cut(1)?);
    // No change to a file's metadata.
    always(
        &mut rules,
        &[
            libc::SYS_fchmod,
            libc::SYS_fchmodat,
            SYS_FCHMODAT2,
            libc::SYS_fchown,
            libc::SYS_fchownat,
            libc::SYS_utimensat,
            libc::SYS_setxattr,
            libc::SYS_lsetxattr,
            libc::SYS_fsetxattr,
            SYS_SETXATTRAT,
            libc::SYS_removexattr,
            libc::SYS_lremovexattr,
            libc::SYS_fremovexattr,
            SYS_REMOVEXATTRAT,
        ],
    );
    #[cfg(target_arch = "x86_64")]
    always(
        &mut rules,
        &[
            libc::SYS_chmod,
            libc::SYS_chown,
            libc::SYS_lchown,
            libc::SYS_utime,
            libc::SYS_utimes,
            libc::SYS_futimesat,
        ],
    );
    // Other processes left alone.
    for call in [
        libc::SYS_prlimit64,
        libc::SYS_sched_setaffinity,
        libc::SYS_sched_setscheduler,
        libc::SYS_sched_setparam,
        libc::SYS_sched_setattr,
    ] {
        rules.insert(call, other(0)?);
    }
    for (call, one_process) in [
        (libc::SYS_setpriority, PRIO_PROCESS),
        (libc::SYS_ioprio_set, IOPRIO_WHO_PROCESS),
    ] {
        let mut refused = other(1)?;
        refused.push(when(vec![is(0, SeccompCmpOp::Ne, one_process)?])?);
        rules.insert(call, refused);
    }
    if signals {
        rules.insert(libc::SYS_kill, other(0)?);
        for call in [
            libc::SYS_tkill,
            libc::SYS_tgkill,
            libc::SYS_rt_sigqueueinfo,
            libc::SYS_rt_tgsigqueueinfo,
        ] {
            rules.insert(call, vec![when(vec![is(0, SeccompCmpOp::Ne, own)?])?]);
        }
        always(
            &mut rules,
            &[libc::SYS_pidfd_open, libc::SYS_pidfd_send_signal],
        );
    }
    let filter = SeccompFilter::new(
        rules,
        SeccompAction::Allow,
        SeccompAction::Errno(libc::EACCES as u32),
        ARCH,
    )
    .map_err(said)?;
    #[allow(unused_mut)]
    let mut program = BpfProgram::try_from(filter).map_err(said)?;
    #[cfg(target_arch = "x86_64")]
    refuse_x32(&mut program);
    Ok(program)
}

/// The instructions right after the architecture's test (three: load, compare, end the process), where the x32 guard
/// goes.
#[cfg(target_arch = "x86_64")]
const AFTER_ARCH: usize = 3;

/// x32 calls refused, all of them: `nr >= 0x40000000`. Put right after the architecture's test and before
/// seccompiler's own rules, which load the number again; three instructions whose own jump stays within them, and the
/// rules' jumps are relative, so nothing else moves.
#[cfg(target_arch = "x86_64")]
fn refuse_x32(program: &mut BpfProgram) {
    use seccompiler::sock_filter;
    // BPF_LD | BPF_W | BPF_ABS, BPF_JMP | BPF_JGE | BPF_K, BPF_RET | BPF_K (linux/bpf_common.h).
    const LOAD_WORD: u16 = 0x20;
    const JUMP_IF_AT_LEAST: u16 = 0x35;
    const RETURN: u16 = 0x06;
    let guard = [
        // seccomp_data.nr, at offset 0.
        sock_filter {
            code: LOAD_WORD,
            jt: 0,
            jf: 0,
            k: 0,
        },
        sock_filter {
            code: JUMP_IF_AT_LEAST,
            jt: 0,
            jf: 1,
            k: 0x4000_0000,
        },
        sock_filter {
            code: RETURN,
            jt: 0,
            jf: 0,
            k: libc::SECCOMP_RET_ERRNO | libc::EACCES as u32,
        },
    ];
    program.splice(AFTER_ARCH..AFTER_ARCH, guard);
}

/// Whether this kernel takes the filter: it has seccomp's filters with the actions the filter returns (the file
/// exists from Linux 4.14), and the filter compiles. Asked without installing anything.
pub fn available() -> Result<(), String> {
    let actions =
        std::fs::read_to_string("/proc/sys/kernel/seccomp/actions_avail").map_err(|e| {
            format!("çekirdekte seccomp süzgeci yok ya da söylenmiyor (actions_avail: {e})")
        })?;
    for needed in ["allow", "errno", "kill_process"] {
        if !actions.split_whitespace().any(|a| a == needed) {
            return Err(format!(
                "çekirdeğin seccomp süzgecinde “{needed}” eylemi yok"
            ));
        }
    }
    program(std::process::id(), true).map(|_| ())
}

/// Installs the filter on the calling thread (the box mode's only one), irreversibly; inherited across exec. Sets
/// `no_new_privs`, as Landlock already has. `signals`: the kill family's rules too (Landlock below ABI 6).
pub fn install(signals: bool) -> Result<(), String> {
    let program = program(std::process::id(), signals)?;
    seccompiler::apply_filter(&program).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The x32 guard sits right after the architecture's test, whose last instruction ends the process.
    #[cfg(target_arch = "x86_64")]
    #[test]
    fn the_x32_guard_follows_the_architecture_s_test() {
        let program = program(4242, true).unwrap();
        assert_eq!(program[AFTER_ARCH - 1].k, libc::SECCOMP_RET_KILL_PROCESS);
        assert_eq!(
            (program[AFTER_ARCH + 1].code, program[AFTER_ARCH + 1].k),
            (0x35, 0x4000_0000)
        );
        assert_eq!(
            program[AFTER_ARCH + 2].k,
            libc::SECCOMP_RET_ERRNO | libc::EACCES as u32
        );
    }

    /// The signal rules make the filter longer only where they are asked for.
    #[test]
    fn the_kill_family_s_rules_come_only_below_landlock_abi_6() {
        assert!(program(4242, true).unwrap().len() > program(4242, false).unwrap().len());
    }
}

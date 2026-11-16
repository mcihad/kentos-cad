//! The box's seccomp filter (docs/adr/8057 §63): no socket. Landlock closes TCP but not UDP or the other socket kinds,
//! and pathname UNIX sockets only from ABI 9; the filter closes them all where they are made.
//!
//! - `socket(2)`: refused for every family.
//! - `socketpair(2)`: refused but for a UNIX pair of a stream or sequenced packets, which stays inside the box
//!   (asyncio's self-pipe). A UNIX datagram pair is refused too: it can send to any pathname socket by its name.
//! - `io_uring_setup(2)`: refused; a ring can open sockets itself (`IORING_OP_SOCKET`).
//! - On x86_64 the same three calls by their x32 numbers, which a kernel built with the x32 ABI serves.
//! - Everything else is allowed. A call of another architecture (32-bit compatibility) ends the process: seccompiler's
//!   first test. Refused calls fail with `EACCES`, as Landlock's refusals do (Python's `PermissionError`).
//!
//! The numbers are the target's own (`libc`): x86_64's 41, 53 and 425; aarch64's 198, 199 and 425.

use std::collections::BTreeMap;

use seccompiler::{
    BpfProgram, SeccompAction, SeccompCmpArgLen, SeccompCmpOp, SeccompCondition, SeccompFilter,
    SeccompRule, TargetArch,
};

#[cfg(target_arch = "x86_64")]
const ARCH: TargetArch = TargetArch::x86_64;
#[cfg(target_arch = "aarch64")]
const ARCH: TargetArch = TargetArch::aarch64;

/// The bit x86_64's x32 calls carry on their numbers (`__X32_SYSCALL_BIT`).
#[cfg(target_arch = "x86_64")]
const X32: i64 = 0x4000_0000;

/// A socket's type without its flags (`SOCK_NONBLOCK`, `SOCK_CLOEXEC`): `SOCK_TYPE_MASK`.
const SOCK_TYPE_MASK: u64 = 0xf;

/// The filter, compiled for this architecture.
fn program() -> Result<BpfProgram, String> {
    let said = |e: seccompiler::BackendError| e.to_string();
    let condition = |arg, op, value: i32| {
        SeccompCondition::new(arg, SeccompCmpArgLen::Dword, op, value as u64).map_err(said)
    };
    // A pair is refused when it is not UNIX, or when it is a UNIX datagram pair.
    let pair = vec![
        SeccompRule::new(vec![condition(0, SeccompCmpOp::Ne, libc::AF_UNIX)?]).map_err(said)?,
        SeccompRule::new(vec![condition(
            1,
            SeccompCmpOp::MaskedEq(SOCK_TYPE_MASK),
            libc::SOCK_DGRAM,
        )?])
        .map_err(said)?,
    ];
    let mut rules: BTreeMap<i64, Vec<SeccompRule>> = BTreeMap::new();
    rules.insert(libc::SYS_socket, Vec::new());
    rules.insert(libc::SYS_socketpair, pair.clone());
    rules.insert(libc::SYS_io_uring_setup, Vec::new());
    #[cfg(target_arch = "x86_64")]
    {
        rules.insert(libc::SYS_socket | X32, Vec::new());
        rules.insert(libc::SYS_socketpair | X32, pair);
        rules.insert(libc::SYS_io_uring_setup | X32, Vec::new());
    }
    let filter = SeccompFilter::new(
        rules,
        SeccompAction::Allow,
        SeccompAction::Errno(libc::EACCES as u32),
        ARCH,
    )
    .map_err(said)?;
    BpfProgram::try_from(filter).map_err(said)
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
    program().map(|_| ())
}

/// Installs the filter on the calling thread (the box mode's only one), irreversibly; inherited across exec. Sets
/// `no_new_privs`, as Landlock already has.
pub fn install() -> Result<(), String> {
    let program = program()?;
    seccompiler::apply_filter(&program).map_err(|e| e.to_string())
}

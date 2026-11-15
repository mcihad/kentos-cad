//! The job's whole process tree (docs/adr/8057 §58): the child and what it starts (an audit rule's Python, which
//! `kentos-jobrun` runs as a child of its own) measured together against the job's budget and ended together.
//!
//! - **Measured:** where the system has `/proc`, every process under the child (each task's `children`, else every
//!   process's parent): their CPU seconds with their waited-for children's (a rule's process ended and reaped by the job
//!   runner is in its CPU time) and the sum of their resident memory; elsewhere the same from one `ps` of every process.
//! - **Ended:** the child is started in a process group of its own (Unix), and the group is ended while the child is
//!   still unreaped, so the group's id is nobody else's: past the budget, at a stop, and (Linux) when the child ends by
//!   itself leaving a process behind.
//! - **Windows:** no `/proc` and no `ps`: nothing is measured (as before this change) and only the child is ended.

use std::process::{Child, Command, ExitStatus, Stdio};
use std::sync::OnceLock;

use super::{Taken, parse_ps_time, parse_vmrss};

/// A line of `/proc/<pid>/stat`: its state, its parent and its CPU seconds, its own and its waited-for children's.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ProcLine {
    pub state: char,
    pub parent: u32,
    pub own: f64,
    pub reaped: f64,
}

/// A `/proc/<pid>/stat` line read after the command's closing parenthesis (a command may hold spaces): the state is
/// the 3rd field, the parent the 4th, utime, stime, cutime and cstime the 14th to the 17th, in clock ticks (`hz`).
pub fn parse_proc_line(stat: &str, hz: f64) -> Option<ProcLine> {
    let rest = stat.get(stat.rfind(')')? + 1..)?;
    let fields: Vec<&str> = rest.split_whitespace().collect();
    let tick = |i: usize| fields.get(i).and_then(|v| v.parse::<f64>().ok());
    Some(ProcLine {
        state: fields.first()?.chars().next()?,
        parent: fields.get(1)?.parse().ok()?,
        own: (tick(11)? + tick(12)?) / hz,
        reaped: (tick(13)? + tick(14)?) / hz,
    })
}

/// Whether the kernel keeps each task's `children` (`/proc/<pid>/task/<tid>/children`).
fn children_kept() -> bool {
    static KEPT: OnceLock<bool> = OnceLock::new();
    *KEPT.get_or_init(|| {
        let me = std::process::id();
        std::path::Path::new(&format!("/proc/{me}/task/{me}/children")).exists()
    })
}

fn children_of(pid: u32) -> Vec<u32> {
    let Ok(tasks) = std::fs::read_dir(format!("/proc/{pid}/task")) else {
        return Vec::new();
    };
    tasks
        .flatten()
        .filter_map(|task| std::fs::read_to_string(task.path().join("children")).ok())
        .flat_map(|text| {
            text.split_whitespace()
                .filter_map(|w| w.parse::<u32>().ok())
                .collect::<Vec<_>>()
        })
        .collect()
}

/// Every process and its parent, from `/proc/<pid>/stat`.
fn parents() -> Vec<(u32, u32)> {
    let Ok(entries) = std::fs::read_dir("/proc") else {
        return Vec::new();
    };
    entries
        .flatten()
        .filter_map(|e| {
            let pid: u32 = e.file_name().to_str()?.parse().ok()?;
            let stat = std::fs::read_to_string(e.path().join("stat")).ok()?;
            Some((pid, parse_proc_line(&stat, 100.0)?.parent))
        })
        .collect()
}

/// The processes under `root`, `root` first.
fn tree_of(root: u32) -> Vec<u32> {
    let all = (!children_kept()).then(parents);
    let mut tree = vec![root];
    let mut i = 0;
    while i < tree.len() {
        let pid = tree[i];
        let kids = match &all {
            None => children_of(pid),
            Some(all) => all
                .iter()
                .filter(|(_, p)| *p == pid)
                .map(|(c, _)| *c)
                .collect(),
        };
        for kid in kids {
            if !tree.contains(&kid) {
                tree.push(kid);
            }
        }
        i += 1;
    }
    tree
}

/// What the tree under `root` takes, from `/proc`; none when `root` cannot be read.
fn measure_proc(root: u32) -> Option<Taken> {
    let mut taken = Taken::default();
    let mut read = false;
    for pid in tree_of(root) {
        let Ok(stat) = std::fs::read_to_string(format!("/proc/{pid}/stat")) else {
            continue;
        };
        let Some(line) = parse_proc_line(&stat, 100.0) else {
            continue;
        };
        read |= pid == root;
        taken.cpu_seconds += line.own + line.reaped;
        // A process that ended and waits to be reaped has no resident memory line.
        if let Ok(status) = std::fs::read_to_string(format!("/proc/{pid}/status")) {
            taken.rss_bytes += parse_vmrss(&status).unwrap_or(0);
        }
    }
    read.then_some(taken)
}

/// A process as one `ps -A -S -o pid=,ppid=,rss=,time=` row says: its id, its parent, its resident kilobytes and its
/// CPU seconds with its ended children's.
pub type PsRow = (u32, u32, u64, f64);

pub fn parse_ps_rows(text: &str) -> Vec<PsRow> {
    text.lines()
        .filter_map(|line| {
            let mut w = line.split_whitespace();
            Some((
                w.next()?.parse().ok()?,
                w.next()?.parse().ok()?,
                w.next()?.parse().ok()?,
                parse_ps_time(w.next()?)?,
            ))
        })
        .collect()
}

/// What the tree under `root` takes, from every process's row; none when `root` has none.
pub fn tree_taken(rows: &[PsRow], root: u32) -> Option<Taken> {
    if !rows.iter().any(|r| r.0 == root) {
        return None;
    }
    let mut tree = vec![root];
    let mut i = 0;
    while i < tree.len() {
        let pid = tree[i];
        for row in rows {
            if row.1 == pid && !tree.contains(&row.0) {
                tree.push(row.0);
            }
        }
        i += 1;
    }
    let mut taken = Taken::default();
    for row in rows.iter().filter(|r| tree.contains(&r.0)) {
        taken.cpu_seconds += row.3;
        taken.rss_bytes += row.2 * 1024;
    }
    Some(taken)
}

fn measure_ps(root: u32) -> Option<Taken> {
    let out = Command::new("ps")
        .args(["-A", "-S", "-o", "pid=,ppid=,rss=,time="])
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .ok()?;
    tree_taken(&parse_ps_rows(&String::from_utf8_lossy(&out.stdout)), root)
}

/// What the child `pid` and every process under it take now; none when it is gone or cannot be read.
pub fn measure(pid: u32) -> Option<Taken> {
    if std::path::Path::new("/proc/self/stat").exists() {
        measure_proc(pid)
    } else {
        measure_ps(pid)
    }
}

/// `command` started in a process group of its own (Unix): ending the group ends whatever the child started.
pub fn own_group(command: &mut Command) {
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        command.process_group(0);
    }
    #[cfg(not(unix))]
    let _ = command;
}

/// Kills the process group `leader` leads.
#[cfg(unix)]
fn kill_group(leader: u32) {
    let Ok(id) = i32::try_from(leader) else {
        return;
    };
    // SAFETY: kill(2) with a negative id signals the group `leader` leads; the group is the child's own (`own_group`)
    // and the child is not reaped yet, so its id is nobody else's.
    unsafe {
        libc::kill(-id, libc::SIGKILL);
    }
}

/// Ends the child and its group (the rule's Python with it), then reaps the child.
pub fn end(child: &mut Child) {
    #[cfg(unix)]
    kill_group(child.id());
    let _ = child.kill();
    let _ = child.wait();
}

/// Whether the child `pid` has ended, leaving it unreaped (`WNOWAIT`): its id, its group's, stays its own.
#[cfg(target_os = "linux")]
fn ended_unreaped(pid: u32) -> bool {
    let id = libc::id_t::from(pid);
    // SAFETY: a zeroed siginfo_t is a valid place for waitid(2) to write to; WNOWAIT leaves the child to `Child::wait`.
    unsafe {
        let mut info: libc::siginfo_t = std::mem::zeroed();
        libc::waitid(
            libc::P_PID,
            id,
            &mut info,
            libc::WEXITED | libc::WNOHANG | libc::WNOWAIT,
        ) == 0
            && info.si_pid() != 0
    }
}

/// The child's status once it has ended, none while it runs. On Linux its group is ended first, while it is unreaped:
/// a process it left behind (a rule's Python) does not outlive it. Elsewhere it is reaped as it is.
pub fn ended(child: &mut Child) -> std::io::Result<Option<ExitStatus>> {
    #[cfg(target_os = "linux")]
    {
        if !ended_unreaped(child.id()) {
            return Ok(None);
        }
        kill_group(child.id());
        child.wait().map(Some)
    }
    #[cfg(not(target_os = "linux"))]
    {
        child.try_wait()
    }
}

//! What the box closes on this system and how the rule's process starts (docs/adr/8057 §61, §63, §65): Landlock's
//! ABI, whether the filter can be installed, and the server's policy where the box cannot be whole; the words that say
//! what is closed and what is open, for the job runner's error output and the job's summary.

use super::POLICY_ENV;

/// What the server does where the box cannot be whole.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Policy {
    /// The rule does not run (the default: the cloud box's promise holds or nothing runs).
    Require,
    /// The rule runs with what closes and the job says what is open.
    Report,
}

/// The server's choice, from its setting; anything but `report` requires the box.
pub fn policy() -> Policy {
    match std::env::var(POLICY_ENV).as_deref() {
        Ok("report") => Policy::Report,
        _ => Policy::Require,
    }
}

/// What Landlock closes here: the newest of its ABIs whose additions the box uses (1, 2, 3, 4, 5, 6 or 9; 7 and 8 add
/// nothing it uses).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Landlock {
    pub abi: u8,
}

impl Landlock {
    fn truncate(self) -> bool {
        self.abi >= 3
    }

    fn tcp(self) -> bool {
        self.abi >= 4
    }

    /// Signals and abstract UNIX sockets kept inside the domain.
    pub(crate) fn scopes(self) -> bool {
        self.abi >= 6
    }

    fn unix_paths(self) -> bool {
        self.abi >= 9
    }
}

/// How the rule's process starts.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Plan {
    /// In the whole box, Landlock and the filter: what it closes, in words for the job runner's error output; `abi`, a
    /// Landlock ABI lower than the kernel's to build it with (the tests only; a job's is `None`).
    Whole { words: String, abi: Option<u8> },
    /// `report` where the filter cannot be installed: in Landlock alone; why, and what is open.
    LandlockOnly(String),
    /// `report` where Landlock cannot close: outside the box; why.
    Outside(String),
}

impl Plan {
    /// What the job's summary adds: nothing for the whole box, what is open for any other.
    pub fn note(&self) -> Option<String> {
        match self {
            Plan::Whole { .. } => None,
            Plan::LandlockOnly(open) => Some(format!(" (kutu tam kapatılamadı: {open})")),
            Plan::Outside(why) => Some(format!(" (kutu kapatılamadı: {why})")),
        }
    }
}

/// The plan from what this system closes and the server's policy, or why the rule may not run. A missing filter
/// counts as a box that cannot close, as missing Landlock does: without it UDP and the other socket kinds are open,
/// and the box's promise is no network. Under `report` the rule keeps what does close.
pub fn plan(
    landlock: Result<Landlock, String>,
    filter: Result<(), String>,
    policy: Policy,
) -> Result<Plan, String> {
    let why = match (&landlock, &filter) {
        (Ok(landlock), Ok(())) => {
            return Ok(Plan::Whole {
                words: whole(*landlock),
                abi: None,
            });
        }
        (Ok(_), Err(filter)) => filter.clone(),
        (Err(landlock), Ok(())) => landlock.clone(),
        (Err(landlock), Err(filter)) => format!("{landlock}; {filter}"),
    };
    match (policy, landlock) {
        (Policy::Require, _) => Err(why),
        (Policy::Report, Ok(landlock)) => {
            Ok(Plan::LandlockOnly(format!("{why}; {}", alone(landlock))))
        }
        (Policy::Report, Err(_)) => Ok(Plan::Outside(why)),
    }
}

/// This system's plan under the server's setting.
pub fn here() -> Result<Plan, String> {
    plan(landlock(), filter(), policy())
}

/// What the whole box closes, in words: the same at every Landlock ABI, Landlock or the filter closing it.
fn whole(landlock: Landlock) -> String {
    let signals = if landlock.scopes() {
        "sinyaller Landlock'la"
    } else {
        "sinyaller süzgeçle (Landlock ABI 6 yok)"
    };
    format!(
        "dosyalar kapalı: içerik, yol adıyla kısaltma, izin, sahip, zaman, xattr; ağ kapalı (soket açılamaz): TCP, \
         UDP, yol adlı ve soyut UNIX soketleri, öteki türler, io_uring; yalnız kutunun içinde kalan UNIX akış çifti \
         açılır; öteki süreçler kapalı: {signals}, kaynak sınırı, öncelik ve zamanlama süzgeçle"
    )
}

/// What Landlock alone closes at its ABI, and what stays open, in words.
fn alone(landlock: Landlock) -> String {
    let mut closed = vec!["dosyaların içeriği"];
    let mut open = Vec::new();
    for (has, what, missing) in [
        (
            landlock.truncate(),
            "kısaltma",
            "yol adıyla kısaltma (ABI 3 yok)",
        ),
        (landlock.tcp(), "TCP", "TCP (ABI 4 yok)"),
        (
            landlock.scopes(),
            "sinyaller ve soyut UNIX soketleri",
            "sinyaller ve soyut UNIX soketleri (ABI 6 yok)",
        ),
        (
            landlock.unix_paths(),
            "yol adlı UNIX soketleri",
            "yol adlı UNIX soketleri (ABI 9 yok)",
        ),
    ] {
        if has {
            closed.push(what);
        } else {
            open.push(missing);
        }
    }
    open.extend([
        "UDP ve öteki soket türleri",
        "io_uring",
        "dosyaların izni, sahibi, zamanı ve xattr'ı",
        "öteki süreçlerin kaynak sınırı, önceliği ve zamanlaması",
    ]);
    format!("kapalı: {}; açık: {}", closed.join(", "), open.join(", "))
}

/// What Landlock closes on this system, or why it cannot close.
pub fn landlock() -> Result<Landlock, String> {
    #[cfg(target_os = "linux")]
    {
        super::linux::landlock()
    }
    #[cfg(not(target_os = "linux"))]
    {
        Err("Landlock yalnız Linux'ta var".to_owned())
    }
}

/// Whether the seccomp filter can be installed on this system, or why not: Linux on x86_64 or aarch64, a kernel
/// that has the filter's actions.
pub fn filter() -> Result<(), String> {
    #[cfg(all(
        target_os = "linux",
        any(target_arch = "x86_64", target_arch = "aarch64")
    ))]
    {
        super::seccomp::available()
    }
    #[cfg(not(all(
        target_os = "linux",
        any(target_arch = "x86_64", target_arch = "aarch64")
    )))]
    {
        Err(format!(
            "seccomp süzgeci yalnız Linux'ta x86_64 ve aarch64 için var; bu sistem {} {}",
            std::env::consts::OS,
            std::env::consts::ARCH
        ))
    }
}

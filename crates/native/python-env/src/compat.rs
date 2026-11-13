//! The compatibility manifest (PY-UV-09): one file says the least uv, the uv KentOS's installation carries, the Python
//! versions and the platforms KentOS's Python runs on, and uv's official ways to install; detection, the window and the
//! packaging step (`scripts/desktop/bundle_uv.py`) read it, nothing repeats it. A manifest that cannot be read refuses
//! everything and says why (it is built in, so a test reads it).

use std::collections::BTreeMap;
use std::sync::OnceLock;

use serde::Deserialize;

/// The manifest.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Compat {
    pub format: String,
    pub schema_version: u32,
    pub uv: UvNeed,
    /// The uv the installation carries (the owner's decision, 13 November 2026).
    pub bundled_uv: BundledUv,
    pub python: PythonNeed,
    pub platforms: Vec<Platform>,
    pub install: Install,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct UvNeed {
    /// The least version, `x.y.z`.
    pub min: String,
    pub note: String,
}

/// The uv KentOS's installation carries: its version, where its official release files are, and each target's file
/// with its SHA-256, for the packaging step (`scripts/desktop/bundle_uv.py`) that downloads, checks and places it with
/// its licences.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BundledUv {
    pub version: String,
    pub note: String,
    /// The release's folder, ending with `/`.
    pub download: String,
    /// The licence files, shipped beside uv: its release archives carry none.
    pub licenses: Vec<Pinned>,
    /// By Rust target (`x86_64-pc-windows-msvc` …).
    pub targets: BTreeMap<String, Asset>,
}

/// A release file and its SHA-256 (lower-case hex).
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Asset {
    pub file: String,
    pub sha256: String,
}

/// A file at an address and its SHA-256 (lower-case hex).
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Pinned {
    pub url: String,
    pub sha256: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PythonNeed {
    /// As uv reads a version request (`>=3.10,<3.15`).
    pub range: String,
    pub note: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Platform {
    /// `linux`, `windows`, `macos` (Rust's `std::env::consts::OS`).
    pub os: String,
    /// `x86_64`, `aarch64` (Rust's `std::env::consts::ARCH`).
    pub arch: String,
}

/// uv's official ways in, each system's in the order offered, for the user to read and run; KentOS does not install
/// uv on its own (PY-UV-03…06). On Windows the PowerShell installer comes first and WinGet after it: an organisation's
/// computers often refuse `-ExecutionPolicy ByPass`.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Install {
    pub linux: Vec<String>,
    pub macos: Vec<String>,
    pub windows: Vec<String>,
    /// The page that explains them.
    pub page: String,
}

const FORMAT: &str = "kentos.python-compat";

impl Compat {
    /// Reads a manifest.
    pub fn read(text: &str) -> Result<Compat, String> {
        let compat: Compat = serde_json::from_str(text).map_err(|e| e.to_string())?;
        if compat.format != FORMAT || compat.schema_version != 1 {
            return Err(format!(
                "“{}” sürüm {}; bu KentOS “{FORMAT}” sürüm 1'i okur.",
                compat.format, compat.schema_version
            ));
        }
        if triple(&compat.uv.min).is_none() {
            return Err(format!(
                "uv'nin en düşük sürümü “{}” okunamadı.",
                compat.uv.min
            ));
        }
        Ok(compat)
    }

    /// A manifest that refuses everything, saying why: what a broken built-in manifest becomes.
    fn broken(why: String) -> Compat {
        Compat {
            format: FORMAT.to_owned(),
            schema_version: 1,
            uv: UvNeed {
                min: "999.0.0".to_owned(),
                note: format!("Uyumluluk dosyası okunamadı: {why}"),
            },
            bundled_uv: BundledUv {
                version: String::new(),
                note: String::new(),
                download: String::new(),
                licenses: Vec::new(),
                targets: BTreeMap::new(),
            },
            python: PythonNeed {
                range: "<0".to_owned(),
                note: why,
            },
            platforms: Vec::new(),
            install: Install {
                linux: Vec::new(),
                macos: Vec::new(),
                windows: Vec::new(),
                page: String::new(),
            },
        }
    }

    /// Whether `platform` is one this KentOS's Python runs on.
    pub fn supports(&self, platform: &Platform) -> bool {
        self.platforms.contains(platform)
    }

    /// uv's official ways in on this computer's system.
    pub fn install_commands(&self) -> &[String] {
        self.install_commands_for(std::env::consts::OS)
    }

    /// uv's official ways in on a system (`linux`, `windows`, `macos`).
    pub fn install_commands_for(&self, os: &str) -> &[String] {
        match os {
            "windows" => &self.install.windows,
            "macos" => &self.install.macos,
            _ => &self.install.linux,
        }
    }

    /// Whether a uv version (`x.y.z`) is new enough.
    pub fn uv_new_enough(&self, version: &str) -> bool {
        match (triple(version), triple(&self.uv.min)) {
            (Some(v), Some(min)) => v >= min,
            _ => false,
        }
    }
}

/// The manifest built into this program.
pub fn compat() -> &'static Compat {
    static BUILT_IN: OnceLock<Compat> = OnceLock::new();
    BUILT_IN
        .get_or_init(|| Compat::read(include_str!("../compat.json")).unwrap_or_else(Compat::broken))
}

/// This computer's platform as the manifest names it.
pub fn this_platform() -> Platform {
    Platform {
        os: std::env::consts::OS.to_owned(),
        arch: std::env::consts::ARCH.to_owned(),
    }
}

/// `x.y.z` (a pre-release or build suffix ignored).
pub fn triple(version: &str) -> Option<(u64, u64, u64)> {
    let core = version.split(['-', '+', ' ']).next()?;
    let mut parts = core.split('.').map(|p| p.parse::<u64>().ok());
    let major = parts.next()??;
    let minor = parts.next().unwrap_or(Some(0))?;
    let patch = parts.next().unwrap_or(Some(0))?;
    Some((major, minor, patch))
}

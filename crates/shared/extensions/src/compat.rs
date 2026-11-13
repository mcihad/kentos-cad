//! Whether a package runs on a host (plan/eklenti.md §3.2 "Uyumluluk", docs/adr/8057 §1): one function, used alike by
//! the inspector, the installer, the catalog and the updater.
//!
//! QGIS's 2026 lesson (docs/research/2026-11-13-qgis-eklenti-sistemi.md): a missing upper bound there was read as "the
//! same major version", so every plugin had to be republished for QGIS 4 though most ran unchanged. Here **an upper
//! bound not written is no upper bound**; the meaning is not implicit and changing it takes an ADR. A catalog that
//! learns of a breakage narrows the range from outside the package (a narrower [`Compatibility`] handed to
//! [`compatible`]), so a user does not get an incompatible version though its author has gone.

use std::fmt;

use serde::{Deserialize, Serialize};

use crate::manifest::Compatibility;
use crate::problem::Problem;
use crate::version::Version;

/// The extension protocol this KentOS speaks: the `kentos.ext` Python API, the broker's messages and the manifest's
/// meaning. Raised only by a breaking change; a package names the range it was written for.
pub const SDK_PROTOCOL: u32 = 1;

/// An operating system a package says it runs on.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Os {
    Windows,
    Macos,
    Linux,
}

impl Os {
    pub const ALL: [Os; 3] = [Os::Windows, Os::Macos, Os::Linux];

    /// The system this build runs on; none on another (the browser).
    pub fn current() -> Option<Os> {
        if cfg!(target_os = "windows") {
            Some(Os::Windows)
        } else if cfg!(target_os = "macos") {
            Some(Os::Macos)
        } else if cfg!(target_os = "linux") {
            Some(Os::Linux)
        } else {
            None
        }
    }

    /// Its name in the manifest.
    pub fn id(self) -> &'static str {
        match self {
            Os::Windows => "windows",
            Os::Macos => "macos",
            Os::Linux => "linux",
        }
    }

    /// Python's `sys.platform` on it: what `[tool.uv] required-environments` names it by.
    pub fn sys_platform(self) -> &'static str {
        match self {
            Os::Windows => "win32",
            Os::Macos => "darwin",
            Os::Linux => "linux",
        }
    }
}

impl fmt::Display for Os {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Os::Windows => "Windows",
            Os::Macos => "macOS",
            Os::Linux => "Linux",
        })
    }
}

/// The host a package is to run on.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Host {
    /// The extension protocol it speaks ([`SDK_PROTOCOL`] for this build).
    pub sdk_protocol: u32,
    /// Its KentOS version.
    pub kentos: Version,
    /// Its operating system; none for a host that runs no package code on a system of its own (the catalog's
    /// browser view), which every package's systems then answer.
    pub os: Option<Os>,
}

impl Host {
    /// This build: its protocol, its version and its system.
    pub fn this(kentos: Version) -> Host {
        Host {
            sdk_protocol: SDK_PROTOCOL,
            kentos,
            os: Os::current(),
        }
    }
}

/// Whether a package with this compatibility runs on `host`: the protocol range (both ends inclusive), the KentOS
/// range (`min` inclusive, `maxExclusive` exclusive; an end not written is open) and the systems (none written: every
/// system). The first reason it does not, said for the person who would install it.
pub fn compatible(compatibility: &Compatibility, host: &Host) -> Result<(), Problem> {
    let protocol = &compatibility.sdk_protocol;
    if host.sdk_protocol < protocol.min || host.sdk_protocol > protocol.max {
        let fix = if host.sdk_protocol < protocol.min {
            "KentOS'u güncelleyin"
        } else {
            "eklentinin yeni sürümünü bekleyin ya da yazarına bildirin"
        };
        return Err(Problem::new(
            "compat_sdk_protocol",
            format!(
                "Bu eklenti eklenti arayüzünün {} sürümü için yazılmış; bu KentOS {} sürümünü kullanıyor. {fix}.",
                range_words(protocol.min, protocol.max),
                host.sdk_protocol
            ),
        ));
    }
    if let Some(range) = &compatibility.kentos {
        if let Some(min) = range.min.as_deref().and_then(Version::parse)
            && host.kentos < min
        {
            return Err(Problem::new(
                "compat_kentos_min",
                format!(
                    "Bu eklenti KentOS {min} ya da daha yenisini istiyor; bu KentOS {}. KentOS'u güncelleyin.",
                    host.kentos
                ),
            ));
        }
        if let Some(max) = range.max_exclusive.as_deref().and_then(Version::parse)
            && host.kentos >= max
        {
            return Err(Problem::new(
                "compat_kentos_max",
                format!(
                    "Bu eklenti KentOS {max} öncesi için; bu KentOS {}. Eklentinin yeni sürümünü bekleyin ya da \
                     yazarına bildirin.",
                    host.kentos
                ),
            ));
        }
    }
    if let Some(os) = host.os
        && !compatibility.operating_systems.is_empty()
        && !compatibility.operating_systems.contains(&os)
    {
        let names: Vec<String> = compatibility
            .operating_systems
            .iter()
            .map(Os::to_string)
            .collect();
        return Err(Problem::new(
            "compat_os",
            format!(
                "Bu eklenti yalnız {} için; bu bilgisayar {os}.",
                names.join(", ")
            ),
        ));
    }
    Ok(())
}

fn range_words(min: u32, max: u32) -> String {
    if min == max {
        min.to_string()
    } else {
        format!("{min}–{max}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest::{KentosRange, ProtocolRange};

    fn host(version: &str, os: Option<Os>) -> Host {
        Host {
            sdk_protocol: 1,
            kentos: Version::parse(version).unwrap(),
            os,
        }
    }

    fn range(min: Option<&str>, max: Option<&str>, systems: &[Os]) -> Compatibility {
        Compatibility {
            sdk_protocol: ProtocolRange { min: 1, max: 1 },
            kentos: (min.is_some() || max.is_some()).then(|| KentosRange {
                min: min.map(str::to_owned),
                max_exclusive: max.map(str::to_owned),
            }),
            operating_systems: systems.to_vec(),
        }
    }

    fn code(c: &Compatibility, h: &Host) -> Option<&'static str> {
        compatible(c, h).err().map(|p| p.code)
    }

    #[test]
    fn an_upper_bound_not_written_is_none() {
        let open = range(Some("0.2.0"), None, &[]);
        assert_eq!(
            code(&open, &host("0.2.0", Some(Os::Linux))),
            None,
            "min is inclusive"
        );
        assert_eq!(
            code(&open, &host("9.0.0", Some(Os::Linux))),
            None,
            "no implicit major bound"
        );
        assert_eq!(
            code(&open, &host("0.1.9", Some(Os::Linux))),
            Some("compat_kentos_min")
        );
        let none = range(None, None, &[]);
        assert_eq!(code(&none, &host("0.0.1", None)), None);
    }

    #[test]
    fn the_upper_bound_is_exclusive_and_a_preview_is_below_its_release() {
        let closed = range(Some("0.2.0"), Some("0.4.0"), &[]);
        assert_eq!(code(&closed, &host("0.3.9", None)), None);
        assert_eq!(
            code(&closed, &host("0.4.0", None)),
            Some("compat_kentos_max")
        );
        assert_eq!(
            code(&closed, &host("0.4.0-beta.1", None)),
            None,
            "0.4.0-beta.1 < 0.4.0"
        );
        assert_eq!(
            code(&closed, &host("0.2.0-beta.1", None)),
            Some("compat_kentos_min")
        );
    }

    #[test]
    fn the_protocol_range_is_inclusive_at_both_ends() {
        let mut c = range(None, None, &[]);
        c.sdk_protocol = ProtocolRange { min: 1, max: 2 };
        for protocol in [1, 2] {
            let h = Host {
                sdk_protocol: protocol,
                ..host("1.0.0", None)
            };
            assert_eq!(code(&c, &h), None);
        }
        let h3 = Host {
            sdk_protocol: 3,
            ..host("1.0.0", None)
        };
        let err = compatible(&c, &h3).unwrap_err();
        assert_eq!(err.code, "compat_sdk_protocol");
        assert!(
            err.message.contains("1–2") && err.message.contains("yeni sürümünü"),
            "{err}"
        );
        c.sdk_protocol = ProtocolRange { min: 2, max: 2 };
        let err = compatible(&c, &host("1.0.0", None)).unwrap_err();
        assert!(err.message.contains("KentOS'u güncelleyin"), "{err}");
    }

    #[test]
    fn systems_not_written_are_every_system() {
        let any = range(None, None, &[]);
        let windows = range(None, None, &[Os::Windows]);
        for os in Os::ALL {
            assert_eq!(code(&any, &host("1.0.0", Some(os))), None);
        }
        assert_eq!(code(&windows, &host("1.0.0", Some(Os::Windows))), None);
        let err = compatible(&windows, &host("1.0.0", Some(Os::Linux))).unwrap_err();
        assert_eq!(err.code, "compat_os");
        assert_eq!(
            err.message,
            "Bu eklenti yalnız Windows için; bu bilgisayar Linux."
        );
        assert_eq!(
            code(&windows, &host("1.0.0", None)),
            None,
            "the catalog's view runs nothing"
        );
    }
}

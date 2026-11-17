//! Oturum açılışında başlama (docs/adr/9002 §6): **kullanıcı kapsamında**, sistem kapsamında değil.
//!
//! Servis pencere açamaz; PIN penceresini hep bu süreç açar. Yani `native` kipte imzanın çalışması ajanın açık olmasına
//! bağlıdır: oturum açılışında, penceresi gizli olarak (`--gizli`) başlar ve bir imza isteğinde öne gelir.
//!
//! | | Nereye |
//! | --- | --- |
//! | Linux | `$XDG_CONFIG_HOME/autostart/kentos-cad-imza.desktop` |
//! | macOS | `~/Library/LaunchAgents/tr.kentos.cad.imza.plist` |
//! | Windows | `HKCU\Software\Microsoft\Windows\CurrentVersion\Run`, değer `KentOS CAD İmza` |
//!
//! Adlar bu kuruluma özgüdür: aynı makinedeki başka bir e-imza kurulumunun girdisini ezmez.

use std::io;

/// Girdiyi kurar; yerini döndürür.
pub(crate) fn kur() -> io::Result<String> {
    let ikili = std::env::current_exe()?;
    platform::kur(&ikili)
}

/// Girdiyi kaldırır; yerini döndürür.
pub(crate) fn kaldir() -> io::Result<String> {
    platform::kaldir()
}

#[must_use]
pub(crate) fn kurulu() -> bool {
    platform::kurulu()
}

#[cfg(all(unix, not(target_os = "macos")))]
mod platform {
    use std::{io, path::Path, path::PathBuf};

    fn hedef() -> io::Result<PathBuf> {
        let kok = match std::env::var_os("XDG_CONFIG_HOME") {
            Some(x) if !x.is_empty() => PathBuf::from(x),
            _ => {
                let ev = std::env::var_os("HOME")
                    .ok_or_else(|| io::Error::other("HOME tanımlı değil"))?;
                Path::new(&ev).join(".config")
            }
        };
        Ok(kok.join("autostart").join("kentos-cad-imza.desktop"))
    }

    pub(super) fn icerik(ikili: &Path) -> String {
        format!(
            "[Desktop Entry]\n\
             Type=Application\n\
             Name=KentOS İmza\n\
             Comment=E-imza penceresi: imza isteği geldiğinde öne gelir\n\
             Exec=\"{}\" --gizli\n\
             Terminal=false\n\
             Hidden=false\n\
             X-GNOME-Autostart-enabled=true\n",
            ikili.display()
        )
    }

    pub(super) fn kur(ikili: &Path) -> io::Result<String> {
        let p = hedef()?;
        if let Some(d) = p.parent() {
            std::fs::create_dir_all(d)?;
        }
        std::fs::write(&p, icerik(ikili))?;
        Ok(p.display().to_string())
    }

    pub(super) fn kaldir() -> io::Result<String> {
        let p = hedef()?;
        if p.exists() {
            std::fs::remove_file(&p)?;
        }
        Ok(p.display().to_string())
    }

    pub(super) fn kurulu() -> bool {
        hedef().is_ok_and(|p| p.exists())
    }
}

#[cfg(target_os = "macos")]
mod platform {
    use std::{io, path::Path, path::PathBuf};

    fn hedef() -> io::Result<PathBuf> {
        let ev = std::env::var_os("HOME").ok_or_else(|| io::Error::other("HOME tanımlı değil"))?;
        Ok(Path::new(&ev)
            .join("Library/LaunchAgents")
            .join("tr.kentos.cad.imza.plist"))
    }

    pub(super) fn kur(ikili: &Path) -> io::Result<String> {
        let p = hedef()?;
        if let Some(d) = p.parent() {
            std::fs::create_dir_all(d)?;
        }
        let icerik = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key><string>tr.kentos.cad.imza</string>
    <key>ProgramArguments</key><array><string>{}</string><string>--gizli</string></array>
    <key>RunAtLoad</key><true/>
    <key>ProcessType</key><string>Interactive</string>
</dict>
</plist>
"#,
            ikili.display()
        );
        std::fs::write(&p, icerik)?;
        Ok(p.display().to_string())
    }

    pub(super) fn kaldir() -> io::Result<String> {
        let p = hedef()?;
        if p.exists() {
            std::fs::remove_file(&p)?;
        }
        Ok(p.display().to_string())
    }

    pub(super) fn kurulu() -> bool {
        hedef().is_ok_and(|p| p.exists())
    }
}

#[cfg(windows)]
mod platform {
    use std::os::windows::process::CommandExt as _;
    use std::{io, path::Path};

    const ANAHTAR: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
    const AD: &str = "KentOS CAD İmza";
    /// `CREATE_NO_WINDOW`: `reg` bir konsol programıdır; bayraksız çağrılınca ekranda bir an konsol penceresi açılır.
    const PENCERESIZ: u32 = 0x0800_0000;

    fn reg(args: &[&str]) -> io::Result<std::process::Output> {
        std::process::Command::new("reg")
            .args(args)
            .creation_flags(PENCERESIZ)
            .output()
    }

    pub(super) fn kur(ikili: &Path) -> io::Result<String> {
        let deger = format!("\"{}\" --gizli", ikili.display());
        let anahtar = format!("HKCU\\{ANAHTAR}");
        let cikti = reg(&[
            "add", &anahtar, "/v", AD, "/t", "REG_SZ", "/d", &deger, "/f",
        ])?;
        if !cikti.status.success() {
            return Err(io::Error::other(
                String::from_utf8_lossy(&cikti.stderr).to_string(),
            ));
        }
        Ok(format!("{anahtar}\\{AD}"))
    }

    pub(super) fn kaldir() -> io::Result<String> {
        let anahtar = format!("HKCU\\{ANAHTAR}");
        // Değer zaten yoksa `reg delete` başarısız döner ve bu bir hata değildir.
        let _ = reg(&["delete", &anahtar, "/v", AD, "/f"])?;
        Ok(format!("{anahtar}\\{AD}"))
    }

    pub(super) fn kurulu() -> bool {
        reg(&["query", &format!("HKCU\\{ANAHTAR}"), "/v", AD]).is_ok_and(|o| o.status.success())
    }
}

#[cfg(test)]
mod tests {
    #[test]
    #[cfg(all(unix, not(target_os = "macos")))]
    fn linux_girdisi_gizli_baslar() {
        let i = super::platform::icerik(std::path::Path::new("/opt/kentos/bin/kentos-imza-agent"));
        assert!(i.contains("Type=Application"));
        assert!(i.contains("Exec=\"/opt/kentos/bin/kentos-imza-agent\" --gizli"));
        assert!(i.contains("Terminal=false"));
    }
}

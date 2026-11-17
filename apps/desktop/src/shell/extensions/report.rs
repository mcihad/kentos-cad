//! An extension's debugging report (plan/eklenti.md ADIM 21 › API ailesi, ECO-API-14: "hata ayıklama raporu sır
//! içermeden ortam özeti, kaynak sürümleri, şemalar ve hata kodunu toplar"; docs/adr/8057 §110), in Markdown: what the
//! extension is and where it came from, its package's digest and the schemas it was written for, the computer's
//! environment (KentOS, the system, Python and uv, the extension's own environment and requirements, its capabilities),
//! the problem still standing with its code's meaning, and its newest log. Copied for its publisher by Raporu kopyala,
//! read by an agent with `extension.report`.
//!
//! **No secret goes out:** every key the extension's messages or its errors wrote is hidden
//! ([`kentos_extensions::secrets::masked`]) and the user's home folder is written `~`: a traceback carries paths and,
//! sometimes, the key a request was made with.

use super::problems::{REPORTED, head};
use crate::app::App;
use crate::python_setup::OwnPython;

impl App {
    /// The report the manager copies for the extension's publisher or an agent, in Markdown; none for one not
    /// installed.
    pub(crate) fn extension_report(&self, id: &str) -> Option<String> {
        let registry = self.extensions.registry.as_ref()?;
        let e = registry.extensions().into_iter().find(|e| e.id == id)?;
        let manifest = registry.manifest(id);
        let mut out = format!("# Eklenti raporu: {} {}\n\n", e.name, e.version);
        out.push_str(&format!("- Kimlik: `{}`\n", e.id));
        out.push_str(&format!("- Durum: {} — {}\n", e.state_label, e.help));
        if let Some(m) = manifest {
            let publisher = m
                .publisher_id
                .as_ref()
                .map(|p| format!(" ({p})"))
                .unwrap_or_default();
            out.push_str(&format!("- Yayımcı: {}{publisher}\n", m.authors.join(", ")));
        }
        out.push_str(&format!(
            "- Kaynak: {} — {}\n",
            e.source.kind.words(),
            e.source.location
        ));
        out.push_str(&format!("- Güven: {}\n", e.trust_label));
        out.push_str(&format!(
            "- Bu bilgisayardaki sürümler: {}\n",
            e.versions.join(", ")
        ));
        // The sources' versions and the schemas it was written for.
        if let Some(digest) = registry.digest(id) {
            out.push_str(&format!("- Paket özeti: `{digest}`\n"));
        }
        if let Some(m) = manifest {
            let range = &m.compatibility.sdk_protocol;
            let kentos = m
                .compatibility
                .kentos
                .as_ref()
                .map_or_else(String::new, |k| {
                    format!(
                        "; KentOS {}–{}",
                        k.min.as_deref().unwrap_or("…"),
                        k.max_exclusive
                            .as_deref()
                            .map_or_else(|| "…".to_owned(), |v| format!("{v} (hariç)"))
                    )
                });
            out.push_str(&format!(
                "- Manifest: `{}` şema {}; eklenti protokolü {}–{} (bu KentOS'unki {}){kentos}\n",
                m.format,
                m.schema_version,
                range.min,
                range.max,
                kentos_extensions::SDK_PROTOCOL
            ));
            if let Some(schema) = &m.schema_file {
                out.push_str(&format!("- Katman şeması: `{schema}`\n"));
            }
        }
        out.push_str(&format!(
            "- Günlük dosyası: `{}`\n",
            registry.journal().file(Some(id)).display()
        ));
        out.push_str("\n## Ortam\n\n");
        let program = self.program.map_or("KentOS", |m| m.title());
        out.push_str(&format!(
            "- {program} {} · {} {}\n",
            env!("CARGO_PKG_VERSION"),
            std::env::consts::OS,
            std::env::consts::ARCH
        ));
        let setup = &self.python_setup;
        match &setup.python {
            Some(OwnPython::Ready(python, _)) => {
                out.push_str(&format!("- KentOS'un Python'u: {}\n", python.version));
            }
            _ => out.push_str(&format!(
                "- KentOS'un Python'u: hazır değil — {}\n",
                setup.why_not()
            )),
        }
        out.push_str(&format!("- uv: {}\n", uv_words(setup)));
        if let Some(m) = manifest
            && let Some(project) = &m.python
        {
            let requirements = registry
                .package_file(id, &project.project_file)
                .and_then(|bytes| String::from_utf8(bytes).ok())
                .map(|text| kentos_extensions::declared_requirements(&text))
                .unwrap_or_default();
            out.push_str(&format!(
                "- Python gereksinimleri: {}\n",
                if requirements.is_empty() {
                    "yok".to_owned()
                } else {
                    requirements.join(", ")
                }
            ));
            out.push_str(&format!(
                "- Eklentinin Python ortamı: {}\n",
                self.python_reason(id).unwrap_or_else(|| "hazır".to_owned())
            ));
        } else {
            out.push_str("- Python kullanmıyor.\n");
        }
        if let Some(m) = manifest {
            let grant = registry.grant(id);
            let missing: Vec<&str> = m
                .requested_capabilities
                .iter()
                .filter(|c| !grant.is_some_and(|g| g.capabilities.contains(&c.id)))
                .map(|c| c.id.title())
                .collect();
            let asked = m.requested_capabilities.len();
            out.push_str(&format!(
                "- Yetenekler: {} istendi, {} verildi{}\n",
                asked,
                asked - missing.len(),
                if missing.is_empty() {
                    String::new()
                } else {
                    format!(" (verilmeyenler: {})", missing.join(", "))
                }
            ));
        }
        out.push_str("\n## Sorun\n\n");
        match &e.problem {
            Some(p) => {
                out.push_str(&format!("{}\n\n{}\n", head(p), p.message));
                if let Some(meaning) = kentos_commands::codes::info(&p.code) {
                    out.push_str(&format!("\nKod `{}`: {}\n", p.code, meaning.meaning));
                }
                if let Some(detail) = &p.detail {
                    out.push_str(&format!("\n```\n{detail}\n```\n"));
                }
            }
            None => out.push_str("Şu an duran bir hata yok.\n"),
        }
        out.push_str("\n## Son kayıtlar (yeniden eskiye)\n\n");
        for r in e.log.iter().rev().take(REPORTED) {
            out.push_str(&format!("- {} — {}\n", head(r), r.message));
            if let Some(detail) = &r.detail {
                for line in detail.lines().take(8) {
                    out.push_str(&format!("  > {line}\n"));
                }
            }
        }
        Some(without_secrets(&out))
    }
}

/// uv as the report says it: its version and whether KentOS carries it, else why not.
fn uv_words(setup: &crate::python_setup::PythonSetup) -> String {
    use kentos_python_env::uv::UvStatus;
    match &setup.uv {
        Some(UvStatus::Found {
            version, bundled, ..
        }) => format!(
            "{version}{}",
            if *bundled { " (KentOS'la gelen)" } else { "" }
        ),
        Some(UvStatus::TooOld { version, min, .. }) => {
            format!("{version} — eski, en az {min} gerekiyor")
        }
        Some(_) => "bulunamadı ya da çalışmıyor".to_owned(),
        None => "henüz aranmadı".to_owned(),
    }
}

/// `text` with every key hidden and the user's home folder written `~`.
pub(crate) fn without_secrets(text: &str) -> String {
    let mut out = kentos_extensions::secrets::masked(text);
    if let Some(home) = std::env::home_dir()
        && let Some(home) = home.to_str()
        && home.len() > 1
    {
        out = out.replace(home, "~");
    }
    out
}

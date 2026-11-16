//! What a run of an extension's İşlemler tool tells its host (docs/adr/8057 §7, §73): that it started and how it
//! ended. The desktop's window writes the extension's log from it, its failure notice with Ayrıntı and a crash for the
//! watchdog (tools.rs); a headless host writes the same log record, so that Ayrıntı shows a headless run too, and says
//! it where its caller looks: MCP on its standard error, the Python library through Python's logging.

use kentos_extension_host::Record;

/// What a run tells its host.
#[derive(Clone, Debug, PartialEq)]
pub enum Note {
    Started {
        ext: String,
        part: String,
    },
    Worked {
        ext: String,
        part: String,
        code: &'static str,
        message: String,
    },
    Failed {
        ext: String,
        part: String,
        title: String,
        code: &'static str,
        message: String,
        detail: Option<String>,
        /// Its process ended by itself: a crash the watchdog counts.
        crash: bool,
    },
}

impl Note {
    /// The record the extension's log keeps of it, as the desktop's window writes it: a start and an ending as
    /// information, a failure as an error with its detail (the traceback and what the code printed). `version`, the
    /// extension's version that ran; none takes the registry's (`Registry::write_log`).
    pub fn record(&self, version: Option<&str>) -> Record {
        match self {
            Note::Started { ext, part } => Record::info("run_started", "Çalıştırıldı (İşlemler).")
                .of(ext, version)
                .part(part.clone()),
            Note::Worked {
                ext,
                part,
                code,
                message,
            } => Record::info(code, message.clone())
                .of(ext, version)
                .part(part.clone()),
            Note::Failed {
                ext,
                part,
                code,
                message,
                detail,
                ..
            } => {
                let record = Record::error(code, message.clone())
                    .of(ext, version)
                    .part(part.clone());
                match detail {
                    Some(detail) => record.detail(detail.clone()),
                    None => record,
                }
            }
        }
    }

    /// The note as a line a headless host says it: whose, which part, what happened and, for a failure, its detail.
    pub fn said(&self) -> String {
        match self {
            Note::Started { ext, part } => format!("eklenti {ext} {part}: çalıştırıldı"),
            Note::Worked {
                ext, part, message, ..
            } => format!("eklenti {ext} {part}: {message}"),
            Note::Failed {
                ext,
                part,
                message,
                detail,
                ..
            } => match detail {
                Some(detail) => format!("eklenti {ext} {part}: {message}\n{detail}"),
                None => format!("eklenti {ext} {part}: {message}"),
            },
        }
    }

    /// Whether it says the run failed.
    pub fn failed(&self) -> bool {
        matches!(self, Note::Failed { .. })
    }
}

#[cfg(test)]
mod tests {
    use kentos_extension_host::Level;

    use super::*;

    /// A failure is kept as an error with its detail and said with it; a start and an ending as information, the
    /// start in the desktop's words.
    #[test]
    fn a_note_is_kept_and_said_as_the_desktop_keeps_it() {
        let failed = Note::Failed {
            ext: "tr.ornek.deneme".into(),
            part: "processing:olc".into(),
            title: "Ölç".into(),
            code: "run_failed",
            message: "“Ölç” hata verdi: ValueError: boş".into(),
            detail: Some("Traceback …\nValueError: boş".into()),
            crash: false,
        };
        let record = failed.record(Some("1.0.0"));
        assert_eq!(record.level, Level::Error);
        assert_eq!(record.code, "run_failed");
        assert_eq!(record.extension.as_deref(), Some("tr.ornek.deneme"));
        assert_eq!(record.version.as_deref(), Some("1.0.0"));
        assert_eq!(record.part.as_deref(), Some("processing:olc"));
        assert_eq!(
            record.detail.as_deref(),
            Some("Traceback …\nValueError: boş")
        );
        assert!(failed.failed());
        assert_eq!(
            failed.said(),
            "eklenti tr.ornek.deneme processing:olc: “Ölç” hata verdi: ValueError: boş\nTraceback …\nValueError: boş"
        );
        let started = Note::Started {
            ext: "tr.ornek.deneme".into(),
            part: "processing:olc".into(),
        };
        let record = started.record(None);
        assert_eq!(
            (record.level, record.code.as_str(), record.message.as_str()),
            (Level::Info, "run_started", "Çalıştırıldı (İşlemler).")
        );
        assert!(!started.failed());
    }
}

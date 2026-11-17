//! What arrives is looked at before anyone else gets it (docs/adr/9000 §14): a professional's file reaches a
//! municipality's staff, a project's attachment everyone of the project, and a phone or an agent sends what it likes.
//!
//! - **Its kind:** a file is taken only as one of the kinds of [`KINDS`] (drawings, maps, documents, pictures,
//!   archives) by its name's extension, and its bytes must be that kind's (a PDF begins as a PDF does, a picture as
//!   its format does). The media type kept is the kind's, never the one the client said.
//! - **No programs:** a program or a script is refused under any name (`MZ`, ELF, Mach-O, `#!`), and so is an HTML
//!   page passed off as text, a script inside a picture, and an archive holding a program, macros or a password.
//! - **Malware:** the industry's antivirus test file is found by the server itself; when the server has an antivirus
//!   daemon (`KENTOS_AV_ADDRESS`, the INSTREAM protocol over TCP or a Unix socket) every file goes through it too.
//!   With `KENTOS_AV_REQUIRED=1` a file the daemon cannot scan (down, too big) is refused rather than kept unscanned.
//!
//! A kept file says how far it was checked: `clean` (an antivirus daemon scanned it) or `unscanned` (only the rules
//! above did). A refused file is not kept, and the caller says why.

use std::path::PathBuf;
use std::time::Duration;

use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

use crate::blobs::Blobs;
use crate::error::{AppError, AppResult};

/// How far a kept file was checked.
pub use kentos_contracts::ScanState;

/// The kinds a file may be, by its extension: the kind its bytes must be and the media type it is kept with.
pub const KINDS: &[(&str, Kind, &str)] = &[
    ("kcad", Kind::Kcad, "application/vnd.kentos.kcad"),
    ("dxf", Kind::Any, "image/vnd.dxf"),
    ("dwg", Kind::Dwg, "image/vnd.dwg"),
    ("ncz", Kind::Any, "application/octet-stream"),
    ("shp", Kind::Shape, "application/octet-stream"),
    ("shx", Kind::Shape, "application/octet-stream"),
    ("dbf", Kind::Dbase, "application/octet-stream"),
    ("prj", Kind::Text, "text/plain"),
    ("cpg", Kind::Text, "text/plain"),
    ("gpkg", Kind::Sqlite, "application/geopackage+sqlite3"),
    ("geojson", Kind::Text, "application/geo+json"),
    ("json", Kind::Text, "application/json"),
    ("kml", Kind::Text, "application/xml"),
    ("kmz", Kind::Zip, "application/zip"),
    ("gml", Kind::Text, "application/gml+xml"),
    ("xml", Kind::Text, "application/xml"),
    ("las", Kind::Las, "application/octet-stream"),
    ("laz", Kind::Las, "application/octet-stream"),
    ("tif", Kind::Tiff, "image/tiff"),
    ("tiff", Kind::Tiff, "image/tiff"),
    ("pdf", Kind::Pdf, "application/pdf"),
    ("docx", Kind::Office, "application/vnd.openxmlformats-officedocument.wordprocessingml.document"),
    ("xlsx", Kind::Office, "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"),
    ("pptx", Kind::Office, "application/vnd.openxmlformats-officedocument.presentationml.presentation"),
    ("odt", Kind::Office, "application/vnd.oasis.opendocument.text"),
    ("ods", Kind::Office, "application/vnd.oasis.opendocument.spreadsheet"),
    ("odp", Kind::Office, "application/vnd.oasis.opendocument.presentation"),
    ("doc", Kind::Compound, "application/octet-stream"),
    ("xls", Kind::Compound, "application/octet-stream"),
    ("ppt", Kind::Compound, "application/octet-stream"),
    ("rtf", Kind::Rtf, "application/rtf"),
    ("txt", Kind::Text, "text/plain"),
    ("csv", Kind::Text, "text/csv"),
    ("tsv", Kind::Text, "text/tab-separated-values"),
    ("md", Kind::Text, "text/markdown"),
    ("png", Kind::Png, "image/png"),
    ("jpg", Kind::Jpeg, "image/jpeg"),
    ("jpeg", Kind::Jpeg, "image/jpeg"),
    ("gif", Kind::Gif, "image/gif"),
    ("webp", Kind::Webp, "image/webp"),
    ("heic", Kind::Heif, "image/heic"),
    ("heif", Kind::Heif, "image/heif"),
    ("bmp", Kind::Bmp, "image/bmp"),
    ("svg", Kind::Svg, "image/svg+xml"),
    ("mp4", Kind::Movie, "video/mp4"),
    ("mov", Kind::Movie, "video/quicktime"),
    ("zip", Kind::Zip, "application/zip"),
];

/// What a file's bytes must be.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Kcad,
    Pdf,
    Png,
    Jpeg,
    Gif,
    Webp,
    Heif,
    Bmp,
    Tiff,
    Svg,
    Movie,
    Zip,
    /// A zipped office document: a zip with its parts' list and no macros.
    Office,
    /// An older office document (a compound file).
    Compound,
    Rtf,
    Text,
    Dwg,
    Shape,
    Dbase,
    Sqlite,
    Las,
    /// Any bytes but a program's.
    Any,
}

/// Names inside an archive that are refused: programs, scripts and shortcuts.
const PROGRAM_EXTENSIONS: &[&str] = &[
    "exe", "dll", "com", "scr", "msi", "msp", "bat", "cmd", "ps1", "psm1", "vbs", "vbe", "js", "jse", "wsf", "wsh",
    "hta", "jar", "sh", "bash", "app", "lnk", "cpl", "apk", "dmg", "pkg", "deb", "rpm", "appimage", "reg", "pif", "html",
    "htm", "xhtml", "mjs", "py", "pyw", "pl", "rb", "php", "docm", "xlsm", "pptm",
];

/// What the scan found of a file that may be kept.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Scanned {
    pub state: ScanState,
    /// Why it is `unscanned` when an antivirus daemon was meant to scan it, or the daemon's word.
    pub note: Option<String>,
    /// The media type of its kind.
    pub media_type: String,
}

/// Why a file is refused.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Refusal {
    /// Its kind, its bytes or its contents are not taken.
    Blocked(String),
    /// Malware was found: the signature's name.
    Infected(String),
}

impl Refusal {
    /// What the sender is told.
    pub fn message(&self, name: &str) -> String {
        match self {
            Refusal::Blocked(why) => format!("“{name}” alınmadı: {why}"),
            Refusal::Infected(what) => {
                format!("“{name}” alınmadı: dosyada zararlı yazılım bulundu ({what}). Dosya saklanmadı.")
            }
        }
    }

    pub fn code(&self) -> &'static str {
        match self {
            Refusal::Blocked(_) => "blocked",
            Refusal::Infected(_) => "infected",
        }
    }
}

#[derive(Clone, Debug)]
enum Daemon {
    Tcp(String),
    Unix(PathBuf),
}

/// The server's scanner: the type rules, and an antivirus daemon when it has one.
#[derive(Clone, Debug)]
pub struct Scanner {
    daemon: Option<Daemon>,
    required: bool,
    /// The largest file sent to the daemon (its own stream limit is 25 MiB unless set otherwise).
    daemon_max: u64,
}

impl Default for Scanner {
    fn default() -> Self {
        Self::rules_only()
    }
}

/// The largest archive whose list of names is read.
const ARCHIVE_MAX: u64 = 256 * 1024 * 1024;
/// The largest picture read whole for scripts.
const SVG_MAX: u64 = 16 * 1024 * 1024;

impl Scanner {
    /// The type rules alone.
    pub fn rules_only() -> Self {
        Self {
            daemon: None,
            required: false,
            daemon_max: 25 * 1024 * 1024,
        }
    }

    /// As the server's environment sets it: `KENTOS_AV_ADDRESS` (`tcp://host:port`, `host:port`, `unix:///path` or
    /// `/path`), `KENTOS_AV_REQUIRED` (`1` or `true`) and `KENTOS_AV_MAX_BYTES`.
    pub fn from_env() -> Result<Self, String> {
        Self::configured(|k| std::env::var(k).ok())
    }

    /// As [`Self::from_env`], with the settings `get` gives (the server's env file and environment).
    pub fn configured(get: impl Fn(&str) -> Option<String>) -> Result<Self, String> {
        let get = |k: &str| get(k).map(|v| v.trim().to_owned()).filter(|v| !v.is_empty());
        let mut s = Self::rules_only();
        if let Some(address) = get("KENTOS_AV_ADDRESS") {
            s.daemon = Some(Self::daemon_of(&address)?);
        }
        s.required = matches!(get("KENTOS_AV_REQUIRED").as_deref(), Some("1" | "true" | "yes"));
        if s.required && s.daemon.is_none() {
            return Err("KENTOS_AV_REQUIRED açık ama KENTOS_AV_ADDRESS yok: dosyaları tarayacak bir adres verin.".into());
        }
        if let Some(max) = get("KENTOS_AV_MAX_BYTES") {
            s.daemon_max = max
                .parse()
                .ok()
                .filter(|m| *m > 0)
                .ok_or_else(|| format!("KENTOS_AV_MAX_BYTES bir bayt sayısı değil: {max}"))?;
        }
        Ok(s)
    }

    /// With an antivirus daemon at `address`.
    pub fn with_daemon(address: &str, required: bool) -> Result<Self, String> {
        Ok(Self {
            daemon: Some(Self::daemon_of(address)?),
            required,
            ..Self::rules_only()
        })
    }

    fn daemon_of(address: &str) -> Result<Daemon, String> {
        if let Some(path) = address.strip_prefix("unix://") {
            return Ok(Daemon::Unix(PathBuf::from(path)));
        }
        if address.starts_with('/') {
            return Ok(Daemon::Unix(PathBuf::from(address)));
        }
        let hostport = address.strip_prefix("tcp://").unwrap_or(address);
        if hostport.rsplit_once(':').is_some_and(|(h, p)| !h.is_empty() && p.parse::<u16>().is_ok()) {
            Ok(Daemon::Tcp(hostport.to_owned()))
        } else {
            Err(format!("antivirüs adresi anlaşılmadı: {address} (tcp://ana-makine:port ya da unix:///yol)"))
        }
    }

    /// Whether an antivirus daemon scans what arrives.
    pub fn has_daemon(&self) -> bool {
        self.daemon.is_some()
    }

    /// Scans the object `key` sent as `name`. `Ok(Err(…))` is a refusal; `Err` the scanner's own trouble.
    pub async fn check(&self, blobs: &Blobs, key: &str, name: &str) -> AppResult<Result<Scanned, Refusal>> {
        let size = blobs.len(key).await?;
        let head = {
            let mut file = blobs.open(key).await?;
            let mut head = vec![0; usize::try_from(size.min(64 * 1024)).unwrap_or(64 * 1024)];
            file.read_exact(&mut head).await?;
            head
        };
        let Some(ext) = extension(name) else {
            return Ok(Err(Refusal::Blocked(
                "dosyanın adında türünü gösteren bir uzantı yok (örneğin .pdf, .jpg, .kcad).".into(),
            )));
        };
        let Some(&(_, kind, media)) = KINDS.iter().find(|(e, _, _)| *e == ext) else {
            return Ok(Err(Refusal::Blocked(format!(
                ".{ext} dosyaları alınmıyor. Alınan türler: {}.",
                KINDS.iter().map(|(e, _, _)| *e).collect::<Vec<_>>().join(", ")
            ))));
        };
        if let Some(why) = program(&head) {
            return Ok(Err(Refusal::Blocked(why.into())));
        }
        if contains(&head[..head.len().min(4096)], &test_signature()) {
            return Ok(Err(Refusal::Infected("antivirüs deneme dosyası (EICAR)".into())));
        }
        if let Some(why) = mismatch(kind, &head) {
            return Ok(Err(Refusal::Blocked(format!("içeriği .{ext} dosyası değil ({why})."))));
        }
        match kind {
            Kind::Zip | Kind::Office => {
                if size > ARCHIVE_MAX {
                    return Ok(Err(Refusal::Blocked(format!(
                        "{} MiB'tan büyük arşivlerin içi denetlenemiyor; dosyaları ayrı ayrı ya da daha küçük arşivlerle gönderin.",
                        ARCHIVE_MAX / (1024 * 1024)
                    ))));
                }
                if let Some(why) = archive_problem(&blobs.read(key).await?, kind == Kind::Office) {
                    return Ok(Err(Refusal::Blocked(why)));
                }
            }
            Kind::Svg => {
                if size > SVG_MAX {
                    return Ok(Err(Refusal::Blocked("çizim (SVG) 16 MiB'tan büyük.".into())));
                }
                let text = String::from_utf8_lossy(&blobs.read(key).await?).to_lowercase();
                if ["<script", "javascript:", "<foreignobject", "<iframe"].iter().any(|t| text.contains(t))
                    || has_event_attribute(&text)
                {
                    return Ok(Err(Refusal::Blocked("SVG çiziminin içinde betik var.".into())));
                }
            }
            _ => {}
        }
        let (state, note) = match &self.daemon {
            None => (ScanState::Unscanned, None),
            Some(_) if size > self.daemon_max => {
                let why = format!(
                    "dosya antivirüs taramasının sınırından ({} MiB) büyük",
                    self.daemon_max / (1024 * 1024)
                );
                if self.required {
                    return Ok(Err(Refusal::Blocked(format!("{why}; bu sunucuda taranmayan dosya alınmıyor."))));
                }
                (ScanState::Unscanned, Some(format!("{why}; yalnız tür kurallarıyla denetlendi.")))
            }
            Some(daemon) => match self.ask(daemon, blobs, key).await {
                Ok(Answer::Clean) => (ScanState::Clean, None),
                Ok(Answer::Found(what)) => return Ok(Err(Refusal::Infected(what))),
                Err(why) if self.required => {
                    eprintln!("⚠ antivirüs taraması yapılamadı: {why}");
                    return Err(AppError::Limited {
                        message: "Dosyalar şu an taranamıyor; dosya alınmadı. Birkaç dakika sonra yeniden gönderin."
                            .into(),
                        retry_after: 60,
                    });
                }
                Err(why) => {
                    eprintln!("⚠ antivirüs taraması yapılamadı: {why}");
                    (
                        ScanState::Unscanned,
                        Some("antivirüs taraması o an yapılamadı; yalnız tür kurallarıyla denetlendi.".into()),
                    )
                }
            },
        };
        Ok(Ok(Scanned {
            state,
            note,
            media_type: media.to_owned(),
        }))
    }

    async fn ask(&self, daemon: &Daemon, blobs: &Blobs, key: &str) -> Result<Answer, String> {
        let file = blobs.open(key).await.map_err(|e| e.to_string())?;
        let talk = async {
            match daemon {
                Daemon::Tcp(address) => {
                    let stream = tokio::net::TcpStream::connect(address).await?;
                    instream(stream, file).await
                }
                #[cfg(unix)]
                Daemon::Unix(path) => {
                    let stream = tokio::net::UnixStream::connect(path).await?;
                    instream(stream, file).await
                }
                #[cfg(not(unix))]
                Daemon::Unix(_) => Err(std::io::Error::other("Unix soketi bu sistemde yok")),
            }
        };
        let reply = tokio::time::timeout(Duration::from_secs(120), talk)
            .await
            .map_err(|_| "antivirüs iki dakikada yanıt vermedi".to_owned())?
            .map_err(|e| e.to_string())?;
        answer_of(&reply)
    }
}

#[derive(Debug, PartialEq, Eq)]
enum Answer {
    Clean,
    Found(String),
}

/// The daemon's reply: `stream: OK`, `stream: <signature> FOUND`, or an error.
fn answer_of(reply: &str) -> Result<Answer, String> {
    let text = reply.trim_end_matches('\0').trim();
    let body = text.strip_prefix("stream:").map(str::trim).unwrap_or(text);
    if body == "OK" {
        Ok(Answer::Clean)
    } else if let Some(what) = body.strip_suffix("FOUND") {
        Ok(Answer::Found(what.trim().chars().take(120).collect()))
    } else {
        Err(format!("antivirüs yanıtı: {}", text.chars().take(200).collect::<String>()))
    }
}

/// The INSTREAM exchange: the command, the file in length-prefixed chunks, an empty chunk, then the reply.
async fn instream<S: AsyncRead + AsyncWrite + Unpin>(mut stream: S, mut file: tokio::fs::File) -> std::io::Result<String> {
    stream.write_all(b"zINSTREAM\0").await?;
    let mut buffer = vec![0; 64 * 1024];
    loop {
        let n = file.read(&mut buffer).await?;
        if n == 0 {
            break;
        }
        stream.write_all(&u32::try_from(n).unwrap_or(u32::MAX).to_be_bytes()).await?;
        stream.write_all(&buffer[..n]).await?;
    }
    stream.write_all(&[0, 0, 0, 0]).await?;
    stream.flush().await?;
    let mut reply = Vec::new();
    stream.take(4096).read_to_end(&mut reply).await?;
    Ok(String::from_utf8_lossy(&reply).into_owned())
}

/// A name's extension, lower-case (the last one: `rapor.pdf.exe` is an `exe`).
pub fn extension(name: &str) -> Option<String> {
    let base = name.rsplit(['/', '\\']).next().unwrap_or(name);
    let (stem, ext) = base.rsplit_once('.')?;
    (!stem.is_empty() && !ext.is_empty() && ext.len() <= 10).then(|| ext.to_lowercase())
}

/// The media type a name's kind is kept with (`application/octet-stream` for a kind not taken).
pub fn media_type_of(name: &str) -> &'static str {
    extension(name)
        .and_then(|e| KINDS.iter().find(|(k, _, _)| *k == e).map(|(_, _, m)| *m))
        .unwrap_or("application/octet-stream")
}

/// Why these first bytes are a program's, if they are.
fn program(head: &[u8]) -> Option<&'static str> {
    const PROGRAMS: [&[u8]; 8] = [
        b"MZ",
        b"\x7fELF",
        b"dex\n",
        &[0xfe, 0xed, 0xfa, 0xce],
        &[0xfe, 0xed, 0xfa, 0xcf],
        &[0xce, 0xfa, 0xed, 0xfe],
        &[0xcf, 0xfa, 0xed, 0xfe],
        &[0xca, 0xfe, 0xba, 0xbe],
    ];
    if PROGRAMS.iter().any(|m| head.starts_with(m)) {
        Some("dosya çalıştırılabilir bir program.")
    } else if head.starts_with(b"#!") {
        Some("dosya bir betik (komut dosyası).")
    } else {
        None
    }
}

/// Why these first bytes are not `kind`'s, if they are not.
fn mismatch(kind: Kind, head: &[u8]) -> Option<&'static str> {
    let starts = |m: &[u8]| head.starts_with(m);
    let ok = match kind {
        Kind::Kcad => starts(&kentos_kcad::MAGIC[..5]),
        Kind::Pdf => contains(&head[..head.len().min(1024)], b"%PDF-"),
        Kind::Png => starts(b"\x89PNG\r\n\x1a\n"),
        Kind::Jpeg => starts(&[0xff, 0xd8, 0xff]),
        Kind::Gif => starts(b"GIF87a") || starts(b"GIF89a"),
        Kind::Webp => starts(b"RIFF") && head.get(8..12) == Some(b"WEBP"),
        Kind::Heif => {
            const BRANDS: [&[u8]; 7] = [b"heic", b"heix", b"hevc", b"heim", b"heis", b"mif1", b"msf1"];
            head.get(4..8) == Some(b"ftyp") && head.get(8..12).is_some_and(|b| BRANDS.contains(&b))
        }
        Kind::Movie => head.get(4..8) == Some(b"ftyp"),
        Kind::Bmp => starts(b"BM"),
        Kind::Tiff => starts(b"II*\0") || starts(b"MM\0*") || starts(b"II+\0") || starts(b"MM\0+"),
        Kind::Zip | Kind::Office => starts(b"PK\x03\x04") || starts(b"PK\x05\x06"),
        Kind::Compound => starts(&[0xd0, 0xcf, 0x11, 0xe0, 0xa1, 0xb1, 0x1a, 0xe1]),
        Kind::Rtf => starts(b"{\\rtf"),
        Kind::Dwg => starts(b"AC1"),
        Kind::Shape => starts(&[0x00, 0x00, 0x27, 0x0a]),
        Kind::Dbase => head
            .first()
            .is_some_and(|b| [0x02, 0x03, 0x04, 0x05, 0x30, 0x31, 0x32, 0x43, 0x63, 0x83, 0x8b, 0x8e, 0xcb, 0xf5, 0xfb].contains(b)),
        Kind::Sqlite => starts(b"SQLite format 3\0"),
        Kind::Las => starts(b"LASF"),
        Kind::Svg => {
            let text = String::from_utf8_lossy(&head[..head.len().min(4096)]).to_lowercase();
            return (!text.contains("<svg")).then_some("bir SVG çizimi değil");
        }
        Kind::Text => {
            if head.contains(&0) {
                return Some("metin değil, ikili veri");
            }
            let text = String::from_utf8_lossy(&head[..head.len().min(4096)]).to_lowercase();
            let start = text.trim_start_matches('\u{feff}').trim_start();
            return (start.starts_with("<!doctype html") || start.starts_with("<html") || text.contains("<script"))
                .then_some("bir web sayfası ya da betik");
        }
        Kind::Any => true,
    };
    (!ok).then_some("içeriği bu türün imzasıyla başlamıyor")
}

/// Why an archive is refused, if it is: unreadable, locked with a password, holding a program, or (an office
/// document) not one or holding macros.
fn archive_problem(bytes: &[u8], office: bool) -> Option<String> {
    let limits = kentos_zip::Limits {
        entries: 20_000,
        ..kentos_zip::Limits::default()
    };
    let listed = match kentos_zip::list_every(bytes, &limits) {
        Ok(l) => l,
        Err(e) => return Some(format!("arşiv okunamadı ({e}).")),
    };
    if listed.iter().any(|e| e.is_encrypted()) {
        return Some("arşiv parolayla kilitli; içi denetlenemiyor.".into());
    }
    if let Some(e) = listed
        .iter()
        .find(|e| extension(e.file_name()).is_some_and(|x| PROGRAM_EXTENSIONS.contains(&x.as_str())))
    {
        return Some(format!("arşivin içinde bir program ya da betik var ({}).", e.file_name()));
    }
    if office {
        let names: Vec<String> = listed.iter().map(|e| e.file_name().to_lowercase()).collect();
        if !names.iter().any(|n| n == "[content_types].xml" || n == "mimetype") {
            return Some("bir ofis belgesi değil.".into());
        }
        if names.iter().any(|n| n.ends_with("vbaproject.bin") || n.starts_with("basic/") || n.contains("/macros/")) {
            return Some("belgede makro var; makrosuz kaydedip gönderin.".into());
        }
    }
    None
}

/// An `on…=` attribute of a tag (`onload=`, `onclick =`): a script in a picture.
fn has_event_attribute(lower: &str) -> bool {
    let bytes = lower.as_bytes();
    let mut i = 0;
    while let Some(at) = lower[i..].find("on") {
        let start = i + at;
        let before = start.checked_sub(1).map(|b| bytes[b]);
        let mut end = start + 2;
        while end < bytes.len() && bytes[end].is_ascii_lowercase() {
            end += 1;
        }
        let mut eq = end;
        while eq < bytes.len() && bytes[eq].is_ascii_whitespace() {
            eq += 1;
        }
        if before.is_some_and(|b| b.is_ascii_whitespace() || b == b'/' || b == b'"' || b == b'\'')
            && end > start + 2
            && bytes.get(eq) == Some(&b'=')
        {
            return true;
        }
        i = start + 2;
    }
    false
}

fn contains(hay: &[u8], needle: &[u8]) -> bool {
    !needle.is_empty() && hay.windows(needle.len()).any(|w| w == needle)
}

/// The industry's antivirus test signature, put together here so that no copy of this source is itself taken for
/// malware.
pub fn test_signature() -> Vec<u8> {
    [&b"X5O!P%@AP[4\\PZX54(P^)7CC)7}$EICAR-STANDARD-"[..], &b"ANTIVIRUS-TEST-FILE!$H+H*"[..]].concat()
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn verdict(scanner: &Scanner, name: &str, bytes: &[u8]) -> Result<Scanned, Refusal> {
        let dir = std::env::temp_dir().join(format!("kentos-scan-{}", uuid::Uuid::now_v7()));
        let blobs = Blobs::new(&dir);
        let key = format!("uploads/accounts/{}", uuid::Uuid::now_v7());
        let mut w = blobs.create(&key, bytes.len() as u64 + 1).await.unwrap();
        w.write(bytes).await.unwrap();
        w.finish().await.unwrap();
        let v = scanner.check(&blobs, &key, name).await.unwrap();
        let _ = std::fs::remove_dir_all(dir);
        v
    }

    fn zip(entries: &[(&str, &str)]) -> Vec<u8> {
        let entries: Vec<(&str, &[u8])> = entries.iter().map(|(n, b)| (*n, b.as_bytes())).collect();
        kentos_zip::write(&entries).unwrap()
    }

    fn blocked(v: Result<Scanned, Refusal>) -> String {
        match v {
            Err(Refusal::Blocked(why)) => why,
            other => panic!("not blocked: {other:?}"),
        }
    }

    #[tokio::test]
    async fn a_file_is_taken_as_its_kind_and_kept_with_that_kinds_type() {
        let rules = Scanner::rules_only();
        let pdf = verdict(&rules, "Rapor.PDF", b"%PDF-1.7\n1 0 obj\n").await.unwrap();
        assert_eq!((pdf.state, pdf.media_type.as_str(), pdf.note), (ScanState::Unscanned, "application/pdf", None));
        let png = verdict(&rules, "foto.png", b"\x89PNG\r\n\x1a\n....").await.unwrap();
        assert_eq!(png.media_type, "image/png");
        let csv = verdict(&rules, "liste.csv", "ada;parsel\n101;5\nŞişli;7\n".as_bytes()).await.unwrap();
        assert_eq!(csv.media_type, "text/csv");
        let archive = zip(&[("plan.dxf", "0\nSECTION\n"), ("okuyun.txt", "metin")]);
        assert!(verdict(&rules, "paket.zip", &archive).await.is_ok());
        let docx = zip(&[("[Content_Types].xml", "<Types/>"), ("word/document.xml", "<w/>")]);
        assert!(verdict(&rules, "dilekce.docx", &docx).await.is_ok());
    }

    #[tokio::test]
    async fn programs_scripts_and_disguises_are_refused_under_any_name() {
        let rules = Scanner::rules_only();
        assert!(blocked(verdict(&rules, "kurulum.exe", b"MZ\x90\0").await).contains(".exe dosyaları alınmıyor"));
        assert!(blocked(verdict(&rules, "rapor.pdf", b"MZ\x90\0").await).contains("program"));
        assert!(blocked(verdict(&rules, "notlar.txt", b"#!/bin/sh\nrm -rf /\n").await).contains("betik"));
        assert!(blocked(verdict(&rules, "plan.pdf", b"\x89PNG\r\n\x1a\n").await).contains("içeriği .pdf"));
        assert!(blocked(verdict(&rules, "sayfa.txt", b"<!DOCTYPE html><html>").await).contains("web sayfası"));
        assert!(blocked(verdict(&rules, "rapor.pdf.exe", b"%PDF-1.7").await).contains(".exe"));
        assert!(blocked(verdict(&rules, "uzantisiz", b"%PDF-1.7").await).contains("uzantı yok"));
        let svg = b"<svg xmlns=\"http://www.w3.org/2000/svg\" onload=\"alert(1)\"></svg>";
        assert!(blocked(verdict(&rules, "logo.svg", svg).await).contains("betik"));
        assert!(verdict(&rules, "logo.svg", b"<svg xmlns=\"http://www.w3.org/2000/svg\"><path d=\"M0 0\"/></svg>").await.is_ok());
        let archive = zip(&[("plan.dxf", "0\nSECTION\n"), ("oku/beni.exe", "MZ")]);
        assert!(blocked(verdict(&rules, "paket.zip", &archive).await).contains("beni.exe"));
        let macro_doc = zip(&[("[Content_Types].xml", "<Types/>"), ("word/vbaProject.bin", "..")]);
        assert!(blocked(verdict(&rules, "dilekce.docx", &macro_doc).await).contains("makro"));
        assert!(blocked(verdict(&rules, "bozuk.zip", b"PK\x03\x04bozuk").await).contains("okunamadı"));
    }

    #[tokio::test]
    async fn the_test_signature_is_found_without_a_daemon() {
        let mut eicar = test_signature();
        eicar.extend_from_slice(b"\r\n");
        let v = verdict(&Scanner::rules_only(), "deneme.txt", &eicar).await;
        assert!(matches!(v, Err(Refusal::Infected(_))), "{v:?}");
    }

    /// A daemon on a local port answering every scan with `reply`; how many files it was sent.
    async fn daemon(reply: &'static str) -> (String, std::sync::Arc<std::sync::atomic::AtomicUsize>) {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap().to_string();
        let seen = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let count = seen.clone();
        tokio::spawn(async move {
            while let Ok((mut s, _)) = listener.accept().await {
                let count = count.clone();
                tokio::spawn(async move {
                    let mut command = [0u8; 10];
                    s.read_exact(&mut command).await.unwrap();
                    assert_eq!(&command, b"zINSTREAM\0");
                    loop {
                        let mut len = [0u8; 4];
                        s.read_exact(&mut len).await.unwrap();
                        let n = u32::from_be_bytes(len) as usize;
                        if n == 0 {
                            break;
                        }
                        let mut chunk = vec![0; n];
                        s.read_exact(&mut chunk).await.unwrap();
                    }
                    count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    s.write_all(reply.as_bytes()).await.unwrap();
                    s.write_all(b"\0").await.unwrap();
                });
            }
        });
        (address, seen)
    }

    #[tokio::test]
    async fn a_daemon_scans_what_the_rules_let_through_and_its_absence_is_said() {
        let (clean_at, seen) = daemon("stream: OK").await;
        let clean = Scanner::with_daemon(&format!("tcp://{clean_at}"), false).unwrap();
        let v = verdict(&clean, "rapor.pdf", b"%PDF-1.7").await.unwrap();
        assert_eq!(v.state, ScanState::Clean);
        assert_eq!(seen.load(std::sync::atomic::Ordering::SeqCst), 1);
        // A program never reaches the daemon.
        let _ = verdict(&clean, "kurulum.exe", b"MZ").await;
        assert_eq!(seen.load(std::sync::atomic::Ordering::SeqCst), 1);
        let (found_at, _) = daemon("stream: Win.Test.Deneme-1 FOUND").await;
        let found = Scanner::with_daemon(&found_at, false).unwrap();
        assert_eq!(
            verdict(&found, "rapor.pdf", b"%PDF-1.7").await,
            Err(Refusal::Infected("Win.Test.Deneme-1".into()))
        );
        // Nobody listening: kept unscanned, or refused for now when the daemon is required.
        let gone = Scanner::with_daemon("127.0.0.1:9", false).unwrap();
        let v = verdict(&gone, "rapor.pdf", b"%PDF-1.7").await.unwrap();
        assert_eq!(v.state, ScanState::Unscanned);
        assert!(v.note.unwrap().contains("yapılamadı"));
        let dir = std::env::temp_dir().join(format!("kentos-scan-{}", uuid::Uuid::now_v7()));
        let blobs = Blobs::new(&dir);
        let mut w = blobs.create("uploads/accounts/x", 16).await.unwrap();
        w.write(b"%PDF-1.7").await.unwrap();
        w.finish().await.unwrap();
        let required = Scanner::with_daemon("127.0.0.1:9", true).unwrap();
        assert!(matches!(
            required.check(&blobs, "uploads/accounts/x", "rapor.pdf").await,
            Err(AppError::Limited { .. })
        ));
        let _ = std::fs::remove_dir_all(dir);
        assert!(Scanner::with_daemon("bir yer", false).is_err());
    }

    #[test]
    fn the_daemons_replies_and_names_are_read() {
        assert_eq!(answer_of("stream: OK\0"), Ok(Answer::Clean));
        assert_eq!(answer_of("stream: Eicar-Test-Signature FOUND\0"), Ok(Answer::Found("Eicar-Test-Signature".into())));
        assert!(answer_of("INSTREAM size limit exceeded. ERROR\0").is_err());
        assert_eq!(extension("a/b/Rapor.Final.PDF").as_deref(), Some("pdf"));
        assert_eq!(extension(".gizli"), None);
        assert_eq!(media_type_of("pafta.kcad"), "application/vnd.kentos.kcad");
        assert!(has_event_attribute("<svg onload=\"x\">") && has_event_attribute("<g\nonclick = 'x'>"));
        assert!(!has_event_attribute("<text>bonus=on</text><g id=\"one\">"));
    }
}

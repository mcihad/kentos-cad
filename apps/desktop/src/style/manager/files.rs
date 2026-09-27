//! Stil yöneticisi's files (the web's `styleFiles.ts` and the manager's
//! import and export): a .kstil file or the clipboard's text read, checked
//! and offered for import; items written out with the drawings their
//! symbols use; a PNG or JPEG picture taken in as an image of Kitaplığım.

use iced::Task;
use kentos_native_style::file::{ConflictMode, export_styles, parse_style_file};
use kentos_native_style::library::{Source, new_item_id};
use serde_json::{Value, json};

use super::{Event, ImportDraft, ev};
use crate::app::{App, Message};

/// A picture file as a raster asset of Kitaplığım: a `data:` address and
/// its size in pixels (`rasterAsset`). None when the bytes are not a PNG or
/// a JPEG; why not when they claim to be one and cannot be read.
pub fn raster_asset(name: &str, bytes: &[u8]) -> Option<Result<Value, String>> {
    let (format, size) = if bytes.starts_with(&[0x89, b'P', b'N', b'G']) {
        let size = png::Decoder::new(std::io::Cursor::new(bytes))
            .read_info()
            .ok()
            .map(|reader| (reader.info().width, reader.info().height));
        ("png", size)
    } else if bytes.starts_with(&[0xFF, 0xD8]) {
        ("jpeg", jpeg_size(bytes))
    } else {
        return None;
    };
    let Some((w, h)) = size.filter(|(w, h)| *w > 0 && *h > 0) else {
        return Some(Err(format!(
            "“{name}” okunamadı: {} dosyası bozuk görünüyor.",
            format.to_uppercase()
        )));
    };
    let stem = name
        .rsplit_once('.')
        .map_or(name, |(stem, ext)| {
            if matches!(ext.to_ascii_lowercase().as_str(), "png" | "jpg" | "jpeg") {
                stem
            } else {
                name
            }
        })
        .to_owned();
    Some(Ok(json!({
        "kind": "asset",
        "id": new_item_id("a"),
        "name": stem,
        "path": ["Görüntülerim"],
        "format": format,
        "data": format!("data:image/{format};base64,{}", base64(bytes)),
        "width": w,
        "height": h,
    })))
}

/// A JPEG's width and height from its frame header.
fn jpeg_size(b: &[u8]) -> Option<(u32, u32)> {
    let mut k = 2;
    while k + 9 < b.len() {
        if b[k] != 0xFF {
            k += 1;
            continue;
        }
        let marker = b[k + 1];
        let len = usize::from(u16::from_be_bytes([b[k + 2], b[k + 3]]));
        // Start of frame: every SOFn but DHT (C4), JPG (C8) and DAC (CC).
        if (0xC0..=0xCF).contains(&marker) && !matches!(marker, 0xC4 | 0xC8 | 0xCC) {
            let h = u32::from(u16::from_be_bytes([b[k + 5], b[k + 6]]));
            let w = u32::from(u16::from_be_bytes([b[k + 7], b[k + 8]]));
            return (w > 0 && h > 0).then_some((w, h));
        }
        k += 2 + len;
    }
    None
}

/// Base64 of bytes (the standard alphabet, padded), for `data:` addresses.
pub fn base64(bytes: &[u8]) -> String {
    const ABC: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let n = (u32::from(chunk[0]) << 16)
            | (u32::from(*chunk.get(1).unwrap_or(&0)) << 8)
            | u32::from(*chunk.get(2).unwrap_or(&0));
        out.push(char::from(ABC[(n >> 18) as usize & 63]));
        out.push(char::from(ABC[(n >> 12) as usize & 63]));
        out.push(if chunk.len() > 1 {
            char::from(ABC[(n >> 6) as usize & 63])
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            char::from(ABC[n as usize & 63])
        } else {
            '='
        });
    }
    out
}

/// A file name of a name: Turkish letters folded, the rest to dashes (`fileSlug`).
pub fn file_slug(s: &str) -> String {
    let lower = kentos_processing::text::tr_lower(s);
    let mut out = String::new();
    let mut dash = false;
    for c in lower.chars() {
        let c = match c {
            'ç' => 'c',
            'ğ' => 'g',
            'ı' => 'i',
            'ö' => 'o',
            'ş' => 's',
            'ü' => 'u',
            c => c,
        };
        if c.is_ascii_lowercase() || c.is_ascii_digit() {
            out.push(c);
            dash = false;
        } else if !dash && !out.is_empty() {
            out.push('-');
            dash = true;
        }
    }
    let out = out.trim_end_matches('-').to_owned();
    if out.is_empty() {
        "stiller".into()
    } else {
        out
    }
}

/// How an import went, as the web says it.
pub fn import_said(added: usize, replaced: usize, skipped: usize) -> String {
    let mut text = format!("{added} öğe eklendi");
    if replaced > 0 {
        text.push_str(&format!(", {replaced} güncellendi"));
    }
    if skipped > 0 {
        text.push_str(&format!(", {skipped} atlandı"));
    }
    text.push('.');
    text
}

impl App {
    /// İçe aktar → Dosyadan…: a .kstil, or a PNG or JPEG picture.
    pub(super) fn pick_style_file() -> Task<Message> {
        Task::perform(
            async {
                let file = rfd::AsyncFileDialog::new()
                    .set_title("Stil içe aktar")
                    .add_filter(
                        "KentOS stili, PNG, JPEG",
                        &["kstil", "json", "png", "jpg", "jpeg"],
                    )
                    .pick_file()
                    .await?;
                let name = file.file_name();
                let bytes = file.read().await;
                Some((name, bytes))
            },
            |picked| ev(Event::Picked(picked)),
        )
    }

    /// A picked file: a picture comes in as an image of Kitaplığım, a .kstil is offered for import.
    pub(super) fn take_file(&mut self, name: &str, bytes: &[u8]) {
        match raster_asset(name, bytes) {
            Some(Ok(asset)) => {
                let jpeg = asset["format"] == "jpeg";
                match self.styles.library.add(Source::User, asset) {
                    Ok(item) => {
                        self.library_changed(Source::User);
                        let lib = &self.styles.library;
                        if let Some(m) = &mut self.styles.manager {
                            m.at = (
                                Source::User,
                                item.path().into_iter().map(str::to_owned).collect(),
                            );
                            m.query.clear();
                            m.choose_item(lib, Some(item.id().to_owned()));
                            if jpeg {
                                // No JPEG decoder on the desktop yet: say so rather than draw nothing silently.
                                m.say(
                                    format!("“{}” Kitaplığım'a alındı; JPEG görüntüleri masaüstünde henüz çizilmez (web'de çizilir). PNG olarak kaydedip alırsanız burada da çizilir.", item.name()),
                                    true,
                                );
                            } else {
                                m.say(
                                    format!("“{}” Kitaplığım'a alındı: görüntü dolgusunda ya da görüntü işaretinde kullanılabilir.", item.name()),
                                    false,
                                );
                            }
                        }
                    }
                    Err(e) => self.manager_say(e, true),
                }
            }
            Some(Err(why)) => self.manager_say(why, true),
            None => {
                let text = String::from_utf8_lossy(bytes);
                self.offer_import(name, &text, false);
            }
        }
    }

    /// A .kstil text read and checked: the import panel, or why it cannot be taken in.
    pub(crate) fn offer_import(&mut self, name: &str, text: &str, clipboard: bool) {
        let Some(m) = &mut self.styles.manager else {
            return;
        };
        match parse_style_file(text) {
            Ok(file) => {
                m.import = Some(ImportDraft {
                    name: name.to_owned(),
                    file,
                    to: Source::User,
                    mode: ConflictMode::Copy,
                });
            }
            Err(issues) => {
                let why = if clipboard {
                    format!(
                        "Panodaki metin bir KentOS stili değil: {}.",
                        issues
                            .first()
                            .map_or("biçim tanınmadı", String::as_str)
                            .trim_end_matches('.')
                    )
                } else {
                    format!(
                        "“{name}” okunamadı: {}",
                        issues
                            .iter()
                            .take(2)
                            .cloned()
                            .collect::<Vec<_>>()
                            .join("; ")
                    )
                };
                m.say(why, true);
            }
        }
    }

    /// Writes items (and the drawings their symbols use) to a `.kstil` file the user names.
    pub(super) fn export_items(&mut self, ids: &[String], name: &str) -> Task<Message> {
        self.commit_fields();
        let refs: Vec<&str> = ids.iter().map(String::as_str).collect();
        let file = export_styles(&self.styles.library, &refs);
        let n = file["items"].as_array().map_or(0, Vec::len);
        let text = serde_json::to_string_pretty(&file).unwrap_or_default();
        let suggested = format!("{}.kstil", file_slug(name));
        Task::perform(
            async move {
                let handle = rfd::AsyncFileDialog::new()
                    .set_title("Stilleri dışa aktar")
                    .add_filter("KentOS stili", &["kstil"])
                    .set_file_name(&suggested)
                    .save_file()
                    .await?;
                let mut path = handle.path().to_path_buf();
                if path.extension().is_none() {
                    path.set_extension("kstil");
                }
                Some(
                    std::fs::write(&path, text)
                        .map(|()| {
                            format!(
                                "{n} öğe (kullandıkları çizimlerle) {} dosyasına yazıldı.",
                                path.display()
                            )
                        })
                        .map_err(|e| e.to_string()),
                )
            },
            |outcome| ev(Event::Written(outcome)),
        )
    }

    /// Panoya kopyala: the listed items as .kstil text, to paste in a message.
    pub(super) fn export_to_clipboard(&mut self, ids: &[String]) -> Task<Message> {
        self.commit_fields();
        let refs: Vec<&str> = ids.iter().map(String::as_str).collect();
        let file = export_styles(&self.styles.library, &refs);
        let n = file["items"].as_array().map_or(0, Vec::len);
        self.manager_say(
            format!("{n} öğe panoya kopyalandı: bir iletiye yapıştırıp paylaşabilirsiniz; alan kişi “İçe aktar → Panodan yapıştır” der."),
            false,
        );
        iced::clipboard::write(file.to_string())
    }
}

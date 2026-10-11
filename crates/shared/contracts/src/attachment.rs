//! Nesne ekleri (docs/adr/0225 §1): the photos and documents an object
//! carries, in their order: an embedded photo (a project library image, as
//! a picture's, docs/adr/0192 §2), a linked file (its path) or a web
//! address, each with its name and an optional note. The rules here are the
//! readers', the commands' and the server's; whether an embedded photo's
//! asset is in the project's library is the commands'.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

/// The most attachments an object carries.
pub const MAX_ATTACHMENTS: usize = 64;
/// The longest name, and the longest asset id, in characters.
pub const MAX_ATTACHMENT_NAME: usize = 256;
/// The longest note, in characters.
pub const MAX_ATTACHMENT_NOTE: usize = 1_000;
/// The longest path or address, in characters.
pub const MAX_ATTACHMENT_LINK: usize = 4_096;

/// What an attachment is, by its source (docs/adr/0225 §1).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AttachmentKind {
    /// An embedded photo, or a file or address of a JPEG or PNG.
    Photo,
    /// Any other linked file.
    Document,
    /// Any other web address.
    Link,
}

/// One of an object's attachments: its name, its source (exactly one of
/// `asset`, `file` and `url`) and an optional note.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct Attachment {
    /// The name shown (the file's name by default).
    pub name: String,
    /// Embedded: the project library's PNG or JPEG asset its bytes are.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub asset: Option<String>,
    /// Linked: a file's path, relative to the drawing's folder when it is
    /// relative.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub file: Option<String>,
    /// A web address (`http` or `https`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub url: Option<String>,
    /// A note shown with it (lines allowed).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub note: Option<String>,
}

/// A link's path without its query or fragment, lower case: what its kind is read from.
fn path_part(link: &str) -> String {
    let end = link.find(['?', '#']).unwrap_or(link.len());
    link[..end].to_lowercase()
}

/// Whether a path or address names a photo: it ends in `.jpg`, `.jpeg` or `.png`.
pub fn is_photo_link(link: &str) -> bool {
    let p = path_part(link);
    p.ends_with(".jpg") || p.ends_with(".jpeg") || p.ends_with(".png")
}

/// Whether an address is one an attachment may be: `http://` or `https://`
/// (the scheme in any case), then at least one character.
pub fn is_web_address(url: &str) -> bool {
    let lower = url.to_ascii_lowercase();
    (lower.starts_with("http://") && url.len() > 7)
        || (lower.starts_with("https://") && url.len() > 8)
}

/// A control character other than those `allowed`.
fn has_control(text: &str, allowed: &[char]) -> bool {
    text.chars()
        .any(|c| c.is_control() && !allowed.contains(&c))
}

impl Attachment {
    /// What it is, by its source.
    pub fn kind(&self) -> AttachmentKind {
        if self.asset.is_some() {
            return AttachmentKind::Photo;
        }
        match (&self.file, &self.url) {
            (Some(f), _) if is_photo_link(f) => AttachmentKind::Photo,
            (Some(_), _) => AttachmentKind::Document,
            (None, Some(u)) if is_photo_link(u) => AttachmentKind::Photo,
            _ => AttachmentKind::Link,
        }
    }

    /// What is wrong with it, when anything is.
    pub fn problem(&self) -> Option<String> {
        let name = &self.name;
        if name.trim().is_empty() {
            return Some("ekin adı boş".to_owned());
        }
        if name.chars().count() > MAX_ATTACHMENT_NAME {
            return Some(format!("ekin adı {MAX_ATTACHMENT_NAME} karakterden uzun"));
        }
        if has_control(name, &[]) {
            return Some(format!(
                "“{}” ekinin adında satır sonu ya da denetim karakteri var",
                name.trim()
            ));
        }
        let sources = [&self.asset, &self.file, &self.url]
            .iter()
            .filter(|s| s.is_some())
            .count();
        if sources != 1 {
            return Some(format!(
                "“{name}” ekinin kaynağı tek olmalı: gömülü varlık (asset), bağlı dosya (file) ya da adres (url)"
            ));
        }
        if let Some(asset) = &self.asset
            && (asset.trim().is_empty()
                || asset.chars().count() > MAX_ATTACHMENT_NAME
                || has_control(asset, &[]))
        {
            return Some(format!("“{name}” ekinin varlık kimliği geçersiz"));
        }
        for link in [&self.file, &self.url].into_iter().flatten() {
            if link.trim().is_empty() {
                return Some(format!("“{name}” ekinin yolu ya da adresi boş"));
            }
            if link.chars().count() > MAX_ATTACHMENT_LINK {
                return Some(format!(
                    "“{name}” ekinin yolu ya da adresi {MAX_ATTACHMENT_LINK} karakterden uzun"
                ));
            }
            if has_control(link, &[]) {
                return Some(format!(
                    "“{name}” ekinin yolunda ya da adresinde denetim karakteri var"
                ));
            }
        }
        if let Some(url) = &self.url
            && (!is_web_address(url) || url.chars().any(char::is_whitespace))
        {
            return Some(format!(
                "“{name}” ekinin adresi http:// ya da https:// ile başlamalı ve boşluk içermemeli"
            ));
        }
        if let Some(note) = &self.note {
            if note.chars().count() > MAX_ATTACHMENT_NOTE {
                return Some(format!(
                    "“{name}” ekinin notu {MAX_ATTACHMENT_NOTE} karakterden uzun"
                ));
            }
            if has_control(note, &['\n']) {
                return Some(format!(
                    "“{name}” ekinin notunda denetim karakteri var (yalnız satır sonu olur)"
                ));
            }
        }
        None
    }
}

fn is_false(b: &bool) -> bool {
    !*b
}

/// What a photo's EXIF says (docs/adr/0225 §3): its place in WGS 84, its
/// height, the direction it looks, when it was taken and the camera; and
/// why it has no place, when it has none (`problem`). Read by
/// `kentos_formats::exif` from a JPEG's APP1 or a TIFF's header.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct PhotoExif {
    /// Latitude, degrees, north positive (the rationals' exact value rounded once).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub latitude: Option<f64>,
    /// Longitude, degrees, east positive.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub longitude: Option<f64>,
    /// Height above sea level, metres (below it negative); its datum unsaid.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub altitude: Option<f64>,
    /// The direction the camera looked, degrees clockwise from north.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub direction: Option<f64>,
    /// Whether that north is the magnetic one (EXIF `M`); the true one otherwise.
    #[serde(default, skip_serializing_if = "is_false")]
    #[cfg_attr(feature = "ts", ts(optional, as = "Option<bool>"))]
    pub magnetic: bool,
    /// When it was taken, ISO 8601: the camera's local time with its offset
    /// when it says one (`2024-05-17T10:32:05+03:00`), without one when not;
    /// else the GPS's UTC (`…Z`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub taken: Option<String>,
    /// The camera: its model, after its maker when the model does not name it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub camera: Option<String>,
    /// Why it has no place, as the window says it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub problem: Option<String>,
}

/// What is wrong with an object's attachments, when anything is: more than
/// [`MAX_ATTACHMENTS`], or one of them (in their order).
pub fn attachments_problem(list: &[Attachment]) -> Option<String> {
    if list.len() > MAX_ATTACHMENTS {
        return Some(format!(
            "nesnenin {} eki var; en çok {MAX_ATTACHMENTS}",
            list.len()
        ));
    }
    list.iter().find_map(Attachment::problem)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn photo(name: &str) -> Attachment {
        Attachment {
            name: name.into(),
            asset: Some("resim-0123456789abcdef".into()),
            ..Attachment::default()
        }
    }

    #[test]
    fn the_kind_comes_from_the_source() {
        assert_eq!(photo("a").kind(), AttachmentKind::Photo);
        let file = |f: &str| Attachment {
            name: "a".into(),
            file: Some(f.into()),
            ..Attachment::default()
        };
        assert_eq!(file("tapu/12.JPG").kind(), AttachmentKind::Photo);
        assert_eq!(file("tapu/12.pdf").kind(), AttachmentKind::Document);
        let url = |u: &str| Attachment {
            name: "a".into(),
            url: Some(u.into()),
            ..Attachment::default()
        };
        assert_eq!(
            url("https://x.org/a.png?w=10").kind(),
            AttachmentKind::Photo
        );
        assert_eq!(url("https://x.org/parsel/12").kind(), AttachmentKind::Link);
    }

    #[test]
    fn the_rules_name_what_is_wrong() {
        assert_eq!(attachments_problem(&[photo("Ön cephe")]), None);
        assert!(attachments_problem(&[photo(" ")]).is_some());
        assert!(attachments_problem(&[photo("a\nb")]).is_some());
        let both = Attachment {
            file: Some("a.pdf".into()),
            ..photo("a")
        };
        assert!(attachments_problem(&[both]).is_some());
        let none = Attachment {
            name: "a".into(),
            ..Attachment::default()
        };
        assert!(attachments_problem(&[none]).is_some());
        let ftp = Attachment {
            name: "a".into(),
            url: Some("ftp://x.org/a".into()),
            ..Attachment::default()
        };
        assert!(attachments_problem(&[ftp]).is_some());
        let noted = Attachment {
            note: Some("iki\nsatır".into()),
            ..photo("a")
        };
        assert_eq!(attachments_problem(&[noted]), None);
        let many: Vec<Attachment> = (0..=MAX_ATTACHMENTS)
            .map(|i| photo(&i.to_string()))
            .collect();
        assert!(attachments_problem(&many).is_some());
    }
}

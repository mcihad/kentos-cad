//! Review notes a drawing carries (docs/adr/1000 §2): comments bound to objects, a sheet or a named view, kept apart
//! from the design. A note is never drawn, plotted, picked or measured; its stamps say which state of an object it
//! was written for, so a later change shows the note as stale. The file reader checks the structure here; the
//! commands check the drawing (that an anchor, sheet or view exists).

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

use crate::Vec2;

/// The most notes a drawing carries.
pub const MAX_REVIEW_NOTES: usize = 10_000;
/// The most characters a note's or a reply's text holds.
pub const MAX_REVIEW_TEXT: usize = 4000;
/// The most objects one note is bound to.
pub const MAX_REVIEW_ANCHORS: usize = 1000;
/// The most replies one note holds.
pub const MAX_REVIEW_REPLIES: usize = 200;
/// The most characters of an author's name.
pub const MAX_REVIEW_AUTHOR: usize = 120;
/// The most characters of a creation time and of a revision label.
pub const MAX_REVIEW_LABEL: usize = 64;

/// Where a note stands.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum ReviewStatus {
    /// Açık.
    #[default]
    Open,
    /// Yanıtlandı.
    Answered,
    /// Çözüldü.
    Resolved,
}

/// Who or what wrote a note. A PDF's or an AI's note is a note only: it changes no design.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum ReviewOrigin {
    #[default]
    User,
    Pdf,
    Ai,
}

impl ReviewOrigin {
    fn is_user(&self) -> bool {
        *self == Self::User
    }
}

/// An object a note is bound to, with the stamps of the state it was written for.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ReviewAnchor {
    /// The object's persistent id.
    pub uid: String,
    /// The object's stamp then (docs/adr/0253 §2).
    pub stamp: String,
    /// For a block insert: the stamp of its definition and every definition nested in it, then.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub block_stamp: Option<String>,
}

/// A reply to a note.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ReviewReply {
    pub text: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub author: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub created_at: Option<String>,
}

/// A review note.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ReviewNote {
    /// Lowercase, hyphenated UUID; unique in the drawing.
    pub id: String,
    /// 1, 2, 3 …: one more than the largest a note in the drawing has.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "id")))]
    pub number: u32,
    pub text: String,
    #[serde(default)]
    pub status: ReviewStatus,
    #[serde(default, skip_serializing_if = "ReviewOrigin::is_user")]
    #[cfg_attr(feature = "ts", ts(optional, as = "Option<ReviewOrigin>"))]
    pub origin: ReviewOrigin,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub author: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub created_at: Option<String>,
    /// The source revision the note was written against (“B”, “2026-11-05”).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub revision: Option<String>,
    /// A point in the model, world coordinates.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub at: Option<Vec2>,
    /// A sheet's id.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub sheet: Option<String>,
    /// A named view's id.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub view: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<ReviewAnchor>>", optional))]
    pub anchors: Vec<ReviewAnchor>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<ReviewReply>>", optional))]
    pub replies: Vec<ReviewReply>,
}

/// Whether `text` is a note's or a reply's text: not blank, no space at its edges, no control character but a line
/// break or a tab, at most [`MAX_REVIEW_TEXT`] characters.
pub fn review_text_ok(text: &str) -> bool {
    !text.is_empty()
        && text.trim() == text
        && text.chars().count() <= MAX_REVIEW_TEXT
        && !text.chars().any(|c| c.is_control() && c != '\n' && c != '\t')
}

/// Whether `text` is a short label (author, time, revision): no space at its edges, no control character, at most
/// `most` characters. Empty is not a label: absent is.
pub fn review_label_ok(text: &str, most: usize) -> bool {
    !text.is_empty()
        && text.trim() == text
        && text.chars().count() <= most
        && !text.chars().any(char::is_control)
}

/// Whether `id` is a lowercase, hyphenated UUID.
pub fn review_id_ok(id: &str) -> bool {
    id.len() == 36
        && id.char_indices().all(|(i, c)| match i {
            8 | 13 | 18 | 23 => c == '-',
            _ => c.is_ascii_digit() || ('a'..='f').contains(&c),
        })
}

fn note_problem(n: &ReviewNote) -> Option<String> {
    if !review_id_ok(&n.id) {
        return Some(format!("“{}” küçük harfli bir UUID değil", n.id));
    }
    if n.number == 0 {
        return Some(format!("{} notunun numarası 0 olamaz", n.id));
    }
    if !review_text_ok(&n.text) {
        return Some(format!("{}. notun metni geçersiz", n.number));
    }
    let labels = [
        (&n.author, MAX_REVIEW_AUTHOR),
        (&n.created_at, MAX_REVIEW_LABEL),
        (&n.revision, MAX_REVIEW_LABEL),
        (&n.sheet, MAX_REVIEW_LABEL),
        (&n.view, MAX_REVIEW_LABEL),
    ];
    if labels.iter().any(|(t, most)| t.as_deref().is_some_and(|t| !review_label_ok(t, *most))) {
        return Some(format!("{}. notun yazar, tarih, revizyon ya da bağ alanı geçersiz", n.number));
    }
    if n.at.is_some_and(|p| !(p.x.is_finite() && p.y.is_finite())) {
        return Some(format!("{}. notun noktası sonlu değil", n.number));
    }
    if n.anchors.len() > MAX_REVIEW_ANCHORS {
        return Some(format!("{}. not en çok {MAX_REVIEW_ANCHORS} nesneye bağlanabilir", n.number));
    }
    let mut seen = std::collections::HashSet::new();
    for a in &n.anchors {
        if !review_id_ok(&a.uid) || !seen.insert(a.uid.as_str()) {
            return Some(format!("{}. notun nesne bağı geçersiz ya da yinelenmiş: “{}”", n.number, a.uid));
        }
        if !review_label_ok(&a.stamp, MAX_REVIEW_LABEL)
            || a.block_stamp.as_deref().is_some_and(|s| !review_label_ok(s, MAX_REVIEW_LABEL))
        {
            return Some(format!("{}. notun nesne damgası geçersiz", n.number));
        }
    }
    if n.replies.len() > MAX_REVIEW_REPLIES {
        return Some(format!("{}. not en çok {MAX_REVIEW_REPLIES} yanıt taşıyabilir", n.number));
    }
    for r in &n.replies {
        if !review_text_ok(&r.text)
            || r.author.as_deref().is_some_and(|t| !review_label_ok(t, MAX_REVIEW_AUTHOR))
            || r.created_at.as_deref().is_some_and(|t| !review_label_ok(t, MAX_REVIEW_LABEL))
        {
            return Some(format!("{}. notun bir yanıtı geçersiz", n.number));
        }
    }
    None
}

/// Structural validation at the file and command boundary: what is wrong with `notes`, or none.
pub fn review_notes_problem(notes: &[ReviewNote]) -> Option<String> {
    if notes.len() > MAX_REVIEW_NOTES {
        return Some(format!("Bir çizim en çok {MAX_REVIEW_NOTES} inceleme notu taşıyabilir"));
    }
    let mut ids = std::collections::HashSet::new();
    let mut numbers = std::collections::HashSet::new();
    for n in notes {
        if let Some(problem) = note_problem(n) {
            return Some(problem);
        }
        if !ids.insert(n.id.as_str()) {
            return Some(format!("“{}” kimlikli inceleme notu iki kez yazılmış", n.id));
        }
        if !numbers.insert(n.number) {
            return Some(format!("{} numaralı inceleme notu iki kez yazılmış", n.number));
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn note(id: &str, number: u32) -> ReviewNote {
        ReviewNote {
            id: id.into(),
            number,
            text: "Kapı genişliğini kontrol edin.".into(),
            status: ReviewStatus::Open,
            origin: ReviewOrigin::User,
            author: None,
            created_at: None,
            revision: None,
            at: None,
            sheet: None,
            view: None,
            anchors: vec![],
            replies: vec![],
        }
    }

    const A: &str = "0192f5b0-0000-7000-8000-0000000000a1";
    const B: &str = "0192f5b0-0000-7000-8000-0000000000b2";

    #[test]
    fn identity_number_and_text_are_checked() {
        assert_eq!(review_notes_problem(&[note(A, 1), note(B, 2)]), None);
        assert!(review_notes_problem(&[note(A, 1), note(A, 2)]).is_some());
        assert!(review_notes_problem(&[note(A, 1), note(B, 1)]).is_some());
        assert!(review_notes_problem(&[note(&A.to_uppercase(), 1)]).is_some());
        assert!(review_notes_problem(&[note(A, 0)]).is_some());
        let mut blank = note(A, 1);
        blank.text = " x".into();
        assert!(review_notes_problem(&[blank]).is_some());
        let mut lines = note(A, 1);
        lines.text = "Bir\nİki\tüç".into();
        assert_eq!(review_notes_problem(&[lines]), None);
        let mut bell = note(A, 1);
        bell.text = "a\u{7}".into();
        assert!(review_notes_problem(&[bell]).is_some());
    }

    #[test]
    fn anchors_are_unique_and_the_origin_defaults_to_the_user() {
        let mut n = note(A, 1);
        let anchor = ReviewAnchor { uid: B.into(), stamp: "0123456789abcdef".into(), block_stamp: None };
        n.anchors = vec![anchor.clone(), anchor];
        assert!(review_notes_problem(&[n]).is_some());
        let json = serde_json::to_value(note(A, 1)).unwrap();
        assert!(json.get("origin").is_none());
        assert_eq!(json["status"], "open");
        let back: ReviewNote = serde_json::from_value(json).unwrap();
        assert_eq!(back.origin, ReviewOrigin::User);
        assert!(serde_json::from_value::<ReviewNote>(serde_json::json!({"id":A,"number":1,"text":"x","extra":1})).is_err());
    }
}

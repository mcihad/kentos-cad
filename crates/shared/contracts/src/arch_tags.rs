//! Tags (docs/adr/0344 §1): an object's tag, tied to it by its persistent id. Its words are its template filled with
//! the object's values, which the document writes (`text`), with the point it ties to (`anchor`); its frame, words and
//! leader are drawn from these on paper.
//!
//! A template is words with places in braces: `{number}`, `{type}` … and `{attr:Name}`; lines are split by `\n`. A
//! place the object's kind has no value for writes nothing.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

use crate::entity::{EntityBase, Vec2};

/// The places a template may name, besides `attr:Name`.
pub const TAG_PLACES: [&str; 22] = [
    "number",
    "type",
    "form",
    "width",
    "height",
    "sill",
    "head",
    "name",
    "usage",
    "area",
    "perimeter",
    "level",
    "phase",
    "tag",
    "place",
    "label",
    "style",
    "risers",
    "slope",
    "floorElevation",
    "ceilingElevation",
    "netArea",
];
/// The most letters a template or a tag's words hold.
pub const MAX_TAG_LETTERS: usize = 200;
/// The most lines a tag holds.
pub const MAX_TAG_LINES: usize = 6;
/// A tag's words' height on paper by default, millimetres.
pub const TAG_SIZE_MM: f64 = 2.5;

/// A tag's frame.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum TagShape {
    #[default]
    Circle,
    Hexagon,
    Rectangle,
    Rounded,
    None,
}

impl TagShape {
    pub const ALL: [TagShape; 5] = [
        TagShape::Circle,
        TagShape::Hexagon,
        TagShape::Rectangle,
        TagShape::Rounded,
        TagShape::None,
    ];

    pub fn label(self) -> &'static str {
        match self {
            TagShape::Circle => "Daire",
            TagShape::Hexagon => "Altıgen",
            TagShape::Rectangle => "Dikdörtgen",
            TagShape::Rounded => "Yuvarlak köşeli",
            TagShape::None => "Çerçevesiz",
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            TagShape::Circle => "circle",
            TagShape::Hexagon => "hexagon",
            TagShape::Rectangle => "rectangle",
            TagShape::Rounded => "rounded",
            TagShape::None => "none",
        }
    }
}

fn is_circle(s: &TagShape) -> bool {
    *s == TagShape::Circle
}

/// Why a tag's words are no longer its object's.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum TagBreak {
    /// Its object is gone.
    TargetGone,
}

impl TagBreak {
    pub fn words(self) -> &'static str {
        "Etiketin nesnesi çizimde yok; etiket son yazısıyla kaldı. Nesneyi geri getirin ya da etiketi silin."
    }
}

/// A tag's own fields (docs/adr/0344 §1).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct TagFields {
    /// The object it tags, by its persistent id.
    pub of: String,
    /// Its middle.
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub at: Vec2,
    /// Its template; absent: its object's kind's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub content: Option<String>,
    #[serde(default, skip_serializing_if = "is_circle")]
    #[cfg_attr(feature = "ts", ts(as = "Option<TagShape>", optional))]
    pub shape: TagShape,
    /// Its words' height on paper, millimetres; absent: 2.5.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "paper-mm")))]
    pub size: Option<f64>,
    /// A line from its frame to the point it ties to.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub leader: bool,
    /// Its words: its template filled with its object's values, which the document writes.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<String>", optional))]
    pub text: String,
    /// The point of its object it ties to, which the document writes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub anchor: Option<Vec2>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub broken: Option<TagBreak>,
}

/// A tag (docs/adr/0344).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct TagEntity {
    #[serde(flatten)]
    #[cfg_attr(feature = "ts", ts(flatten))]
    pub base: EntityBase,
    #[serde(flatten)]
    #[cfg_attr(feature = "ts", ts(flatten))]
    pub tag: TagFields,
}

/// A piece of a template: words, or a place to fill.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TagPiece {
    Words(String),
    /// One of `TAG_PLACES`.
    Place(&'static str),
    /// `attr:Name`: the object's attribute so named.
    Attr(String),
}

/// A template's pieces; the words of what is wrong with it otherwise.
pub fn tag_template(t: &str) -> Result<Vec<TagPiece>, String> {
    let n = t.chars().count();
    if n > MAX_TAG_LETTERS {
        return Err(format!("Şablon {n} harf; en çok {MAX_TAG_LETTERS} olur."));
    }
    if t.chars().any(|c| c.is_control() && c != '\n') {
        return Err("Şablonda satır sonundan başka denetim karakteri olmaz.".into());
    }
    let lines = t.split('\n').count();
    if lines > MAX_TAG_LINES {
        return Err(format!(
            "Şablon {lines} satır; en çok {MAX_TAG_LINES} olur."
        ));
    }
    let mut out = Vec::new();
    let mut words = String::new();
    let mut rest = t;
    while let Some(i) = rest.find(['{', '}']) {
        words.push_str(&rest[..i]);
        if rest[i..].starts_with('}') {
            return Err(
                "Şablonda açılmamış “}” var; yer adları süslü ayraç içinde yazılır.".into(),
            );
        }
        let after = &rest[i + 1..];
        let Some(j) = after.find('}') else {
            return Err("Şablonda kapanmamış “{” var; yer adını “}” ile kapatın.".into());
        };
        let name = &after[..j];
        if name.contains('{') {
            return Err("Şablonda iç içe süslü ayraç olmaz.".into());
        }
        if !words.is_empty() {
            out.push(TagPiece::Words(std::mem::take(&mut words)));
        }
        if let Some(a) = name.strip_prefix("attr:") {
            if a.trim().is_empty() {
                return Err("“{attr:…}” bir özniteliğin adını ister.".into());
            }
            out.push(TagPiece::Attr(a.to_owned()));
        } else if let Some(p) = TAG_PLACES.iter().find(|p| **p == name) {
            out.push(TagPiece::Place(p));
        } else {
            return Err(format!(
                "Şablonda bilinmeyen yer “{{{name}}}”; yerler: {}, attr:Ad.",
                TAG_PLACES.join(", ")
            ));
        }
        rest = &after[j + 1..];
    }
    words.push_str(rest);
    if !words.is_empty() {
        out.push(TagPiece::Words(words));
    }
    Ok(out)
}

/// A template filled: each place by `value` (none: nothing), each line's ends trimmed, the empty lines dropped (a room
/// with no number shows its name and area, not a blank line over them: docs/adr/0344's 27 October addendum).
pub fn tag_fill(pieces: &[TagPiece], value: &dyn Fn(&TagPiece) -> Option<String>) -> String {
    let mut out = String::new();
    for p in pieces {
        match p {
            TagPiece::Words(w) => out.push_str(w),
            _ => out.push_str(&value(p).unwrap_or_default()),
        }
    }
    out.split('\n')
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

/// The template a tag of an object of `kind` takes when it names none; none: such an object is not tagged.
pub fn default_template(kind: &str) -> Option<&'static str> {
    Some(match kind {
        "opening" => "{number}",
        "space" => "{number}\n{name}\n{area}",
        "column" => "{tag}",
        "grid" => "{label}",
        "wall" => "{style}",
        "stair" => "{risers}",
        "ramp" => "{slope}",
        "insert" => "{attr:NO}",
        _ => return None,
    })
}

/// The frame a tag of an object of `kind` takes when it names none.
pub fn default_shape(kind: &str) -> TagShape {
    match kind {
        "opening" => TagShape::Hexagon,
        "space" => TagShape::Rectangle,
        _ => TagShape::Circle,
    }
}

type Problem = Option<(String, String)>;

impl TagFields {
    /// What is wrong with the tag: the field and the words; none: it holds.
    pub fn problem(&self) -> Problem {
        if self.of.trim().is_empty() {
            return Some(("of".into(), "Etiketin nesnesi verilmeli.".into()));
        }
        if !(self.at.x.is_finite() && self.at.y.is_finite()) {
            return Some((
                "at".into(),
                "Etiketin yerinin koordinatları sonlu olmalı.".into(),
            ));
        }
        if let Some(c) = &self.content {
            if c.trim().is_empty() {
                return Some((
                    "content".into(),
                    "Boş şablon yazılmaz; alanı kaldırın ya da bir şablon yazın.".into(),
                ));
            }
            if let Err(why) = tag_template(c) {
                return Some(("content".into(), why));
            }
        }
        if let Some(s) = self.size
            && !(s.is_finite() && s > 0.0 && s <= 50.0)
        {
            return Some((
                "size".into(),
                format!("Etiketin yazı yüksekliği {s}; sıfırdan büyük, en çok 50 mm olmalı."),
            ));
        }
        let n = self.text.chars().count();
        if n > MAX_TAG_LETTERS * 4 {
            return Some((
                "text".into(),
                format!("Etiketin yazısı {n} harf; çok uzun."),
            ));
        }
        if self.text.chars().any(|c| c.is_control() && c != '\n') {
            return Some((
                "text".into(),
                "Etiketin yazısında denetim karakteri olmaz.".into(),
            ));
        }
        if let Some(a) = self.anchor
            && !(a.x.is_finite() && a.y.is_finite())
        {
            return Some((
                "anchor".into(),
                "Bağ noktasının koordinatları sonlu olmalı.".into(),
            ));
        }
        None
    }

    /// Its words' height on paper, millimetres.
    pub fn size(&self) -> f64 {
        self.size.unwrap_or(TAG_SIZE_MM)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn templates_read_their_places() {
        assert_eq!(
            tag_template("D{number}\n{attr:Kat}").unwrap(),
            vec![
                TagPiece::Words("D".into()),
                TagPiece::Place("number"),
                TagPiece::Words("\n".into()),
                TagPiece::Attr("Kat".into()),
            ]
        );
        assert!(tag_template("{nummer}").is_err());
        assert!(tag_template("{number").is_err());
        assert!(tag_template("number}").is_err());
        assert!(tag_template("{attr:}").is_err());
    }

    #[test]
    fn a_fill_drops_its_empty_lines() {
        let p = tag_template("{number}\n{name}\n{area}").unwrap();
        let v = |p: &TagPiece| match p {
            TagPiece::Place("number") => Some("101".to_owned()),
            _ => None,
        };
        assert_eq!(tag_fill(&p, &v), "101");
        let v = |p: &TagPiece| match p {
            TagPiece::Place("area") => Some("36.00 m²".to_owned()),
            _ => None,
        };
        assert_eq!(tag_fill(&p, &v), "36.00 m²", "no blank lines over it");
    }
}

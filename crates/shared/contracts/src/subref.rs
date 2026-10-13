//! Places on objects and the references that name them (docs/adr/0249): what an associative dimension's points
//! follow. The rule that follows a place through an edit is the geometry core's (`ops::subref`).

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

use crate::EntityId;

/// The most parts, rings or vertices a place's index may name (a path's own limit is lower).
pub const SUBREF_INDEX_LIMIT: u32 = 10_000_000;

/// A place on an object.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "kind", rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum SubPlace {
    /// A point object's position: part 0 its own, `k` its `k`th extra point (docs/adr/0174).
    Node { part: u32 },
    /// A line's, arc's or elliptical arc's start.
    Start,
    /// Its end.
    End,
    /// A circle's, arc's or ellipse's centre.
    Center,
    /// A path's vertex: part 0 the object's own path, `k` its `k`th extra part; ring 0 the outer ring, `h` its
    /// `h`th hole; the vertex's index in the ring.
    Vertex { part: u32, ring: u32, index: u32 },
    /// The middle of a line (all zero) or of a path's edge from vertex `index` to the next.
    Mid { part: u32, ring: u32, index: u32 },
}

/// Why a reference broke (the core's `Reason`), as its stable code.
pub const SUBREF_BROKEN: [&str; 5] = [
    "object_gone",
    "kind_changed",
    "no_such_place",
    "vertex_gone",
    "ambiguous",
];

/// A place on an object by the object's persistent id: what a dimension's point follows.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SubRef {
    pub uid: EntityId,
    pub place: SubPlace,
    /// Why it no longer follows its object ([`SUBREF_BROKEN`]); none while it does. The point keeps its last
    /// coordinates.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub broken: Option<String>,
}

/// An associative dimension's references: what its points `a`, `b` and `c` follow; a point without one is free.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct DimensionRefs {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub a: Option<SubRef>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub b: Option<SubRef>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub c: Option<SubRef>,
}

impl SubPlace {
    /// What is wrong with the place, in the words a command's refusal says; none when it holds.
    pub fn problem(&self) -> Option<String> {
        let big = |n: u32| n > SUBREF_INDEX_LIMIT;
        match *self {
            SubPlace::Node { part } if big(part) => {
                Some(format!("Yerin parçası {part}: çok büyük."))
            }
            SubPlace::Vertex { part, ring, index } | SubPlace::Mid { part, ring, index }
                if big(part) || big(ring) || big(index) =>
            {
                Some("Yerin parçası, halkası ya da sırası çok büyük.".into())
            }
            _ => None,
        }
    }
}

impl SubRef {
    /// What is wrong with the reference; none when it holds.
    pub fn problem(&self) -> Option<String> {
        if self.uid.0 == [0; 16] {
            return Some("Başvurunun nesne kimliği boş.".into());
        }
        if let Some(b) = self
            .broken
            .as_deref()
            .filter(|b| !SUBREF_BROKEN.contains(b))
        {
            return Some(format!("Başvurunun kopma nedeni “{b}” bilinmiyor."));
        }
        self.place.problem()
    }

    /// Whether it follows its object (not broken).
    pub fn live(&self) -> bool {
        self.broken.is_none()
    }
}

impl DimensionRefs {
    /// The references by their point's name, in order.
    pub fn each(&self) -> impl Iterator<Item = (&'static str, &SubRef)> {
        [("a", &self.a), ("b", &self.b), ("c", &self.c)]
            .into_iter()
            .filter_map(|(k, r)| r.as_ref().map(|r| (k, r)))
    }

    /// Whether no point has a reference (then the field is not written).
    pub fn is_empty(&self) -> bool {
        self.a.is_none() && self.b.is_none() && self.c.is_none()
    }

    /// What is wrong with the references; none when they hold.
    pub fn problem(&self) -> Option<String> {
        if self.is_empty() {
            return Some("Ölçünün başvuruları boş: hiç başvuru yoksa alan yazılmaz.".into());
        }
        self.each()
            .find_map(|(k, r)| r.problem().map(|p| format!("{k}: {p}")))
    }
}

/// The Turkish words of a broken reference's code (docs/adr/0249 §3).
pub fn broken_words(code: &str) -> &'static str {
    match code {
        "object_gone" => "nesne silindi",
        "kind_changed" => "nesnenin türü değişti",
        "no_such_place" => "yer nesnede yok",
        "vertex_gone" => "köşe silindi ya da taşındı",
        "ambiguous" => "köşe birden çok yerde",
        _ => "bilinmeyen neden",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn r(broken: Option<&str>) -> SubRef {
        SubRef {
            uid: EntityId([7; 16]),
            place: SubPlace::Vertex {
                part: 0,
                ring: 0,
                index: 2,
            },
            broken: broken.map(str::to_owned),
        }
    }

    #[test]
    fn the_json_names_the_place_by_its_kind() {
        let text = serde_json::to_string(&r(None)).unwrap();
        assert!(
            text.contains(r#""place":{"kind":"vertex","part":0,"ring":0,"index":2}"#),
            "{text}"
        );
        assert!(!text.contains("broken"));
        let back: SubRef = serde_json::from_str(&text).unwrap();
        assert_eq!(back, r(None));
        let start: SubPlace = serde_json::from_str(r#"{"kind":"start"}"#).unwrap();
        assert_eq!(start, SubPlace::Start);
    }

    #[test]
    fn the_rules_say_what_is_wrong() {
        assert_eq!(r(None).problem(), None);
        assert_eq!(r(Some("vertex_gone")).problem(), None);
        assert!(r(Some("lost")).problem().unwrap().contains("bilinmiyor"));
        let mut z = r(None);
        z.uid = EntityId([0; 16]);
        assert!(z.problem().unwrap().contains("boş"));
        let mut big = r(None);
        big.place = SubPlace::Vertex {
            part: 0,
            ring: 0,
            index: SUBREF_INDEX_LIMIT + 1,
        };
        assert!(big.problem().is_some());
        assert!(DimensionRefs::default().problem().unwrap().contains("boş"));
        let refs = DimensionRefs {
            b: Some(r(None)),
            ..Default::default()
        };
        assert_eq!(refs.problem(), None);
        assert_eq!(refs.each().map(|(k, _)| k).collect::<Vec<_>>(), ["b"]);
    }
}

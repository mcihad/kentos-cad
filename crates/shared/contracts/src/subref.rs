//! Places on objects and the references that name them (docs/adr/0249, 0263): what an associative dimension's points
//! follow. The rule that follows a place through an edit is the geometry core's (`ops::subref`).

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

use crate::EntityId;

/// The most parts, rings or vertices a place's index may name (a path's own limit is lower).
pub const SUBREF_INDEX_LIMIT: u32 = 10_000_000;

/// The most levels a block path has (the block core's `MAX_DEPTH`, docs/adr/0263 §3).
pub const SUBREF_PATH_DEPTH: usize = 16;

/// A place on an object.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "kind", rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum SubPlace {
    /// A point object's position: part 0 its own, `k` its `k`th extra point (docs/adr/0174).
    Node {
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "index")))]
        part: u32,
    },
    /// A line's, arc's or elliptical arc's start; an opening's start jamb midpoint.
    Start,
    /// Its end; an opening's end jamb midpoint.
    End,
    /// A circle's, arc's or ellipse's centre; an opening's jamb midpoints' midpoint.
    Center,
    /// A path's vertex: part 0 the object's own path, `k` its `k`th extra part; ring 0 the outer ring, `h` its
    /// `h`th hole; the vertex's index in the ring. On an opening, part/ring 0 and index 0..3 name
    /// start-left, start-right, end-left, end-right jamb corners (host direction); no symbol pieces.
    Vertex {
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "index")))]
        part: u32,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "index")))]
        ring: u32,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "index")))]
        index: u32,
    },
    /// The middle of a line (all zero) or of a path's edge from vertex `index` to the next.
    Mid {
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "index")))]
        part: u32,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "index")))]
        ring: u32,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "index")))]
        index: u32,
    },
    /// A point along an edge or a curve (docs/adr/0263 §1): a line's (all zero) or a path's edge from vertex `index`
    /// to the next, at the fraction `t` of its chord or of its arc's sweep; an arc's (`a0 + t·sweep`), a circle's
    /// (`t·2π` from east) or an ellipse's (`t0 + t·sweep`) curve (all zero). `t` within 0 and 1.
    Along {
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "index")))]
        part: u32,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "index")))]
        ring: u32,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "index")))]
        index: u32,
        #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
        t: f64,
    },
}

/// Why a reference broke (the core's `Reason`), as its stable code.
pub const SUBREF_BROKEN: [&str; 6] = [
    "object_gone",
    "kind_changed",
    "no_such_place",
    "vertex_gone",
    "ambiguous",
    "piece_gone",
];

/// A place on an object by the object's persistent id: what a dimension's point follows.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SubRef {
    pub uid: EntityId,
    pub place: SubPlace,
    /// When `uid` is an insert, the piece inside it the place is on (docs/adr/0263 §3): each number the object's
    /// place in that level's definition, every one but the last an insert; empty (not written) for the object itself.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<u32>>", optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "index")))]
    pub path: Vec<u32>,
    /// Why it no longer follows its object ([`SUBREF_BROKEN`]); none while it does. The point keeps its last
    /// coordinates.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub broken: Option<String>,
}

/// How an associative dimension's points that no place names follow (docs/adr/0320 §1).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum DimensionRule {
    /// A radius's, a diameter's, a jogged radius's, an arc length's or an arc's angle: the bound points are on one
    /// curve (a circle, an arc, a path's arc segment); its centre (a radius's `a`, the others' `c`) is the curve's.
    Curve,
    /// An angle between two straight edges: `a` and `b` name the edges its arms lie on; its vertex `c` is where
    /// their lines cross, each arm keeping its length and side.
    Edges,
    /// An axis between two straight edges (docs/adr/0322): `a` and `b` name the edges; the axis runs from the middle
    /// of their starts to the middle of their ends, the second edge turned to run the first's way.
    Pair,
}

impl DimensionRule {
    /// Its name on the wire.
    pub fn name(self) -> &'static str {
        match self {
            DimensionRule::Curve => "curve",
            DimensionRule::Edges => "edges",
            DimensionRule::Pair => "pair",
        }
    }
}

/// An associative dimension's references: what its points `a`, `b` and `c` follow; a point without one is free.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
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
    /// How the points no place names follow (docs/adr/0320 §1); none, each bound point follows its place alone.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub rule: Option<DimensionRule>,
}

impl SubPlace {
    /// What is wrong with the place, in the words a command's refusal says; none when it holds.
    pub fn problem(&self) -> Option<String> {
        let big = |n: u32| n > SUBREF_INDEX_LIMIT;
        match *self {
            SubPlace::Node { part } if big(part) => {
                Some(format!("Yerin parçası {part}: çok büyük."))
            }
            SubPlace::Vertex { part, ring, index }
            | SubPlace::Mid { part, ring, index }
            | SubPlace::Along {
                part, ring, index, ..
            } if big(part) || big(ring) || big(index) => {
                Some("Yerin parçası, halkası ya da sırası çok büyük.".into())
            }
            SubPlace::Along { t, .. } if !(0.0..=1.0).contains(&t) => {
                Some(format!("Yerin t'si {t}: 0 ile 1 arasında olmalı."))
            }
            _ => None,
        }
    }
}

/// What is wrong with a block path (docs/adr/0263 §3); none when it holds. Empty is the object itself.
pub fn path_problem(path: &[u32]) -> Option<String> {
    if path.len() > SUBREF_PATH_DEPTH {
        return Some(format!(
            "Blok yolu {} düzey; en çok {SUBREF_PATH_DEPTH} olur.",
            path.len()
        ));
    }
    path.iter()
        .any(|&i| i > SUBREF_INDEX_LIMIT)
        .then(|| "Blok yolunun bir sırası çok büyük.".into())
}

/// A range of an object (docs/adr/0263 §2): from one `along` place to another on the same part and ring, in the
/// object's direction; around a closed ring, a circle or a whole ellipse when `to` comes first. No object keeps one
/// yet; the rule and its length are the core's (`ops::subref::range_length`, `follow_range`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct SubRange {
    pub uid: EntityId,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<u32>>", optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "index")))]
    pub path: Vec<u32>,
    pub from: SubPlace,
    pub to: SubPlace,
}

impl SubRange {
    /// What is wrong with the range; none when it holds.
    pub fn problem(&self) -> Option<String> {
        if self.uid.0 == [0; 16] {
            return Some("Aralığın nesne kimliği boş.".into());
        }
        if let Some(p) = path_problem(&self.path) {
            return Some(p);
        }
        match (self.from, self.to) {
            (
                SubPlace::Along {
                    part: p0, ring: r0, ..
                },
                SubPlace::Along {
                    part: p1, ring: r1, ..
                },
            ) => {
                if (p0, r0) != (p1, r1) {
                    return Some("Aralığın iki ucu aynı parçada ve halkada olmalı.".into());
                }
                self.from.problem().or_else(|| self.to.problem())
            }
            _ => Some("Aralığın uçları kenar ya da eğri üstündeki yerlerdir (along).".into()),
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
        self.place.problem().or_else(|| path_problem(&self.path))
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

    /// What is wrong with the rule for a dimension of `style` (docs/adr/0320 §1); none when it fits: a curve's on a
    /// radius, a diameter or a jogged radius (`b` bound, not `a`, the centre), on an arc length or an angle (`a` and
    /// `b` bound, not `c`); edges on an angle (`a` and `b` bound, not `c`).
    pub fn rule_problem(&self, style: Option<crate::DimensionStyle>) -> Option<String> {
        use crate::DimensionStyle as S;
        let rule = self.rule?;
        let radial = matches!(
            style,
            Some(S::Radius | S::Diameter | S::Jogged | S::CenterMark)
        );
        let fits = match rule {
            DimensionRule::Curve => radial || matches!(style, Some(S::ArcLength | S::Angular)),
            DimensionRule::Edges => style == Some(S::Angular),
            DimensionRule::Pair => style == Some(S::CenterLine),
        };
        if !fits {
            return Some(format!(
                "“{}” kuralı bu ölçü türüne uymaz: eğri kuralı yarıçap, çap, kırıklı yarıçap, yay uzunluğu, açı ve merkez işaretinde, kenar kuralı açıda, çift kuralı eksen çizgisinde olur.",
                rule.name()
            ));
        }
        let bound = (radial && self.b.is_some() && self.a.is_none())
            || (!radial && self.a.is_some() && self.b.is_some() && self.c.is_none());
        (!bound).then(|| {
            if radial {
                "Eğri kuralındaki yarıçap ölçüsünde yalnız b (eğrinin üstündeki nokta) bağlanır; merkez eğriden gelir.".into()
            } else {
                "Kuralın a ve b'si bağlı olmalı; üçüncü nokta (c) kuraldan gelir.".into()
            }
        })
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
        "piece_gone" => "blok parçası yok",
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
            path: Vec::new(),
            broken: broken.map(str::to_owned),
        }
    }

    #[test]
    fn along_places_paths_and_ranges_say_what_is_wrong() {
        let along = |t: f64| SubPlace::Along {
            part: 0,
            ring: 0,
            index: 1,
            t,
        };
        assert!(along(0.5).problem().is_none());
        assert!(along(1.5).problem().unwrap().contains("0 ile 1"));
        assert!(along(f64::NAN).problem().is_some());
        let text = serde_json::to_string(&SubRef {
            path: vec![1, 0],
            place: along(0.25),
            ..r(None)
        })
        .unwrap();
        assert!(text.contains(r#""path":[1,0]"#), "{text}");
        assert!(
            text.contains(r#""kind":"along","part":0,"ring":0,"index":1,"t":0.25"#),
            "{text}"
        );
        assert!(!serde_json::to_string(&r(None)).unwrap().contains("path"));
        assert!(path_problem(&[0; 17]).unwrap().contains("17 düzey"));
        assert!(path_problem(&[0; 16]).is_none());
        let range = |from, to| SubRange {
            uid: EntityId([7; 16]),
            path: Vec::new(),
            from,
            to,
        };
        assert!(range(along(0.1), along(0.9)).problem().is_none());
        assert!(range(SubPlace::Start, along(0.9)).problem().is_some());
        let other_ring = SubPlace::Along {
            part: 0,
            ring: 1,
            index: 0,
            t: 0.5,
        };
        assert!(
            range(along(0.1), other_ring)
                .problem()
                .unwrap()
                .contains("halkada")
        );
        assert_eq!(broken_words("piece_gone"), "blok parçası yok");
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

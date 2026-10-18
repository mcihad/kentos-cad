//! View marks (docs/adr/0345 §1): the section, elevation and detail marks on a plan, each pointing at the view that
//! shows what it marks: a named view of the project or a map frame on a sheet. The bubble's top half holds the mark's
//! number, its bottom half the target's number (the sheet's name, the named view's name), which the document writes
//! from the target (`words`); a target gone leaves the last words and `broken`.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

use crate::entity::{EntityBase, Vec2};

/// A mark's words' height on paper by default, millimetres.
pub const VIEWMARK_SIZE_MM: f64 = 2.5;
/// The most letters a mark's number holds.
pub const MAX_MARK_LETTERS: usize = 16;
/// The most points a section's cut line holds.
pub const MAX_CUT_POINTS: usize = 32;
/// The bottom half's words when a mark points at nothing.
pub const NO_TARGET: &str = "—";

/// What a view mark marks.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum ViewMarkForm {
    /// Kesit: a cut line with a bubble at each end, looking to one side.
    #[default]
    Section,
    /// Görünüş: a bubble looking one way.
    Elevation,
    /// Detay: a circle around the detail and a bubble on a leader.
    Detail,
}

impl ViewMarkForm {
    pub const ALL: [ViewMarkForm; 3] = [
        ViewMarkForm::Section,
        ViewMarkForm::Elevation,
        ViewMarkForm::Detail,
    ];

    pub fn label(self) -> &'static str {
        match self {
            ViewMarkForm::Section => "Kesit",
            ViewMarkForm::Elevation => "Görünüş",
            ViewMarkForm::Detail => "Detay",
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            ViewMarkForm::Section => "section",
            ViewMarkForm::Elevation => "elevation",
            ViewMarkForm::Detail => "detail",
        }
    }
}

/// What a mark points at: a named view (`view`, its id), or a map frame on a sheet (`sheet` and `item`, their ids).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ViewTarget {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub view: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub sheet: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub item: Option<String>,
}

impl ViewTarget {
    /// A named view, by its id.
    pub fn view(id: &str) -> ViewTarget {
        ViewTarget {
            view: Some(id.to_owned()),
            ..ViewTarget::default()
        }
    }

    /// A map frame on a sheet.
    pub fn sheet(sheet: &str, item: &str) -> ViewTarget {
        ViewTarget {
            sheet: Some(sheet.to_owned()),
            item: Some(item.to_owned()),
            ..ViewTarget::default()
        }
    }

    /// What is wrong with it: a view alone, or a sheet with its item.
    pub fn problem(&self) -> Option<String> {
        let blank = |s: &Option<String>| s.as_ref().is_some_and(|s| s.trim().is_empty());
        if blank(&self.view) || blank(&self.sheet) || blank(&self.item) {
            return Some("Hedefin kimlikleri boş olamaz.".into());
        }
        match (&self.view, &self.sheet, &self.item) {
            (Some(_), None, None) | (None, Some(_), Some(_)) => None,
            _ => Some(
                "Hedef ya bir adlı görünümdür (view) ya da bir paftadaki görünüm penceresi (sheet ve item).".into(),
            ),
        }
    }
}

/// The bubble's words, which the document writes from the target.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ViewMarkLabel {
    /// The top half: the mark's number.
    pub top: String,
    /// The bottom half: the target's number.
    pub bottom: String,
}

impl ViewMarkLabel {
    fn is_empty(&self) -> bool {
        self.top.is_empty() && self.bottom.is_empty()
    }
}

/// Why a mark's words are no longer its target's.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum ViewMarkBreak {
    /// Its named view or sheet frame is gone.
    TargetGone,
}

impl ViewMarkBreak {
    pub fn words(self) -> &'static str {
        "İşaretin hedefi (adlı görünüm ya da paftadaki pencere) yok; işaret son yazısıyla kaldı. Başvurular'dan yeniden bağlayın ya da bağı kopartın."
    }
}

/// A view mark's own fields (docs/adr/0345 §1).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ViewMarkFields {
    pub form: ViewMarkForm,
    /// A section's cut line (2 or more points); an elevation's bubble, a detail's circle's middle (one point).
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub points: Vec<Vec2>,
    /// A detail's circle's radius.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub radius: Option<f64>,
    /// A detail's bubble's middle.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "length")))]
    pub at: Option<Vec2>,
    /// An elevation's looking direction, degrees from east, counter-clockwise; absent: 90 (north).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "deg")))]
    pub angle: Option<f64>,
    /// A section looks to the right of its cut line's direction (absent: the left).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub flip: bool,
    /// The mark's number; empty: a sheet frame's order on its sheet.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<String>", optional))]
    pub mark: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub target: Option<ViewTarget>,
    /// Its words' height on paper, millimetres; absent: 2.5.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "paper-mm")))]
    pub size: Option<f64>,
    /// The bubble's words, which the document writes.
    #[serde(default, skip_serializing_if = "ViewMarkLabel::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<ViewMarkLabel>", optional))]
    pub words: ViewMarkLabel,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub broken: Option<ViewMarkBreak>,
}

/// A view mark (docs/adr/0345).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ViewMarkEntity {
    #[serde(flatten)]
    #[cfg_attr(feature = "ts", ts(flatten))]
    pub base: EntityBase,
    #[serde(flatten)]
    #[cfg_attr(feature = "ts", ts(flatten))]
    pub mark: ViewMarkFields,
}

type Problem = Option<(String, String)>;

fn words_problem(field: &str, s: &str, most: usize) -> Problem {
    let n = s.chars().count();
    if n > most {
        return Some((field.into(), format!("{n} harf; en çok {most} olur.")));
    }
    if s.chars().any(char::is_control) {
        return Some((field.into(), "Denetim karakteri olmaz.".into()));
    }
    None
}

impl ViewMarkFields {
    /// What is wrong with the mark: the field and the words; none: it holds.
    pub fn problem(&self) -> Problem {
        let finite = |p: &Vec2| p.x.is_finite() && p.y.is_finite();
        if !self.points.iter().all(finite) {
            return Some((
                "points".into(),
                "İşaretin noktalarının koordinatları sonlu olmalı.".into(),
            ));
        }
        match self.form {
            ViewMarkForm::Section => {
                if self.points.len() < 2 || self.points.len() > MAX_CUT_POINTS {
                    return Some((
                        "points".into(),
                        format!(
                            "Kesitin kesme çizgisi 2 ile {MAX_CUT_POINTS} arası nokta ister, {} verildi.",
                            self.points.len()
                        ),
                    ));
                }
                if self.points.windows(2).any(|w| w[0] == w[1]) {
                    return Some((
                        "points".into(),
                        "Kesme çizgisinin art arda iki noktası aynı olamaz.".into(),
                    ));
                }
            }
            ViewMarkForm::Elevation | ViewMarkForm::Detail => {
                if self.points.len() != 1 {
                    return Some((
                        "points".into(),
                        format!(
                            "{} tek nokta ister, {} verildi.",
                            self.form.label(),
                            self.points.len()
                        ),
                    ));
                }
            }
        }
        if self.form == ViewMarkForm::Detail {
            match self.radius {
                Some(r) if r.is_finite() && r > 0.0 => {}
                _ => {
                    return Some((
                        "radius".into(),
                        "Detayın sınırının yarıçapı sıfırdan büyük olmalı.".into(),
                    ));
                }
            }
        } else if self.radius.is_some() {
            return Some((
                "radius".into(),
                "Yarıçap yalnız detayın sınırınındır.".into(),
            ));
        }
        if let Some(a) = self.at
            && !finite(&a)
        {
            return Some((
                "at".into(),
                "Balonun yerinin koordinatları sonlu olmalı.".into(),
            ));
        }
        if let Some(a) = self.angle
            && !a.is_finite()
        {
            return Some(("angle".into(), "Bakış yönü sonlu olmalı.".into()));
        }
        if let Some(why) = words_problem("mark", &self.mark, MAX_MARK_LETTERS) {
            return Some(why);
        }
        if let Some(s) = self.size
            && !(s.is_finite() && s > 0.0 && s <= 50.0)
        {
            return Some((
                "size".into(),
                format!("İşaretin yazı yüksekliği {s}; sıfırdan büyük, en çok 50 mm olmalı."),
            ));
        }
        if let Some(t) = &self.target
            && let Some(why) = t.problem()
        {
            return Some(("target".into(), why));
        }
        for (f, s) in [("words.top", &self.words.top), ("words.bottom", &self.words.bottom)] {
            if let Some(why) = words_problem(f, s, 128) {
                return Some(why);
            }
        }
        None
    }

    /// Its words' height on paper, millimetres.
    pub fn size(&self) -> f64 {
        self.size.unwrap_or(VIEWMARK_SIZE_MM)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn section(points: Vec<Vec2>) -> ViewMarkFields {
        ViewMarkFields {
            form: ViewMarkForm::Section,
            points,
            radius: None,
            at: None,
            angle: None,
            flip: false,
            mark: "1".into(),
            target: None,
            size: None,
            words: ViewMarkLabel::default(),
            broken: None,
        }
    }

    #[test]
    fn a_section_needs_two_points() {
        let p = |x, y| Vec2 { x, y };
        assert!(section(vec![p(0.0, 0.0)]).problem().is_some());
        assert!(section(vec![p(0.0, 0.0), p(0.0, 0.0)]).problem().is_some());
        assert!(section(vec![p(0.0, 0.0), p(5.0, 0.0)]).problem().is_none());
    }

    #[test]
    fn a_target_is_a_view_or_a_sheet_frame() {
        assert!(ViewTarget::view("v-1").problem().is_none());
        assert!(ViewTarget::sheet("s", "i").problem().is_none());
        let both = ViewTarget {
            view: Some("v".into()),
            sheet: Some("s".into()),
            item: Some("i".into()),
        };
        assert!(both.problem().is_some());
        let half = ViewTarget {
            sheet: Some("s".into()),
            ..ViewTarget::default()
        };
        assert!(half.problem().is_some());
    }
}

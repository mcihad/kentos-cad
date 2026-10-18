//! Schedules (docs/adr/0344 §3): a table whose rows are the drawing's openings, spaces, columns or walls, one row each
//! (or one row for those alike when grouped), written again by the document at every step. A column reads one field of
//! them; one that is the objects' own field (a number, a type, a sill …) is edited in the schedule and written to the
//! objects, one that is worked out (a count, an area, a height …) is not.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

use crate::{ArchPhase, OpeningForm};

/// The most columns a schedule has.
pub const MAX_SCHEDULE_COLUMNS: usize = 30;

/// What a schedule lists.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum ScheduleOf {
    #[default]
    Openings,
    Spaces,
    Columns,
    Walls,
}

impl ScheduleOf {
    pub const ALL: [ScheduleOf; 4] = [
        ScheduleOf::Openings,
        ScheduleOf::Spaces,
        ScheduleOf::Columns,
        ScheduleOf::Walls,
    ];

    pub fn label(self) -> &'static str {
        match self {
            ScheduleOf::Openings => "Kapı ve pencere çizelgesi",
            ScheduleOf::Spaces => "Mahal çizelgesi",
            ScheduleOf::Columns => "Kolon çizelgesi",
            ScheduleOf::Walls => "Duvar çizelgesi",
        }
    }

    /// The object kind it lists, as the contract names it.
    pub fn kind(self) -> &'static str {
        match self {
            ScheduleOf::Openings => "opening",
            ScheduleOf::Spaces => "space",
            ScheduleOf::Columns => "column",
            ScheduleOf::Walls => "wall",
        }
    }

    /// Its columns when none are given.
    pub fn default_columns(self) -> Vec<ScheduleColumn> {
        use ScheduleField::*;
        let fields: &[ScheduleField] = match self {
            ScheduleOf::Openings => &[Number, Form, Type, Width, Height, Sill, Wall],
            ScheduleOf::Spaces => &[Number, Name, Usage, Level, Area, Perimeter, Height],
            ScheduleOf::Columns => &[Tag, Place, Type, Width, Level],
            ScheduleOf::Walls => &[Type, Thickness, Length, Height, Area, Volume],
        };
        fields
            .iter()
            .map(|&field| ScheduleColumn {
                field,
                title: None,
                attr: None,
            })
            .collect()
    }
}

/// What one column of a schedule reads.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum ScheduleField {
    #[default]
    Number,
    Type,
    Form,
    Width,
    Height,
    Sill,
    Head,
    Wall,
    Level,
    Phase,
    Count,
    Area,
    Perimeter,
    Volume,
    Name,
    Usage,
    Tag,
    Place,
    Length,
    Thickness,
    /// The attribute the column's `attr` names.
    Attr,
}

impl ScheduleField {
    pub const ALL: [ScheduleField; 21] = [
        ScheduleField::Number,
        ScheduleField::Type,
        ScheduleField::Form,
        ScheduleField::Width,
        ScheduleField::Height,
        ScheduleField::Sill,
        ScheduleField::Head,
        ScheduleField::Wall,
        ScheduleField::Level,
        ScheduleField::Phase,
        ScheduleField::Count,
        ScheduleField::Area,
        ScheduleField::Perimeter,
        ScheduleField::Volume,
        ScheduleField::Name,
        ScheduleField::Usage,
        ScheduleField::Tag,
        ScheduleField::Place,
        ScheduleField::Length,
        ScheduleField::Thickness,
        ScheduleField::Attr,
    ];

    /// Its heading when the column gives none.
    pub fn title(self) -> &'static str {
        match self {
            ScheduleField::Number => "No",
            ScheduleField::Type => "Tür",
            ScheduleField::Form => "Biçim",
            ScheduleField::Width => "Genişlik",
            ScheduleField::Height => "Yükseklik",
            ScheduleField::Sill => "Denizlik",
            ScheduleField::Head => "Lento",
            ScheduleField::Wall => "Duvar",
            ScheduleField::Level => "Kat",
            ScheduleField::Phase => "Evre",
            ScheduleField::Count => "Adet",
            ScheduleField::Area => "Alan",
            ScheduleField::Perimeter => "Çevre",
            ScheduleField::Volume => "Hacim",
            ScheduleField::Name => "Ad",
            ScheduleField::Usage => "Kullanım",
            ScheduleField::Tag => "Etiket",
            ScheduleField::Place => "Konum",
            ScheduleField::Length => "Uzunluk",
            ScheduleField::Thickness => "Kalınlık",
            ScheduleField::Attr => "Öznitelik",
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            ScheduleField::Number => "number",
            ScheduleField::Type => "type",
            ScheduleField::Form => "form",
            ScheduleField::Width => "width",
            ScheduleField::Height => "height",
            ScheduleField::Sill => "sill",
            ScheduleField::Head => "head",
            ScheduleField::Wall => "wall",
            ScheduleField::Level => "level",
            ScheduleField::Phase => "phase",
            ScheduleField::Count => "count",
            ScheduleField::Area => "area",
            ScheduleField::Perimeter => "perimeter",
            ScheduleField::Volume => "volume",
            ScheduleField::Name => "name",
            ScheduleField::Usage => "usage",
            ScheduleField::Tag => "tag",
            ScheduleField::Place => "place",
            ScheduleField::Length => "length",
            ScheduleField::Thickness => "thickness",
            ScheduleField::Attr => "attr",
        }
    }

    /// Whether a schedule of `of` may read it.
    pub fn reads(self, of: ScheduleOf) -> bool {
        use ScheduleField::*;
        match self {
            Count | Level | Phase | Attr => true,
            Number => matches!(of, ScheduleOf::Openings | ScheduleOf::Spaces),
            Type => matches!(
                of,
                ScheduleOf::Openings | ScheduleOf::Columns | ScheduleOf::Walls
            ),
            Width => matches!(of, ScheduleOf::Openings | ScheduleOf::Columns),
            Height => matches!(
                of,
                ScheduleOf::Openings | ScheduleOf::Spaces | ScheduleOf::Walls
            ),
            Area | Volume => {
                matches!(
                    of,
                    ScheduleOf::Openings | ScheduleOf::Spaces | ScheduleOf::Walls
                ) && !(self == Volume && of == ScheduleOf::Openings)
            }
            Form | Sill | Head | Wall => of == ScheduleOf::Openings,
            Name | Usage | Perimeter => of == ScheduleOf::Spaces,
            Tag | Place => of == ScheduleOf::Columns,
            Length | Thickness => of == ScheduleOf::Walls,
        }
    }

    /// Whether its cell is the objects' own field, edited in the schedule; otherwise it is worked out.
    pub fn editable(self) -> bool {
        use ScheduleField::*;
        matches!(
            self,
            Number | Type | Width | Sill | Head | Name | Usage | Tag | Thickness | Attr
        )
    }

    /// Whether a grouped row and the total add it up rather than share it.
    pub fn summed(self) -> bool {
        use ScheduleField::*;
        matches!(self, Count | Area | Perimeter | Volume | Length)
    }

    /// Whether it is a length the project's unit writes.
    pub fn length(self) -> bool {
        use ScheduleField::*;
        matches!(
            self,
            Width | Height | Sill | Head | Perimeter | Length | Thickness
        )
    }
}

/// One column of a schedule.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ScheduleColumn {
    pub field: ScheduleField,
    /// Its heading; absent: the field's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub title: Option<String>,
    /// The attribute an `attr` column reads.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub attr: Option<String>,
}

impl ScheduleColumn {
    /// Its heading.
    pub fn heading(&self) -> String {
        match (&self.title, &self.attr) {
            (Some(t), _) => t.clone(),
            (None, Some(a)) if self.field == ScheduleField::Attr => a.clone(),
            _ => self.field.title().to_owned(),
        }
    }
}

/// Which objects a schedule lists: those of every list given.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ScheduleFilter {
    /// Their levels' ids (an opening's its wall's).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub levels: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub phases: Option<Vec<ArchPhase>>,
    /// An openings schedule's forms.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub forms: Option<Vec<OpeningForm>>,
    /// Their layers' ids.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub layers: Option<Vec<String>>,
}

impl ScheduleFilter {
    pub fn is_empty(&self) -> bool {
        self.levels.is_none()
            && self.phases.is_none()
            && self.forms.is_none()
            && self.layers.is_none()
    }
}

/// A sort key: a column by its index, in order or reversed.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ScheduleSort {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "index")))]
    pub column: u32,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub descending: bool,
}

/// What is wrong with a schedule's spec: the field and the words; none: it holds.
pub fn schedule_problem(
    of: ScheduleOf,
    columns: &[ScheduleColumn],
    sort: &[ScheduleSort],
    filter: Option<&ScheduleFilter>,
) -> Option<(String, String)> {
    if columns.is_empty() || columns.len() > MAX_SCHEDULE_COLUMNS {
        return Some((
            "columns".into(),
            format!(
                "Çizelgenin 1 ile {MAX_SCHEDULE_COLUMNS} arası sütunu olur; {} verildi.",
                columns.len()
            ),
        ));
    }
    for (i, c) in columns.iter().enumerate() {
        if !c.field.reads(of) {
            return Some((
                format!("columns[{i}].field"),
                format!(
                    "“{}” {} okunmaz.",
                    c.field.title(),
                    of.label().to_lowercase() + "nde"
                ),
            ));
        }
        match (&c.attr, c.field == ScheduleField::Attr) {
            (None, true) => {
                return Some((
                    format!("columns[{i}].attr"),
                    "Öznitelik sütunu okuyacağı özniteliğin adını ister.".into(),
                ));
            }
            (Some(_), false) => {
                return Some((
                    format!("columns[{i}].attr"),
                    "Öznitelik adı yalnız öznitelik sütununda verilir.".into(),
                ));
            }
            (Some(a), true) if a.trim().is_empty() => {
                return Some((
                    format!("columns[{i}].attr"),
                    "Özniteliğin adı boş olamaz.".into(),
                ));
            }
            _ => {}
        }
        if let Some(t) = &c.title
            && (t.trim().is_empty() || t.chars().any(char::is_control) || t.chars().count() > 100)
        {
            return Some((
                format!("columns[{i}].title"),
                "Sütun başlığı boş olamaz, tek satırdır, en çok 100 harftir.".into(),
            ));
        }
    }
    for (i, s) in sort.iter().enumerate() {
        if s.column as usize >= columns.len() {
            return Some((
                format!("sort[{i}].column"),
                format!(
                    "Sıralama {}. sütunu istiyor; çizelgenin {} sütunu var.",
                    s.column + 1,
                    columns.len()
                ),
            ));
        }
    }
    if let Some(f) = filter
        && f.forms.is_some()
        && of != ScheduleOf::Openings
    {
        return Some((
            "filter.forms".into(),
            "Biçim süzgeci yalnız kapı ve pencere çizelgesindedir.".into(),
        ));
    }
    None
}

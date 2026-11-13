//! What Öznitelikler shows, as data (the web's `PropertiesPanel.ts`): the
//! summary over the grid, the header's meta, and the sections and rows for
//! no selection, one object and several, each row with its editor. The view
//! draws them (`properties::view`); the edits come back as `Event`s.

use std::borrow::Cow;

use kentos_commands::{DimensionStyle, Entity, GradientShape, HatchPatternType, TableSource};
use kentos_domain::{LayerTree, Slot};
use kentos_interaction::elevation::{self, Summary as Elevations};
use kentos_interaction::{
    Format, Vec2, angle_deg, arc_sweep, bearing_grad, dimension_layout, dist, fixed, full_ellipse,
    measures,
};
use kentos_interaction::{leader, text};
use kentos_ui::icon::Icon;

use super::{ArchField, Event, Field, Spot};

mod arch;
pub(crate) mod attachments;
pub(crate) mod casework;
mod opening_types;
mod materials;
mod modules;
mod elevations;
mod building_sections;
pub(crate) mod cloud;
mod dimension;
mod dimension_set;
pub(crate) mod params;
pub(crate) mod raster;

// A kind's own rows, out of `entity_sections` (docs/adr/8054 §7).
mod opening;
mod slab;
mod space;
mod wall;

use crate::app::Message;
use crate::document::Document;
use crate::selecting::kind_title;
pub(super) use dimension::is_y_axis;

/// The colours the panel offers (the web's `DRAW_COLORS`, fields.ts).
use crate::ribbon_panels::{DRAW_COLORS, LINE_WEIGHTS, weight_text as weight_label};

/// The panel's content.
#[derive(Clone)]
pub(crate) struct Panel {
    pub summary: Summary,
    /// The header's meta: `#12`, `3 nesne`.
    pub meta: Option<String>,
    pub sections: Vec<Section>,
    /// All selected types; a property-only filter never changes scene selection.
    pub types: Vec<(&'static str, usize)>,
    pub total: usize,
    pub editable: usize,
}

/// Over the grid: the object's kind and label and its layer, or the selection's size and kinds.
#[derive(Clone)]
pub(crate) enum Summary {
    /// Nothing selected: “Seçili nesne yok” and how to select.
    Empty,
    One {
        kind: &'static str,
        label: Option<String>,
        /// The layer's colour value and path.
        layer: Option<(String, String)>,
    },
    Many {
        count: usize,
        kinds: String,
    },
}

/// A section of the grid; its id keeps it closed while the app runs.
#[derive(Clone)]
pub(crate) struct Section {
    pub id: Cow<'static, str>,
    /// Its heading; a count in some (“Yazılar (3)”).
    pub title: Cow<'static, str>,
    pub rows: Vec<Row>,
}

/// A row: its name, its value as shown, and how it is edited, if it is.
#[derive(Clone)]
pub(crate) struct Row {
    pub label: Cow<'static, str>,
    pub value: String,
    /// Figures: tabular.
    pub numeric: bool,
    pub unit: Option<Cow<'static, str>>,
    pub editor: Option<Editor>,
    /// A note under the row above it, quiet: no name, no cell (Kot's “(bazı
    /// köşeler kotsuz)”, docs/adr/0142).
    pub note: bool,
    /// The note warns (a value its layer's field refuses, docs/adr/0199 §5):
    /// in the warning colour.
    pub warn: bool,
    /// A picture before the cell: an attachment's photo or its kind's icon,
    /// which opens it (docs/adr/0225 §2).
    pub thumb: Option<Thumb>,
}

/// An attachment's picture in its row.
#[derive(Clone)]
pub(crate) struct Thumb {
    /// The photo's key in the store (`attachments::photo_key`); none: the icon.
    pub key: Option<String>,
    pub icon: &'static str,
    /// Where it is, under the pointer.
    pub title: String,
    pub open: Message,
}

/// How a row is edited.
#[derive(Clone)]
pub(crate) enum Editor {
    /// A text cell: the committed text goes to the field.
    Text(Field),
    /// A number cell; the field reads the number as the web does.
    Number(Field),
    /// A drop-down: what it shows (a colour value for the swatch, or a web
    /// icon's name before it) and its choices.
    Select {
        text: String,
        swatch: Option<String>,
        icon: Option<&'static str>,
        items: Vec<Choice>,
    },
}

/// One entry of a drop-down.
#[derive(Clone)]
pub(crate) enum Choice {
    /// One of a group; a colour value for its swatch, or a web icon's name
    /// (a text's alignment); not enabled when locked.
    Pick {
        label: String,
        swatch: Option<String>,
        icon: Option<&'static str>,
        /// The icon is a wide sample, drawn as one (a hatch pattern's,
        /// docs/adr/0186 §11): one more flag, not another name, keeps the
        /// choice small.
        wide: bool,
        chosen: bool,
        enabled: bool,
        message: Message,
    },
    Separator,
    /// A command, with its icon.
    Command {
        label: &'static str,
        icon: Icon,
        id: &'static str,
    },
    /// An item that sends its message, with a web icon's name; none: not enabled.
    Action {
        label: &'static str,
        icon: &'static str,
        message: Option<Message>,
    },
}

impl Choice {
    /// A pick with a wide sample in its icon's place in the menu (a hatch
    /// pattern's, docs/adr/0186 §11).
    fn previewed(mut self, name: &'static str) -> Self {
        if let Choice::Pick { icon, wide, .. } = &mut self {
            *icon = Some(name);
            *wide = true;
        }
        self
    }
}

impl Row {
    pub(super) fn text(label: impl Into<Cow<'static, str>>, value: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            value: value.into(),
            numeric: false,
            unit: None,
            editor: None,
            note: false,
            warn: false,
            thumb: None,
        }
    }

    /// A quiet line under the row above it, with no name of its own.
    pub(super) fn note(text: &str) -> Self {
        Self {
            note: true,
            ..Self::text("", text)
        }
    }

    fn figure(label: impl Into<Cow<'static, str>>, value: impl Into<String>) -> Self {
        Self {
            numeric: true,
            ..Self::text(label, value)
        }
    }

    fn unit(mut self, unit: impl Into<Cow<'static, str>>) -> Self {
        self.unit = Some(unit.into());
        self
    }

    /// Its editor kept only when `editable` (a row on a locked layer shows its value only).
    fn editor_if(mut self, editable: bool) -> Self {
        if !editable {
            self.editor = None;
        }
        self
    }

    fn editor(mut self, editor: Option<Editor>) -> Self {
        self.editor = editor;
        self
    }
}

/// A layer's path from the top of the tree: `Yapılar / Bina` (the web's `layers.path`).
pub(crate) fn layer_path(layers: &LayerTree, id: &str) -> String {
    let mut names = Vec::new();
    let mut at = layers.get(id);
    while let Some(node) = at {
        names.push(node.name.clone());
        at = layers.parent(&node.id);
    }
    if names.is_empty() {
        return id.to_owned();
    }
    names.reverse();
    names.join(" / ")
}

/// The rows only some programs show (module_surfaces.rs; kentos-cad every one).
#[derive(Clone, Copy, Debug)]
pub(crate) struct Shown {
    /// The drawing's coordinate system with nothing selected: a map's programs'.
    pub crs: bool,
    /// A polyline's Genişlik (docs/adr/0258): not a GIS's.
    pub widths: bool,
    /// An area's Maske (docs/adr/0310 §5): not a GIS's.
    pub mask: bool,
    /// A polyline's Mahal ayırıcı (docs/adr/0679): KentOS Mimari's.
    pub separators: bool,
}

/// What a kind's rows read besides the object (`entity_sections`' measures and editors): the kinds whose rows have
/// their own file (wall.rs, space.rs, slab.rs, opening.rs) take it.
#[derive(Clone, Copy)]
pub(super) struct Kit<'a> {
    pub(super) doc: &'a Document,
    pub(super) slot: Slot,
    pub(super) locked: bool,
    pub(super) e: &'a Entity,
    pub(super) f: Format,
    pub(super) decimals: usize,
    /// The object's area and length (`measures`).
    pub(super) area_of: Option<f64>,
    pub(super) length_of: Option<f64>,
}

impl Kit<'_> {
    /// `editor` unless the object's layer is locked.
    pub(super) fn edit(&self, editor: Editor) -> Option<Editor> {
        (!self.locked).then_some(editor)
    }

    pub(super) fn number(&self, field: Field) -> Option<Editor> {
        self.edit(Editor::Number(field))
    }

    pub(super) fn len(&self, label: &'static str, value: f64) -> Row {
        Row::figure(axis_named(label, self.f.axes), self.f.length_bare(value))
    }

    /// Lengths in the project's unit: a local project's millimetres (docs/adr/0165 §2).
    pub(super) fn metres(&self, label: &'static str, value: f64) -> Row {
        self.len(label, value).unit(self.f.length_unit_label())
    }

    /// An area: one row in m² (or a local project's unit squared), or m² and the project's unit.
    pub(super) fn area(&self, m2: f64) -> Vec<Row> {
        let f = self.f;
        if f.area_unit_label() == "m²" || f.unit != kentos_commands::DrawingUnit::M {
            vec![Row::figure("Alan", f.area_bare(m2)).unit(f.area_unit_label())]
        } else {
            vec![
                Row::figure("Alan", fixed(m2, self.decimals)).unit("m²"),
                Row::figure("Alan", f.area_bare(m2)).unit(f.area_unit_label()),
            ]
        }
    }
}

/// What the panel shows for this selection, the rows `shown` says.
pub(crate) fn panel(
    doc: &Document,
    selected: &[Slot],
    totals: impl Fn(&[Slot]) -> (f64, f64),
    shown: Shown,
) -> Panel {
    let objects: Vec<&Entity> = selected.iter().filter_map(|s| doc.model.get(*s)).collect();
    let layers = doc.model.layers();
    match objects.as_slice() {
        [] => Panel {
            summary: Summary::Empty,
            meta: None,
            sections: std::iter::once(document_section(doc, shown.crs))
                .chain(zoning_section(doc))
                .collect(),
            types: Vec::new(),
            total: 0,
            editable: 0,
        },
        [one] => {
            let base = one.base();
            Panel {
                summary: Summary::One {
                    kind: kind_title(one.kind()),
                    label: base.label.clone().filter(|l| !l.is_empty()),
                    layer: layers
                        .get(&base.layer_id)
                        .map(|n| (n.style.color.clone(), layer_path(layers, &n.id))),
                },
                meta: Some(format!("#{}", base.id)),
                sections: entity_sections(doc, one, shown),
                types: Vec::new(),
                total: 1,
                editable: usize::from(!layers.is_locked(&base.layer_id)),
            }
        }
        many => {
            // Kinds in the order first met, lower case (the web's summary).
            let mut kinds: Vec<(&'static str, usize)> = Vec::new();
            for e in many {
                let title = kind_title(e.kind());
                match kinds.iter_mut().find(|(k, _)| *k == title) {
                    Some((_, n)) => *n += 1,
                    None => kinds.push((title, 1)),
                }
            }
            let kinds = kinds
                .iter()
                .map(|(k, n)| format!("{n} {}", kind_lower(k)))
                .collect::<Vec<_>>()
                .join(", ");
            Panel {
                summary: Summary::Many {
                    count: many.len(),
                    kinds,
                },
                meta: Some(format!("{} nesne", many.len())),
                sections: many_sections(doc, many, totals(selected), shown),
                types: Vec::new(),
                total: many.len(),
                editable: many.iter().filter(|e| !layers.is_locked(&e.base().layer_id)).count(),
            }
        }
    }
}

/// A kind's title in lower case, as Turkish writes it (`toLocaleLowerCase('tr-TR')`).
pub(crate) fn kind_lower(title: &str) -> String {
    title
        .chars()
        .flat_map(|c| match c {
            'I' => vec!['ı'],
            'İ' => vec!['i'],
            c => c.to_lowercase().collect(),
        })
        .collect()
}

/// `+n.toFixed(places)`: a number rounded as the web shows it.
fn fixed_number(n: f64, places: usize) -> f64 {
    kentos_interaction::fixed(n, places)
        .parse::<f64>()
        .unwrap_or(n)
}

mod document;
pub(crate) use document::*;

mod look;
pub(crate) use look::*;

mod entity;
use entity::*;

mod fields;
use fields::*;

mod annotation;
use annotation::*;

mod many;
use many::*;

/// A row's name for a point's east or north value in the project's axes (UR-009): the rows are written in a CBS
/// project's (`Başlangıç Y` the east value); a CAD project names the east value X and the north Y.
fn axis_named(label: &'static str, axes: kentos_interaction::Axes) -> &'static str {
    if axes != kentos_interaction::Axes::Cad {
        return label;
    }
    match label {
        "Başlangıç Y" => "Başlangıç X",
        "Başlangıç X" => "Başlangıç Y",
        "Bitiş Y" => "Bitiş X",
        "Bitiş X" => "Bitiş Y",
        "Merkez Y" => "Merkez X",
        "Merkez X" => "Merkez Y",
        "Geçtiği nokta Y" => "Geçtiği nokta X",
        "Geçtiği nokta X" => "Geçtiği nokta Y",
        "Ölçülen Y" => "Ölçülen X",
        "Ölçülen X" => "Ölçülen Y",
        "Konum Y" => "Konum X",
        "Konum X" => "Konum Y",
        other => other,
    }
}

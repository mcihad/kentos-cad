//! What Öznitelikler shows, as data (the web's `PropertiesPanel.ts`): the
//! summary over the grid, the header's meta, and the sections and rows for
//! no selection, one object and several, each row with its editor. The view
//! draws them (`properties::view`); the edits come back as `Event`s.

use std::borrow::Cow;

use kentos_contracts::{DimensionStyle, Entity, GradientShape, HatchPatternType};
use kentos_domain::{LayerTree, Slot};
use kentos_interaction::elevation::{self, Summary as Elevations};
use kentos_interaction::{leader, text};
use kentos_interaction::{
    Format, Vec2, angle_deg, arc_sweep, bearing_grad, dimension_layout, dist, fixed, full_ellipse,
    measures,
};
use kentos_ui::icon::Icon;

use super::{Event, Field, Spot};

mod dimension;

pub(super) use dimension::is_y_axis;
use crate::app::Message;
use crate::document::Document;
use crate::selecting::kind_title;

/// The colours the panel offers (the web's `DRAW_COLORS`, fields.ts).
use crate::ribbon_panels::{DRAW_COLORS, LINE_WEIGHTS, weight_text as weight_label};

/// The panel's content.
#[derive(Clone)]
pub(crate) struct Panel {
    pub summary: Summary,
    /// The header's meta: `#12`, `3 nesne`.
    pub meta: Option<String>,
    pub sections: Vec<Section>,
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
    pub id: &'static str,
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
    fn text(label: impl Into<Cow<'static, str>>, value: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            value: value.into(),
            numeric: false,
            unit: None,
            editor: None,
            note: false,
        }
    }

    /// A quiet line under the row above it, with no name of its own.
    fn note(text: &str) -> Self {
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

/// What the panel shows for this selection.
pub(crate) fn panel(
    doc: &Document,
    selected: &[Slot],
    totals: impl Fn(&[Slot]) -> (f64, f64),
) -> Panel {
    let objects: Vec<&Entity> = selected.iter().filter_map(|s| doc.model.get(*s)).collect();
    let layers = doc.model.layers();
    match objects.as_slice() {
        [] => Panel {
            summary: Summary::Empty,
            meta: None,
            sections: vec![document_section(doc)],
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
                sections: entity_sections(doc, one),
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
                sections: many_sections(doc, many, totals(selected)),
            }
        }
    }
}

/// A kind's title in lower case, as Turkish writes it (`toLocaleLowerCase('tr-TR')`).
fn kind_lower(title: &str) -> String {
    title
        .chars()
        .flat_map(|c| match c {
            'I' => vec!['ı'],
            'İ' => vec!['i'],
            c => c.to_lowercase().collect(),
        })
        .collect()
}

/// No selection: the drawing's summary.
fn document_section(doc: &Document) -> Section {
    let s = doc.settings();
    let layers = doc.model.layers();
    let active = layers
        .get(layers.active())
        .map_or_else(|| "—".to_owned(), |n| layer_path(layers, &n.id));
    Section {
        id: "doc",
        title: "Çizim".into(),
        rows: vec![
            Row::text("Dosya", doc.name()),
            Row::text("Koordinat sistemi", crate::crs::project_name(s)),
            Row::figure("SRID", crate::crs::project_code(s)),
            Row::figure("Çizim ölçeği", format!("1:{}", s.plot_scale)),
            Row::figure("Nesne sayısı", doc.entity_count().to_string()),
            Row::text("Etkin katman", active),
        ],
    }
}

/// A colour as the panel names it: “Çeşitli” for a mixed selection, “Katmana göre”, or its name.
fn color_text(current: Option<Option<&str>>) -> String {
    match current {
        None => "Çeşitli".to_owned(),
        Some(None) => "Katmana göre".to_owned(),
        Some(Some(value)) => DRAW_COLORS
            .iter()
            .find(|(_, v)| *v == value)
            .map_or_else(|| value.to_owned(), |(name, _)| (*name).to_owned()),
    }
}

/// A line weight as the panel names it: “Çeşitli” for a mixed selection, “Katmana göre”, or “0.35 mm”.
fn weight_text(current: Option<Option<f64>>) -> String {
    match current {
        None => "Çeşitli".to_owned(),
        Some(None) => "Katmana göre".to_owned(),
        Some(Some(w)) => weight_label(w),
    }
}

/// Kalınlık ▾: by layer, then the weights, an imported one not in the list
/// among them (the web's `weightEditor`, docs/adr/0139).
fn weight_editor(ids: &[Slot], current: Option<Option<f64>>) -> Editor {
    let set = |w: Option<f64>| Message::Properties(Event::Weight(ids.to_vec(), w));
    let mut weights = LINE_WEIGHTS.to_vec();
    if let Some(Some(w)) = current
        && !weights.contains(&w)
    {
        weights.push(w);
        weights.sort_by(f64::total_cmp);
    }
    let mut items = vec![
        Choice::Pick {
            label: "Katmana göre".to_owned(),
            swatch: None,
            icon: None,
            wide: false,
            chosen: current == Some(None),
            enabled: true,
            message: set(None),
        },
        Choice::Separator,
    ];
    items.extend(weights.into_iter().map(|w| Choice::Pick {
        label: weight_label(w),
        swatch: None,
        icon: None,
        wide: false,
        chosen: current == Some(Some(w)),
        enabled: true,
        message: set(Some(w)),
    }));
    Editor::Select {
        text: weight_text(current),
        swatch: None,
        icon: None,
        items,
    }
}

/// A symbol as the panel names it: “Çeşitli”, “Katman stiline göre”, or its
/// name among the project's styles; one of the system library, which the
/// desktop does not hold yet, by its id (docs/adr/0063).
fn symbol_text(doc: &Document, current: Option<Option<&str>>) -> String {
    match current {
        None => "Çeşitli".to_owned(),
        Some(None) => "Katman stiline göre".to_owned(),
        Some(Some(id)) => doc
            .model
            .styles()
            .items
            .iter()
            .find(|item| item.get("id").and_then(|v| v.as_str()) == Some(id))
            .and_then(|item| item.get("name").and_then(|v| v.as_str()))
            .unwrap_or(id)
            .to_owned(),
    }
}

/// Katman ▾: every layer by its path, the locked ones not to be chosen (the web's `layerEditor`).
fn layer_editor(doc: &Document, ids: &[Slot], current: Option<&str>) -> Editor {
    let layers = doc.model.layers();
    let shown = current.and_then(|id| layers.get(id));
    Editor::Select {
        text: shown.map_or_else(|| "Çeşitli".to_owned(), |n| n.name.clone()),
        swatch: shown.map(|n| n.style.color.clone()),
        icon: None,
        items: layers
            .leaves()
            .into_iter()
            .map(|l| Choice::Pick {
                label: layer_path(layers, &l.id),
                swatch: Some(l.style.color.clone()),
                icon: None,
                wide: false,
                chosen: Some(l.id.as_str()) == current,
                enabled: !layers.is_locked(&l.id),
                message: Message::Properties(Event::Layer(ids.to_vec(), l.id.clone())),
            })
            .collect(),
    }
}

/// Renk ▾: by layer, then the colours (the web's `colorEditor`).
fn color_editor(ids: &[Slot], current: Option<Option<&str>>) -> Editor {
    let known = current
        .flatten()
        .and_then(|value| DRAW_COLORS.iter().find(|(_, v)| *v == value));
    let set = |color: Option<&str>| {
        Message::Properties(Event::Color(ids.to_vec(), color.map(str::to_owned)))
    };
    let mut items = vec![
        Choice::Pick {
            label: "Katmana göre".to_owned(),
            swatch: None,
            icon: None,
            wide: false,
            chosen: current == Some(None),
            enabled: true,
            message: set(None),
        },
        Choice::Separator,
    ];
    items.extend(DRAW_COLORS.iter().map(|(name, value)| Choice::Pick {
        label: (*name).to_owned(),
        swatch: Some((*value).to_owned()),
        icon: None,
        wide: false,
        chosen: current == Some(Some(*value)),
        enabled: true,
        message: set(Some(value)),
    }));
    Editor::Select {
        text: known.map_or_else(|| color_text(current), |(name, _)| (*name).to_owned()),
        swatch: known.map(|(_, value)| (*value).to_owned()),
        icon: None,
        items,
    }
}

/// Sembol ▾: the layer's style, or one from the library (the web's `symbolEditor`).
fn symbol_editor(doc: &Document, current: Option<Option<&str>>) -> Editor {
    Editor::Select {
        text: symbol_text(doc, current),
        swatch: None,
        icon: None,
        items: vec![
            Choice::Pick {
                label: "Katman stiline göre".to_owned(),
                swatch: None,
                icon: None,
                wide: false,
                chosen: current == Some(None),
                enabled: true,
                message: Message::Run("style.clearSymbol"),
            },
            Choice::Separator,
            Choice::Command {
                label: "Kitaplıktan seç…",
                icon: crate::icons::from_web(Some("styles")),
                id: "style.assign",
            },
            Choice::Command {
                label: "Katman stili…",
                icon: crate::icons::from_web(Some("layerStyle")),
                id: "style.layerStyle",
            },
        ],
    }
}

/// Whether an object takes a symbol of its own: all but texts and dimensions
/// (the web's `geometryClassOf`).
fn takes_symbol(e: &Entity) -> bool {
    !matches!(e, Entity::Text(_) | Entity::Dimension(_))
}

/// A Kot row (docs/adr/0142): the elevations as `summary` says them, in
/// metres: a value or a range beside its unit, or `kot yok`; with vertices
/// that have none, a note under it names them.
fn elevation_row(
    label: &'static str,
    summary: Elevations,
    f: &Format,
    editor: Option<Editor>,
) -> Vec<Row> {
    let (text, unit) = summary.shown(f);
    let row = match unit {
        Some(unit) => Row::figure(label, text).unit(unit),
        None => Row::text(label, text),
    };
    let mut rows = vec![row.editor(editor)];
    rows.extend(summary.note().map(Row::note));
    rows
}

fn v(p: kentos_contracts::Vec2) -> Vec2 {
    Vec2::new(p.x, p.y)
}

/// Degrees from radians with four decimals (the web's `(rad * 180 / π).toFixed(4)`).
fn degrees(rad: f64) -> String {
    fixed((rad * 180.0) / std::f64::consts::PI, 4)
}

/// Degrees brought into [0, 360) with four decimals.
fn turn(deg: f64) -> String {
    fixed(((deg % 360.0) + 360.0) % 360.0, 4)
}

/// `3B uzunluk` of a line or a polyline, `3B çevre` of an area, when every
/// vertex has an elevation (docs/adr/0142): the length in space beside the
/// plan one, which stays the measure of record.
fn space_length(e: &Entity, f: &Format) -> Option<Row> {
    let (label, length) = elevation::space_length(e)?;
    Some(Row::figure(label, f.length_bare(length)).unit(f.length_unit_label()))
}

/// One object's sections: Genel, Geometri, an insert's Blok öznitelikleri, and Öznitelik bilgileri when it has other attributes.
fn entity_sections(doc: &Document, e: &Entity) -> Vec<Section> {
    let base = e.base();
    let slot = Slot(base.id);
    let layers = doc.model.layers();
    let locked = layers.is_locked(&base.layer_id);
    let ids = [slot];
    let edit = |editor: Editor| (!locked).then_some(editor);
    let color = Some(base.color.as_deref());
    let symbol = Some(base.symbol.as_deref());

    let mut general = vec![
        Row::text("Tür", kind_title(e.kind())),
        Row::text(
            "Katman",
            if locked {
                format!("{} (kilitli)", layer_path(layers, &base.layer_id))
            } else {
                String::new()
            },
        )
        .editor(edit(layer_editor(doc, &ids, Some(&base.layer_id)))),
        Row::text("Renk", color_text(color)).editor(edit(color_editor(&ids, color))),
    ];
    if e.draws_lines() {
        let weight = Some(base.line_weight);
        general.push(
            Row::text("Kalınlık", weight_text(weight)).editor(edit(weight_editor(&ids, weight))),
        );
    }
    if takes_symbol(e) {
        general.push(
            Row::text("Sembol", symbol_text(doc, symbol)).editor(edit(symbol_editor(doc, symbol))),
        );
    }

    let f = Format::of(doc.settings());
    let decimals = doc.settings().area_decimals as usize;
    let len = |label: &'static str, value: f64| Row::figure(label, f.length_bare(value));
    // Lengths in the project's unit: a local project's millimetres (docs/adr/0165 §2).
    let metres = |label: &'static str, value: f64| len(label, value).unit(f.length_unit_label());
    // An area: one row in m² (or a local project's unit squared), or m² and the project's unit.
    let area = |m2: f64| -> Vec<Row> {
        if f.area_unit_label() == "m²" || f.unit != kentos_contracts::DrawingUnit::M {
            vec![Row::figure("Alan", f.area_bare(m2)).unit(f.area_unit_label())]
        } else {
            vec![
                Row::figure("Alan", fixed(m2, decimals)).unit("m²"),
                Row::figure("Alan", f.area_bare(m2)).unit(f.area_unit_label()),
            ]
        }
    };
    // A direction as the project's type reads it: a semt, or a CAD project's angle (docs/adr/0165 §4).
    let bearing = |a: Vec2, b: Vec2| {
        Row::figure(f.direction_name(), f.direction_bare(bearing_grad(a, b)))
            .unit(f.angle_unit_label())
    };
    let number = |field: Field| edit(Editor::Number(field));
    let (area_of, length_of) = measures(e);

    let mut geo: Vec<Row> = Vec::new();
    match e {
        Entity::Point(p) => {
            // East and north as the project's type names them (docs/adr/0165 §4).
            let (east, north) = match f.axes {
                kentos_interaction::Axes::Cad => ("X (sağa)", "Y (yukarı)"),
                kentos_interaction::Axes::Gis => ("Y (sağa)", "X (yukarı)"),
            };
            geo.push(metres(east, p.p.x).editor(number(Field::PointX(slot))));
            geo.push(metres(north, p.p.y).editor(number(Field::PointY(slot))));
            match p.parts.as_deref().filter(|ps| !ps.is_empty()) {
                // A multi-point object: how many points, and their elevations as one row (docs/adr/0174).
                Some(parts) => {
                    geo.push(Row::figure("Nokta sayısı", (parts.len() + 1).to_string()));
                    geo.extend(elevation_row(
                        "Kot",
                        Elevations::of_object(e),
                        &f,
                        number(Field::Elevation(slot, Spot::All)),
                    ));
                }
                None => {
                    if let Some(z) = p.z {
                        geo.push(metres("Z (kot)", z));
                    }
                }
            }
        }
        Entity::Line(l) => {
            // Each end's elevation follows its coordinates (docs/adr/0142).
            geo.extend([len("Başlangıç Y", l.a.x), len("Başlangıç X", l.a.y)]);
            geo.extend(elevation_row(
                "Kot (başlangıç)",
                Elevations::of(&[l.za]),
                &f,
                number(Field::Elevation(slot, Spot::Start)),
            ));
            geo.extend([len("Bitiş Y", l.b.x), len("Bitiş X", l.b.y)]);
            geo.extend(elevation_row(
                "Kot (bitiş)",
                Elevations::of(&[l.zb]),
                &f,
                number(Field::Elevation(slot, Spot::End)),
            ));
            geo.push(metres("Uzunluk", dist(v(l.a), v(l.b))));
            geo.extend(space_length(e, &f));
            geo.push(bearing(v(l.a), v(l.b)));
        }
        Entity::Polyline(p) | Entity::Polygon(p) => {
            let polygon = matches!(e, Entity::Polygon(_));
            // A multi-part area's corners, parts and holes are every part's (docs/adr/0143).
            let parts = p.parts.as_deref().unwrap_or(&[]);
            let corners = p.pts.len() + parts.iter().map(|q| q.pts.len()).sum::<usize>();
            geo.push(Row::figure("Köşe sayısı", corners.to_string()));
            if !parts.is_empty() {
                geo.push(Row::figure("Parça sayısı", (parts.len() + 1).to_string()));
            }
            geo.extend(elevation_row(
                "Kot",
                Elevations::of_object(e),
                &f,
                number(Field::Elevation(slot, Spot::All)),
            ));
            geo.push(metres(
                if polygon { "Çevre" } else { "Uzunluk" },
                length_of.unwrap_or(0.0),
            ));
            geo.extend(space_length(e, &f));
            if polygon {
                // Net area: the holes (adalar) are taken out already.
                let holes = p.holes.iter().flatten().count()
                    + parts
                        .iter()
                        .map(|q| q.holes.iter().flatten().count())
                        .sum::<usize>();
                if holes > 0 {
                    geo.push(Row::figure("Ada (delik)", holes.to_string()));
                }
                geo.extend(area(area_of.unwrap_or(0.0)));
            }
        }
        Entity::Circle(c) => {
            geo.extend([
                len("Merkez Y", c.c.x),
                len("Merkez X", c.c.y),
                metres("Yarıçap", c.r),
            ]);
            geo.extend(area(area_of.unwrap_or(0.0)));
        }
        Entity::Arc(a) => {
            geo.extend([
                len("Merkez Y", a.c.x),
                len("Merkez X", a.c.y),
                metres("Yarıçap", a.r),
                Row::figure("Başlangıç açısı", degrees(a.a0)).unit("°"),
                Row::figure("Bitiş açısı", degrees(a.a1)).unit("°"),
                Row::figure("Yay açısı", degrees(arc_sweep(a.a0, a.a1))).unit("°"),
                metres("Yay uzunluğu", length_of.unwrap_or(0.0)),
            ]);
        }
        Entity::Ellipse(el) => {
            let a = el.major.x.hypot(el.major.y);
            geo.extend([
                len("Merkez Y", el.c.x),
                len("Merkez X", el.c.y),
                metres("Büyük yarı eksen", a),
                metres("Küçük yarı eksen", a * el.ratio),
                Row::figure(
                    "Eksen açısı",
                    turn(angle_deg(Vec2::new(0.0, 0.0), v(el.major))),
                )
                .unit("°"),
            ]);
            if full_ellipse(el) {
                geo.push(metres("Çevre", length_of.unwrap_or(0.0)));
                geo.extend(area(area_of.unwrap_or(0.0)));
            } else {
                let param = |rad: f64| turn((rad * 180.0) / std::f64::consts::PI);
                geo.extend([
                    Row::figure("Başlangıç parametresi", param(el.t0)).unit("°"),
                    Row::figure("Bitiş parametresi", param(el.t1)).unit("°"),
                    metres("Yay uzunluğu", length_of.unwrap_or(0.0)),
                ]);
            }
        }
        Entity::Xline(x) | Entity::Ray(x) => {
            let ray = matches!(e, Entity::Ray(_));
            let p = v(x.p);
            geo.extend([
                len(
                    if ray {
                        "Başlangıç Y"
                    } else {
                        "Geçtiği nokta Y"
                    },
                    x.p.x,
                ),
                len(
                    if ray {
                        "Başlangıç X"
                    } else {
                        "Geçtiği nokta X"
                    },
                    x.p.y,
                ),
                Row::figure("Doğrultu", turn(angle_deg(Vec2::new(0.0, 0.0), v(x.dir)))).unit("°"),
                bearing(p, Vec2::new(p.x + x.dir.x, p.y + x.dir.y)),
            ]);
        }
        Entity::Spline(s) => {
            geo.extend([
                Row::figure("Nokta sayısı", s.pts.len().to_string()),
                Row::text("Kapalı", if s.closed { "Evet" } else { "Hayır" }),
                metres("Uzunluk", length_of.unwrap_or(0.0)),
            ]);
        }
        Entity::Dimension(d) => {
            let style = d.style.unwrap_or(DimensionStyle::Aligned);
            geo.push(Row::text(
                "Tür",
                match style {
                    DimensionStyle::Aligned => "Hizalı",
                    DimensionStyle::Linear => "Doğrusal",
                    DimensionStyle::Angular => "Açı",
                    DimensionStyle::Radius => "Yarıçap",
                    DimensionStyle::Diameter => "Çap",
                    DimensionStyle::Ordinate => "Koordinat",
                    DimensionStyle::ArcLength => "Yay uzunluğu",
                    DimensionStyle::Jogged => "Kırıklı yarıçap",
                    DimensionStyle::Azimuth => "Semt",
                    DimensionStyle::Slope => "Eğim",
                },
            ));
            // What it measured, by its unit (docs/adr/0147 §2).
            match dimension_layout(e) {
                Some(l) if l.unit == "angle" => geo.push(
                    Row::figure(
                        if style == DimensionStyle::Azimuth {
                            "Ölçülen semt"
                        } else {
                            "Ölçülen açı"
                        },
                        f.angle_bare(l.value),
                    )
                    .unit(f.angle_unit_label()),
                ),
                Some(l) if l.unit == "percent" => {
                    geo.push(Row::figure("Ölçülen eğim", f.percent(l.value)).unit("%"))
                }
                Some(l) if l.unit == "coordinate" => geo.push(metres(
                    if l.prefix == "Y=" {
                        "Ölçülen Y"
                    } else {
                        "Ölçülen X"
                    },
                    l.value,
                )),
                Some(l) => geo.push(metres(
                    match style {
                        DimensionStyle::Diameter => "Ölçülen çap",
                        DimensionStyle::Radius | DimensionStyle::Jogged => "Ölçülen yarıçap",
                        _ => "Ölçülen uzunluk",
                    },
                    l.value,
                )),
                None => {}
            }
            if style == DimensionStyle::Aligned {
                geo.push(bearing(v(d.a), v(d.b)));
            }
            // An ordinate has no offset; a jogged radius's is where its jog is.
            if style != DimensionStyle::Ordinate {
                geo.push(
                    metres(
                        match style {
                            DimensionStyle::Angular => "Yay yarıçapı",
                            DimensionStyle::Radius | DimensionStyle::Diameter => "Dışa uzantı",
                            DimensionStyle::Jogged => "Kırık uzaklığı",
                            _ => "Ötelenme",
                        },
                        d.offset,
                    )
                    .editor(number(Field::DimensionOffset(slot))),
                );
            }
            // Ölçü stili, a CAD project's (docs/adr/0183 §6).
            geo.extend(style_rows(
                doc.model.settings(),
                false,
                &[d.look.dim_style.as_ref()],
                &ids,
                locked,
            ));
            geo.extend([
                metres("Yazı yüksekliği", d.height).editor(number(Field::DimensionHeight(slot))),
                Row::text("Yazı", d.text.clone().unwrap_or_default())
                    .editor(edit(Editor::Text(Field::DimensionText(slot)))),
            ]);
            // Zemin, an ordinate's axis, a slope's elevations, an arc length's radius and angle (docs/adr/0147 §7).
            geo.extend(dimension::rows(&[d], &ids, locked, &f));
        }
        Entity::Hatch(h) => {
            geo.extend(hatch_rows(
                h,
                slot,
                locked,
                doc.model.settings().plot_scale,
                &f,
            ));
            geo.extend(area(area_of.unwrap_or(0.0)));
        }
        Entity::Text(t) => {
            // Yazı stili, a CAD project's (docs/adr/0183 §6).
            geo.extend(style_rows(
                doc.model.settings(),
                true,
                &[t.face.text_style.as_ref()],
                &ids,
                locked,
            ));
            geo.extend([
                // A multi-line text's lines and formats are its editor's (a double click,
                // docs/adr/0182 §4): here they only show, ⏎ its breaks.
                if crate::paragraph_editor::is_paragraph(t) {
                    Row::text("Metin", t.text.replace('\n', " ⏎ "))
                } else {
                    Row::text("Metin", t.text.clone()).editor(edit(Editor::Text(Field::Text(slot))))
                },
                metres("Yükseklik", t.height).editor(number(Field::TextHeight(slot))),
                Row::figure("Açı", fixed(t.rotation, 2))
                    .unit("°")
                    .editor(number(Field::TextAngle(slot))),
            ]);
            // Hiza, Genişlik çarpanı and Zemin (docs/adr/0145 §6).
            geo.extend(text_rows(&[t], &ids, locked));
            // Bağlı nesne (docs/adr/0175 §4).
            geo.extend(link_rows(&doc.model, &[t], &ids, locked));
            geo.extend([len("Konum Y", t.p.x), len("Konum X", t.p.y)]);
        }
        // Its style, size, turn and source (docs/adr/0184 §6); its cells are its editor's (a double click).
        Entity::Table(t) => {
            geo.extend(style_rows(
                doc.model.settings(),
                true,
                &[t.face.text_style.as_ref()],
                &ids,
                locked,
            ));
            geo.extend([
                Row::figure("Satır sayısı", t.rows.len().to_string()),
                Row::figure("Sütun sayısı", t.columns.len().to_string()),
                metres("Yazı yüksekliği", t.height).editor(number(Field::TextHeight(slot))),
                Row::figure("Açı", fixed(t.rotation, 2))
                    .unit("°")
                    .editor(number(Field::TextAngle(slot))),
                metres("Genişlik", t.columns.iter().sum()),
                metres("Derinlik", t.rows.iter().sum()),
                Row::text("Kaynak", crate::tables::source_words(t)),
                len("Konum Y", t.p.x),
                len("Konum X", t.p.y),
            ]);
        }
        // The block, its place, scale, turn and mirroring, through
        // `cad.entities.edit`'s properties (docs/adr/0144 §6), as the web's.
        Entity::Insert(i) => {
            let name = doc
                .model
                .block(i.block)
                .map_or_else(|| "(tanımsız)".to_owned(), |b| b.name.clone());
            let blocks = Editor::Select {
                text: name.clone(),
                swatch: None,
                icon: None,
                items: doc
                    .model
                    .blocks()
                    .iter()
                    .map(|b| Choice::Pick {
                        label: b.name.clone(),
                        swatch: None,
                        icon: None,
                        wide: false,
                        chosen: b.id == i.block,
                        enabled: true,
                        message: Message::Properties(Event::InsertBlock(slot, b.id)),
                    })
                    .collect(),
            };
            let yes_no = |m: bool| if m { "Evet" } else { "Hayır" };
            let mirror = Editor::Select {
                text: yes_no(i.mirror).to_owned(),
                swatch: None,
                icon: None,
                items: [true, false]
                    .into_iter()
                    .map(|m| Choice::Pick {
                        label: yes_no(m).to_owned(),
                        swatch: None,
                        icon: None,
                        wide: false,
                        chosen: m == i.mirror,
                        enabled: true,
                        message: Message::Properties(Event::InsertMirror(slot, m)),
                    })
                    .collect(),
            };
            geo.extend([
                Row::text("Blok", name).editor(edit(blocks)),
                len("Konum Y", i.p.x).editor(number(Field::InsertX(slot))),
                len("Konum X", i.p.y).editor(number(Field::InsertY(slot))),
                Row::figure("Ölçek", fixed(i.scale, 4)).editor(number(Field::InsertScale(slot))),
                Row::figure("Dönüş", degrees(i.rotation))
                    .unit("°")
                    .editor(number(Field::InsertTurn(slot))),
                Row::text("Aynalı", yes_no(i.mirror)).editor(edit(mirror)),
            ]);
        }
        // Its note, height, turn, arrowhead and mask, its corners and length (docs/adr/0146 §7).
        Entity::Leader(l) => {
            geo.extend(leader_rows(&[l], &ids, locked, &f));
            geo.push(Row::figure("Köşe sayısı", l.pts.len().to_string()));
            geo.push(metres("Uzunluk", length_of.unwrap_or(0.0)));
        }
    }

    let mut sections = vec![
        Section {
            id: "general",
            title: "Genel".into(),
            rows: general,
        },
        Section {
            id: "geometry",
            title: "Geometri".into(),
            rows: geo,
        },
    ];
    // An insert's block attributes (docs/adr/0144 §7): the definition's tags in
    // its order, with what the insert shows (its value, else the default).
    let tags: Vec<(String, String)> = match e {
        Entity::Insert(i) => doc
            .model
            .block(i.block)
            .map(|b| {
                b.attributes
                    .iter()
                    .map(|a| {
                        let own = base.attrs.get(&a.tag).filter(|v| !v.is_empty());
                        let shown = own.or(a.value.as_ref()).cloned().unwrap_or_default();
                        (a.tag.clone(), shown)
                    })
                    .collect()
            })
            .unwrap_or_default(),
        _ => Vec::new(),
    };
    if !tags.is_empty() {
        sections.push(Section {
            id: "blockAttrs",
            title: "Blok öznitelikleri".into(),
            rows: tags
                .iter()
                .map(|(tag, value)| Row {
                    label: Cow::Owned(tag.clone()),
                    value: value.clone(),
                    numeric: looks_numeric(value),
                    unit: None,
                    editor: edit(Editor::Text(Field::Attribute(slot, tag.clone()))),
                    note: false,
                })
                .collect(),
        });
    }
    let others: Vec<(&String, &String)> = base
        .attrs
        .iter()
        .filter(|(key, _)| !tags.iter().any(|(tag, _)| tag == *key))
        .collect();
    if !others.is_empty() {
        sections.push(Section {
            id: "attrs",
            title: "Öznitelik bilgileri".into(),
            rows: others
                .into_iter()
                .map(|(key, value)| Row {
                    label: Cow::Owned(key.clone()),
                    value: value.clone(),
                    numeric: looks_numeric(value),
                    unit: None,
                    editor: edit(Editor::Text(Field::Attribute(slot, key.clone()))),
                    note: false,
                })
                .collect(),
        });
    }
    sections
}

/// Öznitelikler's Yazı stili or Ölçü stili row (docs/adr/0183 §6), a CAD
/// project's: the style the objects follow by name (a link to a style the
/// project no longer has shows Standart), or “Çeşitli”; choosing one applies
/// it to them in one step “Değiştir”. On a locked layer it only shows. The
/// web's `ui/properties/styleRows.ts`.
fn style_rows(
    settings: &kentos_contracts::ProjectSettings,
    text: bool,
    ids: &[Option<&String>],
    slots: &[Slot],
    locked: bool,
) -> Vec<Row> {
    if !kentos_interaction::styles::shown(settings) || ids.is_empty() {
        return Vec::new();
    }
    let styles: Vec<(String, String)> = if text {
        settings
            .text_styles
            .iter()
            .map(|s| (s.id.clone(), s.name.clone()))
            .collect()
    } else {
        settings
            .dimension_styles
            .iter()
            .map(|s| (s.id.clone(), s.name.clone()))
            .collect()
    };
    let name_of = |id: Option<&String>| {
        id.and_then(|id| styles.iter().find(|(i, _)| i == id))
            .map_or(kentos_contracts::STANDARD_STYLE.to_owned(), |(_, n)| {
                n.clone()
            })
    };
    let names: Vec<String> = ids.iter().map(|id| name_of(*id)).collect();
    let value = if names.iter().all(|n| *n == names[0]) {
        names[0].clone()
    } else {
        "Çeşitli".to_owned()
    };
    let pick = |label: String, id: Option<String>| Choice::Pick {
        chosen: value == label,
        label,
        swatch: None,
        icon: None,
        wide: false,
        enabled: true,
        message: Message::Properties(if text {
            Event::TextStyle(slots.to_vec(), id)
        } else {
            Event::DimensionStyle(slots.to_vec(), id)
        }),
    };
    let items = std::iter::once(pick(kentos_contracts::STANDARD_STYLE.to_owned(), None))
        .chain(
            styles
                .iter()
                .map(|(id, name)| pick(name.clone(), Some(id.clone()))),
        )
        .collect();
    let editor = Editor::Select {
        text: value.clone(),
        swatch: None,
        icon: None,
        items,
    };
    vec![
        Row::text(if text { "Yazı stili" } else { "Ölçü stili" }, value)
            .editor((!locked).then_some(editor)),
    ]
}

/// A value that reads as a number: `12`, `-3,5` (the web's `/^-?\d+([.,]\d+)?$/`).
/// A text's Hiza, Genişlik çarpanı, Kutu genişliği, Satır aralığı (docs/adr/0182
/// §4) and Zemin rows (docs/adr/0145 §6), for one
/// text or the texts of a selection: their common value, or “Çeşitli”. A new
/// alignment keeps each text where it is; the texts are written in one step
/// “Değiştir”. On a locked layer they only show. The web's `textRows`.
fn text_rows(texts: &[&kentos_contracts::TextEntity], slots: &[Slot], locked: bool) -> Vec<Row> {
    const MIXED: &str = "Çeşitli";
    let common = |of: &dyn Fn(&kentos_contracts::TextEntity) -> String| {
        let first = of(texts[0]);
        texts.iter().all(|t| of(t) == first).then_some(first)
    };
    let align = texts
        .iter()
        .all(|t| t.align == texts[0].align)
        .then_some(texts[0].align);
    let factor = common(&|t| text::width_factor_text(t.width_factor.unwrap_or(1.0)));
    let mask = texts
        .iter()
        .all(|t| t.mask == texts[0].mask)
        .then_some(texts[0].mask);
    let on_off = |on: bool| if on { "Açık" } else { "Kapalı" };
    let align_text = align.map_or_else(|| MIXED.to_owned(), |a| text::align_label(a).to_owned());
    let mask_text = mask.map_or(MIXED, on_off).to_owned();
    let edit = |editor: Editor| (!locked).then_some(editor);
    let aligns = Editor::Select {
        text: align_text.clone(),
        swatch: None,
        icon: align.map(text::align_icon),
        items: text::ALIGNS
            .iter()
            .map(|&(a, _, label, icon)| Choice::Pick {
                label: label.to_owned(),
                swatch: None,
                icon: Some(icon),
                wide: false,
                chosen: align == Some(a),
                enabled: true,
                message: Message::Properties(Event::TextAlign(slots.to_vec(), a)),
            })
            .collect(),
    };
    let masks = Editor::Select {
        text: mask_text.clone(),
        swatch: None,
        icon: None,
        items: [true, false]
            .into_iter()
            .map(|on| Choice::Pick {
                label: on_off(on).to_owned(),
                swatch: None,
                icon: None,
                wide: false,
                chosen: mask == Some(on),
                enabled: true,
                message: Message::Properties(Event::TextMask(slots.to_vec(), on)),
            })
            .collect(),
    };
    let factor_row = match factor {
        Some(value) => Row::figure("Genişlik çarpanı", value),
        None => Row::text("Genişlik çarpanı", MIXED),
    };
    // A multi-line text's box (none: “Kutusuz”) and line spacing (docs/adr/0182 §4).
    let box_row = match common(&|t| match t.paragraph.box_width {
        Some(w) => crate::crs::js_number(fixed_number(w, 3)),
        None => "Kutusuz".to_owned(),
    }) {
        Some(value) if value == "Kutusuz" => Row::text("Kutu genişliği", value),
        Some(value) => Row::figure("Kutu genişliği", value),
        None => Row::text("Kutu genişliği", MIXED),
    };
    let spacing_row = match common(&|t| {
        crate::crs::js_number(fixed_number(t.paragraph.line_spacing.unwrap_or(1.0), 4))
    }) {
        Some(value) => Row::figure("Satır aralığı", value),
        None => Row::text("Satır aralığı", MIXED),
    };
    vec![
        Row::text("Hiza", align_text).editor(edit(aligns)),
        factor_row.editor(edit(Editor::Number(Field::TextWidth(slots.to_vec())))),
        box_row.editor(edit(Editor::Number(Field::TextBox(slots.to_vec())))),
        spacing_row.editor(edit(Editor::Number(Field::TextSpacing(slots.to_vec())))),
        Row::text("Zemin", mask_text).editor(edit(masks)),
    ]
}

/// A leader's Not, Yükseklik, Dönüş, Ok and Zemin rows (docs/adr/0146 §7),
/// for one leader or the leaders of a selection: their common value, or
/// “Çeşitli”; each change is one step “Değiştir”. The web's `leaderRows`.
/// A linked text's Bağlı nesne row (docs/adr/0175 §4): for one text its
/// object's kind and label, with Nesneyi seç and Bağı kopar; for several,
/// how many are linked, with Bağı kopar for them all. None when no text of
/// them is linked; on a locked layer only Nesneyi seç. The web's `linkRow`
/// (ui/properties/textRows.ts) is the same.
fn link_rows(
    model: &kentos_domain::Document,
    texts: &[&kentos_contracts::TextEntity],
    slots: &[Slot],
    locked: bool,
) -> Vec<Row> {
    let linked: Vec<Slot> = texts
        .iter()
        .zip(slots)
        .filter(|(t, _)| t.label_of.is_some())
        .map(|(_, slot)| *slot)
        .collect();
    if linked.is_empty() {
        return Vec::new();
    }
    let object = match texts {
        [one] => one
            .label_of
            .and_then(|id| model.slot_of(kentos_domain::Uuid::from_bytes(id.0)))
            .and_then(|slot| Some((slot, model.get(slot)?))),
        _ => None,
    };
    let value = match (texts.len(), object) {
        (1, Some((_, e))) => match e.base().label.as_deref().filter(|l| !l.is_empty()) {
            Some(label) => format!("{} “{label}”", kind_title(e.kind())),
            None => kind_title(e.kind()).to_owned(),
        },
        (1, None) => "Çizimde yok".to_owned(),
        (_, _) => format!("{} yazı bağlı", linked.len()),
    };
    let action = |label: &str, event: Event| Choice::Pick {
        label: label.to_owned(),
        swatch: None,
        icon: None,
        wide: false,
        chosen: false,
        enabled: true,
        message: Message::Properties(event),
    };
    let mut items = Vec::new();
    if let Some((slot, _)) = object {
        items.push(action("Nesneyi seç", Event::SelectObject(slot)));
    }
    if !locked {
        items.push(action("Bağı kopar", Event::Unlink(linked)));
    }
    let editor = (!items.is_empty()).then(|| Editor::Select {
        text: value.clone(),
        swatch: None,
        icon: None,
        items,
    });
    vec![Row::text("Bağlı nesne", value).editor(editor)]
}

/// A hatch's rows (docs/adr/0186 §7): Desen (every choice with its icon,
/// its Ölçek and Açı carried over), Açı, Ölçek (a pattern's, on the paper),
/// Aralık (user lines'), İkinci renk, Degrade biçimi and Ters (a
/// gradient's), İlişkili with İlişkiyi kopar. The web's `hatchRows`.
fn hatch_rows(
    h: &kentos_contracts::HatchEntity,
    slot: Slot,
    locked: bool,
    plot_scale: f64,
    f: &Format,
) -> Vec<Row> {
    use kentos_interaction::hatch_options::{choice_of, icon_name};
    let edit = |editor: Editor| (!locked).then_some(editor);
    let number = |field: Field| edit(Editor::Number(field));
    let p = &h.pattern;
    let list = kentos_geometry_core::tools::hatch::choices();
    let now = choice_of(p);
    let name = match (now.and_then(|k| list.get(k)), &p.name) {
        (Some(c), _) => c.name.clone(),
        (None, Some(n)) => n.clone(),
        (None, None) => "Desen".to_owned(),
    };
    let pick =
        |label: String, icon: Option<&'static str>, chosen: bool, event: Event| Choice::Pick {
            label,
            swatch: None,
            icon,
            wide: false,
            chosen,
            enabled: true,
            message: Message::Properties(event),
        };
    let patterns = Editor::Select {
        text: name.clone(),
        swatch: None,
        icon: now.and_then(|k| list.get(k)).map(|c| icon_name(&c.icon)),
        items: list
            .iter()
            .enumerate()
            .map(|(k, c)| {
                pick(
                    c.label.clone(),
                    Some(icon_name(&c.icon)),
                    now == Some(k),
                    Event::Pattern(slot, k),
                )
                .previewed(icon_name(&c.preview))
            })
            .collect(),
    };
    let mut rows = vec![
        Row::text("Desen", name).editor(edit(patterns)),
        Row::figure("Açı", fixed(p.angle, 2))
            .unit("°")
            .editor(number(Field::HatchAngle(slot))),
    ];
    match p.kind {
        HatchPatternType::Pattern => {
            let paper = p.scale.unwrap_or(0.0) * 1000.0 / plot_scale;
            rows.push(
                Row::figure("Ölçek", fixed(paper, 3)).editor(number(Field::HatchScale(slot))),
            );
        }
        HatchPatternType::Lines | HatchPatternType::Cross => rows.push(
            Row::figure("Aralık", f.length(p.spacing))
                .unit(f.length_unit_label())
                .editor(number(Field::HatchSpacing(slot))),
        ),
        _ => {}
    }
    if let Some(g) = &p.gradient {
        let colours = Editor::Select {
            text: g.color2.clone(),
            swatch: Some(g.color2.clone()),
            icon: None,
            items: kentos_geometry_core::tools::hatch::COLOURS
                .iter()
                .map(|(n, hex)| Choice::Pick {
                    label: (*n).to_owned(),
                    swatch: Some((*hex).to_owned()),
                    icon: None,
                    wide: false,
                    chosen: g.color2 == *hex,
                    enabled: true,
                    message: Message::Properties(Event::GradientColour(slot, (*hex).to_owned())),
                })
                .collect(),
        };
        const SHAPES: [(GradientShape, &str, &str); 3] = [
            (GradientShape::Linear, "Doğrusal", "hatchGradientLinear"),
            (GradientShape::Cylinder, "Silindir", "hatchGradientCylinder"),
            (GradientShape::Spherical, "Küre", "hatchGradientSpherical"),
        ];
        let shape = SHAPES
            .iter()
            .find(|s| s.0 == g.shape)
            .copied()
            .unwrap_or(SHAPES[0]);
        let shapes = Editor::Select {
            text: shape.1.to_owned(),
            swatch: None,
            icon: Some(shape.2),
            items: SHAPES
                .iter()
                .map(|&(s, label, icon)| {
                    pick(
                        label.to_owned(),
                        Some(icon),
                        s == g.shape,
                        Event::GradientShape(slot, s),
                    )
                })
                .collect(),
        };
        let yes_no = |b: bool| if b { "Evet" } else { "Hayır" };
        let inverted = Editor::Select {
            text: yes_no(g.inverted).to_owned(),
            swatch: None,
            icon: None,
            items: [true, false]
                .into_iter()
                .map(|b| {
                    pick(
                        yes_no(b).to_owned(),
                        None,
                        b == g.inverted,
                        Event::GradientInverted(slot, b),
                    )
                })
                .collect(),
        };
        rows.extend([
            Row::text("İkinci renk", g.color2.clone()).editor(edit(colours)),
            Row::text("Degrade biçimi", shape.1).editor(edit(shapes)),
            Row::text("Ters", yes_no(g.inverted)).editor(edit(inverted)),
        ]);
    }
    let tie = match &h.assoc {
        Some(a) => format!("Evet ({} nesne)", 1 + a.islands.len() + a.cutouts.len()),
        None => "Hayır".to_owned(),
    };
    let untie = (h.assoc.is_some() && !locked).then(|| Editor::Select {
        text: tie.clone(),
        swatch: None,
        icon: Some("hatchAssoc"),
        items: vec![pick(
            "İlişkiyi kopar".to_owned(),
            None,
            false,
            Event::Unlink(vec![slot]),
        )],
    });
    rows.push(Row::text("İlişkili", tie).editor(untie));
    rows
}

fn leader_rows(
    leaders: &[&kentos_contracts::LeaderEntity],
    slots: &[Slot],
    locked: bool,
    f: &Format,
) -> Vec<Row> {
    const MIXED: &str = "Çeşitli";
    let first = leaders[0];
    let same = |of: &dyn Fn(&kentos_contracts::LeaderEntity) -> bool| leaders.iter().all(|l| of(l));
    let note = same(&|l| l.text == first.text).then(|| first.text.clone().unwrap_or_default());
    let height = same(&|l| l.height == first.height).then_some(first.height);
    let turn = same(&|l| l.rotation == first.rotation).then_some(first.rotation);
    let arrow = same(&|l| l.arrow == first.arrow).then_some(first.arrow);
    let mask = same(&|l| l.mask == first.mask).then_some(first.mask);
    let on_off = |on: bool| if on { "Açık" } else { "Kapalı" };
    let edit = |editor: Editor| (!locked).then_some(editor);
    let arrow_text = arrow.map_or_else(|| MIXED.to_owned(), |a| leader::arrow_label(a).to_owned());
    let arrows = Editor::Select {
        text: arrow_text.clone(),
        swatch: None,
        icon: arrow.map(leader::arrow_icon),
        items: leader::ARROWS
            .iter()
            .map(|&(a, _, label, icon)| Choice::Pick {
                label: label.to_owned(),
                swatch: None,
                icon: Some(icon),
                wide: false,
                chosen: arrow == Some(a),
                enabled: true,
                message: Message::Properties(Event::LeaderArrow(slots.to_vec(), a)),
            })
            .collect(),
    };
    let mask_text = mask.map_or(MIXED, on_off).to_owned();
    let masks = Editor::Select {
        text: mask_text.clone(),
        swatch: None,
        icon: None,
        items: [true, false]
            .into_iter()
            .map(|on| Choice::Pick {
                label: on_off(on).to_owned(),
                swatch: None,
                icon: None,
                wide: false,
                chosen: mask == Some(on),
                enabled: true,
                message: Message::Properties(Event::LeaderMask(slots.to_vec(), on)),
            })
            .collect(),
    };
    let height_row = match height {
        Some(h) => Row::figure("Yükseklik", f.length_bare(h)).unit(f.length_unit_label()),
        None => Row::text("Yükseklik", MIXED),
    };
    let turn_row = match turn {
        Some(t) => Row::figure("Dönüş", fixed(t, 2)).unit("°"),
        None => Row::text("Dönüş", MIXED),
    };
    vec![
        Row::text("Not", note.unwrap_or_else(|| MIXED.to_owned()))
            .editor(edit(Editor::Text(Field::LeaderNote(slots.to_vec())))),
        height_row.editor(edit(Editor::Number(Field::LeaderHeight(slots.to_vec())))),
        turn_row.editor(edit(Editor::Number(Field::LeaderTurn(slots.to_vec())))),
        Row::text("Ok", arrow_text).editor(edit(arrows)),
        Row::text("Zemin", mask_text).editor(edit(masks)),
    ]
}

fn looks_numeric(value: &str) -> bool {
    let digits = value.strip_prefix('-').unwrap_or(value);
    let mut parts = digits.splitn(2, ['.', ',']);
    let whole = parts.next().unwrap_or_default();
    let all_digits = |s: &str| !s.is_empty() && s.chars().all(|c| c.is_ascii_digit());
    all_digits(whole) && parts.next().is_none_or(all_digits)
}

/// Several objects: what they share, editable unless one is on a locked
/// layer, and their totals (the web's `multiSections`).
fn many_sections(doc: &Document, objects: &[&Entity], (length, area): (f64, f64)) -> Vec<Section> {
    let ids: Vec<Slot> = objects.iter().map(|e| Slot(e.base().id)).collect();
    let first = objects[0].base();
    let same = |f: &dyn Fn(&kentos_contracts::EntityBase) -> Option<&str>| {
        objects
            .iter()
            .all(|e| f(e.base()) == f(first))
            .then(|| f(first))
    };
    let layer = objects
        .iter()
        .all(|e| e.base().layer_id == first.layer_id)
        .then_some(first.layer_id.as_str());
    let color = same(&|b| b.color.as_deref());
    let symbol = same(&|b| b.symbol.as_deref());
    let layers = doc.model.layers();
    // Each layer looked up once: a large selection has many objects on few layers.
    let mut seen = std::collections::HashSet::new();
    let any_locked = objects.iter().any(|e| {
        let layer = e.base().layer_id.as_str();
        seen.insert(layer) && layers.is_locked(layer)
    });
    let edit = |editor: Editor| (!any_locked).then_some(editor);

    // The weight of the objects drawn with lines; the others have none to show.
    let lined: Vec<&&Entity> = objects.iter().filter(|e| e.draws_lines()).collect();
    let weight = lined
        .iter()
        .all(|e| e.base().line_weight == lined[0].base().line_weight)
        .then(|| lined.first().and_then(|e| e.base().line_weight));
    let mut rows = vec![
        Row::text("Katman", "Kilitli katman içeriyor").editor(edit(layer_editor(doc, &ids, layer))),
        Row::text("Renk", color_text(color)).editor(edit(color_editor(&ids, color))),
    ];
    if !lined.is_empty() {
        let lined_ids: Vec<Slot> = lined.iter().map(|e| Slot(e.base().id)).collect();
        rows.push(
            Row::text("Kalınlık", weight_text(weight))
                .editor(edit(weight_editor(&lined_ids, weight))),
        );
    }
    rows.push(
        Row::text("Sembol", symbol_text(doc, symbol)).editor(edit(symbol_editor(doc, symbol))),
    );
    let f = Format::of(doc.settings());
    // The vertices' elevations of the lines, polylines, areas and points (docs/adr/0142), all
    // together as one list, as the web's row takes them: the value when every vertex has the
    // same, `kot yok` when none has one, else Çeşitli. What takes no elevation is left out.
    let takers: Vec<Slot> = objects
        .iter()
        .filter(|e| elevation::takes(e))
        .map(|e| Slot(e.base().id))
        .collect();
    if !takers.is_empty() {
        let summary = Elevations::of_objects(objects.iter().copied());
        let editor = edit(Editor::Number(Field::Elevations(takers)));
        match summary {
            Elevations::Value(_) | Elevations::None => {
                rows.extend(elevation_row("Kot", summary, &f, editor));
            }
            _ => rows.push(Row::text("Kot", "Çeşitli").editor(editor)),
        }
    }
    let mut sections = vec![Section {
        id: "general",
        title: "Ortak özellikler".into(),
        rows,
    }];
    // The selection's texts: their Hiza, Genişlik çarpanı and Zemin, common or Çeşitli (docs/adr/0145 §6).
    let texts: Vec<&kentos_contracts::TextEntity> = objects
        .iter()
        .filter_map(|e| match e {
            Entity::Text(t) => Some(t),
            _ => None,
        })
        .collect();
    if !texts.is_empty() {
        let slots: Vec<Slot> = texts.iter().map(|t| Slot(t.base.id)).collect();
        sections.push(Section {
            id: "texts",
            title: if texts.len() == objects.len() {
                "Yazı".into()
            } else {
                format!("Yazılar ({})", texts.len()).into()
            },
            rows: [
                style_rows(
                    doc.model.settings(),
                    true,
                    &texts
                        .iter()
                        .map(|t| t.face.text_style.as_ref())
                        .collect::<Vec<_>>(),
                    &slots,
                    any_locked,
                ),
                text_rows(&texts, &slots, any_locked),
                link_rows(&doc.model, &texts, &slots, any_locked),
            ]
            .concat(),
        });
    }
    // The selection's leaders: their note, height, turn, arrowhead and mask, common or “Çeşitli” (docs/adr/0146 §7).
    let leaders: Vec<&kentos_contracts::LeaderEntity> = objects
        .iter()
        .filter_map(|e| match e {
            Entity::Leader(l) => Some(l),
            _ => None,
        })
        .collect();
    if !leaders.is_empty() {
        let slots: Vec<Slot> = leaders.iter().map(|l| Slot(l.base.id)).collect();
        sections.push(Section {
            id: "leaders",
            title: if leaders.len() == objects.len() {
                "Kılavuz".into()
            } else {
                format!("Kılavuzlar ({})", leaders.len()).into()
            },
            rows: leader_rows(&leaders, &slots, any_locked, &f),
        });
    }
    // The selection's dimensions: their Zemin, and an ordinate's axis, a slope's elevations, common or “Çeşitli” (docs/adr/0147 §7).
    let dims: Vec<&kentos_contracts::DimensionEntity> = objects
        .iter()
        .filter_map(|e| match e {
            Entity::Dimension(d) => Some(d),
            _ => None,
        })
        .collect();
    if !dims.is_empty() {
        let slots: Vec<Slot> = dims.iter().map(|d| Slot(d.base.id)).collect();
        sections.push(Section {
            id: "dimensions",
            title: if dims.len() == objects.len() {
                "Ölçü".into()
            } else {
                format!("Ölçüler ({})", dims.len()).into()
            },
            rows: [
                style_rows(
                    doc.model.settings(),
                    false,
                    &dims
                        .iter()
                        .map(|d| d.look.dim_style.as_ref())
                        .collect::<Vec<_>>(),
                    &slots,
                    any_locked,
                ),
                dimension::rows(&dims, &slots, any_locked, &f),
            ]
            .concat(),
        });
    }
    let mut totals = Vec::new();
    if length > 0.0 {
        totals
            .push(Row::figure("Toplam uzunluk", f.length_bare(length)).unit(f.length_unit_label()));
    }
    if area > 0.0 {
        totals.push(Row::figure("Toplam alan", f.area_bare(area)).unit(f.area_unit_label()));
    }
    if !totals.is_empty() {
        sections.push(Section {
            id: "totals",
            title: "Toplamlar".into(),
            rows: totals,
        });
    }
    sections
}

/// `+n.toFixed(places)`: a number rounded as the web shows it.
fn fixed_number(n: f64, places: usize) -> f64 {
    kentos_interaction::fixed(n, places)
        .parse::<f64>()
        .unwrap_or(n)
}

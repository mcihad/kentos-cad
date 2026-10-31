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

use super::{Event, Field, Spot};

mod arch;
pub(crate) mod attachments;
pub(crate) mod casework;
mod opening_types;
pub(crate) mod cloud;
mod dimension;
mod dimension_set;
pub(crate) mod params;
pub(crate) mod raster;

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
    fn text(label: impl Into<Cow<'static, str>>, value: impl Into<String>) -> Self {
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
            sections: vec![document_section(doc, shown.crs)],
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

/// No selection: the drawing's summary; its coordinate system with `crs`.
fn document_section(doc: &Document, crs: bool) -> Section {
    let s = doc.settings();
    let layers = doc.model.layers();
    let active = layers
        .get(layers.active())
        .map_or_else(|| "—".to_owned(), |n| layer_path(layers, &n.id));
    let mut rows = vec![Row::text("Dosya", doc.name())];
    if crs {
        rows.push(Row::text("Koordinat sistemi", crate::crs::project_name(s)));
        rows.push(Row::figure("SRID", crate::crs::project_code(s)));
    }
    rows.extend([
        Row::figure("Çizim ölçeği", format!("1:{}", s.plot_scale)),
        Row::figure("Nesne sayısı", doc.entity_count().to_string()),
        Row::text("Etkin katman", active),
    ]);
    Section {
        id: "doc",
        title: "Çizim".into(),
        rows,
    }
}

/// A layer drawn from a map service (docs/adr/0208 §14): its kind, address,
/// layers, system, levels, connection, opacity, credits and how it is going;
/// a layer whose objects came from a service: where from and when.
pub(crate) fn service_section(
    doc: &Document,
    node: &kentos_commands::LayerNode,
    failure: Option<String>,
) -> Option<Section> {
    let settings = doc.settings();
    let connection = |id: &Option<String>| {
        id.as_ref().map(
            |id| match settings.connections.iter().find(|c| &c.id == id) {
                Some(c) => {
                    let here = crate::services::secrets::secrets()
                        .get(&c.origin, &c.id)
                        .is_some();
                    format!(
                        "{} ({}){}",
                        c.name,
                        c.auth.label(),
                        if c.auth == kentos_commands::AuthKind::None || here {
                            ""
                        } else {
                            ", değerleri bu cihazda yok"
                        }
                    )
                }
                None => id.clone(),
            },
        )
    };
    let system = |srid: u32| {
        kentos_project::crs::system(srid).map_or_else(
            || format!("EPSG:{srid}"),
            |c| format!("{} (EPSG:{srid})", c.name),
        )
    };
    if let Some(s) = &node.service {
        let mut rows = vec![
            Row::text("Tür", s.kind.label()),
            Row::text(
                "Adres",
                if s.url.is_empty() {
                    "Google Map Tiles API".to_owned()
                } else {
                    s.url.clone()
                },
            ),
        ];
        if !s.layers.is_empty() {
            rows.push(Row::text("Katmanlar", s.layers.join(", ")));
        }
        if let Some(st) = &s.style {
            rows.push(Row::text(
                if s.kind == kentos_commands::ServiceKind::Google {
                    "Harita türü"
                } else {
                    "Stil"
                },
                st.clone(),
            ));
        }
        if let Some(srid) = s.srid.or(s.grid.as_ref().map(|g| g.srid)) {
            rows.push(Row::text("Sistem", system(srid)));
        } else if matches!(
            s.kind,
            kentos_commands::ServiceKind::Xyz
                | kentos_commands::ServiceKind::Google
                | kentos_commands::ServiceKind::Vector
        ) {
            rows.push(Row::text("Sistem", system(3857)));
        }
        rows.push(Row::figure(
            "Katlar",
            format!(
                "{}–{}",
                s.min_zoom.unwrap_or(0),
                s.max_zoom.map_or_else(|| "—".to_owned(), |z| z.to_string())
            ),
        ));
        if let Some(c) = connection(&s.connection) {
            rows.push(Row::text("Bağlantı", c));
        }
        rows.push(Row::figure(
            "Saydamlık",
            format!("% {}", (s.opacity.unwrap_or(1.0) * 100.0).round()),
        ));
        if let Some(a) = &s.attribution {
            rows.push(Row::text("Atıf", a.clone()));
        }
        rows.push(Row::text(
            "Durum",
            failure.unwrap_or_else(|| "Çiziliyor".to_owned()),
        ));
        return Some(Section {
            id: "service",
            title: "Servis katmanı".into(),
            rows,
        });
    }
    let f = node.feed.as_ref()?;
    let mut rows = vec![
        Row::text("Tür", f.kind.label()),
        Row::text("Adres", f.url.clone()),
    ];
    if let Some(n) = &f.name {
        rows.push(Row::text("Tür adı", n.clone()));
    }
    if let Some(srid) = f.srid {
        rows.push(Row::text("İstenen sistem", system(srid)));
    }
    if let Some(q) = &f.filter {
        rows.push(Row::text("Süzgeç", q.clone()));
    }
    rows.push(Row::text(
        "Alan",
        if f.bbox.is_some() {
            "İstenen alan"
        } else {
            "Hepsi"
        },
    ));
    if let Some(k) = &f.key {
        rows.push(Row::text("Anahtar alan", k.clone()));
    }
    if let Some(c) = connection(&f.connection) {
        rows.push(Row::text("Bağlantı", c));
    }
    if let Some(t) = &f.fetched {
        rows.push(Row::text(
            "Son alınış",
            t.replace('T', " ").trim_end_matches('Z').to_owned() + " UTC",
        ));
    }
    Some(Section {
        id: "feed",
        title: "Veri kaynağı".into(),
        rows,
    })
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

/// A pick of a look's menu.
fn pick(label: String, chosen: bool, message: Message) -> Choice {
    Choice::Pick {
        label,
        swatch: None,
        icon: None,
        wide: false,
        chosen,
        enabled: true,
        message,
    }
}

/// Çizgi tipi ▾ (docs/adr/0260 §2, 0313 §7): by layer, then the four types, then the project's types with patterns;
/// `pattern` the objects' own pattern (none: various).
fn line_type_row(
    doc: &Document,
    ids: &[Slot],
    current: Option<Option<kentos_commands::LineType>>,
    pattern: Option<Option<&str>>,
) -> Row {
    let text = match (current, pattern) {
        (_, Some(Some(name))) => name.to_owned(),
        (None, _) | (_, None) => "Çeşitli".to_owned(),
        (Some(t), Some(None)) => crate::ribbon_panels::line_type_text(t).to_owned(),
    };
    let plain = pattern == Some(None);
    let set = |t| Message::Properties(Event::LineType(ids.to_vec(), t));
    let mut items = vec![
        pick(
            "Katmana göre".into(),
            plain && current == Some(None),
            set(None),
        ),
        Choice::Separator,
    ];
    items.extend(kentos_commands::LineType::ALL.into_iter().map(|t| {
        pick(
            crate::ribbon_panels::line_type_text(Some(t)).to_owned(),
            plain && current == Some(Some(t)),
            set(Some(t)),
        )
    }));
    let types = &doc.model.settings().line_types;
    if !types.is_empty() {
        items.push(Choice::Separator);
        items.extend(types.iter().map(|t| {
            pick(
                t.name.clone(),
                pattern
                    .flatten()
                    .is_some_and(|p| kentos_commands::same_line_type_name(p, &t.name)),
                Message::Properties(Event::LinePattern(ids.to_vec(), Some(t.name.clone()))),
            )
        }));
    }
    Row::text("Çizgi tipi", text.clone()).editor(Some(Editor::Select {
        text,
        swatch: None,
        icon: None,
        items,
    }))
}

/// Çizgi tipi ölçeği ▾ (docs/adr/0260 §2): the pattern's factor, 1 its absence.
fn line_type_scale_row(ids: &[Slot], current: Option<Option<f64>>) -> Row {
    let shown = plain_number;
    let text = match current {
        None => "Çeşitli".to_owned(),
        Some(s) => shown(s.unwrap_or(1.0)),
    };
    let set =
        |s: f64| Message::Properties(Event::LineTypeScale(ids.to_vec(), (s != 1.0).then_some(s)));
    let mut scales = vec![0.25, 0.5, 1.0, 2.0, 5.0, 10.0];
    if let Some(Some(s)) = current
        && !scales.contains(&s)
    {
        scales.push(s);
        scales.sort_by(f64::total_cmp);
    }
    let items = scales
        .into_iter()
        .map(|s| {
            pick(
                shown(s),
                current.map(|c| c.unwrap_or(1.0)) == Some(s),
                set(s),
            )
        })
        .collect();
    Row::text("Çizgi tipi ölçeği", text.clone()).editor(Some(Editor::Select {
        text,
        swatch: None,
        icon: None,
        items,
    }))
}

/// Saydamlık ▾ (docs/adr/0260 §3): by layer, then whole percents up to 90.
fn transparency_row(ids: &[Slot], current: Option<Option<f64>>) -> Row {
    let percent = |t: f64| format!("%{}", (t * 100.0).round());
    let text = match current {
        None => "Çeşitli".to_owned(),
        Some(None) => "Katmana göre".to_owned(),
        Some(Some(t)) => percent(t),
    };
    let set = |t| Message::Properties(Event::Transparency(ids.to_vec(), t));
    let mut items = vec![
        pick("Katmana göre".into(), current == Some(None), set(None)),
        Choice::Separator,
    ];
    let mut steps = vec![0.0, 0.1, 0.25, 0.5, 0.75, 0.9];
    if let Some(Some(t)) = current
        && !steps.contains(&t)
    {
        steps.push(t);
        steps.sort_by(f64::total_cmp);
    }
    items.extend(
        steps
            .into_iter()
            .map(|t| pick(percent(t), current == Some(Some(t)), set(Some(t)))),
    );
    Row::text("Saydamlık", text.clone()).editor(Some(Editor::Select {
        text,
        swatch: None,
        icon: None,
        items,
    }))
}

/// Maske ▾ of areas (docs/adr/0310 §5): none, framed or frameless; `current` none when they differ.
fn wipeout_row(ids: &[Slot], current: Option<Option<kentos_commands::Wipeout>>) -> Row {
    use kentos_commands::Wipeout;
    let name = |w: Option<Wipeout>| match w {
        None => "Yok",
        Some(Wipeout { frame: true }) => "Çerçeveli",
        Some(Wipeout { frame: false }) => "Çerçevesiz",
    };
    let text = current.map_or("Çeşitli", name).to_owned();
    let set = |w| Message::Properties(Event::Wipeout(ids.to_vec(), w));
    let items = [
        None,
        Some(Wipeout { frame: true }),
        Some(Wipeout { frame: false }),
    ]
    .into_iter()
    .map(|w| pick(name(w).into(), current == Some(w), set(w)))
    .collect();
    Row::text("Maske", text.clone()).editor(Some(Editor::Select {
        text,
        swatch: None,
        icon: None,
        items,
    }))
}

/// A number as the panel writes a factor: no trailing zeros (`2.5`, `10`).
pub(crate) fn plain_number(x: f64) -> String {
    let s = format!("{x:.4}");
    s.trim_end_matches('0').trim_end_matches('.').to_owned()
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

fn v(p: kentos_commands::Vec2) -> Vec2 {
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

/// A route's measures (docs/adr/0224 §6), read-only (the vertex table
/// writes them): a line's `M (başlangıç)` and `M (bitiş)`, a polyline's `M`
/// from its first measured vertex to its last, with a note when some
/// vertices have none; nothing for an object without a measure. The
/// numbers are the route's own, with the project's length decimals.
fn measure_rows(e: &Entity, f: &Format) -> Vec<Row> {
    let t = |m: f64| fixed(m, f.length_decimals);
    let shown = |m: Option<f64>| m.map_or_else(|| "ölçü yok".to_owned(), t);
    match e {
        Entity::Line(l) if l.ma.is_some() || l.mb.is_some() => vec![
            Row::figure("M (başlangıç)", shown(l.ma)),
            Row::figure("M (bitiş)", shown(l.mb)),
        ],
        Entity::Polyline(p) => {
            let own = p.ms.clone().unwrap_or_else(|| vec![None; p.pts.len()]);
            let all: Vec<Option<f64>> = own
                .into_iter()
                .chain(
                    p.parts
                        .iter()
                        .flatten()
                        .flat_map(|q| q.ms.clone().unwrap_or_else(|| vec![None; q.pts.len()])),
                )
                .collect();
            let ms: Vec<f64> = all.iter().flatten().copied().collect();
            let (Some(first), Some(last)) = (ms.first(), ms.last()) else {
                return Vec::new();
            };
            let mut rows = vec![Row::figure("M", format!("{} → {}", t(*first), t(*last)))];
            if ms.len() < all.len() {
                rows.push(Row::note("(bazı köşeler ölçüsüz)"));
            }
            rows
        }
        _ => Vec::new(),
    }
}

fn yes_no(on: bool) -> &'static str {
    if on { "Evet" } else { "Hayır" }
}

/// A polyline's one width for every edge, 0 without widths; none when its edges' differ
/// (docs/adr/0258 §2).
fn constant_width(line: &kentos_commands::PathEntity) -> Option<f64> {
    let Some(widths) = &line.widths else {
        return Some(0.0);
    };
    let n = line.pts.len();
    let edges = if line.closed { n } else { n.saturating_sub(1) };
    let first = widths.first()?[0];
    widths
        .get(..edges)?
        .iter()
        .all(|[a, b]| *a == first && *b == first)
        .then_some(first)
}

/// `3B uzunluk` of a line or a polyline, `3B çevre` of an area, when every
/// vertex has an elevation (docs/adr/0142): the length in space beside the
/// plan one, which stays the measure of record.
fn space_length(e: &Entity, f: &Format) -> Option<Row> {
    let (label, length) = elevation::space_length(e)?;
    Some(Row::figure(label, f.length_bare(length)).unit(f.length_unit_label()))
}

/// One object's sections: Genel, Geometri, an insert's Blok öznitelikleri, and Öznitelik bilgileri when it has other attributes.
fn entity_sections(doc: &Document, e: &Entity, shown: Shown) -> Vec<Section> {
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
    ];
    // A record is not drawn (docs/adr/0410 §4): no colour, transparency or symbol.
    let drawn = e.has_geometry();
    if drawn {
        general.push(Row::text("Renk", color_text(color)).editor(edit(color_editor(&ids, color))));
    }
    if e.draws_lines() {
        let weight = Some(base.line_weight);
        general.push(
            Row::text("Kalınlık", weight_text(weight)).editor(edit(weight_editor(&ids, weight))),
        );
        // Its own line type, its scale and transparency (docs/adr/0260).
        general.push(
            line_type_row(
                doc,
                &ids,
                Some(base.look.line_type),
                Some(base.look.line_pattern.as_deref()),
            )
            .editor_if(!locked),
        );
        general.push(line_type_scale_row(&ids, Some(base.look.line_type_scale)).editor_if(!locked));
    }
    if drawn {
        general.push(transparency_row(&ids, Some(base.look.transparency)).editor_if(!locked));
    }
    // An area's mask (docs/adr/0310 §5), where the program has masks.
    if shown.mask && matches!(e, Entity::Polygon(_)) {
        general.push(wipeout_row(&ids, Some(base.look.wipeout)).editor_if(!locked));
    }
    if drawn && takes_symbol(e) {
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
        if f.area_unit_label() == "m²" || f.unit != kentos_commands::DrawingUnit::M {
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
            geo.extend(measure_rows(e, &f));
            geo.push(metres("Uzunluk", dist(v(l.a), v(l.b))));
            geo.extend(space_length(e, &f));
            geo.push(bearing(v(l.a), v(l.b)));
        }
        Entity::Polyline(p) | Entity::Polygon(p) => {
            let polygon = matches!(e, Entity::Polygon(_));
            if !polygon
                && ((shown.separators
                    && doc.settings().workspace == Some(kentos_commands::Workspace::Cad))
                    || p.space_separator.is_some())
            {
                geo.extend(arch::separator_rows(doc,slot,p,locked));
            }
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
                if p.space_separator.is_some(){None}else{number(Field::Elevation(slot, Spot::All))},
            ));
            geo.extend(measure_rows(e, &f));
            geo.push(metres(
                if polygon { "Çevre" } else { "Uzunluk" },
                length_of.unwrap_or(0.0),
            ));
            geo.extend(space_length(e, &f));
            // A one-part polyline's Kapalı and Genişlik; a closed one encloses an area, shown,
            // while it has none of its own (docs/adr/0258).
            if let Entity::Polyline(line) = e
                && parts.is_empty()
            {
                let closings = Editor::Select {
                    text: yes_no(line.closed).to_owned(),
                    swatch: None,
                    icon: None,
                    items: [true, false]
                        .into_iter()
                        .map(|on| Choice::Pick {
                            label: yes_no(on).to_owned(),
                            swatch: None,
                            icon: None,
                            wide: false,
                            chosen: line.closed == on,
                            enabled: true,
                            message: Message::Properties(Event::PolylineClosed(vec![slot], on)),
                        })
                        .collect(),
                };
                geo.push(Row::text("Kapalı", yes_no(line.closed)).editor(edit(closings)));
                // Its widths where the program has a polyline's widths (docs/adr/0802).
                if shown.widths {
                    let width = if line.space_separator.is_some() {
                        None
                    } else {
                        number(Field::PolylineWidth(slot))
                    };
                    geo.push(match constant_width(line) {
                        Some(w) => metres("Genişlik", w).editor(width),
                        None => Row::text("Genişlik", "Değişken").editor(width),
                    });
                }
                if line.closed {
                    geo.extend(area(kentos_interaction::enclosed(e).unwrap_or(0.0)));
                }
            }
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
        // A wall (docs/adr/0339 §6): its width, justification, offset, style, ends, height, level and phase, each
        // written through `cad.walls.edit`; its length along its path and its body's area.
        Entity::Wall(w) => {
            let x = &w.wall;
            let change = |c: kentos_commands::WallChange| Message::Properties(Event::Wall(slot, c));
            let pick = |label: String, chosen: bool, message: Message, icon: Option<&'static str>| Choice::Pick {
                label,
                swatch: None,
                icon,
                wide: false,
                chosen,
                enabled: true,
                message,
            };
            let select = |text: String, items: Vec<Choice>| {
                edit(Editor::Select {
                    text,
                    swatch: None,
                    icon: None,
                    items,
                })
            };
            let fixed = kentos_commands::fixed_width(&x.components).is_some();
            geo.push(metres("Genişlik", x.width).editor(if fixed { None } else { number(Field::WallWidth(slot)) }));
            let justify = kentos_commands::WallJustify::ALL
                .iter()
                .map(|&j| {
                    let icon = match j {
                        kentos_commands::WallJustify::Left => "wallLeft",
                        kentos_commands::WallJustify::Center => "wallCenter",
                        kentos_commands::WallJustify::Right => "wallRight",
                    };
                    pick(
                        j.label().to_owned(),
                        j == x.justify,
                        change(kentos_commands::WallChange {
                            justify: Some(j),
                            ..Default::default()
                        }),
                        Some(icon),
                    )
                })
                .collect();
            geo.push(Row::text("Hiza", x.justify.label()).editor(select(x.justify.label().to_owned(), justify)));
            geo.push(metres("Öteleme", x.offset).editor(number(Field::WallOffset(slot))));
            let offered = kentos_interaction::wall::offered(doc.settings());
            let style_name = x
                .style
                .as_deref()
                .and_then(|id| offered.iter().find(|s| s.id == id))
                .map_or_else(|| kentos_interaction::wall::NO_STYLE.to_owned(), |s| s.name.clone());
            let mut styles = vec![pick(
                kentos_interaction::wall::NO_STYLE.to_owned(),
                x.style.is_none(),
                change(kentos_commands::WallChange {
                    style: Some(None),
                    ..Default::default()
                }),
                None,
            )];
            styles.extend(offered.iter().map(|s| {
                pick(
                    s.name.clone(),
                    x.style.as_deref() == Some(s.id.as_str()),
                    change(kentos_commands::WallChange {
                        style: Some(Some(s.id.clone())),
                        ..Default::default()
                    }),
                    None,
                )
            }));
            styles.push(Choice::Separator);
            styles.push(Choice::Command {
                label: kentos_interaction::wall::WALL_STYLES_ENTRY,
                icon: crate::icons::from_web(Some("wallStyles")),
                id: kentos_interaction::wall::WALL_STYLES,
            });
            geo.push(Row::text("Stil", style_name.clone()).editor(select(style_name, styles)));
            let open = x.caps == kentos_commands::WallCaps::Open;
            let caps = [(false, "Kapalı"), (true, "Açık")]
                .into_iter()
                .map(|(o, l)| {
                    let caps = if o { kentos_commands::WallCaps::Open } else { kentos_commands::WallCaps::Closed };
                    pick(
                        l.to_owned(),
                        o == open,
                        change(kentos_commands::WallChange {
                            caps: Some(caps),
                            ..Default::default()
                        }),
                        None,
                    )
                })
                .collect();
            let caps_text = if open { "Açık" } else { "Kapalı" };
            geo.push(Row::text("Uçlar", caps_text).editor(select(caps_text.to_owned(), caps)));
            // Its ends' joins and its cleanup (docs/adr/0340 §6).
            let op = |o: kentos_commands::WallsOperation| Message::Properties(Event::WallOp(slot, o));
            let join_icon = |j: kentos_commands::WallJoin| match j {
                kentos_commands::WallJoin::Auto => "wallJoinAuto",
                kentos_commands::WallJoin::None => "wallJoinNone",
                kentos_commands::WallJoin::Miter => "wallJoinMiter",
                kentos_commands::WallJoin::Butt => "wallJoinButt",
                kentos_commands::WallJoin::Through => "wallJoinThrough",
            };
            if !x.closed {
                for (title, now, at_end) in [("Başlangıç birleşimi", x.joins.start, false), ("Son birleşimi", x.joins.end, true)] {
                    let items = kentos_commands::WallJoin::ALL
                        .iter()
                        .map(|&j| {
                            let (start, end) = if at_end { (None, Some(j)) } else { (Some(j), None) };
                            pick(
                                j.label().to_owned(),
                                j == now,
                                op(kentos_commands::WallsOperation::Joins { start, end, cleanup: None }),
                                Some(join_icon(j)),
                            )
                        })
                        .collect();
                    geo.push(Row::text(title, now.label()).editor(select(now.label().to_owned(), items)));
                }
            }
            let clean = [(true, "Önceliğe göre"), (false, "Kapalı")]
                .into_iter()
                .map(|(c, l)| {
                    pick(
                        l.to_owned(),
                        c == x.cleanup,
                        op(kentos_commands::WallsOperation::Joins { start: None, end: None, cleanup: Some(c) }),
                        None,
                    )
                })
                .collect();
            let clean_text = if x.cleanup { "Önceliğe göre" } else { "Kapalı" };
            geo.push(Row::text("Temizlik", clean_text).editor(select(clean_text.to_owned(), clean)));
            let height = x.height.map_or_else(|| "Katın".to_owned(), |h| f.length_bare(h));
            geo.push(
                Row::figure("Yükseklik", height)
                    .unit(if x.height.is_some() { f.length_unit_label() } else { "" })
                    .editor(number(Field::WallHeight(slot))),
            );
            let levels = doc.settings().arch.as_ref().map(|a| a.levels.clone()).unwrap_or_default();
            let level_name = x
                .arch
                .level
                .as_deref()
                .map(|id| levels.iter().find(|l| l.id == id).map_or_else(|| id.to_owned(), |l| l.name.clone()))
                .unwrap_or_else(|| "Yok".to_owned());
            let mut level_items = vec![pick(
                "Yok".to_owned(),
                x.arch.level.is_none(),
                change(kentos_commands::WallChange {
                    level: Some(None),
                    ..Default::default()
                }),
                None,
            )];
            level_items.extend(levels.iter().map(|l| {
                pick(
                    l.name.clone(),
                    x.arch.level.as_deref() == Some(l.id.as_str()),
                    change(kentos_commands::WallChange {
                        level: Some(Some(l.id.clone())),
                        ..Default::default()
                    }),
                    None,
                )
            }));
            level_items.push(Choice::Separator);
            level_items.push(Choice::Command {
                label: "Katlar…",
                icon: crate::icons::from_web(Some("archSettings")),
                id: "arch.settings",
            });
            geo.push(Row::text("Kat", level_name.clone()).editor(select(level_name, level_items)));
            let phase_name = x.arch.phase.map_or("Yok", kentos_commands::ArchPhase::label);
            let mut phases = vec![pick(
                "Yok".to_owned(),
                x.arch.phase.is_none(),
                change(kentos_commands::WallChange {
                    phase: Some(None),
                    ..Default::default()
                }),
                None,
            )];
            phases.extend(kentos_commands::ArchPhase::ALL.iter().map(|&p| {
                pick(
                    p.label().to_owned(),
                    x.arch.phase == Some(p),
                    change(kentos_commands::WallChange {
                        phase: Some(Some(p)),
                        ..Default::default()
                    }),
                    None,
                )
            }));
            geo.push(Row::text("Evre", phase_name).editor(select(phase_name.to_owned(), phases)));
            geo.push(metres("Uzunluk", length_of.unwrap_or(0.0)));
            geo.extend(area(area_of.unwrap_or(0.0)));
        }
        // A grid, column, stair or ramp (docs/adr/0343 §5): their rows below.
        Entity::Grid(_)
        | Entity::Column(_)
        | Entity::Stair(_)
        | Entity::Ramp(_)
        | Entity::Tag(_)
        | Entity::ViewMark(_) => {
            geo.extend(arch::rows(doc, slot, e, locked, &f));
        }
        // A space (docs/adr/0342 §5): its number, name, usage, method, level, phase, height, finishes, measures and
        // boundary, each written through `cad.spaces.edit`; why it lost its walls.
        Entity::Space(sp) => {
            use kentos_commands::{SpaceChange, SpaceColumns, SpaceMethod, SpaceScope, SpacesOperation};
            let x = &sp.space;
            let set = |c: SpaceChange| Message::Properties(Event::Space(slot, SpacesOperation::Set(c)));
            let pick = |label: String, chosen: bool, message: Message, icon: Option<&'static str>| Choice::Pick {
                label,
                swatch: None,
                icon,
                wide: false,
                chosen,
                enabled: true,
                message,
            };
            let select = |text: String, items: Vec<Choice>| {
                edit(Editor::Select {
                    text,
                    swatch: None,
                    icon: None,
                    items,
                })
            };
            let words = |label: &'static str, v: &Option<String>, field: Field| {
                Row::text(label, v.clone().unwrap_or_default()).editor(edit(Editor::Text(field)))
            };
            geo.push(words("Numara", &x.number, Field::SpaceNumber(slot)));
            geo.push(words("Ad", &x.name, Field::SpaceName(slot)));
            geo.push(words("Kullanım", &x.usage, Field::SpaceUsage(slot)));
            if let Some(arch) = &doc.settings().arch {
                for (label, values, current, is_name) in [
                    ("Ad listesi", &arch.space_names, &x.name, true),
                    ("Kullanım listesi", &arch.space_usages, &x.usage, false),
                ] {
                    if values.is_empty() {
                        continue;
                    }
                    let items = values
                        .iter()
                        .map(|value| {
                            pick(
                                value.clone(),
                                current.as_ref() == Some(value),
                                set(if is_name {
                                    SpaceChange {
                                        name: Some(Some(value.clone())),
                                        ..Default::default()
                                    }
                                } else {
                                    SpaceChange {
                                        usage: Some(Some(value.clone())),
                                        ..Default::default()
                                    }
                                }),
                                None,
                            )
                        })
                        .collect();
                    let shown = current
                        .clone()
                        .filter(|v| values.contains(v))
                        .unwrap_or_else(|| "Listeden seç".into());
                    geo.push(Row::text(label, shown.clone()).editor(select(shown, items)));
                }
            }
            // Boşluk (docs/adr/0708): a shaft or an opening in the floor, drawn crossed, its area summed apart.
            let void_words = if x.void { "Evet (şaft)" } else { "Hayır (oda)" };
            let voids = [(true, "Evet (şaft)"), (false, "Hayır (oda)")]
                .into_iter()
                .map(|(v, label)| pick(label.into(), v == x.void, set(SpaceChange { void: Some(v), ..Default::default() }), None))
                .collect();
            geo.push(Row::text("Boşluk", void_words).editor(select(void_words.into(), voids)));
            let methods = SpaceMethod::ALL
                .iter()
                .map(|&m| {
                    pick(
                        m.label().to_owned(),
                        m == x.method,
                        set(SpaceChange { method: Some(m), ..Default::default() }),
                        Some(kentos_interaction::space_tools::method_icon(Some(m))),
                    )
                })
                .collect();
            geo.push(Row::text("Yöntem", x.method.label()).editor(select(x.method.label().to_owned(), methods)));
            if x.seed.is_some() {
                let items=SpaceColumns::ALL.iter().map(|&m|pick(m.label().into(),m==x.columns,
                    set(SpaceChange {columns:Some(m),..Default::default()}),None)).collect();
                geo.push(Row::text("Kolon alanı",x.columns.label()).editor(select(x.columns.label().into(),items)));
            } else {
                geo.push(Row::text("Kolon alanı","Çizilen sınır korunur"));
            }
            let levels = doc.settings().arch.as_ref().map(|a| a.levels.clone()).unwrap_or_default();
            let level_name = x
                .arch
                .level
                .as_deref()
                .map(|id| levels.iter().find(|l| l.id == id).map_or_else(|| id.to_owned(), |l| l.name.clone()))
                .unwrap_or_else(|| if x.scope==SpaceScope::Unassigned { "Atanmamış".into() }
                    else if x.seed.is_some() { "Tüm katlar".into() } else { "Yok".into() });
            let mut level_items = vec![pick(
                if x.seed.is_some() { "Tüm katlar".into() } else { "Yok".into() },
                x.arch.level.is_none() && x.scope.is_project(),
                set(SpaceChange { level: Some(None), scope:Some(SpaceScope::Project), ..Default::default() }),
                None,
            )];
            if x.seed.is_some() {
                level_items.push(pick("Yalnız atanmamış".into(),x.scope==SpaceScope::Unassigned,
                    set(SpaceChange {level:Some(None),scope:Some(SpaceScope::Unassigned),..Default::default()}),None));
            }
            level_items.extend(levels.iter().map(|l| {
                pick(
                    l.name.clone(),
                    x.arch.level.as_deref() == Some(l.id.as_str()),
                    set(SpaceChange { level: Some(Some(l.id.clone())), scope:Some(SpaceScope::Project), ..Default::default() }),
                    None,
                )
            }));
            level_items.push(Choice::Separator);
            level_items.push(Choice::Command {
                label: "Katlar…",
                icon: crate::icons::from_web(Some("archSettings")),
                id: "arch.settings",
            });
            geo.push(Row::text("Kat", level_name.clone()).editor(select(level_name, level_items)));
            let phase_name = x.arch.phase.map_or("Yok", kentos_commands::ArchPhase::label);
            let mut phases = vec![pick(
                "Yok".to_owned(),
                x.arch.phase.is_none(),
                set(SpaceChange { phase: Some(None), ..Default::default() }),
                None,
            )];
            phases.extend(kentos_commands::ArchPhase::ALL.iter().map(|&p| {
                pick(
                    p.label().to_owned(),
                    x.arch.phase == Some(p),
                    set(SpaceChange { phase: Some(Some(p)), ..Default::default() }),
                    None,
                )
            }));
            geo.push(Row::text("Evre", phase_name).editor(select(phase_name.to_owned(), phases)));
            let level_height = x
                .arch
                .level
                .as_deref()
                .and_then(|id| levels.iter().find(|l| l.id == id))
                .and_then(|l| l.height);
            let height = x.height.or(level_height);
            let cad = doc.settings().workspace == Some(kentos_commands::Workspace::Cad);
            let shown = match (x.height, level_height) {
                (Some(h), _) => f.length_bare(h),
                (None, Some(h)) if cad => f.length_bare(h),
                (None, Some(h)) => format!("{} (katın)", f.length_bare(h)),
                (None, None) => "Yok".to_owned(),
            };
            geo.push(
                Row::figure("Yükseklik", shown)
                    .unit(if height.is_some() { f.length_unit_label() } else { "" })
                    .editor(number(Field::SpaceHeight(slot))),
            );
            if cad && x.height.is_none() && level_height.is_some() {
                // A numeric editor must not clip or parse a decorated fallback.
                geo.push(Row::note("Katlar arası"));
            }
            geo.push(words("Zemin kaplaması", &x.finishes.floor, Field::SpaceFloor(slot)));
            geo.push(words("Duvar kaplaması", &x.finishes.wall, Field::SpaceWall(slot)));
            geo.push(words("Tavan kaplaması", &x.finishes.ceiling, Field::SpaceCeiling(slot)));
            let a = kentos_native_application::spaces::area_of(&x.parts);
            geo.extend(area(a));
            geo.push(metres("Çevre", kentos_native_application::spaces::perimeter_of(&x.parts)));
            if let Some(h) = height {
                geo.push(Row::figure("Hacim", fixed(a * h, decimals)).unit("m³"));
            }
            let bound = if x.seed.is_some() { "Duvarlardan" } else { "Çizilmiş" };
            let source_items=vec![Choice::Command {label:"Kaynakları seç…",icon:crate::icons::from_web(Some("space")),id:"tool.spaceSources"}];
            let source_summary: String = x.sources.as_ref().map_or_else(
                || "Tüm uygun kaynaklar".into(),
                |ids| format!("Seçilen {} kaynak", ids.len()),
            );
            geo.push(Row::text("Kaynaklar", source_summary.clone())
                .editor(select(source_summary, source_items)));
            let rebound = vec![Choice::Command {label:"İç nokta seç…",icon:crate::icons::from_web(Some("space")),id:"tool.spaceRebound"},pick(
                "Duvarlardan yeniden bul".to_owned(),
                false,
                Message::Properties(Event::Space(slot, SpacesOperation::Rebound { seed: None })),
                Some("space"),
            )];
            geo.push(Row::text("Sınır", bound).editor(select(bound.to_owned(), rebound)));
            if let Some(b) = x.broken {
                match b {
                    kentos_commands::SpaceBreak::LostEnclosure if !x.scope.is_project() => {
                        geo.push(Row::text("Durum","Oda bulunamadı"));
                        let mut note=Row::note("Seçilen kaynaklarda mahalın iç noktasını çevreleyen kullanılabilir oda bulunamadı. Son sınır korunuyor. Duvarları ve iç noktayı kontrol edin.");
                        note.warn=true;
                        geo.push(note);
                    }
                    kentos_commands::SpaceBreak::LostEnclosure => geo.push(Row::text("Durum", b.words())),
                    kentos_commands::SpaceBreak::NoFreeArea | kentos_commands::SpaceBreak::InvalidBoundarySource | kentos_commands::SpaceBreak::SourceUnavailable => {
                        geo.push(Row::text("Durum", if b == kentos_commands::SpaceBreak::NoFreeArea { "Alan kalmadı" } else { "Kaynak hatalı" }));
                        let mut note = Row::note(b.words());
                        note.warn = true;
                        geo.push(note);
                    }
                }
            }
        }
        // A slab (docs/adr/0722 §6): its role, thickness, top's offset and elevation, edge and its width, material,
        // level, phase and whether Metraj sums it, each written through `cad.slabs.edit`; its area, perimeter, volume
        // and edge's length.
        Entity::Slab(sl) => {
            use kentos_commands::{NewSlabEdge, SlabChange, SlabEdgeKind, SlabRole, SlabsOperation};
            let x = &sl.slab;
            let set = |c: SlabChange| Message::Properties(Event::Slab(slot, SlabsOperation::Set(c)));
            let pick = |label: String, chosen: bool, message: Message, icon: Option<&'static str>| Choice::Pick {
                label,
                swatch: None,
                icon,
                wide: false,
                chosen,
                enabled: true,
                message,
            };
            let select = |text: String, items: Vec<Choice>| {
                edit(Editor::Select {
                    text,
                    swatch: None,
                    icon: None,
                    items,
                })
            };
            let roles = SlabRole::ALL
                .iter()
                .map(|&r| pick(r.label().into(), r == x.role, set(SlabChange { role: Some(r), ..Default::default() }), Some("slab")))
                .collect();
            geo.push(Row::text("Döşeme türü", x.role.label()).editor(select(x.role.label().into(), roles)));
            geo.push(
                Row::figure("Kalınlık", f.length_bare(x.thickness))
                    .unit(f.length_unit_label())
                    .editor(number(Field::SlabThickness(slot))),
            );
            geo.push(
                Row::figure("Kot farkı", f.length_bare(x.offset))
                    .unit(f.length_unit_label())
                    .editor(number(Field::SlabOffset(slot))),
            );
            let levels = doc.settings().arch.as_ref().map(|a| a.levels.clone()).unwrap_or_default();
            let level = x.arch.level.as_deref().and_then(|id| levels.iter().find(|l| l.id == id));
            if let Some(l) = level {
                geo.push(Row::figure("Üst kot", f.length_bare(l.elevation + x.offset)).unit(f.length_unit_label()));
            }
            let edge_name = x.edge.as_ref().map_or("Yok", |e| e.kind.label());
            let edges = std::iter::once(None)
                .chain(SlabEdgeKind::ALL.iter().map(|k| Some(*k)))
                .map(|k| {
                    pick(
                        k.map_or("Yok", SlabEdgeKind::label).into(),
                        k == x.edge.as_ref().map(|e| e.kind),
                        set(SlabChange { edge: Some(k.map(|kind| NewSlabEdge { kind, width: None })), ..Default::default() }),
                        kentos_interaction::slab_tools::edge_icon(k),
                    )
                })
                .collect();
            geo.push(Row::text("Kenar", edge_name).editor(select(edge_name.into(), edges)));
            if let Some(e) = &x.edge {
                geo.push(
                    Row::figure("Kenar eni", f.length_bare(e.width))
                        .unit(f.length_unit_label())
                        .editor(number(Field::SlabEdgeWidth(slot))),
                );
                let run = kentos_interaction::slab_tools::edge_length(x);
                if run > 0.0 {
                    geo.push(metres("Kenar uzunluğu", run));
                } else {
                    geo.push(Row::text("Kenar uzunluğu", "Serbest kenar yok"));
                }
            }
            geo.push(
                Row::text("Malzeme", x.material.clone().unwrap_or_default()).editor(edit(Editor::Text(Field::SlabMaterial(slot)))),
            );
            let level_name = level.map_or_else(|| x.arch.level.clone().unwrap_or_else(|| "Yok".into()), |l| l.name.clone());
            let mut level_items = vec![pick(
                "Yok".into(),
                x.arch.level.is_none(),
                set(SlabChange { level: Some(None), ..Default::default() }),
                None,
            )];
            level_items.extend(levels.iter().map(|l| {
                pick(
                    l.name.clone(),
                    x.arch.level.as_deref() == Some(l.id.as_str()),
                    set(SlabChange { level: Some(Some(l.id.clone())), ..Default::default() }),
                    None,
                )
            }));
            level_items.push(Choice::Separator);
            level_items.push(Choice::Command { label: "Katlar…", icon: crate::icons::from_web(Some("archSettings")), id: "arch.settings" });
            geo.push(Row::text("Kat", level_name.clone()).editor(select(level_name, level_items)));
            let phase_name = x.arch.phase.map_or("Yok", kentos_commands::ArchPhase::label);
            let mut phases =
                vec![pick("Yok".to_owned(), x.arch.phase.is_none(), set(SlabChange { phase: Some(None), ..Default::default() }), None)];
            phases.extend(kentos_commands::ArchPhase::ALL.iter().map(|&p| {
                pick(p.label().to_owned(), x.arch.phase == Some(p), set(SlabChange { phase: Some(Some(p)), ..Default::default() }), None)
            }));
            geo.push(Row::text("Evre", phase_name).editor(select(phase_name.to_owned(), phases)));
            let summed = if x.excluded { "Dışında" } else { "Toplamda" };
            let sums = [(false, "Toplamda"), (true, "Dışında")]
                .into_iter()
                .map(|(v, label)| pick(label.into(), v == x.excluded, set(SlabChange { excluded: Some(v), ..Default::default() }), None))
                .collect();
            geo.push(Row::text("Metraj", summed).editor(select(summed.into(), sums)));
            let a = kentos_native_application::spaces::area_of(&x.parts);
            geo.extend(area(a));
            geo.push(metres("Çevre", kentos_native_application::spaces::perimeter_of(&x.parts)));
            geo.push(Row::figure("Hacim", fixed(a * x.thickness, decimals)).unit("m³"));
        }
        // An opening (docs/adr/0341 §5): its form, type, width, place along its wall, side, hinge, leaves, operation,
        // swing, frame, sill, head and number, each written through `cad.openings.edit`; its wall, or why it is in none.
        Entity::Opening(o) => {
            use kentos_commands::{OpeningChange, OpeningForm, OpeningHand, OpeningOperation, OpeningSide, OpeningsOperation};
            let x = &o.opening;
            let set = |c: OpeningChange| Message::Properties(Event::Opening(slot, OpeningsOperation::Set(c)));
            let pick = |label: String, chosen: bool, message: Message, icon: Option<&'static str>| Choice::Pick {
                label,
                swatch: None,
                icon,
                wide: false,
                chosen,
                enabled: true,
                message,
            };
            let select = |text: String, items: Vec<Choice>| {
                edit(Editor::Select {
                    text,
                    swatch: None,
                    icon: None,
                    items,
                })
            };
            let forms = OpeningForm::ALL
                .iter()
                .map(|&v| {
                    let icon = match v {
                        OpeningForm::Door => "door",
                        OpeningForm::Window => "window",
                        OpeningForm::Opening => "wallOpening",
                        OpeningForm::Niche => "wallNiche",
                        OpeningForm::Flue => "wallFlue",
                    };
                    pick(v.label().to_owned(), v == x.form, set(OpeningChange { form: Some(v), ..Default::default() }), Some(icon))
                })
                .collect();
            geo.push(Row::text("Biçim", x.form.label()).editor(select(x.form.label().to_owned(), forms)));
            // The project's types first (docs/adr/0772 §1), then KentOS's own, copied in.
            let project: Vec<(String, String)> = doc
                .settings()
                .arch
                .as_ref()
                .map(|a| a.opening_types.iter().filter(|t| t.form == x.form).map(|t| (t.id.clone(), t.name.clone())).collect())
                .unwrap_or_default();
            let builtin: Vec<(String, String)> =
                kentos_commands::builtin_opening_types().into_iter().map(|t| (t.id, t.name)).collect();
            let type_name = x
                .style
                .as_deref()
                .and_then(|id| project.iter().chain(builtin.iter()).find(|t| t.0 == id))
                .map_or_else(|| "Yok".to_owned(), |t| t.1.clone());
            let project_count = project.len();
            let mut type_items: Vec<Choice> = project
                .iter()
                .chain(builtin.iter())
                .map(|t| {
                    pick(
                        t.1.clone(),
                        x.style.as_deref() == Some(t.0.as_str()),
                        Message::Properties(Event::Opening(
                            slot,
                            OpeningsOperation::ReplaceType { from: None, to: t.0.clone(), level: None },
                        )),
                        None,
                    )
                })
                .collect();
            if project_count > 0 {
                type_items.insert(project_count, Choice::Separator);
            }
            type_items.push(Choice::Separator);
            type_items.push(Choice::Command {
                label: "Kapı ve pencere tipleri…",
                icon: crate::icons::from_web(Some("openingTypes")),
                id: "arch.openingTypes",
            });
            geo.push(Row::text("Tür", type_name.clone()).editor(select(type_name, type_items)));
            geo.push(metres("Genişlik", x.width).editor(number(Field::OpeningWidth(slot))));
            geo.push(metres("Konum", x.at).editor(number(Field::OpeningAt(slot))));
            // A niche's or flue's face, depth and a flue's distance from its face (docs/adr/0719).
            if x.form.is_recess() {
                let sides = [(OpeningSide::Left, "Sol", "openingSideLeft"), (OpeningSide::Right, "Sağ", "openingSideRight")]
                    .into_iter()
                    .map(|(v, l, i)| pick(l.to_owned(), v == x.side, set(OpeningChange { side: Some(v), ..Default::default() }), Some(i)))
                    .collect();
                let side = if x.side == OpeningSide::Right { "Sağ" } else { "Sol" };
                geo.push(Row::text("Yüz", side).editor(select(side.to_owned(), sides)));
                match x.depth {
                    Some(d) => geo.push(metres("Derinlik", d).editor(number(Field::OpeningDepth(slot)))),
                    None => geo.push(Row::figure("Derinlik", "Yok").editor(number(Field::OpeningDepth(slot)))),
                }
                if x.form == OpeningForm::Flue {
                    match x.inset {
                        Some(v) => geo.push(metres("Yüzden uzaklık", v).editor(number(Field::OpeningInset(slot)))),
                        None => geo.push(Row::text("Yüzden uzaklık", "Ortada").editor(number(Field::OpeningInset(slot)))),
                    }
                }
            }
            if x.form == OpeningForm::Door {
                let sides = [(OpeningSide::Left, "Sol", "openingSideLeft"), (OpeningSide::Right, "Sağ", "openingSideRight")]
                    .into_iter()
                    .map(|(v, l, i)| pick(l.to_owned(), v == x.side, set(OpeningChange { side: Some(v), ..Default::default() }), Some(i)))
                    .collect();
                let side = if x.side == OpeningSide::Right { "Sağ" } else { "Sol" };
                geo.push(Row::text("Yan", side).editor(select(side.to_owned(), sides)));
                let hands = [(OpeningHand::Start, "Baş"), (OpeningHand::End, "Son")]
                    .into_iter()
                    .map(|(v, l)| pick(l.to_owned(), v == x.hand, set(OpeningChange { hand: Some(v), ..Default::default() }), None))
                    .collect();
                let hand = if x.hand == OpeningHand::End { "Son" } else { "Baş" };
                geo.push(Row::text("Menteşe", hand).editor(select(hand.to_owned(), hands)));
            }
            if matches!(x.form, OpeningForm::Door | OpeningForm::Window) {
                let leaves = [1u8, 2]
                    .into_iter()
                    .map(|n| pick(n.to_string(), n == x.leaves, set(OpeningChange { leaves: Some(n), ..Default::default() }), None))
                    .collect();
                geo.push(Row::figure("Kanat", x.leaves.to_string()).editor(select(x.leaves.to_string(), leaves)));
                let ops = OpeningOperation::ALL
                    .iter()
                    .map(|&v| pick(v.label().to_owned(), v == x.operation, set(OpeningChange { operation: Some(v), ..Default::default() }), None))
                    .collect();
                geo.push(Row::text("Açılış", x.operation.label()).editor(select(x.operation.label().to_owned(), ops)));
            }
            if x.form == OpeningForm::Door && x.operation == OpeningOperation::Swing {
                geo.push(
                    Row::figure("Açı", format!("{}", x.swing_angle()))
                        .unit("°")
                        .editor(number(Field::OpeningSwing(slot))),
                );
                let arcs = [(true, "Evet"), (false, "Hayır")]
                    .into_iter()
                    .map(|(v, l)| pick(l.to_owned(), v == x.show_swing, set(OpeningChange { show_swing: Some(v), ..Default::default() }), None))
                    .collect();
                let arc = if x.show_swing { "Evet" } else { "Hayır" };
                geo.push(Row::text("Açılım yayı", arc).editor(select(arc.to_owned(), arcs)));
            }
            let optional = |label: &'static str, v: Option<f64>, field: Field| match v {
                Some(v) => metres(label, v).editor(number(field)),
                None => Row::figure(label, "Yok").editor(number(field)),
            };
            if matches!(x.form, OpeningForm::Door | OpeningForm::Window) {
                geo.push(optional("Kasa", x.frame, Field::OpeningFrame(slot)));
            }
            match x.form {
                OpeningForm::Window => geo.push(optional("Denizlik", x.sill, Field::OpeningSill(slot))),
                OpeningForm::Opening | OpeningForm::Niche | OpeningForm::Flue => {
                    geo.push(optional("Alt kot", x.sill, Field::OpeningSill(slot)));
                }
                OpeningForm::Door => {}
            }
            if x.form.is_recess() {
                geo.push(optional("Üst kot", x.head, Field::OpeningHead(slot)));
            } else {
                geo.push(optional("Lento", x.head, Field::OpeningHead(slot)));
            }
            // Whether plan cuts it, at the plan's cut height (docs/adr/0719 §2).
            if x.form.cut_by_height() {
                let cut = kentos_commands::PLAN_CUT_HEIGHT;
                let words = if x.cut_in_plan(cut) {
                    format!("Kesilir ({} kesme)", f.length(cut))
                } else {
                    format!("Kesik çizgi ({} kesmenin dışında)", f.length(cut))
                };
                geo.push(Row::text("Planda", words));
            }
            geo.push(
                Row::text("Numara", x.number.clone().unwrap_or_default())
                    .editor(edit(Editor::Text(Field::OpeningNumber(slot)))),
            );
            let wall = kentos_domain::Uuid::parse_str(&x.host)
                .ok()
                .and_then(|u| doc.model.slot_of(u))
                .and_then(|s| match doc.model.get(s) {
                    Some(Entity::Wall(w)) => Some(w.base.label.clone().unwrap_or_else(|| format!("Duvar {}", s.0))),
                    _ => None,
                })
                .unwrap_or_else(|| "Duvar yok".to_owned());
            geo.push(Row::text("Duvar", wall));
            if let Some(b) = x.broken {
                geo.push(Row::text("Durum", b.words()));
            }
        }
        // A NURBS curve (docs/adr/0259 §6): its degree, control and fit points, whether it is rational
        // and closed, its length and, closed, the area it encloses.
        Entity::Nurbs(n) => {
            geo.extend([
                Row::figure("Derece", n.degree.to_string()),
                Row::figure("Denetim noktası", n.ctrl.len().to_string()),
            ]);
            if let Some(fit) = &n.fit {
                geo.push(Row::figure("Geçiş noktası", fit.len().to_string()));
            }
            geo.extend([
                Row::text(
                    "Ağırlıklı",
                    if n.weights.is_some() {
                        "Evet"
                    } else {
                        "Hayır"
                    },
                ),
                Row::text("Kapalı", if n.closed { "Evet" } else { "Hayır" }),
                metres("Uzunluk", length_of.unwrap_or(0.0)),
            ]);
            if n.closed {
                geo.extend(area(kentos_interaction::enclosed(e).unwrap_or(0.0)));
            }
        }
        Entity::Dimension(d) => {
            let set_member = doc
                .model
                .uid(slot)
                .and_then(|uid| doc.model.dimension_set_of_item(uid));
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
                    DimensionStyle::CenterMark => "Merkez işareti",
                    DimensionStyle::CenterLine => "Eksen çizgisi",
                    DimensionStyle::Elevation => "Kot",
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
                // docs/adr/0322: a mark's curve or axis, a level's elevation.
                Some(l) if l.unit == "none" => geo.push(metres(
                    if style == DimensionStyle::CenterMark {
                        "Eğrinin yarıçapı"
                    } else {
                        "Eksenin uzunluğu"
                    },
                    l.value,
                )),
                // A Kot bound to a level reads it (docs/adr/0721 §2): its value is shown only.
                Some(l) if l.unit == "level" => geo.push(metres("Kot", l.value).editor(if d.level.is_some() {
                    None
                } else {
                    number(Field::DimensionZa(vec![slot]))
                })),
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
            // An ordinate and a level have no offset; a jogged radius's is where its jog is.
            if !matches!(style, DimensionStyle::Ordinate | DimensionStyle::Elevation) {
                geo.push(
                    metres(
                        match style {
                            DimensionStyle::Angular => "Yay yarıçapı",
                            DimensionStyle::Radius | DimensionStyle::Diameter => "Dışa uzantı",
                            DimensionStyle::Jogged => "Kırık uzaklığı",
                            DimensionStyle::CenterMark | DimensionStyle::CenterLine => "Uzantı",
                            _ => {
                                if set_member.is_some() {
                                    "Sıra uzaklığı"
                                } else {
                                    "Ötelenme"
                                }
                            }
                        },
                        set_member
                            .and_then(|(set, index)| {
                                doc.model
                                    .dimension_set_row_offset(set, set.members[index].row)
                                    .ok()
                            })
                            .unwrap_or(d.offset),
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
            geo.extend(override_rows(&[&d.look.overrides], &ids, locked));
            geo.extend(scale_rows(&[e], &ids, locked));
            geo.extend([
                metres("Yazı yüksekliği", d.height).editor(number(Field::DimensionHeight(slot))),
                Row::text("Yazı", d.text.clone().unwrap_or_default())
                    .editor(edit(Editor::Text(Field::DimensionText(slot)))),
            ]);
            // Zemin, an ordinate's axis, a slope's elevations, an arc length's radius and angle (docs/adr/0147 §7).
            let levels = doc.model.settings().arch.as_ref().map_or(&[][..], |a| a.levels.as_slice());
            let mut own = dimension::rows(&[d], &ids, locked, &f, levels);
            if set_member.is_some() {
                // Letting go is explicit for a recipe; freeing just one reference would be recreated.
                if let Some(relation) = own.iter_mut().find(|row| row.label == "İlişki") {
                    relation.editor = Some(Editor::Select {
                        text: relation.value.clone(),
                        swatch: None,
                        icon: None,
                        items: vec![Choice::Action {
                            label: "Diziyi bağımsız ölçülere dönüştür",
                            icon: "dimension",
                            message: (!locked).then_some(Message::Properties(Event::DimensionSet(
                                slot, "detach",
                            ))),
                        }],
                    });
                }
                geo.extend(dimension_set::rows(doc, slot, locked));
            }
            // İlişki right under its kind: whether the dimension follows its objects (docs/adr/0249 §3).
            if let Some(i) = own.iter().position(|r| r.label == "İlişki") {
                geo.insert(1.min(geo.len()), own.remove(i));
            }
            // A Kot's Kat and Fark right under its value: the level it reads (docs/adr/0721 §2).
            if let Some(at) = geo.iter().position(|r| r.label == "Kot") {
                for (k, label) in ["Kat", "Fark"].into_iter().enumerate() {
                    if let Some(i) = own.iter().position(|r| r.label == label) {
                        geo.insert(at + 1 + k, own.remove(i));
                    }
                }
            }
            geo.extend(own);
        }
        Entity::Hatch(h) => {
            geo.extend(hatch_rows(
                h,
                slot,
                locked,
                doc.model.settings().plot_scale,
                &f,
            ));
            geo.extend(scale_rows(&[e], &ids, locked));
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
            geo.extend(override_rows(&[&t.face.overrides], &ids, locked));
            geo.extend(scale_rows(&[e], &ids, locked));
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
            // Alanlar (docs/adr/0325 §4).
            geo.extend(text_field_rows(&doc.model, t, slot, locked));
            // Along a curve, its curve's length (docs/adr/0196 §4).
            if let Some(path) = &t.path {
                let curve = kentos_native_application::geometry::core_curve(path);
                let p = kentos_geometry_core::Vec2::new(t.p.x, t.p.y);
                geo.push(metres("Eğri", curve.length(p, t.rotation)));
            }
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
            // A schedule's settings and file (docs/adr/0733 §5, §7).
            if let (Some(TableSource::Schedule { of, .. }), Some(uid)) = (&t.source, doc.model.uid(slot)) {
                let uid = uid.to_string();
                let items = vec![
                    Choice::Action {
                        label: "Çizelge ayarları…",
                        icon: "settings",
                        message: (!locked).then(|| Message::Schedule(crate::schedule_window::Event::Open(uid.clone()))),
                    },
                    Choice::Action {
                        label: "Dosyaya yaz…",
                        icon: "tableFile",
                        message: Some(Message::Schedule(crate::schedule_window::Event::Export(uid.clone()))),
                    },
                ];
                geo.push(Row::text("Çizelge", of.label()).editor(edit(Editor::Select {
                    text: of.label().to_owned(),
                    swatch: None,
                    icon: None,
                    items,
                })));
            }
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
            // Its level (docs/adr/0733 §1): a level chosen goes to it as Kat ver writes it.
            let levels = doc.model.settings().arch.as_ref().map(|a| a.levels.clone()).unwrap_or_default();
            if !levels.is_empty() {
                let shown = i
                    .arch
                    .level
                    .as_deref()
                    .map_or_else(|| "Yok".to_owned(), |id| levels.iter().find(|l| l.id == id).map_or_else(|| id.to_owned(), |l| l.name.clone()));
                let items = levels
                    .iter()
                    .map(|l| Choice::Pick {
                        label: l.name.clone(),
                        swatch: None,
                        icon: None,
                        wide: false,
                        chosen: i.arch.level.as_deref() == Some(l.id.as_str()),
                        enabled: !locked,
                        message: Message::Properties(Event::Levels(vec![slot], l.id.clone())),
                    })
                    .collect();
                geo.push(Row::text("Kat", shown.clone()).editor(edit(Editor::Select {
                    text: shown,
                    swatch: None,
                    icon: None,
                    items,
                })));
            }
            geo.extend([
                Row::text("Blok", name).editor(edit(blocks)),
                len("Konum Y", i.p.x).editor(number(Field::InsertX(slot))),
                len("Konum X", i.p.y).editor(number(Field::InsertY(slot))),
                // One factor, or its own y factor beside the x one (docs/adr/0315 §3).
                Row::figure(
                    if i.y_scale.is_some() {
                        "X ölçeği"
                    } else {
                        "Ölçek"
                    },
                    fixed(i.scale, 4),
                )
                .editor(number(Field::InsertScale(slot))),
                Row::figure("Y ölçeği", fixed(i.y_scale.unwrap_or(i.scale), 4))
                    .editor(number(Field::InsertYScale(slot))),
                Row::figure("Dönüş", degrees(i.rotation))
                    .unit("°")
                    .editor(number(Field::InsertTurn(slot))),
                Row::text("Aynalı", yes_no(i.mirror)).editor(edit(mirror)),
            ]);
            // What it places, real size, and its block's unit (docs/adr/0726 §3): the insert's factors hold a
            // unitless block's unit already.
            if let Some(def) = doc.model.block(i.block) {
                let values = (!i.parameters.is_empty()).then_some(&i.parameters);
                if let Some([x0, y0, x1, y1]) =
                    kentos_native_application::blocks::definition_box(&doc.model, i.block, values)
                {
                    let w = (x1 - x0) * i.scale.abs();
                    let d = (y1 - y0) * i.y_scale.unwrap_or(i.scale).abs();
                    geo.push(Row::text(
                        "En × boy",
                        format!("{} × {} {}", f.length_bare(w), f.length_bare(d), f.length_unit_label()),
                    ));
                }
                geo.push(Row::text(
                    "Bloğun birimi",
                    kentos_interaction::block_insert::unit_words(def.units),
                ));
            }
        }
        // A picture's source, place, size, turn, see-through share, clip and mirror (docs/adr/0192 §4).
        Entity::Image(im) => {
            let i = &im.image;
            let (source, linked) = match (&i.asset, &i.file) {
                (Some(a), _) => (crate::pictures::asset_words(&doc.model, a), false),
                (None, Some(path)) => {
                    let search = &doc.model.settings().xref_paths;
                    (crate::pictures::file_words(path, doc.path.as_deref(), search), true)
                }
                _ => ("—".to_owned(), false),
            };
            // Göm and Yeniden bağla… (docs/adr/0350 §1), both by `cad.images.edit`.
            let pick = |label: &str, message: Message| Choice::Pick {
                label: label.to_owned(),
                swatch: None,
                icon: None,
                wide: false,
                chosen: false,
                enabled: true,
                message,
            };
            let mut choices = Vec::new();
            if linked {
                choices.push(pick("Göm", Message::Properties(Event::EmbedImage(slot))));
            }
            choices.push(pick(
                "Yeniden bağla…",
                Message::Xrefs(crate::xrefs_window::Event::RelinkImage(slot)),
            ));
            let embed = (!locked).then(|| Editor::Select {
                text: source.clone(),
                swatch: None,
                icon: None,
                items: choices,
            });
            let yes_no = |m: bool| if m { "Evet" } else { "Hayır" };
            geo.extend([
                Row::text("Kaynak", source).editor(embed),
                len("Konum Y", i.p.x).editor(number(Field::ImageX(slot))),
                len("Konum X", i.p.y).editor(number(Field::ImageY(slot))),
                metres("Genişlik", i.width).editor(number(Field::ImageWidth(slot))),
                metres("Yükseklik", i.height).editor(number(Field::ImageHeight(slot))),
                Row::figure("Dönüş", degrees(i.rotation))
                    .unit("°")
                    .editor(number(Field::ImageTurn(slot))),
                Row::figure(
                    "Saydamlık",
                    fixed((1.0 - i.opacity.unwrap_or(1.0)) * 100.0, 0),
                )
                .unit("%")
                .editor(number(Field::ImageClear(slot))),
                Row::text("Kırpma", if i.clip.is_some() { "Var" } else { "Yok" }),
                Row::text("Aynalı", yes_no(i.mirror)),
            ]);
        }
        // Its file, size, bands, pixel, system, nodata and opacity (docs/adr/0204 §8).
        Entity::Raster(r) => geo.extend(raster::rows(doc, slot, r, locked, &f)),
        // Its files, points, bounds, density, system, index, look and opacity (docs/adr/0207 §9).
        Entity::PointCloud(c) => geo.extend(cloud::rows(doc, slot, c, locked, &f)),
        // A record has no geometry (docs/adr/0410): its attributes are its rows.
        Entity::Record(_) => {}
        // Its note, height, turn, arrowhead and mask, its corners and length (docs/adr/0146 §7).
        Entity::Leader(l) => {
            geo.extend(leader_rows(&[l], &ids, locked, &f));
            // Its style, arms, tips and content blocks (docs/adr/0326).
            geo.extend(multileader_rows(&doc.model, &[l], &ids, locked));
            geo.extend(scale_rows(&[e], &ids, locked));
            geo.push(Row::figure("Köşe sayısı", l.pts.len().to_string()));
            geo.push(metres("Uzunluk", length_of.unwrap_or(0.0)));
        }
    }

    let mut sections = vec![Section {
        id: "general",
        title: "Genel".into(),
        rows: general,
    }];
    // A record has no geometry (docs/adr/0410 §4): no section for it.
    if drawn {
        sections.push(Section {
            id: "geometry",
            title: "Geometri".into(),
            rows: geo,
        });
    }
    // The kitchen run the object is part of (docs/adr/0727 §6).
    let in_run = casework::section(doc, slot, locked);
    let run_member = in_run.is_some();
    sections.extend(in_run);
    // An opening's type and its four widths (docs/adr/0772 §1, §2).
    if let Entity::Opening(o) = e {
        sections.extend(opening_types::section(doc, slot, &o.opening, &f, locked));
    }
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
    // A dynamic block's parameters (docs/adr/0319 §3).
    if let Entity::Insert(i) = e {
        let rows = params::rows(doc, slot, i, locked);
        if !rows.is_empty() {
            sections.push(Section {
                id: "blockParams",
                title: "Blok parametreleri".into(),
                rows,
            });
        }
    }
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
                    warn: false,
                    thumb: None,
                })
                .collect(),
        });
    }
    // The layer's fields first, each by its kind (docs/adr/0199 §5); the keys no field names after them, as text.
    let fields = layers
        .get(&base.layer_id)
        .map(|n| n.fields.clone())
        .unwrap_or_default();
    if !fields.is_empty() {
        let mut rows = form_rows(doc.model.settings(), &base.layer_id);
        let labels = crate::features::lookup_labels(&doc.model, &base.layer_id, &fields);
        let subtypes = layers.get(&base.layer_id).and_then(|n| n.subtypes.clone());
        let groups = layers
            .get(&base.layer_id)
            .map(|n| n.contingents.clone())
            .unwrap_or_default();
        rows.extend(field_rows(
            slot,
            base,
            &fields,
            subtypes.as_ref(),
            &groups,
            locked,
            &labels,
        ));
        sections.push(Section {
            id: "fields",
            title: "Alanlar".into(),
            rows,
        });
    }
    let others: Vec<(&String, &String)> = base
        .attrs
        .iter()
        .filter(|(key, _)| !tags.iter().any(|(tag, _)| tag == *key))
        .filter(|(key, _)| !fields.iter().any(|f| f.name == **key))
        // A run's data is its run's: its section edits it (docs/adr/0727 §5).
        .filter(|(key, _)| !(run_member && matches!(key.as_str(), "Modül" | "Tür" | "Açılma")))
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
                    warn: false,
                    thumb: None,
                })
                .collect(),
        });
    }
    sections
}

/// The Form row of a layer that has a form (docs/adr/0401 §9): the form's name and whether it is
/// published, with Formla düzenle and Formlar….
fn form_rows(settings: &kentos_commands::ProjectSettings, layer: &str) -> Vec<Row> {
    let forms: Vec<&kentos_commands::ProjectForm> =
        settings.forms.iter().filter(|f| f.layer == layer).collect();
    let Some(form) = forms
        .iter()
        .find(|f| !f.releases.is_empty())
        .or(forms.first())
    else {
        return Vec::new();
    };
    let value = if form.releases.is_empty() {
        format!("{} (taslak)", form.name)
    } else {
        form.name.clone()
    };
    let items = vec![
        Choice::Command {
            label: "Formla düzenle",
            icon: Icon::Properties,
            id: "data.formEdit",
        },
        Choice::Command {
            label: "Formlar…",
            icon: Icon::Layout,
            id: "data.forms",
        },
    ];
    vec![
        Row::text("Form", value.clone()).editor(Some(Editor::Select {
            text: value,
            swatch: None,
            icon: None,
            items,
        })),
    ]
}

/// Öznitelikler's form of a layer's fields (docs/adr/0199 §5; the web's
/// `ui/properties/fieldRows.ts`): a row per field, in order, its alias and a
/// mark when it is required; a value list and yes or no in a list, anything
/// else in a cell showing the value as the field displays it; a value the
/// field refuses as written, why under it in the warning colour. Written
/// through `cad.entities.set` (“Değiştir”), which checks the value by the
/// same rules.
fn field_rows(
    slot: Slot,
    base: &kentos_commands::EntityBase,
    fields: &[kentos_commands::LayerField],
    subtypes: Option<&kentos_commands::LayerSubtypes>,
    groups: &[kentos_commands::FieldGroup],
    locked: bool,
    labels: &[(String, std::collections::BTreeMap<String, String>)],
) -> Vec<Row> {
    use kentos_commands::{LayerFieldKind, MULTIPLE_JOIN, check_value, display_value};
    let mut rows = Vec::new();
    for f in fields {
        let raw = base.attrs.get(&f.name).cloned().unwrap_or_default();
        let checked = check_value(f, &raw);
        // A form's list from another layer (docs/adr/0404): shown by its labels, written through the form.
        let lookup = labels
            .iter()
            .find(|(name, _)| *name == f.name)
            .map(|(_, l)| l);
        let (value, shown) = match (&checked, lookup) {
            (Ok(v), Some(codes)) => (
                v.clone(),
                v.split(MULTIPLE_JOIN)
                    .filter(|c| !c.is_empty())
                    .map(|c| codes.get(c).map_or(c, String::as_str))
                    .collect::<Vec<_>>()
                    .join(MULTIPLE_JOIN),
            ),
            (Ok(v), None) => (v.clone(), display_value(f, v)),
            (Err(_), _) => (raw.clone(), raw.clone()),
        };
        let field = Field::Attribute(slot, f.name.clone());
        let listed =
            f.kind == LayerFieldKind::Boolean || f.values.as_ref().is_some_and(|v| !v.is_empty());
        // A value the bound table always makes itself is shown, not written (docs/adr/0754 §3).
        let editor = if locked || lookup.is_some() || f.made_always() {
            None
        } else if listed {
            let pairs: Vec<(String, String)> = match &f.values {
                // A retired value only for the record holding it (docs/adr/0418), narrowed to the
                // record's subtype (docs/adr/0422).
                Some(list) if !list.is_empty() => {
                    kentos_commands::offered_on(subtypes, groups, fields, f, |n| {
                        base.attrs.get(n).cloned()
                    })
                    .into_iter()
                    .map(|c| (c.code, c.label))
                    .collect()
                }
                _ => vec![
                    ("true".to_owned(), "Evet".to_owned()),
                    ("false".to_owned(), "Hayır".to_owned()),
                ],
            };
            let pick = |label: String, code: String, chosen: bool| Choice::Pick {
                label,
                swatch: None,
                icon: None,
                wide: false,
                chosen,
                enabled: true,
                message: Message::Properties(Event::Commit(field.clone(), code)),
            };
            let mut items = vec![pick("—".to_owned(), String::new(), value.is_empty())];
            items.extend(
                pairs
                    .into_iter()
                    .map(|(code, label)| pick(label, code.clone(), code == value)),
            );
            Some(Editor::Select {
                text: if shown.is_empty() {
                    "—".to_owned()
                } else {
                    shown.clone()
                },
                swatch: None,
                icon: None,
                items,
            })
        } else {
            Some(Editor::Text(field))
        };
        rows.push(Row {
            label: Cow::Owned(if f.required {
                format!("{} *", f.label())
            } else {
                f.label().to_owned()
            }),
            value: shown,
            numeric: f.kind.is_number() && !listed,
            unit: None,
            editor,
            note: false,
            warn: false,
            thumb: None,
        });
        if let Err(refused) = checked {
            rows.push(Row {
                warn: true,
                ..Row::note(&refused.message)
            });
        }
    }
    rows
}

/// Ölçekli, Kendi ölçeği, Diğer ölçekler and Ayrı değerler (docs/adr/0323 §4): whether the texts, dimensions or
/// leaders are annotative, the scale their own geometry is drawn at, their other scales (one that sets something
/// apart with an asterisk; typed to add or remove) and Sıfırla; none when none of them can be (a text along a curve,
/// a linked text).
fn scale_rows(objects: &[&Entity], slots: &[Slot], locked: bool) -> Vec<Row> {
    use kentos_commands::{annotative_of, why_not_annotative};
    if objects.iter().all(|e| why_not_annotative(e).is_some()) {
        return Vec::new();
    }
    let on = objects.iter().filter(|e| annotative_of(e).is_some()).count();
    let state = match on {
        0 => "Hayır",
        n if n == objects.len() => "Evet",
        _ => "Çeşitli",
    };
    let pick = |label: &str, value: bool| Choice::Pick {
        label: label.to_owned(),
        swatch: None,
        icon: None,
        wide: false,
        chosen: state == label,
        enabled: true,
        message: Message::Properties(Event::Annotative(slots.to_vec(), value)),
    };
    let editor = Editor::Select {
        text: state.to_owned(),
        swatch: None,
        icon: None,
        items: vec![pick("Evet", true), pick("Hayır", false)],
    };
    let mut rows = vec![Row::text("Ölçekli", state).editor((!locked).then_some(editor))];
    if on != objects.len() {
        return rows;
    }
    let common = |texts: Vec<String>| {
        if texts.iter().all(|t| *t == texts[0]) {
            texts[0].clone()
        } else {
            "Çeşitli".to_owned()
        }
    };
    let own = common(
        objects
            .iter()
            .filter_map(|e| annotative_of(e))
            .map(|a| kentos_interaction::annotative::ratio(a.at))
            .collect(),
    );
    let others = common(
        objects
            .iter()
            .filter_map(|e| kentos_interaction::annotative::others_text(e))
            .collect(),
    );
    rows.push(Row::text("Kendi ölçeği", own));
    rows.push(
        Row::text("Diğer ölçekler", others)
            .editor((!locked).then_some(Editor::Text(Field::AnnotativeScales(slots.to_vec())))),
    );
    let apart: Vec<String> = objects
        .iter()
        .filter_map(|e| annotative_of(e))
        .flat_map(|a| a.scales.iter().filter(|s| !s.is_plain()).map(|s| s.scale))
        .fold(Vec::<f64>::new(), |mut all, s| {
            if !all.contains(&s) {
                all.push(s);
            }
            all
        })
        .into_iter()
        .map(kentos_interaction::annotative::ratio)
        .collect();
    if !apart.is_empty() {
        let text = apart.join(", ");
        let editor = Editor::Select {
            text: text.clone(),
            swatch: None,
            icon: None,
            items: vec![Choice::Pick {
                label: "Sıfırla".to_owned(),
                swatch: None,
                icon: None,
                wide: false,
                chosen: false,
                enabled: true,
                message: Message::Properties(Event::AnnotativeReset(slots.to_vec())),
            }],
        };
        rows.push(Row::text("Ayrı değerler", text).editor((!locked).then_some(editor)));
    }
    rows
}

/// Öznitelikler's Yazı stili or Ölçü stili row (docs/adr/0183 §6), a CAD
/// project's: the style the objects follow by name (a link to a style the
/// project no longer has shows Standart), or “Çeşitli”; choosing one applies
/// it to them in one step “Değiştir”. On a locked layer it only shows. The
/// web's `ui/properties/styleRows.ts`.
/// Stilden ayrılan (docs/adr/0322 §5): the style fields the objects set apart by hand, common or “Çeşitli”, and
/// Stile döndür; none when no object sets one apart.
fn override_rows(lists: &[&[String]], slots: &[Slot], locked: bool) -> Vec<Row> {
    if lists.iter().all(|l| l.is_empty()) {
        return Vec::new();
    }
    let text = if lists.iter().all(|l| *l == lists[0]) {
        kentos_interaction::properties::override_names(lists[0]).join(", ")
    } else {
        "Çeşitli".to_owned()
    };
    let editor = Editor::Select {
        text: text.clone(),
        swatch: None,
        icon: None,
        items: vec![Choice::Pick {
            label: "Stile döndür".to_owned(),
            swatch: None,
            icon: None,
            wide: false,
            chosen: false,
            enabled: true,
            message: Message::Properties(Event::RestoreStyle(slots.to_vec())),
        }],
    };
    vec![Row::text("Stilden ayrılan", text).editor((!locked).then_some(editor))]
}

fn style_rows(
    settings: &kentos_commands::ProjectSettings,
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
            .map_or(kentos_commands::STANDARD_STYLE.to_owned(), |(_, n)| {
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
    let items = std::iter::once(pick(kentos_commands::STANDARD_STYLE.to_owned(), None))
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
fn text_rows(texts: &[&kentos_commands::TextEntity], slots: &[Slot], locked: bool) -> Vec<Row> {
    const MIXED: &str = "Çeşitli";
    let common = |of: &dyn Fn(&kentos_commands::TextEntity) -> String| {
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
    // Its indents, tab stops and columns (docs/adr/0324 §3), the indents and gap in its height.
    let layout = |t: &kentos_commands::TextEntity| t.paragraph.layout.as_deref().cloned().unwrap_or_default();
    let heights = |label: &'static str, value: Option<String>| match value {
        Some(v) => Row::figure(label, v).unit("× yükseklik"),
        None => Row::text(label, MIXED),
    };
    let number = |v: f64| crate::crs::js_number(fixed_number(v, 3));
    let indent_row = heights("Girinti", common(&|t| number(layout(t).indent.unwrap_or(0.0))));
    let left_row = heights("Sol girinti", common(&|t| number(layout(t).left_indent.unwrap_or(0.0))));
    let tabs_row = Row::text(
        "Sekmeler",
        common(&|t| kentos_interaction::text_layout::tabs_text(&layout(t).tabs)).unwrap_or_else(|| MIXED.to_owned()),
    );
    let columns = common(&|t| layout(t).columns.map_or(1, |c| c.count).to_string());
    let columns_row = match &columns {
        Some(n) => Row::figure("Sütunlar", n.clone()),
        None => Row::text("Sütunlar", MIXED),
    };
    let mut rows = vec![
        Row::text("Hiza", align_text).editor(edit(aligns)),
        factor_row.editor(edit(Editor::Number(Field::TextWidth(slots.to_vec())))),
        box_row.editor(edit(Editor::Number(Field::TextBox(slots.to_vec())))),
        spacing_row.editor(edit(Editor::Number(Field::TextSpacing(slots.to_vec())))),
        indent_row.editor(edit(Editor::Number(Field::TextIndent(slots.to_vec())))),
        left_row.editor(edit(Editor::Number(Field::TextLeftIndent(slots.to_vec())))),
        tabs_row.editor(edit(Editor::Text(Field::TextTabs(slots.to_vec())))),
        columns_row.editor(edit(Editor::Number(Field::TextColumns(slots.to_vec())))),
    ];
    // In columns: their height, metres, and the gap between them.
    if columns.as_deref().is_some_and(|n| n != "1") {
        let tall = common(&|t| layout(t).columns.map_or(String::new(), |c| number(c.height)));
        rows.push(
            match tall {
                Some(v) => Row::figure("Sütun yüksekliği", v).unit("m"),
                None => Row::text("Sütun yüksekliği", MIXED),
            }
            .editor(edit(Editor::Number(Field::TextColumnHeight(slots.to_vec())))),
        );
        rows.push(
            heights("Sütun aralığı", common(&|t| layout(t).columns.map_or(String::new(), |c| number(c.gap))))
                .editor(edit(Editor::Number(Field::TextColumnGap(slots.to_vec())))),
        );
    }
    rows.push(Row::text("Zemin", mask_text).editor(edit(masks)));
    rows
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
    texts: &[&kentos_commands::TextEntity],
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

/// A text's fields (docs/adr/0325 §4): each its source and value, or why it is broken; its menu selects the source
/// object and takes the field out (its letters stay as text).
fn text_field_rows(model: &kentos_domain::Document, t: &kentos_commands::TextEntity, slot: Slot, locked: bool) -> Vec<Row> {
    use kentos_commands::FieldSource;
    t.paragraph
        .fields
        .iter()
        .enumerate()
        .map(|(i, f)| {
            let object = f
                .source
                .object()
                .and_then(|id| model.slot_of(kentos_domain::Uuid::from_bytes(id.0)))
                .and_then(|s| Some((s, model.get(s)?)));
            let named = |what: &str, name: &Option<String>| match name {
                Some(n) => format!("{what} “{n}”"),
                None => what.to_owned(),
            };
            let source = match &f.source {
                FieldSource::Object { value, name, .. } => {
                    let of = object.map_or("silinmiş nesne", |(_, e)| kind_title(e.kind()));
                    format!("{of}: {}", named(value.label(), name))
                }
                FieldSource::Project { value, name } => format!("Proje: {}", named(value.label(), name)),
            };
            let letters: String = t.text.chars().skip(f.start as usize).take((f.end - f.start) as usize).collect();
            // The value first (what the row has room for), the source in the menu's title.
            let value = match f.broken {
                Some(b) => format!("Kopuk: {}", b.words()),
                None => letters,
            };
            let action = |label: &str, message: Message| Choice::Pick {
                label: label.to_owned(),
                swatch: None,
                icon: None,
                wide: false,
                chosen: false,
                enabled: true,
                message,
            };
            // The source, as a title the menu does not act on.
            let mut items = vec![Choice::Pick {
                label: source,
                swatch: None,
                icon: None,
                wide: false,
                chosen: true,
                enabled: false,
                message: Message::TextTools(crate::text_tools::Event::Close),
            }];
            if let Some((s, _)) = object {
                items.push(action("Kaynağı seç", Message::Properties(Event::SelectObject(s))));
            }
            if !locked {
                items.push(action("Alanı kaldır", Message::TextTools(crate::text_tools::Event::Remove(slot, i as u32))));
            }
            let editor = (!items.is_empty()).then(|| Editor::Select { text: value.clone(), swatch: None, icon: None, items });
            let mut row = Row::text(format!("Alan {}", i + 1), value).editor(editor);
            row.warn = f.broken.is_some();
            row
        })
        .collect()
}

/// A leader's style, arms, tips and content blocks (docs/adr/0326): Kılavuz stili (a menu of the project's leader
/// styles, Yok first), and for one leader its arm count, each bound tip with Bağı çöz, its content blocks' count.
fn multileader_rows(
    model: &kentos_domain::Document,
    leaders: &[&kentos_commands::LeaderEntity],
    slots: &[Slot],
    locked: bool,
) -> Vec<Row> {
    use crate::leader_tools::Event as L;
    let styles = &model.settings().leader_styles;
    let mut rows = Vec::new();
    let common = leaders.first().map(|l| l.leader_style.clone()).filter(|s| leaders.iter().all(|l| &l.leader_style == s));
    let name_of = |id: &Option<String>| match id {
        Some(id) => styles.iter().find(|s| &s.id == id).map_or("Silinmiş stil".to_owned(), |s| s.name.clone()),
        None => "Yok".to_owned(),
    };
    let value = common.as_ref().map_or_else(|| "Çeşitli".to_owned(), name_of);
    if !styles.is_empty() || leaders.iter().any(|l| l.leader_style.is_some()) {
        let pick = |label: String, id: Option<String>| Choice::Pick {
            label,
            swatch: None,
            icon: None,
            wide: false,
            chosen: common.as_ref() == Some(&id),
            enabled: !locked,
            message: Message::LeaderTools(L::Style(slots.to_vec(), id)),
        };
        let mut items = vec![pick("Yok".to_owned(), None)];
        items.extend(styles.iter().map(|s| pick(s.name.clone(), Some(s.id.clone()))));
        rows.push(Row::text("Kılavuz stili", value.clone()).editor(Some(Editor::Select { text: value, swatch: None, icon: None, items })));
    }
    let [l] = leaders else {
        return rows;
    };
    let slot = Slot(l.base.id);
    rows.push(Row::figure("Kollar", (l.arms.len() + 1).to_string()));
    let tips = std::iter::once(l.tip.as_ref()).chain(l.arms.iter().map(|a| a.tip.as_ref()));
    for (k, t) in tips.enumerate() {
        let Some(t) = t else { continue };
        let of = model.slot_of(kentos_domain::Uuid::from_bytes(t.uid.0)).and_then(|s| model.get(s));
        let what = match (&t.broken, of) {
            (Some(why), _) => format!("Kopuk: {why}"),
            (None, Some(e)) => format!("Bağlı: {}", kind_title(e.kind())),
            (None, None) => "Kopuk".to_owned(),
        };
        let items = vec![Choice::Pick {
            label: "Bağı çöz".to_owned(),
            swatch: None,
            icon: None,
            wide: false,
            chosen: false,
            enabled: !locked,
            message: Message::LeaderTools(L::Unbind(slot, k as u32)),
        }];
        let label = if k == 0 { "Ucu".to_owned() } else { format!("{}. kolun ucu", k + 1) };
        let mut row = Row::text(label, what.clone()).editor(Some(Editor::Select { text: what, swatch: None, icon: None, items }));
        row.warn = t.broken.is_some();
        rows.push(row);
    }
    if !l.blocks.is_empty() {
        rows.push(Row::figure("Blok içerik", l.blocks.len().to_string()));
    }
    rows
}

/// A hatch's rows (docs/adr/0186 §7): Desen (every choice with its icon,
/// its Ölçek and Açı carried over), Açı, Ölçek (a pattern's, on the paper),
/// Aralık (user lines'), İkinci renk, Degrade biçimi and Ters (a
/// gradient's), İlişkili with İlişkiyi kopar. The web's `hatchRows`.
fn hatch_rows(
    h: &kentos_commands::HatchEntity,
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
        Some(a) => format!("Evet ({} nesne)", a.objects().count()),
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
    // Sınır ▾ (docs/adr/0329 §4): exact when it holds an arc or a curve; written again as objects.
    let exact = if h.boundary.is_some() { "Kesin" } else { "Çizilen halka" };
    let recreate = (!locked).then(|| Editor::Select {
        text: exact.to_owned(),
        swatch: None,
        icon: Some("hatchRecreate"),
        items: vec![
            pick(
                "Sınırı yeniden oluştur: alan".to_owned(),
                None,
                false,
                Event::RecreateBoundary(slot, kentos_commands::BoundaryForm::Area),
            ),
            pick(
                "Sınırı yeniden oluştur: çoklu çizgi".to_owned(),
                None,
                false,
                Event::RecreateBoundary(slot, kentos_commands::BoundaryForm::Polyline),
            ),
        ],
    });
    rows.push(Row::text("Sınır", exact).editor(recreate));
    // Where its lines are anchored (docs/adr/0331 §2): the world's origin or its own point.
    if !matches!(p.kind, HatchPatternType::Solid | HatchPatternType::Gradient) {
        let origin = p.origin.map_or_else(
            || "Dünya başlangıcı".to_owned(),
            |o| f.point(kentos_geometry_core::Vec2::new(o.x, o.y)),
        );
        rows.push(Row::text("Başlangıç", origin));
    }
    rows
}

fn leader_rows(
    leaders: &[&kentos_commands::LeaderEntity],
    slots: &[Slot],
    locked: bool,
    f: &Format,
) -> Vec<Row> {
    const MIXED: &str = "Çeşitli";
    let first = leaders[0];
    let same = |of: &dyn Fn(&kentos_commands::LeaderEntity) -> bool| leaders.iter().all(|l| of(l));
    let note = same(&|l| l.text == first.text).then(|| first.text.clone().unwrap_or_default());
    let height = same(&|l| l.height == first.height).then_some(first.height);
    let turn = same(&|l| l.rotation == first.rotation).then_some(first.rotation);
    let arrow = same(&|l| l.arrow == first.arrow).then_some(first.arrow);
    let size = |l: &kentos_commands::LeaderEntity| l.arrow_size.unwrap_or(1.0);
    let arrow_size = same(&|l| size(l) == size(first)).then_some(size(first));
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
    // The arrowhead's length in the note's height (docs/adr/0205 §7).
    let size_row = match arrow_size {
        Some(s) => Row::figure("Ok boyu", fixed(s, 2)).unit("× yükseklik"),
        None => Row::text("Ok boyu", MIXED),
    };
    vec![
        Row::text("Not", note.unwrap_or_else(|| MIXED.to_owned()))
            .editor(edit(Editor::Text(Field::LeaderNote(slots.to_vec())))),
        height_row.editor(edit(Editor::Number(Field::LeaderHeight(slots.to_vec())))),
        turn_row.editor(edit(Editor::Number(Field::LeaderTurn(slots.to_vec())))),
        Row::text("Ok", arrow_text).editor(edit(arrows)),
        size_row.editor(edit(Editor::Number(Field::LeaderArrowSize(slots.to_vec())))),
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
fn many_sections(
    doc: &Document,
    objects: &[&Entity],
    (length, area): (f64, f64),
    shown: Shown,
) -> Vec<Section> {
    let ids: Vec<Slot> = objects.iter().map(|e| Slot(e.base().id)).collect();
    let first = objects[0].base();
    let same = |f: &dyn Fn(&kentos_commands::EntityBase) -> Option<&str>| {
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
    let mut locked_layers: std::collections::HashMap<&str, bool> = std::collections::HashMap::new();
    for e in objects {
        let layer = e.base().layer_id.as_str();
        locked_layers
            .entry(layer)
            .or_insert_with(|| layers.is_locked(layer));
    }
    let is_locked = |e: &Entity| {
        locked_layers
            .get(e.base().layer_id.as_str())
            .copied()
            .unwrap_or(false)
    };
    let locked_count = objects.iter().filter(|e| is_locked(e)).count();
    // The objects on locked layers are selected but not changed (docs/adr/0278, 0286): the set commands skip
    // them and count them; the edits of their geometry and texts are given the others only.
    let all_locked = locked_count == objects.len();
    let open: std::collections::HashSet<u32> = if locked_count == 0 {
        objects.iter().map(|e| e.base().id).collect()
    } else {
        objects
            .iter()
            .filter(|e| !is_locked(e))
            .map(|e| e.base().id)
            .collect()
    };
    let any_locked = all_locked;
    let edit = |editor: Editor| (!all_locked).then_some(editor);

    // The weight of the objects drawn with lines; the others have none to show.
    let lined: Vec<&&Entity> = objects.iter().filter(|e| e.draws_lines()).collect();
    let weight = lined
        .iter()
        .all(|e| e.base().line_weight == lined[0].base().line_weight)
        .then(|| lined.first().and_then(|e| e.base().line_weight));
    let layer_text = layer.map_or_else(|| "Çeşitli".to_owned(), |l| layer_path(layers, l));
    let mut rows = vec![
        Row::text("Katman", layer_text).editor(edit(layer_editor(doc, &ids, layer))),
        Row::text("Renk", color_text(color)).editor(edit(color_editor(&ids, color))),
    ];
    if !lined.is_empty() {
        let lined_ids: Vec<Slot> = lined.iter().map(|e| Slot(e.base().id)).collect();
        rows.push(
            Row::text("Kalınlık", weight_text(weight))
                .editor(edit(weight_editor(&lined_ids, weight))),
        );
        // The look's parts (docs/adr/0260): the shared value, else Çeşitli.
        let same = |f: &dyn Fn(&Entity) -> String| lined.iter().all(|e| f(e) == f(lined[0]));
        let line_type = same(&|e| format!("{:?}", e.base().look.line_type))
            .then(|| lined[0].base().look.line_type);
        let scale = same(&|e| format!("{:?}", e.base().look.line_type_scale))
            .then(|| lined[0].base().look.line_type_scale);
        let pattern = same(&|e| format!("{:?}", e.base().look.line_pattern))
            .then(|| lined[0].base().look.line_pattern.as_deref());
        rows.push(line_type_row(doc, &lined_ids, line_type, pattern).editor_if(!any_locked));
        rows.push(line_type_scale_row(&lined_ids, scale).editor_if(!any_locked));
    }
    {
        let all: Vec<Slot> = objects.iter().map(|e| Slot(e.base().id)).collect();
        let t = objects
            .iter()
            .all(|e| e.base().look.transparency == objects[0].base().look.transparency)
            .then(|| objects[0].base().look.transparency);
        rows.push(transparency_row(&all, t).editor_if(!any_locked));
        // Areas' masks (docs/adr/0310 §5): when every object is an area, where the program has masks.
        if shown.mask && objects.iter().all(|e| matches!(e, Entity::Polygon(_))) {
            let w = objects
                .iter()
                .all(|e| e.base().look.wipeout == objects[0].base().look.wipeout)
                .then(|| objects[0].base().look.wipeout);
            rows.push(wipeout_row(&all, w).editor_if(!any_locked));
        }
    }
    rows.push(
        Row::text("Sembol", symbol_text(doc, symbol)).editor(edit(symbol_editor(doc, symbol))),
    );
    // Kat of several architectural objects, of one kind or of several (docs/adr/0717 §2): their shared level, else
    // Çeşitli; a level chosen goes to them all in one step “Kat ver”.
    let levels = doc.model.settings().arch.as_ref().map(|a| a.levels.clone()).unwrap_or_default();
    if !levels.is_empty() && objects.iter().all(|e| kentos_native_application::levels_assign::levelled(e)) {
        let first = objects[0].own_level();
        let shared = objects.iter().all(|e| e.own_level() == first);
        let name = |id: &str| levels.iter().find(|l| l.id == id).map_or_else(|| id.to_owned(), |l| l.name.clone());
        let shown = if shared { first.map_or_else(|| "Yok".to_owned(), name) } else { "Çeşitli".to_owned() };
        let items = levels
            .iter()
            .map(|l| {
                pick(
                    l.name.clone(),
                    shared && first == Some(l.id.as_str()),
                    Message::Properties(Event::Levels(ids.clone(), l.id.clone())),
                )
            })
            .collect();
        rows.push(Row::text("Kat", shown.clone()).editor(edit(Editor::Select {
            text: shown,
            swatch: None,
            icon: None,
            items,
        })));
    }
    let f = Format::of(doc.settings());
    // The vertices' elevations of the lines, polylines, areas and points (docs/adr/0142), all
    // together as one list, as the web's row takes them: the value when every vertex has the
    // same, `kot yok` when none has one, else Çeşitli. What takes no elevation is left out.
    let takers: Vec<Slot> = objects
        .iter()
        .filter(|e| elevation::takes(e) && open.contains(&e.base().id))
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
    if locked_count > 0 {
        let line = if all_locked {
            format!("{locked_count} nesnenin hepsi kilitli katmanda: değişmez.")
        } else {
            format!("{locked_count} nesne kilitli katmanda: atlanır.")
        };
        // First: it says what every row below does to the selection.
        rows.insert(
            0,
            Row {
                warn: true,
                ..Row::note(&line)
            },
        );
    }
    let mut sections = vec![Section {
        id: "general",
        title: "Ortak özellikler".into(),
        rows,
    }];
    if let Some(attrs) = shared_attributes(objects, &ids, all_locked) {
        sections.push(attrs);
    }
    // Ölçekli açıklamalar (docs/adr/0323 §4): the selection's texts, dimensions and leaders together.
    let scaled: Vec<&Entity> = objects
        .iter()
        .copied()
        .filter(|e| {
            matches!(
                e,
                Entity::Text(_) | Entity::Dimension(_) | Entity::Leader(_) | Entity::Hatch(_)
            )
        })
        .collect();
    if !scaled.is_empty() {
        let slots: Vec<Slot> = scaled
            .iter()
            .filter(|e| open.contains(&e.base().id))
            .map(|e| Slot(e.base().id))
            .collect();
        let rows = scale_rows(&scaled, &slots, slots.is_empty());
        if !rows.is_empty() {
            sections.push(Section {
                id: "scales",
                title: "Ölçekli açıklama".into(),
                rows,
            });
        }
    }
    // The selection's texts: their Hiza, Genişlik çarpanı and Zemin, common or Çeşitli (docs/adr/0145 §6).
    let texts: Vec<&kentos_commands::TextEntity> = objects
        .iter()
        .filter_map(|e| match e {
            Entity::Text(t) => Some(t),
            _ => None,
        })
        .collect();
    if !texts.is_empty() {
        let slots: Vec<Slot> = texts
            .iter()
            .filter(|t| open.contains(&t.base.id))
            .map(|t| Slot(t.base.id))
            .collect();
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
                override_rows(
                    &texts
                        .iter()
                        .map(|t| t.face.overrides.as_slice())
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
    let leaders: Vec<&kentos_commands::LeaderEntity> = objects
        .iter()
        .filter_map(|e| match e {
            Entity::Leader(l) => Some(l),
            _ => None,
        })
        .collect();
    if !leaders.is_empty() {
        let slots: Vec<Slot> = leaders
            .iter()
            .filter(|l| open.contains(&l.base.id))
            .map(|l| Slot(l.base.id))
            .collect();
        sections.push(Section {
            id: "leaders",
            title: if leaders.len() == objects.len() {
                "Kılavuz".into()
            } else {
                format!("Kılavuzlar ({})", leaders.len()).into()
            },
            rows: [leader_rows(&leaders, &slots, any_locked, &f), multileader_rows(&doc.model, &leaders, &slots, any_locked)].concat(),
        });
    }
    // The selection's dimensions: their Zemin, and an ordinate's axis, a slope's elevations, common or “Çeşitli” (docs/adr/0147 §7).
    let dims: Vec<&kentos_commands::DimensionEntity> = objects
        .iter()
        .filter_map(|e| match e {
            Entity::Dimension(d) => Some(d),
            _ => None,
        })
        .collect();
    if !dims.is_empty() {
        let slots: Vec<Slot> = dims
            .iter()
            .filter(|d| open.contains(&d.base.id))
            .map(|d| Slot(d.base.id))
            .collect();
        if let Some(&first) = slots.first()
            && let Some((set, _)) = doc
                .model
                .uid(first)
                .and_then(|uid| doc.model.dimension_set_of_item(uid))
            && slots.iter().all(|slot| {
                doc.model
                    .uid(*slot)
                    .and_then(|uid| doc.model.dimension_set_of_item(uid))
                    .is_some_and(|(other, _)| other.id == set.id)
            })
        {
            let mut rows = dimension_set::rows(doc, first, any_locked);
            // A chain selection can contain both rows; its first member is not the whole selection's row.
            rows.retain(|row| row.label != "Sıra");
            sections.push(Section {
                id: "dimension-set",
                title: "Duvar ölçüleri".into(),
                rows,
            });
        }
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
                override_rows(
                    &dims
                        .iter()
                        .map(|d| d.look.overrides.as_slice())
                        .collect::<Vec<_>>(),
                    &slots,
                    any_locked,
                ),
                dimension::rows(
                    &dims,
                    &slots,
                    any_locked,
                    &f,
                    doc.model.settings().arch.as_ref().map_or(&[][..], |a| a.levels.as_slice()),
                ),
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

/// Ortak öznitelikler of several objects (docs/adr/0286): every key any of them has, in the order first met, its
/// value when every object has that same value, else “Çeşitli” with how many have it; a cell writes the value to
/// them all through `cad.entities.set` (the locked ones skipped and counted). At most 200 keys; none, `None`.
fn shared_attributes(objects: &[&Entity], ids: &[Slot], locked: bool) -> Option<Section> {
    const MOST: usize = 200;
    // Each key: its first value, whether every one met so far is the same, how many objects have it.
    let mut keys: Vec<(&str, &str, bool, usize)> = Vec::new();
    let mut at: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();
    for e in objects {
        for (key, value) in &e.base().attrs {
            match at.get(key.as_str()) {
                Some(&i) => {
                    let k = &mut keys[i];
                    k.2 &= k.1 == value.as_str();
                    k.3 += 1;
                }
                None if keys.len() < MOST => {
                    at.insert(key, keys.len());
                    keys.push((key, value, true, 1));
                }
                None => {}
            }
        }
    }
    if keys.is_empty() {
        return None;
    }
    let n = objects.len();
    let rows = keys
        .into_iter()
        .map(|(key, value, same, count)| {
            let shown = if same && count == n {
                value.to_owned()
            } else if same {
                format!("Çeşitli ({count}/{n} nesnede {value})")
            } else {
                format!("Çeşitli ({count}/{n} nesnede)")
            };
            Row {
                label: Cow::Owned(key.to_owned()),
                numeric: same && count == n && looks_numeric(value),
                editor: (!locked)
                    .then(|| Editor::Text(Field::Attributes(ids.to_vec(), key.to_owned()))),
                ..Row::text("", shown)
            }
        })
        .collect();
    Some(Section {
        id: "sharedAttrs",
        title: "Ortak öznitelikler".into(),
        rows,
    })
}

/// `+n.toFixed(places)`: a number rounded as the web shows it.
fn fixed_number(n: f64, places: usize) -> f64 {
    kentos_interaction::fixed(n, places)
        .parse::<f64>()
        .unwrap_or(n)
}

//! What Öznitelikler shows, as data (the web's `PropertiesPanel.ts`): the
//! summary over the grid, the header's meta, and the sections and rows for
//! no selection, one object and several, each row with its editor. The view
//! draws them (`properties::view`); the edits come back as `Event`s.

use std::borrow::Cow;

use kentos_contracts::{DimensionStyle, Entity, HatchPatternType};
use kentos_domain::{LayerTree, Slot};
use kentos_interaction::{
    Format, Vec2, angle_deg, arc_sweep, bearing_grad, dimension_layout, dist, fixed, full_ellipse,
    measures,
};
use kentos_ui::icon::Icon;

use super::{Event, Field};
use crate::app::Message;
use crate::document::{Document, crs_name};
use crate::selecting::kind_title;

/// The colours the panel offers (the web's `DRAW_COLORS`, fields.ts).
pub(crate) const DRAW_COLORS: [(&str, &str); 8] = [
    ("Siyah", "ink"),
    ("Kırmızı", "#E5484D"),
    ("Sarı", "#F2C94C"),
    ("Yeşil", "#5FBF77"),
    ("Camgöbeği", "#4CC3D9"),
    ("Mavi", "#4F8EF7"),
    ("Eflatun", "#C86DD7"),
    ("Gri", "#8C9AAA"),
];

/// A hatch pattern's name (the web's `HATCH_PATTERN_LABEL`), in its order.
const PATTERNS: [(HatchPatternType, &str); 3] = [
    (HatchPatternType::Solid, "Dolu"),
    (HatchPatternType::Lines, "Çizgili"),
    (HatchPatternType::Cross, "Çapraz"),
];

/// The panel's content.
pub(crate) struct Panel {
    pub summary: Summary,
    /// The header's meta: `#12`, `3 nesne`.
    pub meta: Option<String>,
    pub sections: Vec<Section>,
}

/// Over the grid: the object's kind and label and its layer, or the selection's size and kinds.
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
pub(crate) struct Section {
    pub id: &'static str,
    pub title: &'static str,
    pub rows: Vec<Row>,
}

/// A row: its name, its value as shown, and how it is edited, if it is.
pub(crate) struct Row {
    pub label: Cow<'static, str>,
    pub value: String,
    /// Figures: tabular.
    pub numeric: bool,
    pub unit: Option<Cow<'static, str>>,
    pub editor: Option<Editor>,
}

/// How a row is edited.
pub(crate) enum Editor {
    /// A text cell: the committed text goes to the field.
    Text(Field),
    /// A number cell; the field reads the number as the web does.
    Number(Field),
    /// A drop-down: what it shows (a colour value for the swatch) and its choices.
    Select {
        text: String,
        swatch: Option<String>,
        items: Vec<Choice>,
    },
}

/// One entry of a drop-down.
#[derive(Clone)]
pub(crate) enum Choice {
    /// One of a group; a colour value for its swatch; not enabled when locked.
    Pick {
        label: String,
        swatch: Option<String>,
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

impl Row {
    fn text(label: impl Into<Cow<'static, str>>, value: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            value: value.into(),
            numeric: false,
            unit: None,
            editor: None,
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
        title: "Çizim",
        rows: vec![
            Row::text("Dosya", doc.name()),
            Row::text(
                "Koordinat sistemi",
                crs_name(s.srid).map_or_else(|| format!("EPSG:{}", s.srid), str::to_owned),
            ),
            Row::figure("SRID", format!("EPSG:{}", s.srid)),
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
        items: layers
            .leaves()
            .into_iter()
            .map(|l| Choice::Pick {
                label: layer_path(layers, &l.id),
                swatch: Some(l.style.color.clone()),
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
            chosen: current == Some(None),
            enabled: true,
            message: set(None),
        },
        Choice::Separator,
    ];
    items.extend(DRAW_COLORS.iter().map(|(name, value)| Choice::Pick {
        label: (*name).to_owned(),
        swatch: Some((*value).to_owned()),
        chosen: current == Some(Some(*value)),
        enabled: true,
        message: set(Some(value)),
    }));
    Editor::Select {
        text: known.map_or_else(|| color_text(current), |(name, _)| (*name).to_owned()),
        swatch: known.map(|(_, value)| (*value).to_owned()),
        items,
    }
}

/// Sembol ▾: the layer's style, or one from the library (the web's `symbolEditor`).
fn symbol_editor(doc: &Document, current: Option<Option<&str>>) -> Editor {
    Editor::Select {
        text: symbol_text(doc, current),
        swatch: None,
        items: vec![
            Choice::Pick {
                label: "Katman stiline göre".to_owned(),
                swatch: None,
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

/// One object's sections: Genel, Geometri, and Öznitelik bilgileri when it has attributes.
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
    if takes_symbol(e) {
        general.push(
            Row::text("Sembol", symbol_text(doc, symbol)).editor(edit(symbol_editor(doc, symbol))),
        );
    }

    let f = Format::of(doc.settings());
    let decimals = doc.settings().area_decimals as usize;
    let len = |label: &'static str, value: f64| Row::figure(label, f.length_bare(value));
    let metres = |label: &'static str, value: f64| len(label, value).unit("m");
    // An area: one row in m², or m² and the project's unit.
    let area = |m2: f64| -> Vec<Row> {
        if f.area_unit_label() == "m²" {
            vec![Row::figure("Alan", f.area_bare(m2)).unit("m²")]
        } else {
            vec![
                Row::figure("Alan", fixed(m2, decimals)).unit("m²"),
                Row::figure("Alan", f.area_bare(m2)).unit(f.area_unit_label()),
            ]
        }
    };
    let bearing = |a: Vec2, b: Vec2| {
        Row::figure("Semt", f.bearing_bare(bearing_grad(a, b))).unit(f.angle_unit_label())
    };
    let number = |field: Field| edit(Editor::Number(field));
    let (area_of, length_of) = measures(e);

    let mut geo: Vec<Row> = Vec::new();
    match e {
        Entity::Point(p) => {
            geo.push(metres("Y (sağa)", p.p.x).editor(number(Field::PointX(slot))));
            geo.push(metres("X (yukarı)", p.p.y).editor(number(Field::PointY(slot))));
            if let Some(z) = p.z {
                geo.push(metres("Z (kot)", z));
            }
        }
        Entity::Line(l) => {
            geo.extend([
                len("Başlangıç Y", l.a.x),
                len("Başlangıç X", l.a.y),
                len("Bitiş Y", l.b.x),
                len("Bitiş X", l.b.y),
                metres("Uzunluk", dist(v(l.a), v(l.b))),
                bearing(v(l.a), v(l.b)),
            ]);
        }
        Entity::Polyline(p) | Entity::Polygon(p) => {
            let polygon = matches!(e, Entity::Polygon(_));
            geo.push(Row::figure("Köşe sayısı", p.pts.len().to_string()));
            geo.push(metres(
                if polygon { "Çevre" } else { "Uzunluk" },
                length_of.unwrap_or(0.0),
            ));
            if polygon {
                // Net area: the holes (adalar) are taken out already.
                if let Some(holes) = p.holes.as_ref().filter(|h| !h.is_empty()) {
                    geo.push(Row::figure("Ada (delik)", holes.len().to_string()));
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
                },
            ));
            match dimension_layout(e) {
                Some(l) if l.unit == "angle" => geo.push(
                    Row::figure("Ölçülen açı", f.angle_bare(l.value)).unit(f.angle_unit_label()),
                ),
                Some(l) => geo.push(metres(
                    match style {
                        DimensionStyle::Diameter => "Ölçülen çap",
                        DimensionStyle::Radius => "Ölçülen yarıçap",
                        _ => "Ölçülen uzunluk",
                    },
                    l.value,
                )),
                None => {}
            }
            if style == DimensionStyle::Aligned {
                geo.push(bearing(v(d.a), v(d.b)));
            }
            geo.extend([
                metres(
                    match style {
                        DimensionStyle::Angular => "Yay yarıçapı",
                        DimensionStyle::Radius | DimensionStyle::Diameter => "Dışa uzantı",
                        _ => "Ötelenme",
                    },
                    d.offset,
                )
                .editor(number(Field::DimensionOffset(slot))),
                metres("Yazı yüksekliği", d.height).editor(number(Field::DimensionHeight(slot))),
                Row::text("Yazı", d.text.clone().unwrap_or_default())
                    .editor(edit(Editor::Text(Field::DimensionText(slot)))),
            ]);
        }
        Entity::Hatch(h) => {
            let name = |t: HatchPatternType| {
                PATTERNS
                    .iter()
                    .find(|(p, _)| *p == t)
                    .map_or("", |(_, n)| *n)
            };
            let pattern = Editor::Select {
                text: name(h.pattern.kind).to_owned(),
                swatch: None,
                items: PATTERNS
                    .iter()
                    .map(|(t, n)| Choice::Pick {
                        label: (*n).to_owned(),
                        swatch: None,
                        chosen: *t == h.pattern.kind,
                        enabled: true,
                        message: Message::Properties(Event::Pattern(slot, *t)),
                    })
                    .collect(),
            };
            geo.extend([
                Row::text("Desen", name(h.pattern.kind)).editor(edit(pattern)),
                Row::figure("Açı", fixed(h.pattern.angle, 2))
                    .unit("°")
                    .editor(number(Field::HatchAngle(slot))),
                metres("Aralık", h.pattern.spacing).editor(number(Field::HatchSpacing(slot))),
            ]);
            geo.extend(area(area_of.unwrap_or(0.0)));
        }
        Entity::Text(t) => {
            geo.extend([
                Row::text("Metin", t.text.clone()).editor(edit(Editor::Text(Field::Text(slot)))),
                metres("Yükseklik", t.height).editor(number(Field::TextHeight(slot))),
                Row::figure("Açı", fixed(t.rotation, 2))
                    .unit("°")
                    .editor(number(Field::TextAngle(slot))),
                len("Konum Y", t.p.x),
                len("Konum X", t.p.y),
            ]);
        }
    }

    let mut sections = vec![
        Section {
            id: "general",
            title: "Genel",
            rows: general,
        },
        Section {
            id: "geometry",
            title: "Geometri",
            rows: geo,
        },
    ];
    if !base.attrs.is_empty() {
        sections.push(Section {
            id: "attrs",
            title: "Öznitelik bilgileri",
            rows: base
                .attrs
                .iter()
                .map(|(key, value)| Row {
                    label: Cow::Owned(key.clone()),
                    value: value.clone(),
                    numeric: looks_numeric(value),
                    unit: None,
                    editor: edit(Editor::Text(Field::Attribute(slot, key.clone()))),
                })
                .collect(),
        });
    }
    sections
}

/// A value that reads as a number: `12`, `-3,5` (the web's `/^-?\d+([.,]\d+)?$/`).
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
    let any_locked = objects.iter().any(|e| layers.is_locked(&e.base().layer_id));
    let edit = |editor: Editor| (!any_locked).then_some(editor);

    let mut sections = vec![Section {
        id: "general",
        title: "Ortak özellikler",
        rows: vec![
            Row::text("Katman", "Kilitli katman içeriyor")
                .editor(edit(layer_editor(doc, &ids, layer))),
            Row::text("Renk", color_text(color)).editor(edit(color_editor(&ids, color))),
            Row::text("Sembol", symbol_text(doc, symbol)).editor(edit(symbol_editor(doc, symbol))),
        ],
    }];
    let f = Format::of(doc.settings());
    let mut totals = Vec::new();
    if length > 0.0 {
        totals.push(Row::figure("Toplam uzunluk", f.length_bare(length)).unit("m"));
    }
    if area > 0.0 {
        totals.push(Row::figure("Toplam alan", f.area_bare(area)).unit(f.area_unit_label()));
    }
    if !totals.is_empty() {
        sections.push(Section {
            id: "totals",
            title: "Toplamlar",
            rows: totals,
        });
    }
    sections
}

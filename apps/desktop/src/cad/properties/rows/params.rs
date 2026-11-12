//! A dynamic block's insert's rows of docs/adr/0319 §3 in Öznitelikler, “Blok parametreleri”: each parameter of the
//! definition with the value the insert shows (`block_params::current`: its kept value, else the default; a lookup's
//! row whose values all hold). A length and an angle are typed, a point as “doğu, kuzey”, a flip, a visibility state
//! and a lookup row chosen; a parameter its expression computes is shown, not set. Every change goes through
//! `cad.blocks.parameters` (`set`), the rule's refusal said; Değerler ▾ resets all or named values (`reset`).

use kentos_commands::{BlockParameter, InsertEntity, ParamValue};
use kentos_domain::Slot;
use kentos_interaction::Format;

use super::{Choice, Editor, Row, Section};
use crate::app::Message;
use crate::document::Document;
use crate::properties::{Event, Field};

/// How a value is shown in its cell.
pub(crate) fn shown(p: &BlockParameter, v: &ParamValue, f: Format, factor: f64) -> String {
    match v {
        ParamValue::Flag(b) => if *b { "Evet" } else { "Hayır" }.to_owned(),
        ParamValue::Number(x) if matches!(p, BlockParameter::Angle { .. }) => f.angle_bare(*x),
        ParamValue::Number(x) => f.plain(*x * factor),
        ParamValue::Text(t) => t.clone(),
        // These are definition-local coordinates, not world/UCS coordinates.
        ParamValue::Point(p) => format!("{}, {}", f.plain(p.x * factor), f.plain(p.y * factor)),
    }
}

/// Show the existing constraints in the same units as the edited value; never reimplement their validation.
pub(crate) fn bounds(p: &BlockParameter, f: Format, factor: f64) -> Vec<String> {
    let Some(rule) = p.rule() else {
        return Vec::new();
    };
    let number = |n| {
        let unit = if matches!(p, BlockParameter::Angle { .. }) {
            f.angle_unit_label()
        } else {
            f.length_unit_label()
        };
        format!("{} {unit}", shown(p, &ParamValue::Number(n), f, factor))
    };
    let mut parts = Vec::new();
    if let Some(n) = rule.min {
        parts.push(format!("En az {}", number(n)));
    }
    if let Some(n) = rule.max {
        parts.push(format!("En çok {}", number(n)));
    }
    if let Some(n) = rule.step {
        parts.push(format!("Adım {}", number(n)));
    }
    if !rule.list.is_empty() {
        let values = rule
            .list
            .iter()
            .take(6)
            .map(|n| number(*n))
            .collect::<Vec<_>>()
            .join(", ");
        let more = if rule.list.len() > 6 {
            format!(" (+{} değer)", rule.list.len() - 6)
        } else {
            String::new()
        };
        parts.push(format!("İzin verilen: {values}{more}"));
    }
    parts
}

/// Project-unit input, converted back to definition-local values. A unitless definition uses raw drawing-unit
/// coordinates; physical definitions use metres (`blocks_insert::unit_factor`). The insert's extra scale is not
/// part of its parameter. Public commands keep their canonical contract.
pub(crate) fn typed(p: &BlockParameter, text: &str, f: Format, factor: f64) -> Option<ParamValue> {
    use kentos_geometry_core::tools::point_text::{
        Len, parse_length, parse_length_pair, parse_number,
    };
    // Keep complete numeric input (including an exponent emitted by Format::plain) without parseFloat's
    // dangerous acceptance of a numeric prefix followed by an unknown unit.
    let number = |t: &str| t.trim().replacen(',', ".", 1).parse::<f64>().ok();
    let length = |t: &str| parse_length(t).or_else(|| number(t).map(Len::Unit));
    let local = |v: kentos_geometry_core::tools::point_text::Len| {
        let x = v.metres(|n| f.to_metres(n)) / factor;
        x.is_finite().then_some(x)
    };
    match p {
        BlockParameter::Linear { .. } => local(length(text)?).map(ParamValue::Number),
        BlockParameter::Angle { .. } => {
            let a = f.angle_from_typed(parse_number(text).or_else(|| number(text))?);
            a.is_finite().then_some(ParamValue::Number(a))
        }
        BlockParameter::Polar { .. } | BlockParameter::Point { .. } => {
            let separator = if text.contains(';') { ';' } else { ',' };
            let pair = if text.matches(separator).count() == 1 {
                let (x, y) = text.split_once(separator)?;
                Some((length(x)?, length(y)?))
            } else {
                parse_length_pair(text).or_else(|| {
                    let mut parts = text.split_whitespace();
                    let pair = (length(parts.next()?)?, length(parts.next()?)?);
                    parts.next().is_none().then_some(pair)
                })
            };
            let (x, y) = pair?;
            Some(ParamValue::Point(kentos_commands::Vec2 {
                x: local(x)?,
                y: local(y)?,
            }))
        }
        _ => None,
    }
}

/// A drop-down of `labels`, the one shown chosen, each sending its value.
fn choose(slot: Slot, name: &str, shown: &str, labels: Vec<(String, ParamValue)>) -> Editor {
    Editor::Select {
        text: shown.to_owned(),
        swatch: None,
        icon: None,
        items: labels
            .into_iter()
            .map(|(label, value)| Choice::Pick {
                chosen: label == shown,
                label,
                swatch: None,
                icon: None,
                wide: false,
                enabled: true,
                message: Message::Properties(Event::Parameter(slot, name.to_owned(), value)),
            })
            .collect(),
    }
}

/// Presentation groups only: never reorder definition parameters or actions used by the shared evaluator.
fn group(p: &BlockParameter) -> usize {
    if p.rule().is_some_and(|r| r.expression.is_some()) {
        3
    } else {
        match p {
            BlockParameter::Flip { .. } | BlockParameter::Visibility { .. } => 1,
            BlockParameter::Lookup { .. } => 2,
            _ => 0,
        }
    }
}

fn overridden(p: &BlockParameter, i: &InsertEntity) -> bool {
    match p {
        BlockParameter::Lookup { rows, .. } => rows
            .iter()
            .any(|r| r.values.keys().any(|name| i.parameters.contains_key(name))),
        _ => group(p) != 3 && i.parameters.contains_key(p.name()),
    }
}

/// The insert's identity, reset actions and collapsible parameter groups. No document state is changed here.
pub(super) fn sections(doc: &Document, slot: Slot, i: &InsertEntity, locked: bool) -> Vec<Section> {
    let Some(block) = doc.model.block(i.block) else {
        return Vec::new();
    };
    let Some(d) = block.dynamic.as_ref() else {
        return Vec::new();
    };
    let f = Format::of(doc.model.settings());
    let factor = kentos_native_application::blocks_insert::unit_factor(&doc.model, block);
    let current = kentos_native_application::block_params::current(d, &i.parameters);
    let mut groups: [Vec<Row>; 4] = Default::default();
    let mut custom: Vec<Vec<Row>> = vec![Vec::new(); d.parameter_groups.len()];
    // Display order is explicit metadata; evaluation still uses d.parameters unchanged.
    let parameters: std::collections::HashMap<_, _> = if d.parameter_groups.is_empty() {
        std::collections::HashMap::new()
    } else { d.parameters.iter().map(|p| (p.name(), p)).collect() };
    let membership: std::collections::HashMap<_, _> = d.parameter_groups.iter().enumerate()
        .flat_map(|(i, g)| g.parameters.iter().map(move |name| (name.as_str(), i))).collect();
    let ordered = d.parameter_groups.iter().flat_map(|g| g.parameters.iter())
        .filter_map(|name| parameters.get(name.as_str()).copied())
        .chain(d.parameters.iter().filter(|p| !membership.contains_key(p.name())));
    for p in ordered {
        let out = match membership.get(p.name()) {
            Some(&index) => &mut custom[index],
            None => &mut groups[group(p)],
        };
        let name = p.name();
        let value = current.get(name);
        let text = value.map_or_else(|| "—".to_owned(), |v| shown(p, v, f, factor));
        let derived = p.rule().is_some_and(|r| r.expression.is_some());
        let row = match p {
            BlockParameter::Linear { .. } => {
                Row::figure(name.to_owned(), text.clone()).unit(f.length_unit_label())
            }
            BlockParameter::Angle { .. } => {
                Row::figure(name.to_owned(), text.clone()).unit(f.angle_unit_label())
            }
            BlockParameter::Polar { .. } | BlockParameter::Point { .. } => {
                Row::text(name.to_owned(), text.clone()).unit(f.length_unit_label())
            }
            _ => Row::text(name.to_owned(), text.clone()),
        };
        let editor = match p {
            _ if derived || d.is_type_parameter(name) => None,
            BlockParameter::Linear { .. }
            | BlockParameter::Angle { .. }
            | BlockParameter::Polar { .. }
            | BlockParameter::Point { .. } => {
                Some(Editor::Text(Field::Parameter(slot, name.to_owned())))
            }
            BlockParameter::Flip { .. } => Some(choose(
                slot,
                name,
                &text,
                vec![
                    ("Evet".into(), ParamValue::Flag(true)),
                    ("Hayır".into(), ParamValue::Flag(false)),
                ],
            )),
            BlockParameter::Visibility { states, .. } => Some(choose(
                slot,
                name,
                &text,
                states
                    .iter()
                    .map(|s| (s.name.clone(), ParamValue::Text(s.name.clone())))
                    .collect(),
            )),
            BlockParameter::Lookup { rows, .. } => Some(choose(
                slot,
                name,
                &text,
                rows.iter()
                    .map(|r| (r.label.clone(), ParamValue::Text(r.label.clone())))
                    .collect(),
            )),
        };
        out.push(row.editor(editor).editor_if(!locked));
        if derived {
            let expression = p
                .rule()
                .and_then(|r| r.expression.as_deref())
                .unwrap_or_default();
            out.push(Row::note(&format!("{}Hesaplanan · {expression}", if d.is_type_parameter(name) { "Tip değeri · " } else { "" })));
        } else {
            let source = if d.is_type_parameter(name) { "Tip değeri · bütün kullanımlarda ortak" } else if matches!(p, BlockParameter::Lookup { .. }) {
                if value.is_some() {
                    "Eşleşen katalog satırı"
                } else {
                    "Eşleşen katalog satırı yok"
                }
            } else if i.parameters.contains_key(name) {
                "Örneğe özel"
            } else {
                "Tanım varsayılanı"
            };
            let mut details = vec![source.to_owned()];
            details.extend(bounds(p, f, factor));
            out.push(Row::note(&details.join(" · ")));
        }
    }
    let changed = d
        .parameters
        .iter()
        .filter(|p| !matches!(p, BlockParameter::Lookup { .. }) && overridden(p, i))
        .count();
    let said = match changed {
        0 => "Varsayılan".to_owned(),
        n => format!("{n} değer değişti"),
    };
    let mut reset_items = vec![Choice::Action {
        label: "Hepsini varsayılana döndür",
        icon: "undo",
        message: (changed > 0).then_some(Message::Properties(Event::ParametersReset(slot))),
    }];
    if changed > 0 {
        reset_items.push(Choice::Separator);
        reset_items.extend(d.parameters.iter().filter(|p| overridden(p, i)).map(|p| {
            Choice::Pick {
                label: format!("{} · Varsayılana döndür", p.name()),
                chosen: false,
                swatch: None,
                icon: Some("undo"),
                wide: false,
                enabled: true,
                message: Message::Properties(Event::ParameterReset(slot, p.name().to_owned())),
            }
        }));
    }
    reset_items.push(Choice::Separator);
    reset_items.push(Choice::Action {
        label: "Tip değerleri…", icon: "blockCard",
        message: Some(Message::BlocksPanel(crate::blocks_panel::Event::TypeValues(block.id))),
    });
    let reset = Editor::Select {
        text: said.clone(),
        swatch: None,
        icon: None,
        items: reset_items,
    };
    let mut identity = vec![
        Row::text("Tanım", block.name.clone()),
        Row::text(
            "Tanım sürümü",
            block
                .card
                .as_ref()
                .map_or_else(|| "Kartı yok".into(), |c| c.version.to_string()),
        ),
        Row::note(
            "Örnek değerleri bu yerleştirmeye aittir. Tip değerleri ortaktır; Bloklar menüsündeki Tip değerleri penceresinde düzenlenir.",
        ),
        Row::text("Değerler", said)
            .editor(Some(reset))
            .editor_if(!locked),
    ];
    if d.parameters.is_empty() {
        identity.push(Row::note("Bu tanımda henüz parametre yok."));
    }
    let mut sections = vec![Section {
        id: "blockParams".into(),
        title: "Blok parametreleri".into(),
        rows: identity,
    }];
    for (g, rows) in d.parameter_groups.iter().zip(custom) {
        if !rows.is_empty() {
            sections.push(Section {
                id: format!("blockParamsGroup:{}:{}", block.id.to_text(), g.name).into(),
                title: g.name.clone().into(),
                rows,
            });
        }
    }
    for ((id, title), rows) in [
        ("blockParamsDimensions", "Boyut ve konum"),
        ("blockParamsAppearance", "Görünüş"),
        ("blockParamsCatalog", "Katalog"),
        ("blockParamsComputed", "Hesaplanan"),
    ]
    .into_iter()
    .zip(groups)
    {
        if !rows.is_empty() {
            sections.push(Section {
                id: id.into(),
                title: if d.parameter_groups.is_empty() { title.into() } else { format!("{title} · Otomatik").into() },
                rows,
            });
        }
    }
    sections
}

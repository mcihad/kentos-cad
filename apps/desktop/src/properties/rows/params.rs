//! A dynamic block's insert's rows of docs/adr/0319 §3 in Öznitelikler, “Blok parametreleri”: each parameter of the
//! definition with the value the insert shows (`block_params::current`: its kept value, else the default; a lookup's
//! row whose values all hold). A length and an angle are typed, a point as “doğu, kuzey”, a flip, a visibility state
//! and a lookup row chosen; a parameter its expression computes is shown, not set. Every change goes through
//! `cad.blocks.parameters` (`set`), the rule's refusal said; Değerler ▾ resets them all (`reset`).

use kentos_commands::{BlockParameter, InsertEntity, ParamValue};
use kentos_domain::Slot;
use kentos_interaction::Format;

use super::{Choice, Editor, Row};
use crate::app::Message;
use crate::document::Document;
use crate::properties::{Event, Field};

/// How a value is shown in its cell.
fn shown(p: &BlockParameter, v: &ParamValue, f: Format, factor: f64) -> String {
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
fn bounds(p: &BlockParameter, f: Format, factor: f64) -> Vec<String> {
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

/// The rows of insert `i`, one a parameter, then Değerler; none for a plain block's insert. Editors unless locked.
pub(super) fn rows(doc: &Document, slot: Slot, i: &InsertEntity, locked: bool) -> Vec<Row> {
    let Some(block) = doc.model.block(i.block) else {
        return Vec::new();
    };
    let Some(d) = block.dynamic.as_ref() else {
        return Vec::new();
    };
    let f = Format::of(doc.model.settings());
    let factor = kentos_native_application::blocks_insert::unit_factor(&doc.model, block);
    let current = kentos_native_application::block_params::current(d, &i.parameters);
    let mut out = Vec::new();
    for p in &d.parameters {
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
            _ if derived => None,
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
            out.push(Row::note(&format!("Hesaplanan · {expression}")));
        } else {
            let source = if matches!(p, BlockParameter::Lookup { .. }) {
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
    let changed = i.parameters.len();
    let said = match changed {
        0 => "Varsayılan".to_owned(),
        n => format!("{n} değer değişti"),
    };
    let reset = Editor::Select {
        text: said.clone(),
        swatch: None,
        icon: None,
        items: vec![Choice::Action {
            label: "Hepsini varsayılana döndür",
            icon: "undo",
            message: (changed > 0).then_some(Message::Properties(Event::ParametersReset(slot))),
        }],
    };
    out.push(
        Row::text("Değerler", said)
            .editor(Some(reset))
            .editor_if(!locked),
    );
    out
}

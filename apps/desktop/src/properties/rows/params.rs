//! A dynamic block's insert's rows of docs/adr/0319 §3 in Öznitelikler, “Blok parametreleri”: each parameter of the
//! definition with the value the insert shows (`block_params::current`: its kept value, else the default; a lookup's
//! row whose values all hold). A length and an angle are typed, a point as “doğu, kuzey”, a flip, a visibility state
//! and a lookup row chosen; a parameter its expression computes is shown, not set. Every change goes through
//! `cad.blocks.parameters` (`set`), the rule's refusal said; Değerler ▾ resets them all (`reset`).

use kentos_contracts::{BlockParameter, InsertEntity, ParamValue};
use kentos_domain::Slot;
use kentos_interaction::fixed;

use super::{Choice, Editor, Row};
use crate::app::Message;
use crate::document::Document;
use crate::properties::{Event, Field};

/// How a value is shown in its cell.
pub(crate) fn shown(v: &ParamValue) -> String {
    match v {
        ParamValue::Flag(b) => if *b { "Evet" } else { "Hayır" }.to_owned(),
        ParamValue::Number(x) => fixed(*x, 3),
        ParamValue::Text(t) => t.clone(),
        ParamValue::Point(p) => format!("{}, {}", fixed(p.x, 3), fixed(p.y, 3)),
    }
}

/// The value typed in a cell for `p` (docs/adr/0319 §3): a length as a number, an angle in degrees, a point as two
/// numbers (east, north) apart by a comma, a semicolon or a space; none when it does not read.
pub(crate) fn typed(p: &BlockParameter, text: &str) -> Option<ParamValue> {
    let n = |t: &str| {
        let x = crate::properties::web_number(t.trim());
        x.is_finite().then_some(x)
    };
    match p {
        BlockParameter::Linear { .. } => n(text).map(ParamValue::Number),
        BlockParameter::Angle { .. } => n(text).map(|d| ParamValue::Number(d.to_radians())),
        BlockParameter::Polar { .. } | BlockParameter::Point { .. } => {
            let parts: Vec<&str> = text
                .split([',', ';', ' '])
                .filter(|s| !s.trim().is_empty())
                .collect();
            match parts.as_slice() {
                [x, y] => Some(ParamValue::Point(kentos_contracts::Vec2 {
                    x: n(x)?,
                    y: n(y)?,
                })),
                _ => None,
            }
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
    let Some(d) = doc.model.block(i.block).and_then(|b| b.dynamic.as_ref()) else {
        return Vec::new();
    };
    let current = kentos_native_application::block_params::current(d, &i.parameters);
    let mut out = Vec::new();
    for p in &d.parameters {
        let name = p.name();
        let value = current.get(name);
        let text = value.map_or_else(|| "—".to_owned(), shown);
        let derived = p.rule().is_some_and(|r| r.expression.is_some());
        let row = match p {
            BlockParameter::Linear { .. } => Row::figure(name.to_owned(), text.clone()),
            BlockParameter::Angle { .. } => {
                let deg = match value {
                    Some(ParamValue::Number(a)) => fixed(a.to_degrees(), 4),
                    _ => "—".to_owned(),
                };
                Row::figure(name.to_owned(), deg).unit("°")
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
            out.push(Row::note("ifadeyle hesaplanır"));
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

//! A kitchen run's section (docs/adr/0727 §6): what the run is and, for a module's cabinet, its width, door and what it
//! holds; each change through `cad.casework.edit`, the run laid anew in one undo step.
use std::borrow::Cow;

use kentos_commands::{
    CaseworkBreak, CaseworkCorner, CaseworkDoor, CaseworkEnd, CaseworkHand, CaseworkModuleKind, CaseworkPart,
};
use kentos_domain::Slot;

use super::{Choice, Editor, Event, Field, Row, Section};
use crate::app::Message;
use crate::document::Document;

/// The module widths a run takes (the library's: 0.30–1.20 m, 0.15 apart).
const WIDTHS: [f64; 7] = [0.30, 0.45, 0.60, 0.75, 0.90, 1.05, 1.20];

fn centimetres(v: f64) -> String {
    format!("{} cm", (v * 100.0).round())
}

/// The run the object in `slot` is part of, as a section; none when it is part of none.
pub(super) fn section(doc: &Document, slot: Slot, locked: bool) -> Option<Section> {
    let model = &doc.model;
    let uid = model.uid(slot)?;
    let (run, member) = model.casework_member(uid)?;
    let library = |_: &str| None;
    let mut fresh = || kentos_commands::BlockId(*kentos_domain::Uuid::now_v7().as_bytes());
    let laid = model.lay_casework(&run.recipe, &library, &mut fresh).ok();
    let recipe = &run.recipe;
    let corners = recipe.through.as_ref().map_or(0, Vec::len);
    let pick = |label: String, field: &'static str, value: &str, chosen: bool| Choice::Pick {
        label,
        swatch: None,
        icon: None,
        wide: false,
        chosen,
        enabled: !locked,
        message: Message::Properties(Event::CaseworkValue(slot, field, value.into())),
    };
    let select = |text: &str, items: Vec<Choice>| {
        Some(Editor::Select {
            text: text.into(),
            swatch: None,
            icon: None,
            items,
        })
    };
    let action = |label: &'static str, name: &'static str, editable: bool| Choice::Action {
        label,
        icon: "casework",
        message: (!editable || !locked).then_some(Message::Properties(Event::Casework(slot, name))),
    };
    let form = match corners {
        0 => "Düz",
        1 => "L",
        _ => "U",
    };
    let count = laid.as_ref().map_or(0, |l| l.modules.len());
    let what = format!("{form} · {count} modül");
    let mut rows = vec![Row::text("Dizi", &what).editor(select(
        &what,
        vec![
            action("Dizinin tamamını seç", "select", false),
            Choice::Separator,
            action("Diziyi bırak (nesneler kalır)", "release", true),
            action("Dizinin tamamını sil", "delete", true),
        ],
    ))];
    if let Some(why) = run.broken {
        let (state, words) = match why {
            CaseworkBreak::WallGone => (
                "Duvarı silindi",
                "Dizinin bağlı olduğu duvar silindi; dolaplar yerinde kaldı. Diziyi düzenleyin ya da bırakın.",
            ),
            CaseworkBreak::NotBuilt => (
                "Dizilemedi",
                "Duvarın yeni yerinde dizi kurulamadı; dolaplar son yerinde. Modülleri ya da duvarı düzeltin.",
            ),
        };
        rows.push(Row::text("Durum", state));
        let mut note = Row::note(words);
        note.warn = true;
        rows.push(note);
    }
    if corners > 0 {
        let corner = recipe.corner.unwrap_or_default();
        let text = match corner {
            CaseworkCorner::Blind => "Kör",
            CaseworkCorner::Open => "Açık",
        };
        rows.push(Row::text("Köşe", text).editor(select(
            text,
            vec![
                pick("Kör".into(), "corner", "blind", corner == CaseworkCorner::Blind),
                pick("Açık".into(), "corner", "open", corner == CaseworkCorner::Open),
            ],
        )));
    }
    let f = kentos_interaction::Format::of(doc.settings());
    let depth = recipe.depth.unwrap_or(kentos_commands::DEFAULT_CASEWORK_DEPTH);
    rows.push(
        Row::figure("Tezgâh derinliği", f.length_bare(depth))
            .unit(f.length_unit_label())
            .editor((!locked).then_some(Editor::Number(Field::CaseworkDepth(slot)))),
    );
    let yes_no = |on: bool| if on { "Açık" } else { "Kapalı" };
    let upper = recipe.upper.unwrap_or(false);
    rows.push(Row::text("Üst dolap", yes_no(upper)).editor(select(
        yes_no(upper),
        vec![
            pick("Açık".into(), "upper", "true", upper),
            pick("Kapalı".into(), "upper", "false", !upper),
        ],
    )));
    if recipe.cooktop.is_some() {
        let oven = recipe.oven.unwrap_or(false);
        rows.push(Row::text("Fırın", yes_no(oven)).editor(select(
            yes_no(oven),
            vec![
                pick("Açık".into(), "oven", "true", oven),
                pick("Kapalı".into(), "oven", "false", !oven),
            ],
        )));
    }
    let fridge = match recipe.fridge {
        None => "Yok",
        Some(CaseworkEnd::Start) => "Başta",
        Some(CaseworkEnd::End) => "Sonda",
    };
    rows.push(Row::text("Buzdolabı", fridge).editor(select(
        fridge,
        vec![
            pick("Yok".into(), "fridge", "none", recipe.fridge.is_none()),
            pick("Başta".into(), "fridge", "start", recipe.fridge == Some(CaseworkEnd::Start)),
            pick("Sonda".into(), "fridge", "end", recipe.fridge == Some(CaseworkEnd::End)),
        ],
    )));
    if let Some(l) = &laid {
        rows.push(Row::figure("Dolgu", f.length_bare(l.filler)).unit(f.length_unit_label()));
    }
    // The module this object is: its width, its door and what it holds.
    let number = match member.part {
        CaseworkPart::Module | CaseworkPart::Upper => member.index,
        _ => None,
    };
    if let (Some(k), Some(l)) = (number, &laid)
        && let Some(m) = l.modules.iter().find(|m| m.number == k)
    {
        // A blind corner's module: the next leg's cabinets before its front (docs/adr/0727 §1).
        let blind = m.blind == Some(true);
        rows.push(Row::text(
            "Modül",
            if blind {
                format!("{k}. modül · kör köşede")
            } else {
                format!("{k}. modül")
            },
        ));
        if blind && m.kind != CaseworkModuleKind::Base {
            let next = m.leg.unwrap_or(0) + 2;
            let mut note = Row::note(&format!(
                "Önünü {next}. kolun dolapları kapatıyor; başka bir modüle alın ya da köşeyi açık yapın."
            ));
            note.warn = true;
            rows.push(note);
        }
        let kind_text = |kind: CaseworkModuleKind| match kind {
            CaseworkModuleKind::Base => "Alt dolap",
            CaseworkModuleKind::Sink => "Evye",
            CaseworkModuleKind::Cooktop => "Ocak",
            CaseworkModuleKind::Dishwasher => "Bulaşık makinesi",
        };
        let kinds = [
            (CaseworkModuleKind::Base, "base"),
            (CaseworkModuleKind::Sink, "sink"),
            (CaseworkModuleKind::Cooktop, "cooktop"),
            (CaseworkModuleKind::Dishwasher, "dishwasher"),
        ];
        rows.push(Row::text("Tür", kind_text(m.kind)).editor(select(
            kind_text(m.kind),
            kinds
                .iter()
                .map(|&(kind, value)| pick(kind_text(kind).into(), "holds", value, kind == m.kind))
                .collect(),
        )));
        let width = centimetres(m.width);
        // An appliance's width is the appliance's (a dishwasher, a cooktop over its oven).
        let appliance = m.kind == CaseworkModuleKind::Dishwasher || m.oven == Some(true);
        let widths = if appliance {
            Vec::new()
        } else {
            WIDTHS
                .iter()
                .map(|&w| pick(centimetres(w), "width", &w.to_string(), (w - m.width).abs() <= 1e-9))
                .collect()
        };
        let mut row = Row::text("Modül eni", width.clone());
        if !widths.is_empty() {
            row = row.editor(select(&width, widths));
        }
        rows.push(row);
        if let Some(hand) = m.hand {
            let text = match hand {
                CaseworkHand::Left => "Soldan",
                CaseworkHand::Right => "Sağdan",
                CaseworkHand::Double => "Çift kapı",
            };
            let mut row = Row::text("Açılma", text);
            if hand != CaseworkHand::Double {
                row = row.editor(select(
                    text,
                    vec![
                        pick("Soldan".into(), "hand", "left", hand == CaseworkHand::Left),
                        pick("Sağdan".into(), "hand", "right", hand == CaseworkHand::Right),
                    ],
                ));
            }
            rows.push(row);
        }
    }
    Some(Section {
        id: "casework",
        title: Cow::Borrowed("Mutfak dizisi"),
        rows,
    })
}

/// The door a module's own setting gives, if it gives one.
pub(crate) fn own_hand(recipe: &kentos_commands::CaseworkRun, module: u32) -> Option<CaseworkDoor> {
    recipe
        .per_module
        .iter()
        .flatten()
        .find(|m| m.module == module)
        .and_then(|m| m.hand)
}

/// The width a module's own setting gives, if it gives one.
pub(crate) fn own_width(recipe: &kentos_commands::CaseworkRun, module: u32) -> Option<f64> {
    recipe
        .per_module
        .iter()
        .flatten()
        .find(|m| m.module == module)
        .and_then(|m| m.width)
}

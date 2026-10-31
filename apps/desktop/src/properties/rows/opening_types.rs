//! An opening's type and sizes (docs/adr/0772 §1, §2): the type it follows and at which version, the values it keeps
//! as its own, the actions on its type (Tipten ayır, Tipe uygula, Tipe döndür, Tipi yeniden uygula, Aynı tiptekileri
//! seç) and its four widths: the rough opening (its `width`), the frame's outer width, the leaf and the net clear
//! width with the door open 90°. Each action through `cad.openings.edit`, in one undo step.
use kentos_commands::{OpeningFields, OpeningForm, OpeningsOperation};
use kentos_domain::Slot;
use kentos_interaction::Format;

use super::{Choice, Editor, Event, Row, Section};
use crate::app::Message;
use crate::document::Document;

/// The type section of the opening in `slot`; none for a niche or a flue, which take no type.
pub(super) fn section(
    doc: &Document,
    slot: Slot,
    x: &OpeningFields,
    f: &Format,
    locked: bool,
) -> Option<Section> {
    if x.form.is_recess() {
        return None;
    }
    let arch = doc.model.settings().arch.as_ref();
    let typed = x
        .style
        .as_deref()
        .and_then(|id| arch.and_then(|a| a.opening_type(id)));
    let action =
        |label: &'static str, icon: &'static str, op: Option<OpeningsOperation>| Choice::Action {
            label,
            icon,
            message: op
                .filter(|_| !locked)
                .map(|op| Message::Properties(Event::Opening(slot, op))),
        };
    let mut rows = Vec::new();
    match typed {
        Some(t) => {
            let own = x.type_version.unwrap_or(t.version);
            let version = if own < t.version {
                format!("{own} · eski (tipin sürümü {})", t.version)
            } else {
                format!("{own} · güncel")
            };
            rows.push(Row::text("Tip sürümü", version));
            let kept = if x.overrides.is_empty() {
                "Yok".to_owned()
            } else {
                x.overrides
                    .iter()
                    .map(|o| o.label())
                    .collect::<Vec<_>>()
                    .join(", ")
            };
            let items = vec![
                Choice::Command {
                    label: "Aynı tiptekileri seç",
                    icon: crate::icons::from_web(Some("openingSameType")),
                    id: "arch.openingSameType",
                },
                Choice::Separator,
                action(
                    "Tipten ayır",
                    "openingDetach",
                    Some(OpeningsOperation::Detach),
                ),
                action(
                    "Tipe uygula",
                    "openingPush",
                    (!x.overrides.is_empty())
                        .then(|| OpeningsOperation::Push { fields: Vec::new() }),
                ),
                action(
                    "Tipe döndür",
                    "openingReset",
                    (!x.overrides.is_empty() || own < t.version)
                        .then(|| OpeningsOperation::Reset { fields: Vec::new() }),
                ),
                action(
                    "Tipi yeniden uygula",
                    "openingTypes",
                    (own < t.version).then_some(OpeningsOperation::Refresh),
                ),
                Choice::Separator,
                Choice::Command {
                    label: "Kapı ve pencere tipleri…",
                    icon: crate::icons::from_web(Some("openingTypes")),
                    id: "arch.openingTypes",
                },
            ];
            rows.push(
                Row::text("İstisnalar", kept.clone()).editor(Some(Editor::Select {
                    text: kept,
                    swatch: None,
                    icon: None,
                    items,
                })),
            );
        }
        None => {
            let words = if x.style.is_some() {
                "Tipin kopyası"
            } else {
                "Tipsiz"
            };
            rows.push(Row::text("Tip sürümü", words).editor(Some(Editor::Select {
                text: words.to_owned(),
                swatch: None,
                icon: None,
                items: vec![Choice::Command {
                    label: "Kapı ve pencere tipleri…",
                    icon: crate::icons::from_web(Some("openingTypes")),
                    id: "arch.openingTypes",
                }],
            })));
        }
    }
    let sizes = kentos_commands::opening_sizes(x);
    let unit = f.length_unit_label();
    let length = |label: &'static str, v: f64| Row::figure(label, f.length_bare(v)).unit(unit);
    rows.push(length("Kaba boşluk", sizes.rough));
    if matches!(x.form, OpeningForm::Door | OpeningForm::Window) {
        rows.push(length("Kasa dış eni", sizes.unit));
        rows.push(length("Kanat eni", sizes.leaf));
    }
    match sizes.clear {
        Some(c) => rows.push(length("Net geçiş (90°)", c)),
        None if x.form == OpeningForm::Door => {
            rows.push(Row::text("Net geçiş (90°)", "Yok (sabit)"))
        }
        None => {}
    }
    let title = typed.map_or_else(
        || "Tip ve ölçüler".to_owned(),
        |t| format!("Tip: {}", t.name),
    );
    Some(Section {
        id: "openingType",
        title: title.into(),
        rows,
    })
}

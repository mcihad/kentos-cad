//! A grid's, column's, stair's and ramp's rows of docs/adr/0343 §5 in Öznitelikler: their own fields, each written
//! through `cad.arch.edit`, their level and phase, and a stair's or ramp's checks (information, never a refusal); a
//! tag's of docs/adr/0344 §5, written through `cad.tags.edit`.

use kentos_commands::{
    ArchChange, ArchInfo, ArchOperation, ArchPhase, ColumnShape, Entity, GridBubbles, TagChange, TagShape,
};
use kentos_domain::Slot;
use kentos_interaction::{Format, fixed};

use super::{Choice, Editor, Row};
use crate::app::Message;
use crate::document::Document;
use crate::properties::{ArchField, Event, Field};

fn set(slot: Slot, c: ArchChange) -> Message {
    Message::Properties(Event::Arch(slot, ArchOperation::Set(Box::new(c))))
}

fn pick(label: String, chosen: bool, message: Message, icon: Option<&'static str>) -> Choice {
    Choice::Pick {
        label,
        swatch: None,
        icon,
        wide: false,
        chosen,
        enabled: true,
        message,
    }
}

fn select(text: String, items: Vec<Choice>) -> Editor {
    Editor::Select {
        text,
        swatch: None,
        icon: None,
        items,
    }
}

/// Kat and Evre, as a wall's and a space's.
fn arch_rows(
    doc: &Document,
    slot: Slot,
    a: &ArchInfo,
    edit: &dyn Fn(Editor) -> Option<Editor>,
) -> Vec<Row> {
    let levels = doc
        .settings()
        .arch
        .as_ref()
        .map(|a| a.levels.clone())
        .unwrap_or_default();
    let level_name = a
        .level
        .as_deref()
        .map(|id| {
            levels
                .iter()
                .find(|l| l.id == id)
                .map_or_else(|| id.to_owned(), |l| l.name.clone())
        })
        .unwrap_or_else(|| "Yok".to_owned());
    let mut items = vec![pick(
        "Yok".to_owned(),
        a.level.is_none(),
        set(
            slot,
            ArchChange {
                level: Some(None),
                ..Default::default()
            },
        ),
        None,
    )];
    items.extend(levels.iter().map(|l| {
        pick(
            l.name.clone(),
            a.level.as_deref() == Some(l.id.as_str()),
            set(
                slot,
                ArchChange {
                    level: Some(Some(l.id.clone())),
                    ..Default::default()
                },
            ),
            None,
        )
    }));
    items.push(Choice::Separator);
    items.push(Choice::Command {
        label: "Katlar…",
        icon: crate::icons::from_web(Some("archSettings")),
        id: "arch.settings",
    });
    let phase = a.phase.map_or("Yok", ArchPhase::label);
    let mut phases = vec![pick(
        "Yok".to_owned(),
        a.phase.is_none(),
        set(
            slot,
            ArchChange {
                phase: Some(None),
                ..Default::default()
            },
        ),
        None,
    )];
    phases.extend(ArchPhase::ALL.iter().map(|&p| {
        pick(
            p.label().to_owned(),
            a.phase == Some(p),
            set(
                slot,
                ArchChange {
                    phase: Some(Some(p)),
                    ..Default::default()
                },
            ),
            None,
        )
    }));
    vec![
        Row::text("Kat", level_name.clone()).editor(edit(select(level_name, items))),
        Row::text("Evre", phase).editor(edit(select(phase.to_owned(), phases))),
    ]
}

/// The checks as rows: each a short warning and its whole words beneath as a note.
fn checks(warnings: Vec<kentos_commands::CommandWarning>) -> Vec<Row> {
    if warnings.is_empty() {
        return vec![Row::text("Denetim", "Uyuyor")];
    }
    let short = |code: &str| match code {
        "stair_short" => "Yol kısa",
        "stair_long" => "Yol uzun",
        "stair_step_rule" => "Blondel oranı dışında",
        "stair_rise_level" => "Kat yüksekliğini tutmuyor",
        "ramp_slope_mismatch" => "Eğim uymuyor",
        "ramp_no_run" => "Eğimli kol yok",
        _ => "Uyarı",
    };
    warnings
        .into_iter()
        .flat_map(|w| {
            [
                Row {
                    warn: true,
                    ..Row::text("Denetim", short(&w.code))
                },
                Row::note(&w.message),
            ]
        })
        .collect()
}

/// The rows of one grid, column, stair or ramp; editors unless its layer is locked.
pub(super) fn rows(doc: &Document, slot: Slot, e: &Entity, locked: bool, f: &Format) -> Vec<Row> {
    let edit = |editor: Editor| (!locked).then_some(editor);
    let text = |label: &'static str, v: String, which: ArchField| {
        Row::text(label, v).editor(edit(Editor::Text(Field::Arch(slot, which))))
    };
    let length = |label: &'static str, v: f64, which: ArchField| {
        Row::figure(label, f.length_bare(v))
            .unit(f.length_unit_label())
            .editor(edit(Editor::Number(Field::Arch(slot, which))))
    };
    let mut out = Vec::new();
    match e {
        Entity::Grid(g) => {
            let x = &g.grid;
            out.push(text("Etiket", x.label.clone(), ArchField::Label));
            let bubbles = GridBubbles::ALL
                .iter()
                .map(|&b| {
                    pick(
                        b.label().to_owned(),
                        b == x.bubbles,
                        set(
                            slot,
                            ArchChange {
                                bubbles: Some(b),
                                ..Default::default()
                            },
                        ),
                        None,
                    )
                })
                .collect();
            out.push(
                Row::text("Balonlar", x.bubbles.label())
                    .editor(edit(select(x.bubbles.label().to_owned(), bubbles))),
            );
            out.push(
                Row::figure("Balon çapı", fixed(x.bubble_size.unwrap_or(8.0), 1))
                    .unit("mm")
                    .editor(edit(Editor::Number(Field::Arch(
                        slot,
                        ArchField::BubbleSize,
                    )))),
            );
            if let [a, b] = x.pts.as_slice() {
                out.push(
                    Row::figure("Uzunluk", f.length_bare((b.x - a.x).hypot(b.y - a.y)))
                        .unit(f.length_unit_label()),
                );
            }
            out.extend(arch_rows(doc, slot, &x.arch, &edit));
        }
        Entity::Column(c) => {
            let x = &c.column;
            let shapes = ColumnShape::ALL
                .iter()
                .map(|&s| {
                    pick(
                        s.label().to_owned(),
                        s == x.shape,
                        set(
                            slot,
                            ArchChange {
                                shape: Some(s),
                                ..Default::default()
                            },
                        ),
                        None,
                    )
                })
                .collect();
            out.push(
                Row::text("Biçim", x.shape.label())
                    .editor(edit(select(x.shape.label().to_owned(), shapes))),
            );
            out.push(length(
                if x.shape == ColumnShape::Round {
                    "Çap"
                } else {
                    "En"
                },
                x.width,
                ArchField::Width,
            ));
            if x.shape == ColumnShape::Rect {
                out.push(length("Boy", x.depth(), ArchField::Depth));
                out.push(
                    Row::figure("Dönüş", fixed(x.rotation.to_degrees(), 2))
                        .unit("°")
                        .editor(edit(Editor::Number(Field::Arch(slot, ArchField::Turn)))),
                );
            }
            out.push(text(
                "Kolon türü",
                x.kind.clone().unwrap_or_default(),
                ArchField::Type,
            ));
            out.push(text(
                "Etiket",
                x.tag.clone().unwrap_or_default(),
                ArchField::Tag,
            ));
            let fill = |on: bool| {
                set(
                    slot,
                    ArchChange {
                        fill: Some(on),
                        ..Default::default()
                    },
                )
            };
            let shown = if x.fill { "Dolu" } else { "Boş" };
            out.push(Row::text("Dolgu", shown).editor(edit(select(
                shown.to_owned(),
                vec![
                    pick("Dolu".into(), x.fill, fill(true), None),
                    pick("Boş".into(), !x.fill, fill(false), None),
                ],
            ))));
            let tied = if x.grids.is_some() {
                x.place.clone().unwrap_or_else(|| "Bağlı".into())
            } else {
                "Bağlı değil".into()
            };
            let mut items = Vec::new();
            if x.grids.is_some() {
                items.push(pick(
                    "Bağı kopar".to_owned(),
                    false,
                    Message::Properties(Event::Arch(slot, ArchOperation::Detach)),
                    Some("gridNumber"),
                ));
            }
            let row = Row::text("Akslar", tied.clone());
            out.push(if items.is_empty() {
                row
            } else {
                row.editor(edit(select(tied, items)))
            });
            if let Some(b) = x.broken {
                out.push(Row {
                    warn: true,
                    ..Row::text("Durum", b.words())
                });
            }
            out.extend(arch_rows(doc, slot, &x.arch, &edit));
        }
        Entity::Stair(s) => {
            let x = &s.stair;
            out.push(length("Genişlik", x.width, ArchField::Width));
            out.push(
                Row::figure("Rıht sayısı", x.risers.to_string())
                    .editor(edit(Editor::Number(Field::Arch(slot, ArchField::Risers)))),
            );
            out.push(length("Rıht", x.riser_height, ArchField::RiserHeight));
            out.push(length("Basamak", x.tread, ArchField::Tread));
            out.push(length("Sahanlık", x.landing(), ArchField::Landing));
            out.push(
                Row::figure(
                    "Toplam yükseklik",
                    f.length_bare(f64::from(x.risers) * x.riser_height),
                )
                .unit(f.length_unit_label()),
            );
            out.extend(arch_rows(doc, slot, &x.arch, &edit));
            out.extend(checks(
                kentos_native_application::arch_elements::stair_warnings(&doc.model, x, ""),
            ));
        }
        Entity::Ramp(r) => {
            let x = &r.ramp;
            out.push(length("Genişlik", x.width, ArchField::Width));
            out.push(length("Yükseklik", x.rise, ArchField::Rise));
            out.push(
                Row::figure(
                    "Eğim",
                    x.slope
                        .map_or_else(|| "Hesaplanır".to_owned(), |s| fixed(s, 2)),
                )
                .unit(if x.slope.is_some() { "%" } else { "" })
                .editor(edit(Editor::Number(Field::Arch(slot, ArchField::Slope)))),
            );
            out.push(length(
                "Başlangıç kotu",
                x.base.unwrap_or(0.0),
                ArchField::Base,
            ));
            out.push(length("Sahanlık", x.landing(), ArchField::Landing));
            out.extend(arch_rows(doc, slot, &x.arch, &edit));
            out.extend(checks(
                kentos_native_application::arch_elements::ramp_warnings(x, ""),
            ));
        }
        Entity::Tag(t) => {
            let x = &t.tag;
            let tag = |c: TagChange| Message::Properties(Event::Tag(slot, c));
            let target = doc.model.tag_target(&x.of);
            let of = match &target {
                Some(e) => crate::selecting::kind_title(e.kind()).to_owned(),
                None => "Yok".to_owned(),
            };
            out.push(Row::text("Nesnesi", of));
            let content = x.content.clone().unwrap_or_default().replace('\n', "\\n");
            out.push(
                Row::text(
                    "Şablon",
                    if content.is_empty() {
                        "Türün".to_owned()
                    } else {
                        content
                    },
                )
                .editor(edit(Editor::Text(Field::Arch(slot, ArchField::TagContent)))),
            );
            out.push(Row::text("Yazısı", x.text.replace('\n', " / ")));
            let shapes = TagShape::ALL
                .iter()
                .map(|&sh| {
                    pick(
                        sh.label().to_owned(),
                        sh == x.shape,
                        tag(TagChange {
                            shape: Some(sh),
                            ..Default::default()
                        }),
                        None,
                    )
                })
                .collect();
            out.push(Row::text("Çerçeve", x.shape.label()).editor(edit(select(x.shape.label().to_owned(), shapes))));
            out.push(
                Row::figure("Yazı yüksekliği", fixed(x.size(), 1))
                    .unit("mm")
                    .editor(edit(Editor::Number(Field::Arch(slot, ArchField::TagSize)))),
            );
            let lead = if x.leader { "Var" } else { "Yok" };
            out.push(Row::text("Kol", lead).editor(edit(select(
                lead.to_owned(),
                vec![
                    pick("Var".into(), x.leader, tag(TagChange { leader: Some(true), ..Default::default() }), None),
                    pick("Yok".into(), !x.leader, tag(TagChange { leader: Some(false), ..Default::default() }), None),
                ],
            ))));
            if let Some(b) = x.broken {
                out.push(Row {
                    warn: true,
                    ..Row::text("Durum", b.words())
                });
            }
        }
        Entity::ViewMark(m) => {
            use kentos_commands::{ViewMarkChange, ViewMarkForm, ViewMarksOperation};
            let x = &m.mark;
            let op = |o: ViewMarksOperation| Message::Properties(Event::ViewMark(slot, o));
            out.push(Row::text("Biçim", x.form.label()));
            out.push(
                Row::text("Numara", if x.mark.is_empty() { "Pencerenin sırası" } else { &x.mark })
                    .editor(edit(Editor::Text(Field::Arch(slot, ArchField::MarkNumber)))),
            );
            // The project's targets, none first; the mark's own marked.
            let refs = kentos_native_application::viewmarks::references_of(&doc.model);
            let own = refs.targets.iter().find(|t| Some(&t.target) == x.target.as_ref());
            let now = match (&x.target, own) {
                (None, _) => "Yok".to_owned(),
                (Some(_), Some(t)) => t.name.clone(),
                (Some(_), None) => "Kayıp".to_owned(),
            };
            let mut targets = vec![pick("Yok".into(), x.target.is_none(), op(ViewMarksOperation::Detach), None)];
            for t in &refs.targets {
                targets.push(pick(
                    t.name.clone(),
                    Some(&t.target) == x.target.as_ref(),
                    op(ViewMarksOperation::Retarget { target: t.target.clone() }),
                    None,
                ));
            }
            out.push(Row::text("Hedef", now.clone()).editor(edit(select(now, targets))));
            out.push(Row::text("Yazısı", format!("{} / {}", x.words.top, x.words.bottom)));
            if x.form == ViewMarkForm::Section {
                let side = if x.flip { "Sağa" } else { "Sola" };
                out.push(Row::text("Bakış", side).editor(edit(select(
                    side.to_owned(),
                    vec![
                        pick("Sola".into(), !x.flip, op(ViewMarksOperation::Set(Box::new(ViewMarkChange { flip: Some(false), ..Default::default() }))), None),
                        pick("Sağa".into(), x.flip, op(ViewMarksOperation::Set(Box::new(ViewMarkChange { flip: Some(true), ..Default::default() }))), None),
                    ],
                ))));
            }
            out.push(
                Row::figure("Yazı yüksekliği", fixed(x.size(), 1))
                    .unit("mm")
                    .editor(edit(Editor::Number(Field::Arch(slot, ArchField::MarkSize)))),
            );
            if let Some(b) = x.broken {
                out.push(Row {
                    warn: true,
                    ..Row::text("Durum", b.words())
                });
            }
        }
        _ => {}
    }
    out
}

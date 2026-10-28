//! A wall dimension recipe, shown in the existing inspector without exposing ids; a façade's or a space's too
//! (docs/adr/0729 §7).
use kentos_commands::{
    DimensionSetBreak, DimensionSetFace, DimensionSetOpenings, DimensionSetReference, DimensionSetRow,
};
use kentos_domain::Slot;

use super::{Choice, Editor, Event, Field, Row};
use crate::app::Message;
use crate::document::Document;

pub(super) fn rows(doc: &Document, slot: Slot, locked: bool) -> Vec<Row> {
    let model = &doc.model;
    let Some((set, index)) = model
        .uid(slot)
        .and_then(|uid| model.dimension_set_of_item(uid))
    else {
        return Vec::new();
    };
    let member = &set.members[index];
    let row_name = match member.row {
        DimensionSetRow::Openings => "Açıklıklar",
        DimensionSetRow::Overall => "Toplam boy",
        DimensionSetRow::Walls => "Duvarlar",
        DimensionSetRow::Grids => "Akslar",
    };
    let action = |label: &'static str, name: &'static str, editable: bool| Choice::Action {
        label,
        icon: "dimension",
        message: (!editable || !locked)
            .then_some(Message::Properties(Event::DimensionSet(slot, name))),
    };
    let select = |text: &str, items| {
        Some(Editor::Select {
            text: text.into(),
            swatch: None,
            icon: None,
            items,
        })
    };
    let counts = format!("{} ölçü", set.members.len());
    let mut out = vec![
        Row::text("Ölçü dizisi", &counts).editor(select(
            &counts,
            vec![
                action("Dizinin tamamını seç", "select", false),
                action("Bu sırayı seç", "row", false),
                Choice::Action {
                    label: match &set.span {
                        Some(span) if span.space => "Kaynak mahali göster",
                        Some(_) => "Kaynak duvarları göster",
                        None => "Kaynak duvarı göster",
                    },
                    icon: "wall",
                    message: (set.broken != Some(DimensionSetBreak::HostGone))
                        .then_some(Message::Properties(Event::DimensionSet(slot, "source"))),
                },
                Choice::Separator,
                action("Sıraların yerini sıfırla", "resetRows", true),
                action("Bu sıranın yazı yerlerini sıfırla", "rowText", true),
                action("Gizlenen ölçüleri geri getir", "restoreHidden", true),
                Choice::Separator,
                action("Diziyi bağımsız ölçülere dönüştür", "detach", true),
                action("Dizinin tamamını sil", "delete", true),
            ],
        )),
        Row::text("Sıra", row_name),
    ];
    let pick = |label: &str, name: &'static str, chosen: bool| Choice::Pick {
        label: label.into(),
        swatch: None,
        icon: None,
        wide: false,
        chosen,
        enabled: !locked,
        message: Message::Properties(Event::DimensionSet(slot, name)),
    };
    let value_pick = |label: String, field, value: String, chosen| Choice::Pick {
        label,
        swatch: None,
        icon: None,
        wide: false,
        chosen,
        enabled: !locked,
        message: Message::Properties(Event::DimensionSetValue(slot, field, value)),
    };
    let layer = model.layers().path(&set.layer_id);
    out.push(
        Row::text("Dizi katmanı", &layer).editor(select(
            &layer,
            model
                .layers()
                .leaves()
                .into_iter()
                .filter(|layer| {
                    layer.service.is_none()
                        && !model.layers().is_locked(&layer.id)
                        && layer
                            .geometry
                            .is_none_or(|geometry| geometry.admits("dimension"))
                })
                .map(|layer| {
                    value_pick(
                        model.layers().path(&layer.id),
                        "layerId",
                        layer.id.clone(),
                        layer.id == set.layer_id,
                    )
                })
                .collect(),
        )),
    );
    let chosen_style = model
        .settings()
        .dimension_styles
        .iter()
        .find(|style| Some(&style.id) == set.look.dim_style.as_ref())
        .map_or(kentos_commands::STANDARD_STYLE, |style| style.name.as_str());
    let styles = std::iter::once(value_pick(
        kentos_commands::STANDARD_STYLE.into(),
        "dimensionStyle",
        kentos_commands::STANDARD_STYLE.into(),
        chosen_style == kentos_commands::STANDARD_STYLE,
    ))
    .chain(model.settings().dimension_styles.iter().map(|style| {
        value_pick(
            style.name.clone(),
            "dimensionStyle",
            style.name.clone(),
            chosen_style == style.name,
        )
    }))
    .collect();
    out.push(Row::text("Dizi stili", chosen_style).editor(select(chosen_style, styles)));
    if let Some(span) = &set.span {
        // A façade's or a space's source, direction, rows and lines (docs/adr/0729 §7).
        let source = if span.space {
            "Mahal".to_owned()
        } else {
            format!("Cephe · {} duvar", 1 + span.walls.len())
        };
        out.insert(1, Row::text("Kaynak", &source));
        let angle = span.direction.y.atan2(span.direction.x).to_degrees();
        out.push(Row::text("Doğrultu", format!("{:.2}°", (angle + 360.0) % 360.0)));
        let names = [
            (DimensionSetRow::Openings, "Açıklıklar", "toggle:openings"),
            (DimensionSetRow::Walls, "Duvarlar", "toggle:walls"),
            (DimensionSetRow::Grids, "Akslar", "toggle:grids"),
            (DimensionSetRow::Overall, "Toplam boy", "toggle:overall"),
        ];
        let label = names
            .iter()
            .filter(|(r, ..)| set.rows.contains(r))
            .map(|(_, n, _)| *n)
            .collect::<Vec<_>>()
            .join(", ");
        out.push(Row::text("Ölçü sıraları", &label).editor(select(
            &label,
            names
                .iter()
                .map(|(r, n, name)| pick(&format!("{n} sırası"), name, set.rows.contains(r)))
                .collect(),
        )));
        let centers = set.openings == DimensionSetOpenings::Centers;
        let label = if centers { "Açıklık merkezleri" } else { "Açıklık uçları" };
        out.push(Row::text("Açıklıkları ölç", label).editor(select(
            label,
            vec![
                pick("Açıklık uçları", "ends", !centers),
                pick("Açıklık merkezleri", "centers", centers),
            ],
        )));
        if !span.space {
            let core = span.reference == DimensionSetReference::Core;
            let label = if core { "Çekirdek" } else { "Dış yüzler" };
            out.push(Row::text("Ölçülen çizgi", label).editor(select(
                label,
                vec![pick("Dış yüzler", "faces", !core), pick("Çekirdek", "core", core)],
            )));
        }
        out.push(
            Row::figure("Sıra aralığı", kentos_interaction::fixed(set.gap_mm, 2))
                .unit("mm · kağıtta")
                .editor((!locked).then_some(Editor::Number(Field::DimensionSetGap(slot)))),
        );
        out.push(broken_note(set.broken).unwrap_or_else(|| {
            Row::note(if span.space {
                "Mahal ve kapıları değişince bu dizi güncellenir. Sıra uzaklığı bu sırayı birlikte taşır."
            } else {
                "Cephenin duvarları, açıklıkları, saplanan bölmeleri ve akslar değişince bu dizi güncellenir; duvar bölününce parçası diziye katılır."
            })
        }));
        return out;
    }
    let both = set.rows.contains(&DimensionSetRow::Openings)
        && set.rows.contains(&DimensionSetRow::Overall);
    let rows_label = if both {
        "Açıklıklar ve toplam boy"
    } else if set.rows.contains(&DimensionSetRow::Openings) {
        "Yalnız açıklıklar"
    } else {
        "Yalnız toplam boy"
    };
    out.push(Row::text("Ölçü sıraları", rows_label).editor(select(
        rows_label,
        vec![
            pick("Açıklıklar ve toplam boy", "both", both),
            pick(
                "Yalnız açıklıklar",
                "openings",
                !both && set.rows.contains(&DimensionSetRow::Openings),
            ),
            pick(
                "Yalnız toplam boy",
                "overall",
                !both && set.rows.contains(&DimensionSetRow::Overall),
            ),
        ],
    )));
    let centers = set.openings == DimensionSetOpenings::Centers;
    let label = if centers {
        "Açıklık merkezleri"
    } else {
        "Açıklık uçları"
    };
    out.push(Row::text("Açıklıkları ölç", label).editor(select(
        label,
        vec![
            pick("Açıklık uçları", "ends", !centers),
            pick("Açıklık merkezleri", "centers", centers),
        ],
    )));
    let left = set.face == DimensionSetFace::Left;
    let label = if left {
        "Duvarın sol yüzü"
    } else {
        "Duvarın sağ yüzü"
    };
    out.push(Row::text("Ölçülen yüz", label).editor(select(
        label,
        vec![
            pick("Duvarın sol yüzü", "left", left),
            pick("Duvarın sağ yüzü", "right", !left),
        ],
    )));
    out.push(
        Row::figure("Sıra aralığı", kentos_interaction::fixed(set.gap_mm, 2))
            .unit("mm · kağıtta")
            .editor((!locked).then_some(Editor::Number(Field::DimensionSetGap(slot)))),
    );
    out.push(broken_note(set.broken).unwrap_or_else(|| {
        Row::note("Duvar ve açıklıklar değişince bu dizi güncellenir. Sıra uzaklığı bu sırayı birlikte taşır; yazı ve görünüş yalnız seçili ölçüye uygulanır.")
    }));
    out
}

/// Why a recipe no longer follows its source, as a warning row.
fn broken_note(broken: Option<DimensionSetBreak>) -> Option<Row> {
    let mut note = Row::note(match broken? {
        DimensionSetBreak::HostGone => {
            "Kaynak silindi. Ölçüler son konumlarını gösterir; kaynağı geri alın veya diziyi bağımsız ölçülere dönüştürün."
        }
        DimensionSetBreak::UnsupportedWall => {
            "Kaynak artık ölçülebilir değil (pahlı köşe, yay ya da doğrultuya eğik kenar). Ölçüler güncellenemiyor; kaynak biçimini düzeltin veya diziyi bağımsız ölçülere dönüştürün."
        }
        DimensionSetBreak::OpeningUnplaced => {
            "Bir açıklığın duvardaki yeri çözülemiyor. Açıklığı düzeltince ölçü dizisi yeniden güncellenir."
        }
        DimensionSetBreak::MixedSources => {
            "Cephenin duvarları artık farklı katlarda. Ölçüler güncellenemiyor; duvarlara aynı katı verin veya diziyi bağımsız ölçülere dönüştürün."
        }
    });
    note.warn = true;
    Some(note)
}

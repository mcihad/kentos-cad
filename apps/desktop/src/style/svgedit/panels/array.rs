//! The Dizi tab (the web's `svgArray.ts`), for pattern and symbol design:
//! rows and columns by step or gap, copies round a centre over a full turn
//! or an arc (turning with it or not), and a mirror copy across a vertical,
//! horizontal or slanted line. The copies show on the canvas as ghosts
//! before Uygula makes them.

use iced::widget::{Column, button, column, row};
use iced::{Center, Element, Fill};
use kentos_ui::label;
use kentos_ui::style as ui_style;

use super::{Opt, check, num, seg, spec, title};
use crate::app::Message;
use crate::style::fields;
use crate::style::svgedit::change;
use crate::style::svgedit::measure::fmt_num;
use crate::style::svgedit::pointer::Pick;
use crate::style::svgedit::state::{ArrayKind, At, SvgEditor};

/// Merkez: the selection's centre, the canvas's, or a point picked on it.
fn centre<'a>(at: At, point: Option<[f64; 2]>, pick: Pick) -> Element<'a, Message> {
    let picked = at == At::Point;
    let words = match (picked, point) {
        (true, Some(p)) => format!("Nokta: {}, {}", fmt_num(p[0], 3), fmt_num(p[1], 3)),
        _ => "Tuvalde göster…".to_owned(),
    };
    fields::labelled(
        "Merkez",
        row![
            iced::widget::container(seg(
                &[Opt("box", "Seçim"), Opt("canvas", "Tuval")],
                match at {
                    At::Box => Some("box"),
                    At::Canvas => Some("canvas"),
                    At::Point => None,
                },
                &[],
                move |v| {
                    change(move |ed| {
                        let at = if v == "box" { At::Box } else { At::Canvas };
                        if pick == Pick::Polar {
                            ed.ui.array.polar.at = at;
                        } else {
                            ed.ui.array.mirror.at = at;
                        }
                        ed.marker = None;
                        ed.touch();
                    })
                },
            ))
            .width(Fill),
            button(label::body(words))
                .padding([3, 10])
                .style(move |t: &iced::Theme, s| if picked {
                    ui_style::button::primary(t, s)
                } else {
                    ui_style::button::secondary(t, s)
                })
                .on_press(change(move |ed| ed.pick_point(pick))),
        ]
        .spacing(8)
        .align_y(Center),
        None,
    )
}

pub(super) fn tab<'a>(ed: &SvgEditor, count: usize) -> Element<'a, Message> {
    let a = &ed.ui.array;
    let mut body: Vec<Element<'a, Message>> = Vec::new();
    let any = spec(0.5, f64::NEG_INFINITY, f64::INFINITY);
    match a.kind {
        ArrayKind::Rect => {
            let r = &a.rect;
            body.push(fields::pair(
                num(
                    ed,
                    "rows",
                    "Satır",
                    r.rows,
                    None,
                    spec(1.0, 1.0, f64::INFINITY),
                    |ed, v| {
                        ed.ui.array.rect.rows = kentos_native_style::classify::js_round(v).max(1.0);
                    },
                ),
                num(
                    ed,
                    "cols",
                    "Sütun",
                    r.cols,
                    None,
                    spec(1.0, 1.0, f64::INFINITY),
                    |ed, v| {
                        ed.ui.array.rect.cols = kentos_native_style::classify::js_round(v).max(1.0);
                    },
                ),
            ));
            body.push(fields::labelled(
                "Aralık",
                seg(
                    &[Opt("gap", "Boşluk"), Opt("step", "Adım")],
                    Some(if r.gap { "gap" } else { "step" }),
                    &[
                        "Kopyaların kutuları arası",
                        "Bir kopyadan ötekine (merkezden merkeze)",
                    ],
                    |v| change(move |ed| ed.ui.array.rect.gap = v == "gap"),
                ),
                None,
            ));
            body.push(fields::pair(
                num(ed, "adx", "Yatay", r.dx, None, any, |ed, v| {
                    ed.ui.array.rect.dx = v
                }),
                num(ed, "ady", "Dikey", r.dy, None, any, |ed, v| {
                    ed.ui.array.rect.dy = v
                }),
            ));
        }
        ArrayKind::Polar => {
            let p = &a.polar;
            body.push(fields::pair(
                num(
                    ed,
                    "count",
                    "Adet",
                    p.count,
                    None,
                    spec(1.0, 1.0, f64::INFINITY),
                    |ed, v| {
                        ed.ui.array.polar.count =
                            kentos_native_style::classify::js_round(v).max(1.0);
                    },
                ),
                num(
                    ed,
                    "angle",
                    "Açı",
                    p.angle,
                    Some("°"),
                    spec(15.0, f64::NEG_INFINITY, f64::INFINITY),
                    |ed, v| {
                        ed.ui.array.polar.angle = v;
                    },
                ),
            ));
            body.push(centre(p.at, p.point, Pick::Polar));
            body.push(check(p.rotate, "Kopyalar da dönsün", |ed, v| {
                ed.ui.array.polar.rotate = v
            }));
            body.push(fields::labelled(
                "Yön",
                seg(
                    &[Opt("ccw", "↺"), Opt("cw", "↻")],
                    Some(if p.ccw { "ccw" } else { "cw" }),
                    &["Saat yönünün tersine", "Saat yönünde"],
                    |v| change(move |ed| ed.ui.array.polar.ccw = v == "ccw"),
                ),
                None,
            ));
            body.push(fields::hint(
                "360°: tam tur, eşit aralık. Daha az: ilk ve son kopya yayın uçlarında.",
            ));
        }
        ArrayKind::Mirror => {
            let m = &a.mirror;
            body.push(fields::labelled(
                "Eksen",
                seg(
                    &[Opt("v", "Dikey"), Opt("h", "Yatay"), Opt("angle", "Açılı")],
                    Some(m.axis),
                    &[],
                    |v| change(move |ed| ed.ui.array.mirror.axis = v),
                ),
                None,
            ));
            if m.axis == "angle" {
                body.push(num(
                    ed,
                    "mdeg",
                    "Eksen açısı",
                    m.deg,
                    Some("°"),
                    spec(15.0, f64::NEG_INFINITY, f64::INFINITY),
                    |ed, v| {
                        ed.ui.array.mirror.deg = v;
                    },
                ));
            }
            body.push(centre(m.at, m.point, Pick::Mirror));
        }
    }
    let apply = button(label::body("Uygula").style(ui_style::text::on_accent))
        .padding([4, 14])
        .style(ui_style::button::primary)
        .on_press_maybe((count > 0).then(|| change(|ed| ed.apply_array())));
    column![
        title("Dizi ve aynalı kopya".to_owned(), None),
        seg(
            &[
                Opt("rect", "Satır-sütun"),
                Opt("polar", "Dairesel"),
                Opt("mirror", "Aynalı")
            ],
            Some(match a.kind {
                ArrayKind::Rect => "rect",
                ArrayKind::Polar => "polar",
                ArrayKind::Mirror => "mirror",
            }),
            &[],
            |v| {
                change(move |ed| {
                    ed.ui.array.kind = match v {
                        "polar" => ArrayKind::Polar,
                        "mirror" => ArrayKind::Mirror,
                        _ => ArrayKind::Rect,
                    };
                    ed.marker = None;
                    ed.touch();
                })
            },
        ),
        Column::with_children(body).spacing(8),
        check(
            a.preview,
            "Önizleme (kopyalar soluk görünür)",
            |ed, v| ed.ui.array.preview = v
        ),
        row![
            apply,
            label::caption(if count > 0 {
                format!("{count} şekil çoğaltılır")
            } else {
                "Önce şekil seçin.".to_owned()
            })
            .style(ui_style::text::muted)
        ]
        .spacing(10)
        .align_y(Center),
    ]
    .spacing(10)
    .width(Fill)
    .into()
}

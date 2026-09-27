//! The Dönüştür tab (the web's `svgTransform.ts`, Inkscape's Transform
//! dialog): move (by an amount or to a place), scale in %, rotate by an
//! angle about a box point or a point clicked on the canvas, skew, and a
//! matrix; for the whole selection or each shape on its own. Values stay
//! while the editor is open, so the same step can be applied again.

use iced::widget::tooltip::Position;
use iced::widget::{Column, Row, button, column, container, row};
use iced::{Border, Center, Element, Fill, Theme};
use kentos_ui::label;
use kentos_ui::style as ui_style;
use kentos_ui::theme::{Tokens, typography};
use kentos_ui::widget::{Tip, tip};

use super::{ANY, Opt, check, num, seg, spec, title};
use crate::app::Message;
use crate::style::fields;
use crate::style::svgedit::change;
use crate::style::svgedit::measure::fmt_num;
use crate::style::svgedit::pointer::Pick;
use crate::style::svgedit::state::{SvgEditor, TransformKind};

pub const ANCHORS: [(&str, &str); 9] = [
    ("tl", "Sol üst"),
    ("t", "Üst orta"),
    ("tr", "Sağ üst"),
    ("l", "Sol orta"),
    ("c", "Merkez"),
    ("r", "Sağ orta"),
    ("bl", "Sol alt"),
    ("b", "Alt orta"),
    ("br", "Sağ alt"),
];

/// Nine buttons of the box's points; `value` none: none lit (a picked point).
pub fn anchor_picker<'a>(value: Option<&str>, on: impl Fn(&'static str) -> Message + 'a) -> Element<'a, Message> {
    let side = typography::scaled(16.0);
    let mut grid = Column::new().spacing(3);
    for r in ANCHORS.chunks(3) {
        let mut line = Row::new().spacing(3);
        for (a, name) in r {
            let lit = value == Some(*a);
            let face = container(iced::widget::space())
                .width(side)
                .height(side)
                .style(move |t: &Theme| {
                    let tk = Tokens::of(t);
                    container::Style {
                        background: Some(iced::Background::Color(if lit {
                            tk.accent
                        } else {
                            tk.field
                        })),
                        border: Border {
                            color: if lit { tk.accent } else { tk.border },
                            width: 1.0,
                            radius: 2.0.into(),
                        },
                        ..container::Style::default()
                    }
                });
            line = line.push(tip(
                button(face)
                    .padding(0)
                    .style(ui_style::button::ghost)
                    .on_press(on(a)),
                Tip::new((*name).to_owned()),
                Position::Top,
            ));
        }
        grid = grid.push(line);
    }
    grid.into()
}

const KINDS: [Opt; 5] = [
    Opt("move", "Taşı"),
    Opt("scale", "Ölçek"),
    Opt("rotate", "Döndür"),
    Opt("skew", "Eğ"),
    Opt("matrix", "Matris"),
];

fn kind_key(k: TransformKind) -> &'static str {
    match k {
        TransformKind::Move => "move",
        TransformKind::Scale => "scale",
        TransformKind::Rotate => "rotate",
        TransformKind::Skew => "skew",
        TransformKind::Matrix => "matrix",
    }
}

pub(super) fn tab<'a>(ed: &SvgEditor, count: usize) -> Element<'a, Message> {
    let t = &ed.ui.transform;
    let mut body: Vec<Element<'a, Message>> = Vec::new();
    match t.kind {
        TransformKind::Move => {
            body.push(seg(
                &[Opt("rel", "Kadar"), Opt("abs", "Konuma")],
                Some(if t.relative { "rel" } else { "abs" }),
                &["Bu kadar kaydır", "Kutunun sol üst köşesi bu noktaya"],
                |v| change(move |ed| ed.ui.transform.relative = v == "rel"),
            ));
            body.push(fields::pair(
                num(ed, "tx", if t.relative { "Yatay (X)" } else { "X" }, t.x, None, ANY, |ed, v| ed.ui.transform.x = v),
                num(ed, "ty", if t.relative { "Dikey (Y)" } else { "Y" }, t.y, None, ANY, |ed, v| ed.ui.transform.y = v),
            ));
        }
        TransformKind::Scale => {
            let pct = spec(10.0, f64::NEG_INFINITY, f64::INFINITY);
            body.push(fields::pair(
                num(ed, "sx", "Genişlik", t.sx, Some("%"), pct, |ed, v| {
                    ed.ui.transform.sx = v;
                    if ed.ui.transform.lock {
                        ed.ui.transform.sy = v;
                        ed.typed.remove("sy");
                    }
                }),
                num(ed, "sy", "Yükseklik", t.sy, Some("%"), pct, |ed, v| {
                    ed.ui.transform.sy = v;
                    if ed.ui.transform.lock {
                        ed.ui.transform.sx = v;
                        ed.typed.remove("sx");
                    }
                }),
            ));
            body.push(check(t.lock, "Oranı koru", |ed, v| ed.ui.transform.lock = v));
            body.push(fields::labelled(
                "Sabit nokta",
                anchor_picker(Some(t.anchor), |a| change(move |ed| ed.ui.transform.anchor = a)),
                None,
            ));
        }
        TransformKind::Rotate => {
            let picked = t.about.is_none();
            let pick_words = match (picked, t.point) {
                (true, Some(p)) => format!("Nokta: {}, {}", fmt_num(p[0], 3), fmt_num(p[1], 3)),
                _ => "Tuvalde göster…".to_owned(),
            };
            body.push(fields::pair(
                num(ed, "deg", "Açı", t.deg, Some("°"), spec(15.0, f64::NEG_INFINITY, f64::INFINITY), |ed, v| {
                    ed.ui.transform.deg = v;
                }),
                fields::labelled(
                    "Yön",
                    seg(
                        &[Opt("ccw", "↺"), Opt("cw", "↻")],
                        Some(if t.ccw { "ccw" } else { "cw" }),
                        &["Saat yönünün tersine", "Saat yönünde"],
                        |v| change(move |ed| ed.ui.transform.ccw = v == "ccw"),
                    ),
                    None,
                ),
            ));
            body.push(fields::labelled(
                "Merkez",
                row![
                    anchor_picker(t.about, |a| change(move |ed| {
                        ed.ui.transform.about = Some(a);
                        ed.marker = None;
                        ed.touch();
                    })),
                    button(label::body(pick_words))
                        .padding([3, 10])
                        .style(move |t: &iced::Theme, s| if picked { ui_style::button::primary(t, s) } else { ui_style::button::secondary(t, s) })
                        .on_press(change(|ed| ed.pick_point(Pick::Rotate))),
                ]
                .spacing(10)
                .align_y(Center),
                Some("Kutunun bir noktası ya da tuvalde tıklanan nokta"),
            ));
        }
        TransformKind::Skew => {
            let deg = spec(5.0, f64::NEG_INFINITY, f64::INFINITY);
            body.push(fields::pair(
                num(ed, "ax", "Yatay eğim", t.ax, Some("°"), deg, |ed, v| ed.ui.transform.ax = v),
                num(ed, "ay", "Dikey eğim", t.ay, Some("°"), deg, |ed, v| ed.ui.transform.ay = v),
            ));
            body.push(fields::labelled(
                "Sabit nokta",
                anchor_picker(Some(t.anchor), |a| change(move |ed| ed.ui.transform.anchor = a)),
                None,
            ));
        }
        TransformKind::Matrix => {
            let cell = |i: usize, name: &'static str| {
                num(ed, &format!("m{i}"), name, t.m[i], None, spec(0.1, f64::NEG_INFINITY, f64::INFINITY), move |ed, v| {
                    ed.ui.transform.m[i] = v;
                })
            };
            body.push(fields::pair(cell(0, "a"), cell(2, "c")));
            body.push(fields::pair(cell(1, "b"), cell(3, "d")));
            body.push(fields::pair(cell(4, "e"), cell(5, "f")));
            body.push(fields::hint(
                "x' = a·x + c·y + e, y' = b·x + d·y + f (SVG matrix(a b c d e f)).",
            ));
        }
    }
    let apply = button(label::body("Uygula").style(ui_style::text::on_accent))
        .padding([4, 14])
        .style(ui_style::button::primary)
        .on_press_maybe((count > 0).then(|| change(|ed| ed.apply_transform())));
    column![
        title("Dönüştür".to_owned(), None),
        seg(&KINDS, Some(kind_key(t.kind)), &[], |v| {
            change(move |ed| {
                ed.ui.transform.kind = match v {
                    "scale" => TransformKind::Scale,
                    "rotate" => TransformKind::Rotate,
                    "skew" => TransformKind::Skew,
                    "matrix" => TransformKind::Matrix,
                    _ => TransformKind::Move,
                };
                ed.marker = if v == "rotate" && ed.ui.transform.about.is_none() {
                    ed.ui.transform.point
                } else {
                    None
                };
                ed.touch();
            })
        }),
        Column::with_children(body).spacing(8),
        check(
            t.separately,
            if t.kind == TransformKind::Move {
                "Her birine ayrı (her şekil bir adım daha ileri)"
            } else {
                "Her birine ayrı (kendi kutusuna göre)"
            },
            |ed, v| ed.ui.transform.separately = v,
        ),
        row![
            apply,
            label::caption(if count > 0 {
                format!("{count} şekil")
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

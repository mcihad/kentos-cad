//! Özellikler with a selection (the web's `svgProps.ts` `selectionProps`):
//! the title (a shape's name or kind, how many, whether locked), Dolgu and
//! Çizgi as the symbol's colour, the second colour, a fixed colour or none,
//! the stroke's width and the transparency, the stroke's look and the box
//! (`style.rs`), a single shape's geometry, the path operations with their
//! distance and tolerance, the order and the groups, and the selection
//! helpers; the polygon tool's settings and the node tool's box on top.

use iced::widget::{button, column, container, row};
use iced::{Center, Color, Element, Fill, Theme};
use kentos_svg_core::shape::Obj;
use kentos_ui::icon::{Icon, icon};
use kentos_ui::label;
use kentos_ui::style as ui_style;
use kentos_ui::theme::{Tokens, typography};
use kentos_ui::widget::color::{ColorPicker, parse_hex, to_hex};

use super::{Opt, acts, act, check, group, num, small, spec, title};
use crate::app::Message;
use crate::style::fields;
use crate::style::svgedit::actions::{Action, PathOp, Restack, Same};
use crate::style::svgedit::doc::{id_of, kind_name};
use crate::style::svgedit::icons::svg_icon;
use crate::style::svgedit::state::SvgEditor;
use crate::style::svgedit::{ToolId, change};

const PAINTS: [(&str, &str); 4] = [
    ("none", "Yok"),
    ("fill", "Sembol rengi"),
    ("stroke", "İkinci renk"),
    ("fixed", "Sabit renk"),
];

/// The same value in every chosen shape, or none.
fn same<T: PartialEq + Clone>(list: &[T]) -> Option<T> {
    let first = list.first()?;
    list.iter().all(|v| v == first).then(|| first.clone())
}

/// A colour swatch that opens the colour picker.
pub fn swatch<'a>(value: Color, on: impl Fn(String) -> Message + 'a) -> Element<'a, Message> {
    let side = typography::scaled(26.0);
    let face = container(iced::widget::space())
        .width(typography::scaled(44.0))
        .height(side)
        .style(move |t: &Theme| container::Style {
            background: Some(iced::Background::Color(value)),
            border: iced::Border {
                color: Tokens::of(t).border,
                width: 1.0,
                radius: 3.0.into(),
            },
            ..container::Style::default()
        });
    ColorPicker::new(value, move |c| on(to_hex(c))).anchor(face).into()
}

/// Dolgu or Çizgi: none, the symbol's colour, the second colour or a fixed colour; mixed says so.
fn paint_field<'a>(label_text: &str, key: &'static str, value: Option<String>) -> Element<'a, Message> {
    let kind = match value.as_deref() {
        None => "none",
        Some(v @ ("none" | "fill" | "stroke")) => v,
        Some(_) => "fixed",
    };
    let fixed = value
        .as_deref()
        .filter(|v| kind == "fixed" && !v.is_empty())
        .unwrap_or("#E0457B")
        .to_owned();
    let fixed_for_select = fixed.clone();
    let select = fields::select(&PAINTS, kind, move |v| {
        let paint = if v == "fixed" {
            fixed_for_select.clone()
        } else {
            v.to_owned()
        };
        change(move |ed| {
            let p = paint.clone();
            ed.set_chosen(key, move |s| s.set_text(key, &p));
        })
    });
    let mut r = row![container(select).width(Fill)].spacing(6).align_y(Center);
    if kind == "fixed" {
        let shown = parse_hex(&fixed).unwrap_or(Color::BLACK);
        r = r.push(swatch(shown, move |hex| {
            change(move |ed| {
                let p = hex.clone();
                ed.set_chosen(key, move |s| s.set_text(key, &p));
            })
        }));
    }
    fields::labelled(
        label_text,
        r,
        value.is_none().then_some("Seçilenlerde farklı"),
    )
}

/// The polygon tool's settings.
fn polygon_options<'a>(ed: &SvgEditor) -> Element<'a, Message> {
    tool_box(
        "Çokgen aracı",
        vec![fields::pair(
            num(ed, "sides", "Kenar sayısı", ed.options.sides, None, spec(1.0, 3.0, 24.0), |ed, v| {
                ed.options.sides = kentos_native_style::classify::js_round(v).max(3.0);
                ed.touch();
            }),
            fields::labelled(
                "Biçim",
                check(ed.options.star, "Yıldız", |ed, v| {
                    ed.options.star = v;
                    ed.touch();
                }),
                None,
            ),
        )],
    )
}

/// A tool's box on top of the properties (`svgp__group--tool`).
pub fn tool_box<'a>(title_text: &str, rows: Vec<Element<'a, Message>>) -> Element<'a, Message> {
    let mut c = column![fields::group_title(title_text)].spacing(8);
    for r in rows {
        c = c.push(r);
    }
    container(c)
        .padding([8, 10])
        .width(Fill)
        .style(|t: &Theme| {
            let tk = Tokens::of(t);
            container::Style {
                background: Some(iced::Background::Color(tk.accent.scale_alpha(0.10))),
                border: iced::Border {
                    color: tk.accent.scale_alpha(0.45),
                    width: 1.0,
                    radius: 4.0.into(),
                },
                ..container::Style::default()
            }
        })
        .into()
}

pub(super) fn tab<'a>(ed: &'a SvgEditor) -> Element<'a, Message> {
    let sel = ed.chosen();
    let mut c = column![].spacing(10).width(Fill);
    if ed.tool == ToolId::Polygon {
        c = c.push(polygon_options(ed));
    }
    if ed.node_edit.is_some() && ed.node_shape().is_some() {
        c = c.push(super::nodes::node_box(ed));
    }
    if sel.is_empty() {
        c.push(super::canvas::canvas_props(ed)).into()
    } else {
        c.push(selection_props(ed, &sel)).into()
    }
}

fn selection_props<'a>(ed: &'a SvgEditor, sel: &[Obj]) -> Element<'a, Message> {
    let locked = sel.iter().any(|s| s.is("locked"));
    let name = if sel.len() == 1 {
        sel[0]
            .text("name")
            .map_or_else(|| kind_name(sel[0].kind()).to_owned(), str::to_owned)
    } else {
        format!("{} şekil", sel.len())
    };
    let lock_note: Option<Element<'a, Message>> = locked.then(|| {
        row![
            icon(Icon::Lock).size(13.0),
            label::caption("kilitli").style(ui_style::text::muted)
        ]
        .spacing(4)
        .align_y(Center)
        .into()
    });
    let fills: Vec<String> = sel
        .iter()
        .map(|s| s.text("fill").unwrap_or("none").to_owned())
        .collect();
    let strokes: Vec<String> = sel
        .iter()
        .map(|s| s.text("stroke").unwrap_or("none").to_owned())
        .collect();
    let widths: Vec<u64> = sel.iter().map(|s| s.num("strokeWidth").to_bits()).collect();
    let width = same(&widths).map_or(sel[0].num("strokeWidth"), f64::from_bits);
    let opacities: Vec<u64> = sel
        .iter()
        .map(|s| s.opt_num("opacity").unwrap_or(1.0).to_bits())
        .collect();
    let opacity = same(&opacities).map_or(1.0, f64::from_bits);
    let transparency = kentos_native_style::classify::js_round((1.0 - opacity) * 100.0);
    let mut parts: Vec<Element<'a, Message>> = vec![
        title(name, lock_note),
        paint_field("Dolgu", "fill", same(&fills)),
        paint_field("Çizgi", "stroke", same(&strokes)),
        fields::pair(
            num(ed, "sw", "Çizgi kalınlığı", width, None, spec(0.5, 0.0, f64::INFINITY), |ed, v| {
                ed.set_chosen("sw", move |s| s.set_num("strokeWidth", v.max(0.0)));
            }),
            num(ed, "op", "Saydamlık", transparency, Some("%"), spec(5.0, 0.0, 100.0), |ed, v| {
                ed.set_chosen("op", move |s| {
                    if v > 0.0 {
                        s.set_num("opacity", 1.0 - v.min(100.0) / 100.0);
                    } else {
                        s.set_undefined("opacity");
                    }
                });
            }),
        ),
        super::style::stroke_style(ed, sel),
        super::style::box_fields(ed, sel),
    ];
    if sel.len() == 1 {
        parts.push(geometry(ed, &sel[0]));
    }
    parts.push(path_group(ed, sel.len()));
    parts.push(order_group());
    parts.push(pick_group());
    let mut buttons: Vec<Element<'a, Message>> = Vec::new();
    if sel.len() == 1 && sel[0].kind() == "path" && ed.node_edit.is_none() {
        let id = id_of(&sel[0]).to_owned();
        buttons.push(
            button(
                row![icon(crate::icons::from_web(Some("vertex"))).size(14.0), label::body("Düğümleri düzenle")]
                    .spacing(6)
                    .align_y(Center),
            )
            .padding([3, 10])
            .style(ui_style::button::secondary)
            .on_press(change(move |ed| ed.edit_nodes(Some(id.clone()))))
            .into(),
        );
    }
    if sel.len() == 1 && matches!(sel[0].kind(), "rect" | "ellipse") {
        buttons.push(iced::widget::tooltip(
            button(
                row![icon(Icon::Svg(svg_icon("toPath"))).size(14.0), label::body("Yola çevir")]
                    .spacing(6)
                    .align_y(Center),
            )
            .padding([3, 10])
            .style(ui_style::button::secondary)
            .on_press(change(|ed| ed.path_op(PathOp::ToPath))),
            container(label::caption("Düğümleri düzenlemek ya da köşe yuvarlamak için (Ctrl+Shift+C)"))
                .padding([4, 8])
                .style(ui_style::container::popover),
            iced::widget::tooltip::Position::Top,
        )
        .into());
    }
    if !buttons.is_empty() {
        parts.push(iced::widget::Row::with_children(buttons).spacing(6).into());
    }
    iced::widget::Column::with_children(parts)
        .spacing(10)
        .width(Fill)
        .into()
}

/// The path operations as buttons, with the distance and tolerance they use.
fn path_group<'a>(ed: &SvgEditor, count: usize) -> Element<'a, Message> {
    let p = |op: PathOp, name: &'static str, tip_text: &str, min: usize| {
        act(
            kentos_ui::icon::Icon::Svg(svg_icon(name)),
            tip_text,
            (count >= min).then(|| change(move |ed| ed.path_op(op))),
        )
    };
    let joins = [
        Opt("round", "Yuvarlak"),
        Opt("miter", "Sivri"),
        Opt("bevel", "Pah"),
    ];
    let join_now = match ed.ui.offset_join {
        kentos_svg_core::stroke::Join::Round => "round",
        kentos_svg_core::stroke::Join::Miter => "miter",
        kentos_svg_core::stroke::Join::Bevel => "bevel",
    };
    let join_choices: Vec<(&'static str, &'static str)> = joins.iter().map(|o| (o.0, o.1)).collect();
    group(
        "Yol",
        vec![
            acts(vec![
                p(PathOp::Union, "pathUnion", "Birleşim (Ctrl++)", 2),
                p(PathOp::Difference, "pathDifference", "Fark: alttakinden üsttekiler çıkar (Ctrl+-)", 2),
                p(PathOp::Intersection, "pathIntersection", "Kesişim (Ctrl+*)", 2),
                p(PathOp::Exclusion, "pathExclusion", "Dışlama: ortak yerler boşalır (Ctrl+^)", 2),
                p(PathOp::Division, "pathDivision", "Bölme: alttaki üsttekilerin çizgileriyle bölünür (Ctrl+/)", 2),
                p(PathOp::Cut, "pathCut", "Yolu kes: alttakinin çizgisi kesişimlerde kesilir (Ctrl+Alt+/)", 2),
            ]),
            acts(vec![
                p(PathOp::Combine, "pathCombine", "Tek yolda topla (Ctrl+K)", 2),
                p(PathOp::BreakApart, "pathBreak", "Parçalara ayır (Ctrl+Shift+K)", 1),
                p(PathOp::Split, "pathSplit", "Parçalara ayır, delikler yerinde kalsın", 1),
                p(PathOp::ToPath, "toPath", "Nesneyi yola çevir (Ctrl+Shift+C)", 1),
                p(PathOp::StrokeToPath, "strokeToPath", "Çizgiyi yola çevir (Ctrl+Alt+C)", 1),
                p(PathOp::Reverse, "reverse", "Yönü çevir", 1),
                p(PathOp::Close, "closePath", "Yolu kapat", 1),
                p(PathOp::Open, "openPath", "Yolu aç (kapanış parçası kalır)", 1),
            ]),
            row![
                p(PathOp::Inset, "inset", "İçe küçült (Ctrl+()", 1),
                p(PathOp::Outset, "outset", "Dışa büyüt (Ctrl+))", 1),
                container(super::num_bare(ed, "offset", ed.ui.offset, None, spec(0.5, 0.0, f64::INFINITY), |ed, v| {
                    ed.ui.offset = v.max(0.0);
                }))
                .width(Fill),
                container(fields::select(&join_choices, join_now, |v| {
                    change(move |ed| {
                        ed.ui.offset_join = match v {
                            "miter" => kentos_svg_core::stroke::Join::Miter,
                            "bevel" => kentos_svg_core::stroke::Join::Bevel,
                            _ => kentos_svg_core::stroke::Join::Round,
                        };
                    })
                }))
                .width(Fill),
            ]
            .spacing(4)
            .align_y(Center)
            .into(),
            row![
                p(PathOp::Simplify, "simplify", "Sadeleştir (Ctrl+L)", 1),
                container(super::num_bare(
                    ed,
                    "simplify",
                    ed.ui.simplify,
                    Some("%"),
                    spec(0.1, 0.001, f64::INFINITY),
                    |ed, v| ed.ui.simplify = v.max(0.001),
                ))
                .width(Fill),
            ]
            .spacing(4)
            .align_y(Center)
            .into(),
            fields::hint("Sonuç alttaki şeklin boyasını alır; eğriler eğri kalır. Mesafe çizim biriminde, tolerans seçimin boyuna göre %."),
        ],
    )
}

fn order_group<'a>() -> Element<'a, Message> {
    let r = |op: Restack, name: &'static str, tip_text: &str| {
        act(Icon::Svg(svg_icon(name)), tip_text, Some(change(move |ed| ed.restack(op))))
    };
    let a = |name: Action, glyph: Icon, tip_text: &str| act(glyph, tip_text, Some(change(move |ed| ed.action(name))));
    group(
        "Düzen",
        vec![
            acts(vec![
                r(Restack::Top, "toTop", "En öne (Home)"),
                r(Restack::Raise, "raise", "Bir öne (Page Up)"),
                r(Restack::Lower, "lower", "Bir arkaya (Page Down)"),
                r(Restack::Bottom, "toBottom", "En arkaya (End)"),
                a(Action::FlipH, Icon::Svg(svg_icon("flipH")), "Yatay çevir (H)"),
                a(Action::FlipV, Icon::Svg(svg_icon("flipV")), "Dikey çevir (Shift+H)"),
                a(Action::Rot90, crate::icons::from_web(Some("rotate")), "90° döndür"),
            ]),
            row![
                small("Grupla", Some(change(|ed| ed.action(Action::Group)))),
                small("Çöz", Some(change(|ed| ed.action(Action::Ungroup)))),
                act(crate::icons::from_web(Some("copy")), "Çoğalt (Ctrl+D)", Some(change(|ed| ed.action(Action::Duplicate)))),
                act(crate::icons::from_web(Some("trash")), "Sil (Delete)", Some(change(|ed| ed.action(Action::Delete)))),
            ]
            .spacing(4)
            .align_y(Center)
            .into(),
        ],
    )
}

fn pick_group<'a>() -> Element<'a, Message> {
    let s = |what: Same, tip_text: &str| {
        act(Icon::Svg(svg_icon("selectSame")), tip_text, Some(change(move |ed| ed.select_same(what))))
    };
    group(
        "Seç",
        vec![acts(vec![
            s(Same::Fill, "Aynı dolguyu seç"),
            s(Same::Stroke, "Aynı çizgiyi seç"),
            s(Same::Both, "Aynı dolgu ve çizgiyi seç"),
            act(
                Icon::Svg(svg_icon("selectInvert")),
                "Seçimi ters çevir (!)",
                Some(change(|ed| ed.invert_selection())),
            ),
        ])],
    )
}

/// A single shape's geometry: corner radius and turn, centre and radii, a text's words and type, a path's parts.
fn geometry<'a>(ed: &SvgEditor, s: &Obj) -> Element<'a, Message> {
    let id = id_of(s).to_owned();
    let n = |key: &'static str, label_text: &str, value: f64, unit: Option<&str>, min: f64, apply: fn(&mut Obj, f64)| {
        let id = id.clone();
        num(ed, key, label_text, value, unit, spec(1.0, min, f64::INFINITY), move |ed, v| {
            let id = id.clone();
            ed.set_shape(key, &id, move |s| apply(s, v));
        })
    };
    let turn = |s: &mut Obj, v: f64| {
        if v == 0.0 {
            s.set_undefined("rotate");
        } else {
            s.set_num("rotate", v);
        }
    };
    let mut parts: Vec<Element<'a, Message>> = Vec::new();
    match s.kind() {
        "rect" => parts.push(fields::pair(
            n("r", "Köşe yarıçapı", s.opt_num("r").unwrap_or(0.0), None, 0.0, |s, v| {
                if v > 0.0 {
                    s.set_num("r", v);
                } else {
                    s.set_undefined("r");
                }
            }),
            n("rot", "Döndürme", s.opt_num("rotate").unwrap_or(0.0), Some("°"), f64::NEG_INFINITY, turn),
        )),
        "ellipse" => {
            parts.push(fields::pair(
                n("cx", "Merkez X", s.num("cx"), None, f64::NEG_INFINITY, |s, v| s.set_num("cx", v)),
                n("cy", "Merkez Y", s.num("cy"), None, f64::NEG_INFINITY, |s, v| s.set_num("cy", v)),
            ));
            parts.push(fields::pair(
                n("rx", "Yarıçap X", s.num("rx"), None, 0.01, |s, v| s.set_num("rx", v.max(0.01))),
                n("ry", "Yarıçap Y", s.num("ry"), None, 0.01, |s, v| s.set_num("ry", v.max(0.01))),
            ));
            parts.push(n("rot", "Döndürme", s.opt_num("rotate").unwrap_or(0.0), Some("°"), f64::NEG_INFINITY, turn));
        }
        "text" => {
            let tid = id.clone();
            let words = ed.typed.get("text").cloned().unwrap_or_else(|| s.text("text").unwrap_or("").to_owned());
            parts.push(fields::labelled(
                "Metin",
                fields::text(super::field_id("text"), &words, "", false, move |t| {
                    let tid = tid.clone();
                    change(move |ed| {
                        ed.typed.insert("text".to_owned(), t.clone());
                        let t = t.clone();
                        ed.set_shape("text", &tid, move |s| s.set_text("text", &t));
                    })
                }),
                None,
            ));
            parts.push(fields::pair(
                n("x", "X", s.num("x"), None, f64::NEG_INFINITY, |s, v| s.set_num("x", v)),
                n("y", "Y (taban çizgisi)", s.num("y"), None, f64::NEG_INFINITY, |s, v| s.set_num("y", v)),
            ));
            parts.push(fields::pair(
                n("size", "Boyut", s.num("size"), None, 0.1, |s, v| s.set_num("size", v.max(0.1))),
                n("rot", "Döndürme", s.opt_num("rotate").unwrap_or(0.0), Some("°"), f64::NEG_INFINITY, turn),
            ));
            let fid = id.clone();
            let wid = id.clone();
            let aid = id.clone();
            parts.push(fields::pair(
                fields::labelled(
                    "Yazı tipi",
                    fields::select(&[("sans", "Arial"), ("serif", "Times")], s.text("font").unwrap_or("sans"), move |v| {
                        let fid = fid.clone();
                        change(move |ed| ed.set_shape("font", &fid, move |s| s.set_text("font", v)))
                    }),
                    None,
                ),
                fields::labelled(
                    "Kalınlık",
                    fields::select(
                        &[("400", "Normal"), ("700", "Kalın"), ("900", "Siyah")],
                        &kentos_expression::js::number::to_string(s.num("weight")),
                        move |v| {
                            let wid = wid.clone();
                            let w: f64 = v.parse().unwrap_or(400.0);
                            change(move |ed| ed.set_shape("weight", &wid, move |s| s.set_num("weight", w)))
                        },
                    ),
                    None,
                ),
            ));
            parts.push(fields::labelled(
                "Hizalama",
                fields::select(
                    &[("start", "Soldan"), ("middle", "Ortadan"), ("end", "Sağdan")],
                    s.text("anchor").unwrap_or("start"),
                    move |v| {
                        let aid = aid.clone();
                        change(move |ed| ed.set_shape("anchor", &aid, move |s| s.set_text("anchor", v)))
                    },
                ),
                None,
            ));
        }
        "path" => {
            let subs = s.subs().unwrap_or_default();
            let nodes: usize = subs.iter().map(|sp| sp.nodes.len()).sum();
            parts.push(fields::hint(&format!(
                "{} parça, {nodes} düğüm. Çift tık ya da “Düğümleri düzenle” ile düğümler sürüklenir.",
                subs.len()
            )));
            if subs.len() == 1 {
                let closed = subs[0].closed;
                let pid = id.clone();
                parts.push(fields::labelled(
                    "Uçlar",
                    check(closed, "Kapalı şekil", move |ed, v| {
                        let pid = pid.clone();
                        ed.set_shape("closed", &pid, move |s| {
                            if let Ok(mut subs) = s.subs() {
                                if let Some(sp) = subs.first_mut() {
                                    sp.closed = v;
                                }
                                s.set_subs(&subs);
                            }
                        });
                    }),
                    None,
                ));
            }
        }
        _ => {}
    }
    group("Geometri", parts)
}

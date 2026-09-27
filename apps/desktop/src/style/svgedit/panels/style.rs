//! The stroke's look and the selection's box (the web's `svgStyleProps.ts`):
//! dash (presets scaled by the stroke width, or typed), line ends, corners
//! and fill rule; X, Y, width and height of the selection (the proportions
//! kept on request) and a turn by a typed angle.

use iced::widget::{container, row};
use iced::{Center, Element, Fill};
use kentos_geometry_core::api::json::Json;
use kentos_svg_core::model::shapes_box;
use kentos_svg_core::shape::Obj;
use kentos_ui::icon::Icon;

use super::{Opt, act, act_sized, group, num, seg, spec};
use crate::app::Message;
use crate::style::fields;
use crate::style::svgedit::change;
use crate::style::svgedit::measure::fmt_num;
use crate::style::svgedit::state::SvgEditor;

/// Dash presets in stroke widths (`DASHES`).
const DASHES: [(&str, &str, &[f64]); 5] = [
    ("solid", "Sürekli", &[]),
    ("dash", "Kesikli", &[3.0, 2.0]),
    ("long", "Uzun kesik", &[6.0, 3.0]),
    ("dot", "Noktalı", &[0.01, 2.0]),
    ("dashdot", "Kesik-nokta", &[5.0, 2.0, 0.01, 2.0]),
];

fn same<T: PartialEq + Clone>(list: &[T]) -> Option<T> {
    let first = list.first()?;
    list.iter().all(|v| v == first).then(|| first.clone())
}

/// A shape's dash as the field writes it: `4 1.5`.
fn dash_text(s: &Obj) -> String {
    match s.get("dash") {
        Json::Arr(d) => d
            .iter()
            .map(|v| match v {
                Json::Num(x) => kentos_expression::js::number::to_string(*x),
                _ => String::new(),
            })
            .collect::<Vec<_>>()
            .join(" "),
        _ => String::new(),
    }
}

/// The dash a typed text gives: numbers apart by spaces or commas, none for blank; None when it does not read.
fn read_dash(text: &str) -> Option<Option<Vec<f64>>> {
    let t = kentos_processing::text::js_trim(text);
    if t.is_empty() {
        return Some(None);
    }
    let parts: Vec<f64> = t
        .split(|c: char| c == ',' || c.is_whitespace())
        .filter(|p| !p.is_empty())
        .map(|p| kentos_native_style::classify::js_number(p))
        .collect();
    (!parts.is_empty() && parts.iter().all(|v| v.is_finite() && *v >= 0.0)).then_some(Some(parts))
}

pub(super) fn stroke_style<'a>(ed: &SvgEditor, sel: &[Obj]) -> Element<'a, Message> {
    let widths: Vec<u64> = sel.iter().map(|s| s.num("strokeWidth").to_bits()).collect();
    let width = same(&widths).map_or(sel[0].num("strokeWidth"), f64::from_bits);
    let dashes: Vec<String> = sel.iter().map(dash_text).collect();
    let dash = same(&dashes);
    let preset = match dash.as_deref() {
        Some("") => "solid",
        Some(d) => DASHES
            .iter()
            .find(|(_, _, k)| {
                !k.is_empty() && k.iter().map(|x| fmt_num(x * width, 3)).collect::<Vec<_>>().join(" ") == d
            })
            .map_or("custom", |(v, _, _)| v),
        None => "custom",
    };
    let mut choices: Vec<(&'static str, &'static str)> = DASHES.iter().map(|(v, l, _)| (*v, *l)).collect();
    choices.push(("custom", "Özel"));
    let presets = fields::select(&choices, preset, |v| {
        change(move |ed| {
            let Some((_, _, k)) = DASHES.iter().find(|(x, _, _)| *x == v) else {
                return;
            };
            ed.typed.remove("dash");
            // Presets follow the stroke width, as a pen's dashes do; dots need round ends.
            ed.set_chosen("dash", move |s| {
                if k.is_empty() {
                    s.set_undefined("dash");
                } else {
                    let w = s.num("strokeWidth");
                    s.set(
                        "dash",
                        Json::Arr(
                            k.iter()
                                .map(|x| {
                                    Json::Num(kentos_native_style::classify::js_round(x * w * 1000.0) / 1000.0)
                                })
                                .collect(),
                        ),
                    );
                }
                if v == "dot" || v == "dashdot" {
                    s.set_text("cap", "round");
                }
            });
        })
    });
    let typed = ed.typed.get("dash").cloned().unwrap_or_else(|| dash.clone().unwrap_or_default());
    let invalid = read_dash(&typed).is_none();
    let typed_field = fields::input(Some(super::field_id("dash")), "sürekli (ör. 4 1.5)", &typed, false, invalid)
        .on_input(|t| {
            change(move |ed| {
                ed.typed.insert("dash".to_owned(), t.clone());
                if let Some(d) = read_dash(&t) {
                    ed.set_chosen("dash", move |s| match &d {
                        Some(d) => s.set("dash", Json::Arr(d.iter().map(|x| Json::Num(*x)).collect())),
                        None => s.set_undefined("dash"),
                    });
                }
            })
        })
        .on_submit(crate::style::svgedit::ev(crate::style::svgedit::Event::Settle("dash".to_owned())));
    let is_path = |s: &Obj| s.kind() == "path";
    let caps: Vec<String> = sel
        .iter()
        .map(|s| s.text("cap").unwrap_or(if is_path(s) { "round" } else { "butt" }).to_owned())
        .collect();
    let joins: Vec<String> = sel
        .iter()
        .map(|s| s.text("join").unwrap_or(if is_path(s) { "round" } else { "miter" }).to_owned())
        .collect();
    let rules: Vec<String> = sel
        .iter()
        .map(|s| {
            s.text("fillRule")
                .unwrap_or(if is_path(s) { "evenodd" } else { "nonzero" })
                .to_owned()
        })
        .collect();
    let set = |key: &'static str, field: &'static str| {
        move |v: &'static str| change(move |ed| ed.set_chosen(key, move |s| s.set_text(field, v)))
    };
    group(
        "Çizgi biçimi",
        vec![
            fields::labelled(
                "Kesik",
                row![container(presets).width(Fill), container(typed_field).width(Fill)]
                    .spacing(6)
                    .align_y(Center),
                Some("Boyları çizim biriminde: çizgi, boşluk … (hazırlar kalınlığa göre)"),
            ),
            fields::labelled(
                "Uçlar",
                seg(
                    &[Opt("butt", "Düz"), Opt("round", "Yuvarlak"), Opt("square", "Kare")],
                    same(&caps).as_deref(),
                    &["Uçta biter", "Yarım daire", "Yarım kalınlık uzar"],
                    set("cap", "cap"),
                ),
                None,
            ),
            fields::labelled(
                "Köşeler",
                seg(
                    &[Opt("miter", "Sivri"), Opt("round", "Yuvarlak"), Opt("bevel", "Pah")],
                    same(&joins).as_deref(),
                    &["Gönyeli köşe", "Yuvarlak köşe", "Kesik köşe"],
                    set("join", "join"),
                ),
                None,
            ),
            fields::labelled(
                "Dolgu kuralı",
                seg(
                    &[Opt("evenodd", "Tek-çift"), Opt("nonzero", "Sıfır olmayan")],
                    same(&rules).as_deref(),
                    &["İç içe parçalar delik olur", "Yönleri aynı parçalar dolu kalır"],
                    set("rule", "fillRule"),
                ),
                None,
            ),
        ],
    )
}

/// X, Y, width and height of the selection's box, and a turn by a typed angle.
pub(super) fn box_fields<'a>(ed: &SvgEditor, sel: &[Obj]) -> Element<'a, Message> {
    let Some(b) = shapes_box(sel).ok().flatten() else {
        return iced::widget::space().into();
    };
    let n = |key: &'static str, label_text: &str, value: f64, min: f64| {
        num(ed, key, label_text, value, None, spec(1.0, min, f64::INFINITY), move |ed, v| {
            ed.set_box(key, v);
        })
    };
    let lock = ed.ui.box_lock;
    let lock_btn = act_sized(
        if lock { Icon::Lock } else { Icon::Unlock },
        "Oranı koru",
        Some(change(move |ed| ed.ui.box_lock = !lock)),
        lock,
        14.0,
    );
    group(
        "Kutu",
        vec![
            fields::pair(
                n("bx", "X", b.min_x, f64::NEG_INFINITY),
                n("by", "Y", b.min_y, f64::NEG_INFINITY),
            ),
            row![
                container(n("bw", "Genişlik", b.max_x - b.min_x, 0.001)).width(Fill),
                container(lock_btn).padding(iced::Padding {
                    top: 18.0,
                    ..iced::Padding::ZERO
                }),
                container(n("bh", "Yükseklik", b.max_y - b.min_y, 0.001)).width(Fill),
            ]
            .spacing(6)
            .into(),
            fields::labelled(
                "Döndür",
                row![
                    container(super::num_bare(ed, "turn", ed.ui.turn, Some("°"), spec(15.0, f64::NEG_INFINITY, f64::INFINITY), |ed, v| {
                        ed.ui.turn = v;
                    }))
                    .width(Fill),
                    act(Icon::Svg(ROT_CCW), "Saat yönünün tersine döndür", Some(change(|ed| ed.turn_box(true)))),
                    act(Icon::Svg(ROT_CW), "Saat yönünde döndür", Some(change(|ed| ed.turn_box(false)))),
                ]
                .spacing(4)
                .align_y(Center),
                None,
            ),
        ],
    )
}

/// ↺ and ↻ as icons (the web wrote the glyphs).
const ROT_CCW: &str = r#"<path d="M5.5 7.5A5.5 5.5 0 1 1 4.5 11"/><path d="M5.5 3.5v4h4"/>"#;
const ROT_CW: &str = r#"<path d="M14.5 7.5A5.5 5.5 0 1 0 15.5 11"/><path d="M14.5 3.5v4h-4"/>"#;

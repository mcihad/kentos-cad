//! Özellikler with nothing chosen (the web's `canvasProps`): the canvas's
//! size, the grid, snapping to the grid, the rulers, the tile preview, the
//! snap kinds, deleting the guides, and the preview's colours (the symbol's
//! colour following the theme until one is picked).

use iced::widget::{Column, button, row};
use iced::{Center, Color, Element, Fill};
use kentos_ui::icon::{Icon, icon};
use kentos_ui::label;
use kentos_ui::style as ui_style;
use kentos_ui::widget::color::parse_hex;

use super::props::swatch;
use super::{check, group, num, spec, title};
use crate::app::Message;
use crate::style::fields;
use crate::style::svgedit::change;
use crate::style::svgedit::icons::svg_icon;
use crate::style::svgedit::state::{SNAP_KINDS, SvgEditor};

pub(super) fn canvas_props<'a>(ed: &SvgEditor) -> Element<'a, Message> {
    let o = &ed.options;
    let doc = &ed.doc;
    let mut kinds: Vec<Element<'a, Message>> = Vec::new();
    for pair in SNAP_KINDS.chunks(2) {
        let mut r = row![].spacing(8);
        for (k, name) in pair {
            let k = *k;
            let on = o.snap_kinds.contains(&k);
            r = r.push(
                iced::widget::container(check(on, name, move |ed, v| {
                    let mut next: Vec<_> = SNAP_KINDS
                        .iter()
                        .map(|(x, _)| *x)
                        .filter(|x| {
                            if *x == k {
                                v
                            } else {
                                ed.options.snap_kinds.contains(x)
                            }
                        })
                        .collect();
                    next.dedup();
                    ed.options.snap_kinds = next;
                    ed.snapper.reset();
                }))
                .width(Fill),
            );
        }
        kinds.push(r.into());
    }
    let mut snap_rows: Vec<Element<'a, Message>> = vec![check(
        o.snap_objects,
        "Şekillere, kılavuzlara ve tuvale kenetle",
        |ed, v| {
            ed.options.snap_objects = v;
            ed.snapper.reset();
        },
    )];
    snap_rows.push(Column::with_children(kinds).spacing(2).into());
    if !doc.guides.is_empty() {
        snap_rows.push(
            button(
                row![
                    icon(Icon::Svg(svg_icon("guide"))).size(14.0),
                    label::body(format!("Kılavuzları sil ({})", doc.guides.len()))
                ]
                .spacing(6)
                .align_y(Center),
            )
            .padding([3, 10])
            .style(ui_style::button::secondary)
            .on_press(change(|ed| ed.clear_guides()))
            .into(),
        );
    }
    // The colour the preview paints with: the theme's ink while it follows the theme.
    let ink = parse_hex(&ed.ink()).unwrap_or(Color::BLACK);
    let second = parse_hex(&o.second).unwrap_or(Color::from_rgb8(0x2b, 0x83, 0xba));
    Column::with_children(vec![
        title("Tuval".to_owned(), None),
        fields::pair(
            num(ed, "dw", "Genişlik", doc.width, None, spec(5.0, 1.0, f64::INFINITY), |ed, v| {
                ed.edit("dw", move |ed| ed.doc.width = v.max(1.0));
            }),
            num(ed, "dh", "Yükseklik", doc.height, None, spec(5.0, 1.0, f64::INFINITY), |ed, v| {
                ed.edit("dh", move |ed| ed.doc.height = v.max(1.0));
            }),
        ),
        fields::hint("Birim çizimin kendi birimidir; semboldeki boyutu sembol belirler (genişlik = işaret boyu)."),
        num(ed, "grid", "Izgara aralığı", o.grid, None, spec(1.0, 0.0, f64::INFINITY), |ed, v| {
            ed.options.grid = v.max(0.0);
            ed.touch();
        }),
        check(o.snap_grid, "Izgaraya kenetle", |ed, v| ed.options.snap_grid = v),
        check(o.rulers, "Cetveller (kılavuz için cetvelden sürükleyin)", |ed, v| {
            ed.options.rulers = v;
            ed.touch();
        }),
        check(o.tile, "Döşeme önizlemesi (desen olarak yan yana)", |ed, v| {
            ed.options.tile = v;
            ed.camera.fit(&ed.doc, &ed.options);
            ed.touch();
        }),
        group("Kenetleme", snap_rows),
        group(
            "Önizleme renkleri",
            vec![
                fields::pair(
                    fields::labelled(
                        "Sembol rengi",
                        swatch(ink, |hex| {
                            change(move |ed| {
                                // Picking a symbol colour stops it following the theme.
                                ed.options.ink = hex.clone();
                                ed.options.ink_auto = false;
                                ed.touch();
                            })
                        }),
                        None,
                    ),
                    fields::labelled(
                        "İkinci renk",
                        swatch(second, |hex| {
                            change(move |ed| {
                                ed.options.second = hex.clone();
                                ed.touch();
                            })
                        }),
                        None,
                    ),
                ),
                check(o.ink_auto, "Sembol rengi temayı izlesin (açıkta siyah, koyuda beyaz)", |ed, v| {
                    ed.options.ink_auto = v;
                    ed.touch();
                }),
                fields::hint("Yalnızca burada denemek içindir: haritada sembol hangi rengi verirse o boyanır."),
            ],
        ),
    ])
    .spacing(10)
    .width(Fill)
    .into()
}

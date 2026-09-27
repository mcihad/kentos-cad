//! The Hizala tab (the web's `svgAlign.ts`, Inkscape's Align and
//! Distribute): what to align against (the selection, the first or last
//! chosen, the biggest or smallest, the canvas), edges and centres, and
//! even spacing of edges, centres or gaps. A group counts as one shape.

use iced::widget::column;
use iced::{Element, Fill};
use kentos_ui::icon::Icon;

use super::{act_sized, acts, check, group, title};
use crate::app::Message;
use crate::style::fields;
use crate::style::svgedit::change;
use crate::style::svgedit::icons::svg_icon;
use crate::style::svgedit::state::SvgEditor;

const TO: [(&str, &str); 6] = [
    ("selection", "Seçimin kutusu"),
    ("first", "İlk seçilen"),
    ("last", "Son seçilen"),
    ("biggest", "En büyük"),
    ("smallest", "En küçük"),
    ("canvas", "Tuval"),
];

pub(super) fn tab<'a>(ed: &SvgEditor, count: usize) -> Element<'a, Message> {
    let big = |name: &'static str, tip_text: &str, press: Option<Message>| {
        act_sized(Icon::Svg(svg_icon(name)), tip_text, press, false, 20.0)
    };
    let a = |side: &'static str, name: &'static str, tip_text: &str| {
        big(name, tip_text, (count > 0).then(|| change(move |ed| ed.align(side))))
    };
    let d = |how: &'static str, name: &'static str, tip_text: &str| {
        big(name, tip_text, (count >= 3).then(|| change(move |ed| ed.distribute(how))))
    };
    column![
        title("Hizala ve dağıt".to_owned(), None),
        fields::labelled(
            "Göre",
            fields::select(&TO, ed.ui.align_to, |v| change(move |ed| ed.ui.align_to = v)),
            None,
        ),
        check(ed.ui.align_as_one, "Seçimi tek parça olarak taşı", |ed, v| ed.ui.align_as_one = v),
        group(
            "Hizala",
            vec![
                acts(vec![
                    a("left", "alignLeft", "Sol kenarlar"),
                    a("hcenter", "alignHCenter", "Yatay ortalar"),
                    a("right", "alignRight", "Sağ kenarlar"),
                ]),
                acts(vec![
                    a("top", "alignTop", "Üst kenarlar"),
                    a("vcenter", "alignVCenter", "Dikey ortalar"),
                    a("bottom", "alignBottom", "Alt kenarlar"),
                ]),
            ],
        ),
        group(
            "Dağıt",
            vec![
                acts(vec![
                    d("left", "distLeft", "Sol kenarlar eşit aralıkta"),
                    d("hcenter", "distHCenter", "Yatay ortalar eşit aralıkta"),
                    d("right", "distRight", "Sağ kenarlar eşit aralıkta"),
                    d("hgap", "distHGap", "Yatayda eşit boşluk"),
                ]),
                acts(vec![
                    d("top", "distTop", "Üst kenarlar eşit aralıkta"),
                    d("vcenter", "distVCenter", "Dikey ortalar eşit aralıkta"),
                    d("bottom", "distBottom", "Alt kenarlar eşit aralıkta"),
                    d("vgap", "distVGap", "Dikeyde eşit boşluk"),
                ]),
            ],
        ),
        fields::hint(if count == 0 {
            "Şekil seçin; tek şekil tuvale hizalanır."
        } else if count < 3 {
            "Dağıtmak için en az üç şekil (ya da grup) seçin; en dıştakiler yerinde kalır."
        } else {
            "En dıştakiler yerinde kalır, aradakiler eşit aralanır. Grup tek şekil sayılır."
        }),
    ]
    .spacing(10)
    .width(Fill)
    .into()
}

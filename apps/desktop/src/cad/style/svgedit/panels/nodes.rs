//! The node tool's box on top of the properties (the web's
//! `svgNodeProps.ts`): what is chosen, node types, adding, deleting, joining
//! and breaking, segments to lines or curves, corners rounded or cut (by
//! dragging on the canvas or a typed size) and nodes lined up. Every button
//! says its key.

use iced::widget::row;
use iced::{Center, Element, Fill};
use kentos_svg_core::nodes::node_type_of;
use kentos_ui::icon::Icon;

use super::props::tool_box;
use super::{act_sized, acts, spec};
use crate::app::Message;
use crate::style::fields;
use crate::style::svgedit::actions::NodeCmd;
use crate::style::svgedit::change;
use crate::style::svgedit::icons::svg_icon;
use crate::style::svgedit::node_tool::CornerMode;
use crate::style::svgedit::state::SvgEditor;

pub(super) fn node_box<'a>(ed: &SvgEditor) -> Element<'a, Message> {
    let sel = ed.node_selected();
    let subs = ed
        .node_shape()
        .and_then(|s| s.subs().ok())
        .unwrap_or_default();
    let count: usize = subs.iter().map(|sp| sp.nodes.len()).sum();
    let types: Vec<String> = sel
        .iter()
        .filter_map(|&(si, ni)| subs.get(si).and_then(|sp| node_type_of(sp, ni)))
        .collect();
    let ty = types
        .first()
        .filter(|t| types.iter().all(|x| x == *t))
        .cloned();
    let few = sel.len() < 2;
    let btn =
        |name: &'static str, label_text: &str, key: &str, cmd: Option<NodeCmd>, pressed: bool| {
            let tip = if key.is_empty() {
                label_text.to_owned()
            } else {
                format!("{label_text} ({key})")
            };
            act_sized(
                Icon::Svg(svg_icon(name)),
                &tip,
                cmd.map(|c| change(move |ed| ed.node_cmd(c))),
                pressed,
                16.0,
            )
        };
    let typed = |t: &'static str| (!sel.is_empty()).then_some(NodeCmd::Type(t));
    let mode = ed.nodes.mode;
    let corner_mode = |m: CornerMode, name: &'static str, label_text: &str| {
        act_sized(
            Icon::Svg(svg_icon(name)),
            label_text,
            Some(change(move |ed| {
                let next = if ed.nodes.mode == Some(m) {
                    None
                } else {
                    Some(m)
                };
                ed.set_corner_mode(next);
            })),
            mode == Some(m),
            16.0,
        )
    };
    tool_box(
        "Düğüm aracı",
        vec![
            fields::hint(&if sel.is_empty() {
                format!("{count} düğüm. Tıklayın ya da boşlukta kutu çizin; Shift ekler.")
            } else {
                format!("{} / {count} düğüm seçili.", sel.len())
            }),
            acts(vec![
                btn(
                    "nodeCusp",
                    "Köşe düğüm",
                    "Shift+C",
                    typed("cusp"),
                    ty.as_deref() == Some("cusp"),
                ),
                btn(
                    "nodeSmooth",
                    "Yumuşak düğüm",
                    "Shift+S",
                    typed("smooth"),
                    ty.as_deref() == Some("smooth"),
                ),
                btn(
                    "nodeSymmetric",
                    "Simetrik düğüm",
                    "Shift+Y",
                    typed("symmetric"),
                    ty.as_deref() == Some("symmetric"),
                ),
                btn(
                    "nodeAuto",
                    "Otomatik düğüm",
                    "Shift+A",
                    typed("auto"),
                    ty.as_deref() == Some("auto"),
                ),
            ]),
            acts(vec![
                btn(
                    "nodeInsert",
                    "Seçili parçaların ortasına düğüm ekle",
                    "Insert",
                    (!few).then_some(NodeCmd::Insert),
                    false,
                ),
                btn(
                    "nodeDelete",
                    "Düğümü sil (biçim korunur; Ctrl+Delete korumadan)",
                    "Delete",
                    (!sel.is_empty()).then_some(NodeCmd::Delete { keep_shape: true }),
                    false,
                ),
                btn(
                    "nodeJoin",
                    "Uç düğümleri birleştir",
                    "Shift+J",
                    (!few).then_some(NodeCmd::Join { merge: true }),
                    false,
                ),
                btn(
                    "nodeJoinSeg",
                    "Uçları parçayla birleştir",
                    "Shift+K",
                    (!few).then_some(NodeCmd::Join { merge: false }),
                    false,
                ),
                btn(
                    "nodeBreak",
                    "Düğümde kır",
                    "Shift+B",
                    (!sel.is_empty()).then_some(NodeCmd::Break),
                    false,
                ),
                btn(
                    "segDelete",
                    "İki düğüm arasındaki parçayı sil",
                    "Alt+Delete",
                    (!few).then_some(NodeCmd::DeleteSegment),
                    false,
                ),
                btn(
                    "segLine",
                    "Parçaları düz yap",
                    "Shift+L",
                    (!few).then_some(NodeCmd::Segments { line: true }),
                    false,
                ),
                btn(
                    "segCurve",
                    "Parçaları eğri yap",
                    "Shift+U",
                    (!few).then_some(NodeCmd::Segments { line: false }),
                    false,
                ),
            ]),
            fields::group_title("Köşe"),
            row![
                corner_mode(
                    CornerMode::Fillet,
                    "fillet",
                    "Köşe yuvarla: köşeye basıp çekin"
                ),
                corner_mode(
                    CornerMode::Chamfer,
                    "chamfer",
                    "Pah kır: köşeye basıp çekin"
                ),
                iced::widget::container(super::num_bare(
                    ed,
                    "corner",
                    ed.ui.corner,
                    None,
                    spec(0.5, 0.0, f64::INFINITY),
                    |ed, v| {
                        ed.ui.corner = v.max(0.0);
                    }
                ))
                .width(Fill),
                btn(
                    "fillet",
                    "Seçili köşeleri bu yarıçapla yuvarla",
                    "",
                    (!sel.is_empty()).then_some(NodeCmd::Corner(CornerMode::Fillet)),
                    false
                ),
                btn(
                    "chamfer",
                    "Seçili köşelere bu boyda pah kır",
                    "",
                    (!sel.is_empty()).then_some(NodeCmd::Corner(CornerMode::Chamfer)),
                    false
                ),
            ]
            .spacing(4)
            .align_y(Center)
            .into(),
            fields::hint(if mode.is_some() {
                "Köşe halkayla işaretlenir; basıp kenar boyunca çekin, bırakınca uygulanır. Esc biter."
            } else {
                "Fareyle: düğmeyi basılı yapıp köşeye basın ve çekin. Kesin değer: yazıp yandaki düğme."
            }),
            fields::group_title("Düğümleri hizala"),
            acts(vec![
                btn(
                    "alignLeft",
                    "Solda hizala",
                    "",
                    (!few).then_some(NodeCmd::Align {
                        axis: "x",
                        to: "min",
                    }),
                    false,
                ),
                btn(
                    "alignHCenter",
                    "Yatayda ortala",
                    "",
                    (!few).then_some(NodeCmd::Align {
                        axis: "x",
                        to: "mid",
                    }),
                    false,
                ),
                btn(
                    "alignRight",
                    "Sağda hizala",
                    "",
                    (!few).then_some(NodeCmd::Align {
                        axis: "x",
                        to: "max",
                    }),
                    false,
                ),
                btn(
                    "alignTop",
                    "Üstte hizala",
                    "",
                    (!few).then_some(NodeCmd::Align {
                        axis: "y",
                        to: "min",
                    }),
                    false,
                ),
                btn(
                    "alignVCenter",
                    "Dikeyde ortala",
                    "",
                    (!few).then_some(NodeCmd::Align {
                        axis: "y",
                        to: "mid",
                    }),
                    false,
                ),
                btn(
                    "alignBottom",
                    "Altta hizala",
                    "",
                    (!few).then_some(NodeCmd::Align {
                        axis: "y",
                        to: "max",
                    }),
                    false,
                ),
                btn(
                    "distHCenter",
                    "Yatayda eşit dağıt",
                    "",
                    (sel.len() >= 3).then_some(NodeCmd::Distribute { axis: "x" }),
                    false,
                ),
                btn(
                    "distVCenter",
                    "Dikeyde eşit dağıt",
                    "",
                    (sel.len() >= 3).then_some(NodeCmd::Distribute { axis: "y" }),
                    false,
                ),
            ]),
        ],
    )
}

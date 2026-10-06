//! The pictures of docs/adr/0183 (Yazı ve ölçü stilleri) in a CAD project:
//! a parcel drawn with the project's styles (its number bold in Arimo, the
//! road's name in Barlow italic and slanted, a note in Courier Prime, its
//! sides measured in Mimari (arrows, cm), Kadastro (ticks, the value centred,
//! “L=”), Noktalı (dots) and Açık (open arrows), one dimension in Standart);
//! the Yazı stilleri and Ölçü stilleri windows over it; Yazı with a style
//! chosen and its Stil menu; Öznitelikler's Yazı stili row. The web's are
//! `shots.mjs styles`. `tools_screens` takes them in the dark and the light
//! theme at 1440×900 and 1100×650:
//!
//! ```text
//! KENTOS_SHOTS_ONLY=stil-cizim,stil-yazi-penceresi cargo test -p kentos-desktop tools_screens -- --ignored --nocapture
//! ```
//!
//! Test code only.

use kentos_contracts::{
    DimensionArrow, DimensionStyleDef, DimensionTextPlace, DrawingFont, DrawingUnit, TextStyleDef,
};
use kentos_domain::Slot;
use kentos_ui::widget::docking;
use serde_json::json;

use crate::annotation_styles::Event as StylesEvent;
use crate::app::{App, Message};
use crate::files_testing::make_cad;
use crate::tools_scenes::{E, N, Objects, forget, hover, open, run};
use crate::tools_screens::{Pointed, Scene, press_caption};

const ADA: &str = "0192f1a0-0000-7000-8000-000000000001";
const YOL: &str = "0192f1a0-0000-7000-8000-000000000002";
const NOT: &str = "0192f1a0-0000-7000-8000-000000000003";
const MIMARI: &str = "0192f1a0-0000-7000-8000-0000000000d1";
const KADASTRO: &str = "0192f1a0-0000-7000-8000-0000000000d2";
const NOKTALI: &str = "0192f1a0-0000-7000-8000-0000000000d3";
const ACIK: &str = "0192f1a0-0000-7000-8000-0000000000d4";

/// The drawing's scale: a style's paper mm are half as many metres.
const SCALE: f64 = 500.0;

fn text_styles() -> Vec<TextStyleDef> {
    vec![
        TextStyleDef {
            id: ADA.into(),
            name: "Ada no".into(),
            font: DrawingFont::Arimo,
            bold: true,
            italic: false,
            oblique: None,
            height: Some(3.5),
            width_factor: Some(0.9),
            font_file: None,
        },
        TextStyleDef {
            id: YOL.into(),
            name: "Yol adı".into(),
            font: DrawingFont::Barlow,
            bold: false,
            italic: true,
            oblique: Some(12.0),
            height: Some(3.0),
            width_factor: None,
            font_file: None,
        },
        TextStyleDef {
            id: NOT.into(),
            name: "Not".into(),
            font: DrawingFont::CourierPrime,
            bold: false,
            italic: false,
            oblique: None,
            height: Some(2.0),
            width_factor: None,
            font_file: None,
        },
    ]
}

fn dimension_styles() -> Vec<DimensionStyleDef> {
    let plain = DimensionStyleDef {
        id: String::new(),
        name: String::new(),
        height: 2.5,
        arrow: None,
        arrow_size: None,
        ext_offset: None,
        ext_beyond: None,
        text_gap: None,
        text_place: None,
        decimals: None,
        unit: None,
        prefix: None,
        suffix: None,
        font: None,
    };
    vec![
        DimensionStyleDef {
            id: MIMARI.into(),
            name: "Mimari".into(),
            height: 2.5,
            arrow: Some(DimensionArrow::Closed),
            arrow_size: Some(3.0),
            ext_beyond: Some(1.5),
            decimals: Some(0),
            unit: Some(DrawingUnit::Cm),
            suffix: Some(" cm".into()),
            font: Some(DrawingFont::Arimo),
            ..plain.clone()
        },
        DimensionStyleDef {
            id: KADASTRO.into(),
            name: "Kadastro".into(),
            height: 3.0,
            arrow_size: Some(2.0),
            text_place: Some(DimensionTextPlace::Centre),
            decimals: Some(2),
            prefix: Some("L=".into()),
            ..plain.clone()
        },
        DimensionStyleDef {
            id: NOKTALI.into(),
            name: "Noktalı".into(),
            arrow: Some(DimensionArrow::Dot),
            arrow_size: Some(2.0),
            ..plain.clone()
        },
        DimensionStyleDef {
            id: ACIK.into(),
            name: "Açık".into(),
            arrow: Some(DimensionArrow::Open),
            font: Some(DrawingFont::Overpass),
            ..plain
        },
    ]
}

/// A text's face and width factor in `style`, its height the style's at the scale.
fn in_text_style(o: &mut Objects, id: u32, style: &TextStyleDef) {
    let mut fields = json!({ "textStyle": style.id, "font": style.font });
    if style.bold {
        fields["bold"] = json!(true);
    }
    if style.italic {
        fields["italic"] = json!(true);
    }
    if let Some(oblique) = style.oblique {
        fields["oblique"] = json!(oblique);
    }
    if let Some(w) = style.width_factor {
        fields["widthFactor"] = json!(w);
    }
    if let Some(h) = style.height_at(SCALE) {
        fields["height"] = json!(h);
    }
    o.fields(id, fields);
}

/// A dimension's look and value height in `style` at the scale.
fn in_dimension_style(o: &mut Objects, id: u32, style: &DimensionStyleDef) {
    let mut fields = serde_json::to_value(style.look()).expect("a look");
    fields["height"] = json!(style.height_at(SCALE));
    o.fields(id, fields);
}

/// The parcel: its corners, west to east, south to north.
const PARCEL: [[f64; 2]; 4] = [[0.0, 0.0], [40.0, 0.0], [40.0, 26.0], [0.0, 26.0]];

/// A parcel beside a road: its number, the road's name and a note in the
/// three text styles, a plain text; its sides measured in the four
/// dimension styles, one in Standart. Slots: the parcel 1, the road 2, the
/// texts 3 to 6, the dimensions 7 to 11.
fn ground() -> Objects {
    let (texts, dims) = (text_styles(), dimension_styles());
    let mut o = Objects::new();
    o.path("parsel", &PARCEL, true);
    o.line("yol", [-6.0, -14.0], [52.0, -14.0]);
    let ada = o.text("cizim", [12.0, 12.0], "Ada 101 Parsel 7", 1.75, 0.0);
    in_text_style(&mut o, ada, &texts[0]);
    let yol = o.text("cizim", [4.0, -12.4], "Atatürk Caddesi", 1.5, 0.0);
    in_text_style(&mut o, yol, &texts[1]);
    let not = o.text("cizim", [2.0, 22.0], "Not: ölçüler cm'dir", 1.0, 0.0);
    in_text_style(&mut o, not, &texts[2]);
    o.text("cizim", [2.0, 3.0], "Standart yazı", 1.0, 0.0);
    // Round the parcel anticlockwise: a side's left is inside, its dimension goes out (a negative offset).
    let south = o.dimension("cizim", None, [PARCEL[0], PARCEL[1]], None, -5.0, json!({}));
    in_dimension_style(&mut o, south, &dims[0]);
    let east = o.dimension("cizim", None, [PARCEL[1], PARCEL[2]], None, -6.0, json!({}));
    in_dimension_style(&mut o, east, &dims[1]);
    let north = o.dimension("cizim", None, [PARCEL[2], PARCEL[3]], None, -5.0, json!({}));
    in_dimension_style(&mut o, north, &dims[2]);
    let west = o.dimension("cizim", None, [PARCEL[3], PARCEL[0]], None, -6.0, json!({}));
    in_dimension_style(&mut o, west, &dims[3]);
    o.dimension(
        "cizim",
        None,
        [[0.0, -14.0], [40.0, -14.0]],
        None,
        -4.0,
        json!({ "height": 1.25 }),
    );
    o
}

/// The ground open in a CAD project at 1:500 with its styles, Açıklama's
/// tab (Yazı ▾ and Ölçü ▾ open the windows), the parcel and the road in view.
fn styled(app: &mut App) {
    open(app, ground());
    make_cad(app);
    let doc = app.document.as_mut().expect("a drawing");
    let mut settings = doc.settings().clone();
    settings.plot_scale = SCALE;
    settings.text_styles = text_styles();
    settings.dimension_styles = dimension_styles();
    doc.model.set_settings(settings);
    app.tab = "annotate";
    // The command history shut: the drawing area at its tallest.
    app.command_expanded = false;
    app.viewport.camera.fit(
        // Fitted to the area with the history open: shut, it shows the road and its dimension too.
        &kentos_render_wgpu::Bounds {
            min_x: E - 9.0,
            min_y: N - 9.0,
            max_x: E + 49.0,
            max_y: N + 32.0,
        },
        24.0,
    );
}

/// A styles window open over the drawing, `id`'s style chosen.
fn window(app: &mut App, command: &'static str, id: &str) {
    styled(app);
    run(app, command);
    let _ = app.update(Message::AnnotationStyles(StylesEvent::Choose(Some(
        id.into(),
    ))));
}

/// Yazı with Yol adı chosen, the pointer where the next text goes.
fn tool(app: &mut App) {
    styled(app);
    app.command_expanded = true;
    run(app, "tool.text");
    let _ = app.update(Message::PromptChoice("S", "Yol adı".into()));
    forget(app);
    hover(app, [6.0, 30.0]);
}

/// The parcel's number selected, Öznitelikler with the room to show its rows.
fn props(app: &mut App) {
    styled(app);
    app.selection.set([Slot(3)]);
    let upper = docking::Slot::Docked(docking::Side::Right, 0);
    let _ = app.update(Message::Dock(docking::Event::Collapsed(upper, true)));
    let _ = app.update(Message::Properties(crate::properties::Event::Toggle(
        "general",
    )));
}

pub(crate) fn scenes() -> Vec<Scene> {
    vec![
        ("stil-cizim", styled),
        ("stil-yazi-penceresi", |app| {
            window(app, "style.textStyles", ADA)
        }),
        ("stil-olcu-penceresi", |app| {
            window(app, "style.dimensionStyles", MIMARI)
        }),
        ("stil-yazi-araci", tool),
        ("stil-oznitelikler", props),
    ]
}

/// Stil's menu from the command line's chip.
pub(crate) fn pointed() -> Vec<Pointed> {
    vec![("stil-menusu", tool, |s, app| {
        press_caption(s, app, "Stil: Yol adı", false, false)
    })]
}

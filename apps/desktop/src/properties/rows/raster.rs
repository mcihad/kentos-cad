//! A raster's rows of docs/adr/0204 §8 in Öznitelikler: its file (embedded
//! or linked, and whether it was found), its size, bands and samples, its
//! pixel's size, its file's system, its look, nodata and transparency; Göm
//! for a linked one and the look's and the transparency's editors.

use kentos_contracts::{RasterEntity, RasterRender, RasterStretch};
use kentos_domain::Slot;
use kentos_interaction::{Format, fixed};

use super::{Choice, Editor, Row};
use crate::app::Message;
use crate::document::Document;
use crate::properties::{Event, Field};

/// How a look is named (Raster stili's list).
pub(crate) fn render_name(r: RasterRender) -> &'static str {
    match r {
        RasterRender::Rgb => "Renkli (RGB)",
        RasterRender::Gray => "Gri",
        RasterRender::Palette => "Paletli",
        RasterRender::Ramp => "Renk rampası",
        RasterRender::Hillshade => "Gölgeli kabartma",
        RasterRender::RampShade => "Rampa ve gölge",
    }
}

/// How a stretch is named.
pub(crate) fn stretch_name(s: RasterStretch) -> &'static str {
    match s {
        RasterStretch::None => "Yok",
        RasterStretch::MinMax => "En küçükten en büyüğe",
        RasterStretch::Percent => "%2 – %98",
        RasterStretch::Manual => "Elle",
    }
}

/// The rows of one raster; editors unless its layer is locked.
pub(super) fn rows(
    doc: &Document,
    slot: Slot,
    r: &RasterEntity,
    locked: bool,
    f: &Format,
) -> Vec<Row> {
    let x = &r.raster;
    let (source, linked) = match (&x.asset, &x.file) {
        (Some(a), _) => (crate::pictures::asset_words(&doc.model, a), false),
        (None, Some(path)) => (crate::pictures::file_words(path, doc.path.as_deref()), true),
        _ => ("—".to_owned(), false),
    };
    let embed = (linked && !locked).then(|| Editor::Select {
        text: source.clone(),
        swatch: None,
        icon: None,
        items: vec![Choice::Pick {
            label: "Göm".to_owned(),
            swatch: None,
            icon: None,
            wide: false,
            chosen: false,
            enabled: true,
            message: Message::Properties(Event::EmbedRaster(slot)),
        }],
    });
    let [_, a, b, _, c, d] = x.affine;
    // A pixel's two sides: along a row and down a column.
    let (across, down) = (
        kentos_geometry_core::jsmath::js_hypot(a, c),
        kentos_geometry_core::jsmath::js_hypot(b, d),
    );
    let pixel = format!("{} × {}", f.length_bare(across), f.length_bare(down));
    let system = if x.srid == 0 {
        "Projeninki (dosya söylemiyordu)".to_owned()
    } else {
        crate::crs::title_of(x.srid)
    };
    let nodata = match x.style.nodata {
        Some(v) => crate::crs::js_number(v),
        None => "Dosyanınki".to_owned(),
    };
    let clear = (1.0 - x.opacity.unwrap_or(1.0)) * 100.0;
    let style = (!locked).then(|| Editor::Select {
        text: render_name(x.style.render).to_owned(),
        swatch: None,
        icon: None,
        items: vec![Choice::Command {
            label: "Raster stili…",
            icon: crate::icons::from_web(Some("rasterStyle")),
            id: "raster.style",
        }],
    });
    let mut out = vec![
        Row::text("Kaynak", source).editor(embed),
        Row::figure("Boyut", format!("{} × {}", x.width, x.height)).unit("piksel"),
        Row::text("Bantlar", format!("{} bant, {}", x.bands, x.sample.label())),
        Row::figure("Piksel boyu", pixel).unit(f.length_unit_label()),
        Row::text("Sistem", system),
        Row::text("Görünüş", render_name(x.style.render)).editor(style),
    ];
    if x.style.stretch != RasterStretch::None {
        out.push(Row::text("Gerdirme", stretch_name(x.style.stretch)));
    }
    out.push(Row::text("Nodata", nodata));
    out.push(
        Row::figure("Saydamlık", fixed(clear, 0))
            .unit("%")
            .editor((!locked).then_some(Editor::Number(Field::RasterClear(slot)))),
    );
    out
}

//! A raster's rows of docs/adr/0204 §8 in Öznitelikler: its file (embedded
//! or linked, and whether it was found), its size, bands and samples, its
//! pixel's size, its file's system, its look, nodata and transparency; Göm
//! for a linked one and the look's and the transparency's editors. A NetCDF
//! dataset's raster (docs/adr/0243 §11) adds its variable, each slice
//! dimension's value shown and, for a mesh, its nodes and faces.

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
    let (source, linked) = match (&x.asset, &x.file, &x.url) {
        (Some(a), _, _) => (crate::pictures::asset_words(&doc.model, a), false),
        (None, Some(path), _) => (crate::pictures::file_words(path, doc.path.as_deref()), true),
        // An address is read by ranges, not embedded (docs/adr/0207 §1).
        (None, None, Some(u)) => (format!("Adres: {u}"), false),
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
    ];
    if let Some(d) = &x.dataset {
        let mut name = d.variable.clone();
        if let Some(v) = &d.vector {
            name = format!("{name} / {v} (vektör)");
        }
        out.push(Row::text("Veri seti", name));
        for dim in &d.dims {
            let labels = kentos_formats::multidim::cube::dim_labels(
                &dim.values,
                dim.time,
                dim.units.as_deref(),
            );
            let shown = labels.get(dim.index as usize).cloned().unwrap_or_default();
            let (label, text) = if dim.time {
                let follow = if d.follow_time {
                    " (zaman sürgüsünü izler)"
                } else {
                    ""
                };
                ("Zaman".to_owned(), format!("{shown}{follow}"))
            } else {
                (dim.name.clone(), shown)
            };
            out.push(Row::text(label, text));
        }
        if let Some(mesh) = &d.mesh {
            let key = crate::rasters::key_of(x);
            let counts = crate::rasters::tiles::service().mesh_counts(&key);
            out.push(Row::text(
                "Ağ",
                counts.map_or_else(
                    || format!("“{mesh}”"),
                    |(n, f)| {
                        format!(
                            "“{mesh}”: {} düğüm, {} yüz",
                            crate::crs::grouped(n as f64),
                            crate::crs::grouped(f as f64)
                        )
                    },
                ),
            ));
        }
    }
    out.push(Row::text("Görünüş", render_name(x.style.render)).editor(style));
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

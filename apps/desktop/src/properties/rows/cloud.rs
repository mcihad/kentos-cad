//! A point cloud's rows of docs/adr/0207 §9 in Öznitelikler: its files
//! (linked, an address or embedded; a virtual cloud's count), format, points,
//! bounds, density, system, its index, look and transparency.

use kentos_contracts::{CloudFormat, CloudRender, PointCloudEntity};
use kentos_domain::Slot;
use kentos_interaction::{Format, fixed};

use super::{Editor, Row};
use crate::document::Document;
use crate::properties::Field;

/// How a look is named (Nokta bulutu stili's list).
pub(crate) fn render_name(r: CloudRender) -> &'static str {
    match r {
        CloudRender::Rgb => "Renkler (RGB)",
        CloudRender::Classification => "Sınıflar",
        CloudRender::Elevation => "Yükseklik",
        CloudRender::Intensity => "Yoğunluk",
        CloudRender::Returns => "Dönüşler",
        CloudRender::Single => "Tek renk",
    }
}

/// How a file's format is named.
pub(crate) fn format_name(f: CloudFormat) -> &'static str {
    match f {
        CloudFormat::Las => "LAS",
        CloudFormat::Laz => "LAZ",
        CloudFormat::Copc => "COPC",
        CloudFormat::Xyz => "Metin (XYZ)",
    }
}

/// The rows of one cloud; editors unless its layer is locked.
pub(super) fn rows(
    doc: &Document,
    slot: Slot,
    c: &PointCloudEntity,
    locked: bool,
    f: &Format,
) -> Vec<Row> {
    let x = &c.cloud;
    let source = match x.sources.as_slice() {
        [one] => match (&one.asset, &one.file, &one.url) {
            (Some(a), _, _) => crate::pictures::asset_words(&doc.model, a),
            (None, Some(path), _) => crate::pictures::file_words(path, doc.path.as_deref()),
            (None, None, Some(u)) => u.clone(),
            _ => "—".to_owned(),
        },
        many => format!("{} dosya (sanal bulut)", many.len()),
    };
    let formats: Vec<&str> = {
        let mut v: Vec<&str> = x.sources.iter().map(|s| format_name(s.format)).collect();
        v.dedup();
        v
    };
    let [x1, y1, z1, x2, y2, z2] = x.bounds;
    let area = (x2 - x1) * (y2 - y1);
    let density = if area > 0.0 {
        fixed(x.count as f64 / area, 2)
    } else {
        "—".to_owned()
    };
    let system = if x.srid == 0 {
        "Projeninki (dosya söylemiyordu)".to_owned()
    } else {
        crate::crs::title_of(x.srid)
    };
    let clear = (1.0 - x.opacity.unwrap_or(1.0)) * 100.0;
    let style = (!locked).then(|| Editor::Select {
        text: render_name(x.style.render).to_owned(),
        swatch: None,
        icon: None,
        items: vec![super::Choice::Command {
            label: "Nokta bulutu stili…",
            icon: crate::icons::from_web(Some("pointCloudStyle")),
            id: "pointcloud.style",
        }],
    });
    vec![
        Row::text("Kaynak", source),
        Row::text("Biçim", formats.join(", ")),
        Row::figure("Nokta sayısı", crate::crs::js_number(x.count as f64)),
        Row::figure(
            "Kapsam",
            format!("{} × {}", f.length_bare(x2 - x1), f.length_bare(y2 - y1)),
        )
        .unit(f.length_unit_label()),
        Row::figure(
            "Kot aralığı",
            format!("{} – {}", f.length_bare(z1), f.length_bare(z2)),
        )
        .unit(f.length_unit_label()),
        Row::figure("Yoğunluk", density).unit("nokta/m²"),
        Row::text("Sistem", system),
        Row::text("Dizin", crate::pointclouds::index_words(x)),
        Row::text("Görünüş", render_name(x.style.render)).editor(style),
        Row::figure("Saydamlık", fixed(clear, 0))
            .unit("%")
            .editor((!locked).then_some(Editor::Number(Field::CloudClear(slot)))),
    ]
}

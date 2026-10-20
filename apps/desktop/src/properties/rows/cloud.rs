//! A point cloud's rows of docs/adr/0207 §9 in Öznitelikler: its files
//! (linked, an address or embedded; a virtual cloud's count), format, points,
//! bounds, density, system, its index, look and transparency.

use kentos_commands::{CloudFormat, CloudRender, PointCloudEntity};
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

/// A source's header as `cad.pointclouds.describe` reads it (docs/adr/0536), read once a session for each file.
fn described(
    doc: &Document,
    s: &kentos_commands::CloudSource,
) -> kentos_commands::CloudSourceDescribed {
    use std::collections::HashMap;
    use std::sync::{Mutex, OnceLock, PoisonError};
    static SEEN: OnceLock<Mutex<HashMap<String, kentos_commands::CloudSourceDescribed>>> =
        OnceLock::new();
    let key = format!("{:?}|{:?}|{:?}|{:?}", s.file, s.asset, s.url, doc.path);
    let seen = SEEN.get_or_init(|| Mutex::new(HashMap::new()));
    if let Some(d) = seen
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .get(&key)
    {
        return d.clone();
    }
    let d = kentos_native_application::pointclouds::source_described(
        &doc.model,
        doc.path.as_deref().and_then(|p| p.to_str()),
        s,
    );
    seen.lock()
        .unwrap_or_else(PoisonError::into_inner)
        .insert(key, d.clone());
    d
}

/// A virtual cloud's files summed up as `cad.pointclouds.describe` says it (docs/adr/0552), read once a session.
fn summarised(
    doc: &Document,
    sources: &[kentos_commands::CloudSource],
) -> Option<kentos_commands::CloudSummary> {
    use std::collections::HashMap;
    use std::sync::{Mutex, OnceLock, PoisonError};
    static SEEN: OnceLock<Mutex<HashMap<String, Option<kentos_commands::CloudSummary>>>> =
        OnceLock::new();
    let key = format!("{sources:?}|{:?}", doc.path);
    let seen = SEEN.get_or_init(|| Mutex::new(HashMap::new()));
    if let Some(d) = seen.lock().unwrap_or_else(PoisonError::into_inner).get(&key) {
        return d.clone();
    }
    let d = kentos_native_application::pointclouds::summary_described(
        &doc.model,
        doc.path.as_deref().and_then(|p| p.to_str()),
        sources,
    );
    seen.lock()
        .unwrap_or_else(PoisonError::into_inner)
        .insert(key, d.clone());
    d
}

/// The rows of a virtual cloud's summary: the files read, and where they disagree (at most five).
fn summary_rows(s: &kentos_commands::CloudSummary) -> Vec<Row> {
    let mut out = vec![Row::text(
        "Dosyalar",
        format!("{} dosyanın {} tanesi okundu", s.sources, s.readable),
    )];
    if s.mismatches.is_empty() {
        out.push(Row::text("Uyuşmazlık", "Yok"));
    }
    for m in s.mismatches.iter().take(5) {
        out.push(Row::text("Uyuşmazlık", m.message.clone()));
    }
    if s.mismatches.len() > 5 {
        out.push(Row::text(
            "Uyuşmazlık",
            format!("… {} uyuşmazlık daha", s.mismatches.len() - 5),
        ));
    }
    out
}

/// The rows of a single source's header (docs/adr/0536): the file's version and format, the vertical system, the
/// acquisition day, the GPS time, the extra dimensions and who wrote it.
fn header_rows(d: &kentos_commands::CloudSourceDescribed) -> Vec<Row> {
    if !d.read {
        return vec![Row::text(
            "Başlık",
            format!("Okunamadı: {}", d.why.clone().unwrap_or_default()),
        )];
    }
    let mut out = Vec::new();
    if let (Some(v), Some(fmt)) = (&d.version, d.format) {
        out.push(Row::text(
            "Dosya",
            format!("{} {v}, nokta biçimi {fmt}", d.kind.to_uppercase()),
        ));
    }
    out.push(Row::text(
        "Düşey sistem",
        d.vertical
            .clone()
            .unwrap_or_else(|| "Dosya söylemiyor".to_owned()),
    ));
    if let Some(c) = &d.created {
        out.push(Row::text("Oluşturulma", c.clone()));
    }
    if let Some(g) = &d.gps_time {
        out.push(Row::text(
            "GPS zamanı",
            if g == "standard" {
                "Standart GPS zamanı"
            } else {
                "GPS haftasının saniyesi"
            },
        ));
    }
    if !d.extra.is_empty() {
        let names: Vec<String> = d
            .extra
            .iter()
            .map(|e| format!("{} ({})", e.name, e.kind))
            .collect();
        out.push(Row::text("Ek boyutlar", names.join(", ")));
    }
    if let Some(sw) = &d.software {
        out.push(Row::text("Yazan", sw.clone()));
    }
    out
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
            (None, Some(path), _) => crate::pictures::file_words(path, doc.path.as_deref(), &[]),
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
    let header = match x.sources.as_slice() {
        [one] => header_rows(&described(doc, one)),
        many => summarised(doc, many).map_or_else(Vec::new, |s| summary_rows(&s)),
    };
    let mut rows = vec![
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
    ];
    // After Sistem: the file's own words of its systems and its acquisition.
    let at = rows
        .iter()
        .position(|r| r.label == "Sistem")
        .map_or(rows.len(), |i| i + 1);
    rows.splice(at..at, header);
    rows
}

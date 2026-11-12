//! What the panel shows with nothing selected: the drawing's summary and coordinate system, the
//! zoning's emsal and TAKS; a layer drawn from a map service.

use super::*;

/// No selection: the drawing's summary; its coordinate system with `crs`.
pub(super) fn document_section(doc: &Document, crs: bool) -> Section {
    let s = doc.settings();
    let layers = doc.model.layers();
    let active = layers
        .get(layers.active())
        .map_or_else(|| "—".to_owned(), |n| layer_path(layers, &n.id));
    let mut rows = vec![Row::text("Dosya", doc.name())];
    if crs {
        rows.push(Row::text("Koordinat sistemi", crate::crs::project_name(s)));
        rows.push(Row::figure("SRID", crate::crs::project_code(s)));
    }
    rows.extend([
        Row::figure("Çizim ölçeği", format!("1:{}", s.plot_scale)),
        Row::figure("Nesne sayısı", doc.entity_count().to_string()),
        Row::text("Etkin katman", active),
    ]);
    Section {
        id: "doc".into(),
        title: "Çizim".into(),
        rows,
    }
}

/// No selection, the project with a zoning (docs/adr/8030): its emsal and TAKS against their limits, live as the
/// status bar's badge, with the room the badge has not on a narrow bar.
pub(super) fn zoning_section(doc: &Document) -> Option<Section> {
    let r = doc.model.zoning_report()?;
    let comma = |v: f64| format!("{v:.2}").replace('.', ",");
    let against = |v: f64, l: Option<f64>| match l {
        Some(l) if v > l + 1e-9 => format!("{} / {} · aşıldı", comma(v), comma(l)),
        Some(l) => format!("{} / {}", comma(v), comma(l)),
        None => comma(v),
    };
    Some(Section {
        id: "zoning".into(),
        title: "İmar durumu".into(),
        rows: {
            let mut rows = vec![
                Row::figure("Parsel alanı", comma(r.parcel)).unit("m²"),
                Row::figure("Emsal", against(r.far(), r.far_limit)),
                Row::figure("TAKS", against(r.coverage(), r.coverage_limit)),
            ];
            // The site plan's garden distances (docs/adr/8032).
            if crate::zoning_badge::site(&doc.model).is_some() {
                let short = crate::zoning_badge::short_edges(&doc.model);
                rows.push(Row::text(
                    "Bahçe mesafeleri",
                    if short.is_empty() {
                        "uygun".to_owned()
                    } else {
                        format!("dar: {}", short.join("; "))
                    },
                ));
            }
            rows
        },
    })
}

/// A layer drawn from a map service (docs/adr/0208 §14): its kind, address,
/// layers, system, levels, connection, opacity, credits and how it is going;
/// a layer whose objects came from a service: where from and when.
pub(crate) fn service_section(
    doc: &Document,
    node: &kentos_commands::LayerNode,
    failure: Option<String>,
) -> Option<Section> {
    let settings = doc.settings();
    let connection = |id: &Option<String>| {
        id.as_ref().map(
            |id| match settings.connections.iter().find(|c| &c.id == id) {
                Some(c) => {
                    let here = crate::services::secrets::secrets()
                        .get(&c.origin, &c.id)
                        .is_some();
                    format!(
                        "{} ({}){}",
                        c.name,
                        c.auth.label(),
                        if c.auth == kentos_commands::AuthKind::None || here {
                            ""
                        } else {
                            ", değerleri bu cihazda yok"
                        }
                    )
                }
                None => id.clone(),
            },
        )
    };
    let system = |srid: u32| {
        kentos_project::crs::system(srid).map_or_else(
            || format!("EPSG:{srid}"),
            |c| format!("{} (EPSG:{srid})", c.name),
        )
    };
    if let Some(s) = &node.service {
        let mut rows = vec![
            Row::text("Tür", s.kind.label()),
            Row::text(
                "Adres",
                if s.url.is_empty() {
                    "Google Map Tiles API".to_owned()
                } else {
                    s.url.clone()
                },
            ),
        ];
        if !s.layers.is_empty() {
            rows.push(Row::text("Katmanlar", s.layers.join(", ")));
        }
        if let Some(st) = &s.style {
            rows.push(Row::text(
                if s.kind == kentos_commands::ServiceKind::Google {
                    "Harita türü"
                } else {
                    "Stil"
                },
                st.clone(),
            ));
        }
        if let Some(srid) = s.srid.or(s.grid.as_ref().map(|g| g.srid)) {
            rows.push(Row::text("Sistem", system(srid)));
        } else if matches!(
            s.kind,
            kentos_commands::ServiceKind::Xyz
                | kentos_commands::ServiceKind::Google
                | kentos_commands::ServiceKind::Vector
        ) {
            rows.push(Row::text("Sistem", system(3857)));
        }
        rows.push(Row::figure(
            "Katlar",
            format!(
                "{}–{}",
                s.min_zoom.unwrap_or(0),
                s.max_zoom.map_or_else(|| "—".to_owned(), |z| z.to_string())
            ),
        ));
        if let Some(c) = connection(&s.connection) {
            rows.push(Row::text("Bağlantı", c));
        }
        rows.push(Row::figure(
            "Saydamlık",
            format!("% {}", (s.opacity.unwrap_or(1.0) * 100.0).round()),
        ));
        if let Some(a) = &s.attribution {
            rows.push(Row::text("Atıf", a.clone()));
        }
        rows.push(Row::text(
            "Durum",
            failure.unwrap_or_else(|| "Çiziliyor".to_owned()),
        ));
        return Some(Section {
            id: "service".into(),
            title: "Servis katmanı".into(),
            rows,
        });
    }
    let f = node.feed.as_ref()?;
    let mut rows = vec![
        Row::text("Tür", f.kind.label()),
        Row::text("Adres", f.url.clone()),
    ];
    if let Some(n) = &f.name {
        rows.push(Row::text("Tür adı", n.clone()));
    }
    if let Some(srid) = f.srid {
        rows.push(Row::text("İstenen sistem", system(srid)));
    }
    if let Some(q) = &f.filter {
        rows.push(Row::text("Süzgeç", q.clone()));
    }
    rows.push(Row::text(
        "Alan",
        if f.bbox.is_some() {
            "İstenen alan"
        } else {
            "Hepsi"
        },
    ));
    if let Some(k) = &f.key {
        rows.push(Row::text("Anahtar alan", k.clone()));
    }
    if let Some(c) = connection(&f.connection) {
        rows.push(Row::text("Bağlantı", c));
    }
    if let Some(t) = &f.fetched {
        rows.push(Row::text(
            "Son alınış",
            t.replace('T', " ").trim_end_matches('Z').to_owned() + " UTC",
        ));
    }
    Some(Section {
        id: "feed".into(),
        title: "Veri kaynağı".into(),
        rows,
    })
}

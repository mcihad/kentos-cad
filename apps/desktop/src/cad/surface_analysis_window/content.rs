//! Yüzey analizi's İçerik tab (surface_analysis_window.rs): the result's two tabs, the chart's legend, and the figures
//! and tables of each kind of reading, a list at a time.

use super::chart::Swatch;
use super::*;

/// The ranges' table: bounds, plan and 3D area, share of the whole.
/// The result's two tabs (the owner's words: “grafik ve içerik ayrı sekmelerde olsun”): the chart, or the figures and
/// the tables.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Tab {
    #[default]
    Chart,
    Content,
}

impl std::fmt::Display for Tab {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Tab::Chart => "Grafik",
            Tab::Content => "İçerik",
        })
    }
}

/// The line under the chart that says what it shows (DESIGN.md §7.19: no words inside a picture's lines).
pub(super) fn legend(read: &Read) -> Option<String> {
    Some(match read {
        Read::Analysis(_) => "Renkler aralıklardır, ilki en düşük; alanlar İçerik'te.".to_owned(),
        Read::Compare(c) if c.bands.is_empty() && !c.zones.is_empty() => {
            "Her malzemenin katsayılı kazısı (kırmızı) ve dolgusu (yeşil); bölge dışı karşılaştırmanın katsayılarıyla.".to_owned()
        }
        Read::Compare(c) if !c.bands.is_empty() => "Fark aralıklarının plan alanı: kırmızı kazı, yeşil dolgu, sarı ikisini de kapsayan.".to_owned(),
        Read::Compare(_) => return None,
        Read::Quality(_) => {
            "Sarı üçgen sivri nokta, halka yakın çift; kontrol noktası farkıyla: yeşil yüzeyin üstünde, kırmızı altında.".to_owned()
        }
        Read::Coverage(..) => "Yeşil ölçülen, sarı uzun kenarla tahmin edilen üçgenler; çizgi proje sınırı.".to_owned(),
        Read::Density(..) => "Hücreler nokta sayısıyla koyulaşır; kırmızı boş, sarı seyrek; çizgi proje sınırı.".to_owned(),
    })
}

/// A table of facts: what and how much, the kit's data table (DESIGN.md §7.19: no hand-made label rows).
fn facts<'a>(rows: Vec<(String, String)>) -> Element<'a, Message> {
    use crate::road_dialog::{Col, col};
    crate::road_dialog::table(
        vec![
            col("Ölçü", 170.0),
            Col {
                title: "Değer",
                width: None,
                number: false,
            },
        ],
        rows.into_iter().map(|(k, v)| vec![k, v]).collect(),
        "",
    )
}

/// The content tab's tables: the facts first.
pub(super) const SUMMARY: &str = "Özet";
const RANGES: &str = "Aralıklar";
const MATERIALS: &str = "Malzemeler";
/// Fark by sections' table (docs/adr/4340): each section's distance and areas.
const SECTIONS: &str = "Kesitler";
const CHECKS: &str = "Kontrol noktaları";
const ROBUST: &str = "Sağlam ölçüler";

/// A table with the line under it that says what its numbers are.
fn said<'a>(table: Element<'a, Message>, words: String) -> Element<'a, Message> {
    let muted = |t: &Theme| iced::widget::text::Style {
        color: Some(Tokens::of(t).muted),
    };
    Column::new()
        .spacing(6)
        .push(table)
        .push(label::caption(words).style(muted))
        .into()
}

/// The content tab: at most four figures read at a glance, then the reading's tables by name, one shown at a time.
#[allow(clippy::type_complexity)]
pub(super) fn content<'a>(
    w: &'a Window,
    read: &'a Read,
) -> (
    Vec<(String, String)>,
    Vec<(&'static str, Element<'a, Message>)>,
) {
    use crate::road_dialog::{col, grouped, num};
    let m3 = |v: f64| format!("{} m³", grouped(v, 2));
    let m2 = |v: f64| format!("{} m²", grouped(v, 2));
    let mut figures: Vec<(String, String)> = Vec::new();
    let mut parts: Vec<(&'static str, Element<'a, Message>)> = Vec::new();
    match read {
        Read::Analysis(a) => {
            figures.push(("Alan".into(), m2(a.area)));
            figures.push(("3B alan".into(), m2(a.surface_area)));
            let unit = ["m", "%", "°"][w.measure];
            if let Some(b) = a
                .elevations
                .as_ref()
                .or(a.slopes.as_ref())
                .or(a.aspects.as_ref())
            {
                figures.push(("Aralık".into(), b.bands.len().to_string()));
                parts.push((RANGES, band_table(b, unit, a.area)));
            }
            let mut rows = Vec::new();
            if let Some(r) = &a.roughness {
                rows.push((
                    "Pürüzlülük".into(),
                    format!("ortalama {:.4} · en çok {:.4}", r.mean, r.max),
                ));
            }
            if let Some(g) = &a.grid {
                rows.push((
                    "Raster ile TIN".into(),
                    format!(
                        "{} hücre · ortalama {:+.2} % · en çok {:.2} %",
                        g.cells, g.mean, g.max_absolute
                    ),
                ));
            }
            if !rows.is_empty() {
                parts.push((SUMMARY, facts(rows)));
            }
        }
        Read::Compare(c) => {
            figures.push(("Kazı".into(), m3(c.cut)));
            figures.push(("Dolgu".into(), m3(c.fill)));
            figures.push((
                "Net".into(),
                format!(
                    "{}{} m³",
                    if c.net > 0.0 { "+" } else { "" },
                    grouped(c.net, 2)
                ),
            ));
            if let Some(t) = &c.zoned {
                figures.push((
                    "Malzemeye göre net".into(),
                    format!(
                        "{}{} m³",
                        if t.net > 0.0 { "+" } else { "" },
                        grouped(t.net, 2)
                    ),
                ));
            } else if let Some(a) = &c.adjusted {
                figures.push((
                    "Katsayılı net".into(),
                    format!(
                        "{}{} m³",
                        if a.net > 0.0 { "+" } else { "" },
                        grouped(a.net, 2)
                    ),
                ));
            }
            let method = match (c.cell, c.interval) {
                (Some(cell), _) => format!("Izgara, {cell} m hücre (orta nokta)"),
                // Docs/adr/4340: the sections' spacing, the axis and the formula.
                (None, Some(interval)) => format!(
                    "Kesit, {} m arayla, eksenin semti {}° ({})",
                    format!("{interval}").replace('.', ","),
                    format!("{}", c.azimuth.unwrap_or(0.0)).replace('.', ","),
                    match c.formula {
                        Some(kentos_commands::SectionFormula::Prismoidal) => "prizmoidal",
                        Some(kentos_commands::SectionFormula::Frustum) => "kesik koni",
                        _ => "ortalama alan",
                    }
                ),
                (None, None) => "TIN–TIN, kesin prizmalar".to_owned(),
            };
            let mut rows = vec![
                ("Yöntem".to_owned(), method),
                ("Kazı alanı".to_owned(), m2(c.cut_area)),
                ("Dolgu alanı".to_owned(), m2(c.fill_area)),
                ("Tabanın dışında".to_owned(), m2(c.uncovered)),
            ];
            if let (Some(l), Some(h)) = (&c.least, &c.largest) {
                rows.push((
                    "Fark".into(),
                    format!("{:+.3} … {:+.3} m", l.difference, h.difference).replace('.', ","),
                ));
            }
            // The grid beside the exact TIN: their difference is the method's.
            if let Some(e) = &c.exact {
                rows.push((
                    "Kesin (TIN)".into(),
                    format!(
                        "kazı {} · dolgu {} m³",
                        grouped(e.cut, 2),
                        grouped(e.fill, 2)
                    ),
                ));
                rows.push((
                    "Yöntem farkı".into(),
                    format!(
                        "kazı {:+.2} · dolgu {:+.2} m³",
                        c.cut - e.cut,
                        c.fill - e.fill
                    ),
                ));
            }
            if let Some(a) = &c.adjusted {
                rows.push((
                    format!("Katsayılı (×{} / ×{})", a.cut_factor, a.fill_factor).replace('.', ","),
                    format!(
                        "kazı {} · dolgu {} m³",
                        grouped(a.cut, 2),
                        grouped(a.fill, 2)
                    ),
                ));
            }
            parts.push((SUMMARY, facts(rows)));
            if !c.bands.is_empty() {
                let mut rows: Vec<Vec<String>> = c
                    .bands
                    .iter()
                    .map(|b| {
                        vec![
                            format!("{:+.2} … {:+.2}", b.from, b.to).replace('.', ","),
                            grouped(b.area, 2),
                        ]
                    })
                    .collect();
                if let Some(o) = c.outside_area {
                    rows.push(vec!["Aralıkların dışı".to_owned(), grouped(o, 2)]);
                }
                parts.push((
                    RANGES,
                    crate::road_dialog::table(
                        vec![col("Fark (m)", 170.0), num("Alan m²", 110.0)],
                        rows,
                        "",
                    ),
                ));
            }
            // The sections (docs/adr/4340): each one's distance from the first and its cut and fill areas.
            if !c.sections.is_empty() {
                let rows: Vec<Vec<String>> = c
                    .sections
                    .iter()
                    .enumerate()
                    .map(|(k, s)| {
                        vec![
                            (k + 1).to_string(),
                            grouped(s.station, 2),
                            grouped(s.cut_area, 2),
                            grouped(s.fill_area, 2),
                        ]
                    })
                    .collect();
                let table = crate::road_dialog::table(
                    vec![
                        num("Kesit", 60.0),
                        num("Uzaklık m", 100.0),
                        num("Kazı alanı m²", 120.0),
                        num("Dolgu alanı m²", 120.0),
                    ],
                    rows,
                    "Kesit yok",
                );
                parts.push((
                    SECTIONS,
                    said(
                        table,
                        format!(
                            "{} kesit; alanlar iki yüzeyin ortak yerinde kesin.",
                            c.sections.len()
                        ),
                    ),
                ));
            }
            // The material zones (docs/adr/3540): each zone and the rest (the exact TIN's cut and fill, the factors, the
            // adjusted net), then the quantity by material.
            if let (false, Some(r), Some(t)) = (c.zones.is_empty(), &c.rest, &c.zoned) {
                let factors = |a: &kentos_commands::TerrainAdjusted| {
                    format!("{} / {}", a.cut_factor, a.fill_factor).replace('.', ",")
                };
                let mut rows: Vec<Vec<String>> = c
                    .zones
                    .iter()
                    .map(|z| {
                        vec![
                            z.name.clone(),
                            grouped(z.covered, 2),
                            grouped(z.cut, 2),
                            grouped(z.fill, 2),
                            factors(&z.adjusted),
                            grouped(z.adjusted.net, 2),
                        ]
                    })
                    .collect();
                rows.push(vec![
                    "Bölge dışı".to_owned(),
                    grouped(r.covered, 2),
                    grouped(r.cut, 2),
                    grouped(r.fill, 2),
                    factors(&r.adjusted),
                    grouped(r.adjusted.net, 2),
                ]);
                let table = crate::road_dialog::table(
                    vec![
                        col("Malzeme", 120.0),
                        num("Alan m²", 90.0),
                        num("Kazı m³", 90.0),
                        num("Dolgu m³", 90.0),
                        num("Katsayı", 90.0),
                        num("Net m³", 90.0),
                    ],
                    rows,
                    "Bölge yok",
                );
                let words = format!(
                    "Kazı ve dolgu kesin TIN'in, net katsayılı. Malzemeye göre katsayılı kazı {} · dolgu {} m³.",
                    grouped(t.cut, 2),
                    grouped(t.fill, 2)
                );
                parts.push((MATERIALS, said(table, words)));
            }
        }
        Read::Quality(q) => {
            figures.push(("Köşe".into(), q.corners.to_string()));
            figures.push(("Üçgen".into(), q.triangles.to_string()));
            if let Some(n) = q.near_count {
                figures.push(("Yakın çift".into(), n.to_string()));
            }
            if let Some(n) = q.spike_count {
                figures.push(("Sivri nokta".into(), n.to_string()));
            }
            let mut rows = Vec::new();
            if let Some(s) = q.spikes.first() {
                rows.push((
                    "En büyük sivri".to_owned(),
                    format!("{:+.2} m", s.difference),
                ));
            }
            if let Some(g) = &q.gaps {
                rows.push((
                    "Boşluk".into(),
                    format!(
                        "{} üçgen · {} · en uzun kenar {:.2} m",
                        g.triangles,
                        m2(g.area),
                        g.longest
                    ),
                ));
            }
            if !rows.is_empty() {
                parts.push((SUMMARY, facts(rows)));
            }
            if let Some(c) = &q.checkpoints {
                let name = |k: &'a kentos_commands::TerrainCheckClass| -> &'a str {
                    if k.class.is_empty() {
                        if std::ptr::eq(k, &c.all) {
                            "Hepsi"
                        } else {
                            "Sınıfsız"
                        }
                    } else {
                        k.class.as_str()
                    }
                };
                let d = |v: f64| format!("{v:.3}").replace('.', ",");
                let rows = c
                    .classes
                    .iter()
                    .chain(std::iter::once(&c.all))
                    .map(|k| {
                        vec![
                            name(k).to_owned(),
                            k.count.to_string(),
                            format!("{}{}", if k.mean >= 0.0 { "+" } else { "" }, d(k.mean)),
                            d(k.std),
                            d(k.rmse),
                            k.outside.to_string(),
                        ]
                    })
                    .collect();
                let table = crate::road_dialog::table(
                    vec![
                        col("Sınıf", 120.0),
                        num("Sayı", 60.0),
                        num("Ort.", 80.0),
                        num("Std.", 80.0),
                        num("RMSE", 80.0),
                        num("Dışı", 60.0),
                    ],
                    rows,
                    "Kontrol noktası yok",
                );
                parts.push((CHECKS, said(table, "Nokta − yüzey, m.".to_owned())));
                // Beyond one RMSE (docs/adr/0945): the robust measures and the RMSE's 95 % interval.
                let robust: Vec<Vec<String>> = c
                    .classes
                    .iter()
                    .chain(std::iter::once(&c.all))
                    .filter_map(|k| {
                        let r = k.robust.as_ref()?;
                        Some(vec![
                            name(k).to_owned(),
                            d(r.median),
                            d(r.nmad),
                            d(r.q68),
                            d(r.q95),
                            format!("{}–{}", d(r.rmse_low), d(r.rmse_high)),
                            r.combined.map_or_else(|| "—".to_owned(), d),
                            r.far.to_string(),
                        ])
                    })
                    .collect();
                if !robust.is_empty() {
                    let pick = |ks: &[usize]| {
                        robust
                            .iter()
                            .map(|r| ks.iter().map(|k| r[*k].clone()).collect())
                            .collect::<Vec<Vec<String>>>()
                    };
                    // The robust measures and the RMSE's own interval side by side, one table (Höhle and Höhle).
                    let table = crate::road_dialog::table(
                        vec![
                            col("Sınıf", 110.0),
                            num("Medyan", 70.0),
                            num("NMAD", 70.0),
                            num("%68", 64.0),
                            num("%95", 64.0),
                            num("Uzak", 54.0),
                            num("RMSE %95 aralığı", 130.0),
                            num("Kontrolle birleşik", 130.0),
                        ],
                        pick(&[0, 1, 2, 3, 4, 7, 5, 6]),
                        "",
                    );
                    let words = "Medyan, NMAD ve |fark|'ın yüzdelikleri aykırı noktalardan etkilenmez (Höhle ve Höhle); RMSE'nin aralığı χ² ile %95, birleşik hata kontrol noktalarının doğruluğuyla.";
                    parts.push((ROBUST, said(table, words.to_owned())));
                }
            }
        }
        Read::Coverage(c, _) => {
            figures.push(("Yüzeyin alanı".into(), m2(c.surface_area)));
            if let Some(share) = c.share {
                figures.push((
                    "Örtülen".into(),
                    format!("%{}", format!("{:.1}", 100.0 * share).replace('.', ",")),
                ));
            }
            figures.push((
                "Tahmin edilen".into(),
                format!(
                    "%{}",
                    format!("{:.1}", 100.0 * c.estimated_share).replace('.', ",")
                ),
            ));
            figures.push((
                "En uzun kenar".into(),
                format!("{} m", grouped(c.longest_edge, 2)),
            ));
            let mut rows = Vec::new();
            if let (Some(b), Some(gap), Some(out)) = (c.boundary_area, c.no_data, c.outside) {
                rows.push(("Sınırın alanı".to_owned(), m2(b)));
                rows.push(("Örtülen".to_owned(), m2(c.covered)));
                rows.push(("Verisiz".to_owned(), m2(gap)));
                rows.push(("Sınırın dışında".to_owned(), m2(out)));
            }
            rows.push((
                "Tahmin edilen".to_owned(),
                format!("{} · {} üçgen", m2(c.estimated), c.estimated_triangles),
            ));
            parts.push((SUMMARY, facts(rows)));
        }
        Read::Density(d, _) => {
            figures.push(("Sayılan hücre".into(), d.cells.to_string()));
            figures.push((
                "Noktalı".into(),
                format!(
                    "%{}",
                    format!("{:.1}", 100.0 * d.share.unwrap_or(0.0)).replace('.', ",")
                ),
            ));
            figures.push(("Boş".into(), d.empty.to_string()));
            if let Some(n) = d.sparse {
                figures.push(("Seyrek".into(), n.to_string()));
            }
            let mut rows = vec![
                (
                    "Izgara".to_owned(),
                    format!("{} × {} · {} m hücre", d.columns, d.rows, d.cell),
                ),
                ("Sayılan alan".to_owned(), m2(d.area)),
                ("Noktalı hücre".to_owned(), d.filled.to_string()),
                (
                    "Nokta".to_owned(),
                    format!("{} · en kalabalık hücre {}", d.points, d.most),
                ),
                (
                    "Ortalama yoğunluk".to_owned(),
                    d.mean.map_or("—".into(), |m| {
                        format!("{m:.3} nokta/m²").replace('.', ",")
                    }),
                ),
            ];
            if let Some(meets) = d.meets {
                rows.push((
                    "İstenen pay".into(),
                    if meets {
                        "tutuyor".into()
                    } else {
                        "tutmuyor".into()
                    },
                ));
            }
            parts.push((SUMMARY, facts(rows)));
        }
    }
    figures.truncate(4);
    // The facts first, then the reading's own lists.
    parts.sort_by_key(|(name, _)| *name != SUMMARY);
    (figures, parts)
}

fn band_table<'a>(b: &TerrainBands, unit: &str, whole: f64) -> Element<'a, Message> {
    use crate::road_dialog::{col, num};
    let d = |v: f64| crate::road_dialog::grouped(v, 2);
    let n = b.bands.len();
    let mut rows: Vec<Vec<Element<'a, Message>>> = b
        .bands
        .iter()
        .enumerate()
        .map(|(k, band)| {
            let swatch = iced::widget::canvas(Swatch { k, n }).width(10).height(10);
            let share = if whole > 0.0 {
                band.area / whole * 100.0
            } else {
                0.0
            };
            vec![
                row![
                    swatch,
                    kentos_ui::widget::table::text(format!("{} … {} {unit}", band.from, band.to))
                ]
                .spacing(6)
                .align_y(iced::alignment::Vertical::Center)
                .into(),
                kentos_ui::widget::table::number(d(band.area)).into(),
                kentos_ui::widget::table::number(d(band.surface_area)).into(),
                kentos_ui::widget::table::number(format!(
                    "%{}",
                    format!("{share:.1}").replace('.', ",")
                ))
                .into(),
            ]
        })
        .collect();
    let plain = |what: &'a str, area: f64| -> Vec<Element<'a, Message>> {
        vec![
            kentos_ui::widget::table::text(what).into(),
            kentos_ui::widget::table::number(d(area)).into(),
            kentos_ui::widget::table::text("").into(),
            kentos_ui::widget::table::text("").into(),
        ]
    };
    if b.outside_area > 0.0 {
        rows.push(plain("Aralık dışı", b.outside_area));
    }
    if b.flat_area > 0.0 {
        rows.push(plain("Düz (bakısız)", b.flat_area));
    }
    crate::road_dialog::table_of(
        vec![
            col("Aralık", 116.0),
            num("Alan (m²)", 84.0),
            num("3B (m²)", 84.0),
            num("Pay", 48.0),
        ],
        rows,
        "Aralık yok",
    )
}

//! Shared test data: the sample answers, the host's inputs a sheet is drawn
//! with in the fixtures and the renders (a project in TUREF / TM33, a
//! legend, parcels for tables, a parcel's corners), and a made-up map
//! content for the renders (blocks and parcels, so a reviewer sees a sheet
//! rather than grey boxes). Nothing here is the core's: the core never
//! draws a map's content.
#![allow(dead_code)]

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use kentos_sheet::display::*;
use kentos_sheet::geodesy::TmParams;
use kentos_sheet::profile::Capabilities;
use kentos_sheet::svg::MapImage;
use kentos_sheet::template::VariableValue;
use kentos_sheet::*;

pub fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..")
}

pub fn fixtures() -> PathBuf {
    root().join("fixtures/sheet/v1")
}

/// Whether the golden files are rewritten (`KENTOS_WRITE_SHEET=1`).
pub fn writing() -> bool {
    std::env::var("KENTOS_WRITE_SHEET").is_ok_and(|v| v == "1")
}

pub fn tm33() -> TmParams {
    TmParams {
        central_meridian: 33.0,
        scale_factor: 1.0,
        false_easting: 500_000.0,
        false_northing: 0.0,
        semi_major: 6_378_137.0,
        inverse_flattening: 298.257_222_101,
    }
}

/// Where the sample maps look: Çankaya, Ankara, in TUREF / TM33.
pub const CENTER: GroundPoint = GroundPoint {
    x: 487_150.0,
    y: 4_417_820.0,
};

fn text(s: &str) -> VarValue {
    VarValue::Text(s.to_owned())
}

/// Answers to every question the system templates ask.
pub fn sample_values() -> Vec<VariableValue> {
    [
        ("kurum", "Çankaya Belediyesi"),
        ("birim", "İmar ve Şehircilik Md."),
        ("referans", "TS EN ISO 7200"),
        ("onaylayan", "Dr. S. Aksoy"),
        ("cizim_no", "ÇNK-2026-014"),
        ("revizyon", "A"),
        ("pafta_no", "J29-c-23-b"),
        ("kontrol_eden", "A. Kaya"),
        ("il", "Ankara"),
        ("ilce", "Çankaya"),
        ("mahalle", "Kızılay"),
        ("ada", "1234"),
        ("parsel", "7"),
        ("aciklama", "Parselin köşe noktaları tescilli koordinatlarla zemine aplike edilmiş, işaretlenmiştir. Ölçüler metre cinsindendir."),
        ("rapor_basligi", "Teknik Rapor"),
        ("buro", "Ada Mimarlık"),
        ("buro_adres", "Atatürk Blv. 101/7 Çankaya · (0312) 000 00 00"),
        ("yapi_sahibi", "Örnek Yapı Kooperatifi"),
        ("mimar", "Mimar E. Yıldız"),
        ("plan_adi", "Kızılay Kentsel Yenileme Alanı"),
        ("hazirlayan", "Şehir Plancısı Z. Er"),
        ("meclis_tarihi", "2026-09-04"),
        ("meclis_karar_no", "2026/412"),
        ("onay_makami", "Ankara Büyükşehir Belediyesi"),
        ("onay_tarihi", "2026-09-25"),
        ("harita_basligi", "Kızılay ve çevresi: arazi kullanımı"),
        ("alt_baslik", "Parsel bazında mevcut kullanım, 2026"),
        ("kaynak", "Çankaya Belediyesi CBS, 2026"),
        (
            "plan_notlari",
            "1. Plan onama sınırı içinde kalan alanlarda bu notlar geçerlidir.\n2. Planda gösterilmeyen konularda ilgili mevzuat uygulanır.\n3. Yapı yaklaşma mesafeleri, parsel sınırlarına göre plan lejantında gösterildiği gibidir.\n4. Parsellerin cephe aldığı yolların kotları uygulama projesiyle belirlenir.\n5. Bu notlar örnek metindir; bir yönetmeliğe uygunluk iddiası taşımaz.",
        ),
    ]
    .iter()
    .map(|(n, v)| VariableValue {
        name: (*n).to_owned(),
        value: text(v),
    })
    .collect()
}

/// A rectangle symbol with a fill and a thin outline.
fn patch(fill: &str) -> LegendSymbol {
    LegendSymbol::Patch(PatchSymbol {
        fill: Some(fill.to_owned()),
        stroke: Some(Stroke::solid("#3c3c3c", 180)),
        hatch: None,
    })
}

/// The land uses the sample map colours blocks with (also its legend).
pub const USES: [(&str, &str); 6] = [
    ("Konut", "#f3d9a4"),
    ("Ticaret", "#e9a3a0"),
    ("Konut + ticaret", "#f0bc8c"),
    ("Eğitim", "#a9c9ea"),
    ("Park ve yeşil alan", "#b8dca2"),
    ("Sağlık", "#c9b3dd"),
];

pub fn legend_entries() -> Vec<LegendEntryInput> {
    let mut out: Vec<LegendEntryInput> = USES
        .iter()
        .map(|(l, c)| LegendEntryInput {
            layer: format!("kullanim-{l}"),
            label: (*l).to_owned(),
            group: Some("Arazi kullanımı".to_owned()),
            symbol: patch(c),
            in_map: true,
            in_atlas: true,
        })
        .collect();
    out.push(LegendEntryInput {
        layer: "parsel".into(),
        label: "Parsel sınırı".into(),
        group: Some("Kadastro".into()),
        symbol: LegendSymbol::Line(LineSymbol {
            stroke: Stroke::solid("#1f1f1f", 250),
        }),
        in_map: true,
        in_atlas: true,
    });
    out.push(LegendEntryInput {
        layer: "bina".into(),
        label: "Bina".into(),
        group: Some("Kadastro".into()),
        symbol: patch("#9a9a9a"),
        in_map: true,
        in_atlas: true,
    });
    out.push(LegendEntryInput {
        layer: "yol".into(),
        label: "Yol".into(),
        group: Some("Ulaşım".into()),
        symbol: patch("#ffffff"),
        in_map: true,
        in_atlas: true,
    });
    out.push(LegendEntryInput {
        layer: "nirengi".into(),
        label: "Poligon noktası".into(),
        group: Some("Ulaşım".into()),
        symbol: LegendSymbol::Marker(MarkerSymbol {
            shape: MarkerShape::Triangle,
            size: 2_600,
            fill: None,
            stroke: Some(Stroke::solid("#1f1f1f", 200)),
        }),
        in_map: true,
        in_atlas: true,
    });
    out
}

/// Parcels for an attribute table.
pub fn parcels() -> Vec<FeatureInput> {
    let rows: [(&str, &str, f64, f64); 8] = [
        ("1", "Konut", 612.48, 612.50),
        ("2", "Konut", 587.11, 587.00),
        ("3", "Konut + ticaret", 645.92, 646.00),
        ("4", "Ticaret", 702.35, 702.40),
        ("5", "Konut", 598.77, 598.80),
        ("6", "Park ve yeşil alan", 1_210.05, 1_210.00),
        ("7", "Konut", 633.60, 633.60),
        ("8", "Eğitim", 2_045.18, 2_045.20),
    ];
    rows.iter()
        .map(|(no, n, area, tapu)| FeatureInput {
            id: format!("p{no}"),
            attributes: vec![
                Attribute {
                    name: "parsel_no".into(),
                    value: VarValue::Number(no.parse().unwrap_or(0.0)),
                },
                Attribute {
                    name: "nitelik".into(),
                    value: text(n),
                },
                Attribute {
                    name: "tapu_alani".into(),
                    value: VarValue::Number(*tapu),
                },
                Attribute {
                    name: "ad".into(),
                    value: text(&format!("1234/{no}")),
                },
            ],
            area: Some(*area),
            length: None,
            in_map: true,
            in_atlas: true,
        })
        .collect()
}

/// A parcel's corners (1234 ada 7 parsel).
pub fn corners() -> Vec<CoordPoint> {
    let pts = [
        ("P1", 487_131.27, 4_417_842.51),
        ("P2", 487_163.84, 4_417_846.02),
        ("P3", 487_168.40, 4_417_812.77),
        ("P4", 487_150.12, 4_417_801.45),
        ("P5", 487_135.66, 4_417_803.98),
        ("P6", 487_129.05, 4_417_821.30),
    ];
    pts.iter()
        .map(|(n, x, y)| CoordPoint {
            name: Some((*n).to_owned()),
            x: *x,
            y: *y,
            z: None,
        })
        .collect()
}

/// The inputs a host would give for a sheet of `book`.
pub fn sample_inputs(book: &SheetBook, sheet_id: &str, georeferenced: bool) -> RenderInputs {
    let sheet = book.sheet(sheet_id).expect("the sheet");
    let mut inputs = RenderInputs {
        mode: RenderMode::Export,
        project: ProjectInfo {
            name: "Kızılay 1234 ada düzenlemesi".into(),
            user: "M. Demir".into(),
            date: "2026-10-02".into(),
            crs_name: String::new(),
        },
        capabilities: Capabilities {
            georeferenced,
            attribute_layers: true,
            plot_scale: None,
        },
        crs: georeferenced.then(|| CrsInfo {
            name: "TUREF / TM33".into(),
            tm: Some(tm33()),
        }),
        ..RenderInputs::default()
    };
    for it in &sheet.items {
        match &it.kind {
            ItemKind::Legend(_) => inputs.legends.push(LegendInput {
                item: it.id.clone(),
                entries: legend_entries(),
            }),
            ItemKind::Table(t) if matches!(t.source, TableSource::Layer(_)) => {
                inputs.tables.push(TableInput {
                    item: it.id.clone(),
                    features: parcels(),
                })
            }
            ItemKind::CoordinateList(_) => inputs.coordinates.push(CoordinateInput {
                item: it.id.clone(),
                points: corners(),
                closed: true,
                area: None,
                objects: Some(1),
                missing: None,
            }),
            _ => {}
        }
    }
    inputs
}

// ── A made-up map content for the renders ────────────────────────────────

/// Blocks 90 × 60 m with 14 m roads, each in six parcels with a building, coloured by a land use.
fn block_use(i: i64, j: i64) -> usize {
    let h = (i.wrapping_mul(73_856_093) ^ j.wrapping_mul(19_349_663)).rem_euclid(97);
    match h {
        0..=44 => 0,
        45..=59 => 1,
        60..=74 => 2,
        75..=83 => 3,
        84..=92 => 4,
        _ => 5,
    }
}

const BLOCK_W: f64 = 90.0;
const BLOCK_H: f64 = 60.0;
const ROAD: f64 = 14.0;
const BASE: [f64; 2] = [487_000.0, 4_417_700.0];

/// The map's content as an SVG picture of its clip, from its view.
pub fn map_svg(m: &MapPrim) -> Option<String> {
    let c = m.view.center?;
    let (w, h) = (f64::from(m.clip.width), f64::from(m.clip.height));
    let k = 1_000_000.0 / f64::from(m.view.scale.max(1));
    let rad = f64::from(m.view.rotation) / 1000.0 * std::f64::consts::PI / 180.0;
    let (s, co) = (rad.sin(), rad.cos());
    let to = |g: [f64; 2]| {
        let (a, b) = ((g[0] - c.x) * k, (g[1] - c.y) * k);
        [w / 2.0 + a * co + b * s, h / 2.0 + a * s - b * co]
    };
    let ext = m.extent?;
    let pitch = [BLOCK_W + ROAD, BLOCK_H + ROAD];
    let i0 = ((ext[0] - BASE[0]) / pitch[0]).floor() as i64 - 1;
    let i1 = ((ext[2] - BASE[0]) / pitch[0]).ceil() as i64 + 1;
    let j0 = ((ext[1] - BASE[1]) / pitch[1]).floor() as i64 - 1;
    let j1 = ((ext[3] - BASE[1]) / pitch[1]).ceil() as i64 + 1;
    if (i1 - i0) * (j1 - j0) > 40_000 {
        return None;
    }
    let detailed = m.view.scale <= 2_500;
    let mut out = String::new();
    let _ = write!(
        out,
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="{w}" height="{h}" viewBox="0 0 {w} {h}"><rect width="{w}" height="{h}" fill="#fbfaf7"/>"##
    );
    let poly = |out: &mut String, pts: &[[f64; 2]], fill: &str, stroke: &str, sw: f64| {
        let mut d = String::new();
        for (n, p) in pts.iter().enumerate() {
            let q = to(*p);
            let _ = write!(
                d,
                "{}{:.0} {:.0}",
                if n == 0 { "M" } else { "L" },
                q[0],
                q[1]
            );
        }
        let _ = write!(
            out,
            r#"<path d="{d}Z" fill="{fill}" stroke="{stroke}" stroke-width="{sw:.0}"/>"#
        );
    };
    for i in i0..=i1 {
        for j in j0..=j1 {
            let x0 = BASE[0] + i as f64 * pitch[0];
            let y0 = BASE[1] + j as f64 * pitch[1];
            let rect = |a: f64, b: f64, c: f64, d: f64| [[a, b], [c, b], [c, d], [a, d]];
            let use_ = block_use(i, j);
            let fill = USES[use_].1;
            if !detailed {
                poly(
                    &mut out,
                    &rect(x0, y0, x0 + BLOCK_W, y0 + BLOCK_H),
                    fill,
                    "#6d6d6d",
                    120.0,
                );
                continue;
            }
            // Six parcels.
            let pw = BLOCK_W / 3.0;
            let ph = BLOCK_H / 2.0;
            for pi in 0..3 {
                for pj in 0..2 {
                    let a = x0 + pi as f64 * pw;
                    let b = y0 + pj as f64 * ph;
                    poly(
                        &mut out,
                        &rect(a, b, a + pw, b + ph),
                        fill,
                        "#2b2b2b",
                        220.0,
                    );
                    if use_ != 4 {
                        let (ia, ib) = (a + 4.0, if pj == 0 { b + 5.0 } else { b + 9.0 });
                        poly(
                            &mut out,
                            &rect(ia, ib, a + pw - 4.0, ib + ph - 14.0),
                            "#a6a39d",
                            "#5f5c57",
                            140.0,
                        );
                    }
                    let no = (pi + 3 * pj + 1) as i64;
                    let p = to([a + pw / 2.0, b + ph / 2.0]);
                    let size = 2_200.0;
                    let _ = write!(
                        out,
                        r##"<text x="{:.0}" y="{:.0}" font-family="Barlow, sans-serif" font-size="{size:.0}" font-weight="500" fill="#1f1f1f" text-anchor="middle">{no}</text>"##,
                        p[0],
                        p[1] + size / 3.0
                    );
                }
            }
            let p = to([x0 + BLOCK_W / 2.0, y0 + BLOCK_H + ROAD / 2.0]);
            let _ = write!(
                out,
                r##"<text x="{:.0}" y="{:.0}" font-family="Barlow, sans-serif" font-size="2000" font-style="italic" fill="#6a6a6a" text-anchor="middle">{} ada</text>"##,
                p[0],
                p[1] + 700.0,
                1200 + (i - i0) * 10 + (j - j0)
            );
        }
    }
    // The sample parcel, 1234 ada 7 parsel, and its corners.
    if m.view.scale <= 2_500 {
        let pts = corners();
        let mut d = String::new();
        for (n, p) in pts.iter().enumerate() {
            let q = to([p.x, p.y]);
            let _ = write!(
                d,
                "{}{:.0} {:.0}",
                if n == 0 { "M" } else { "L" },
                q[0],
                q[1]
            );
        }
        let _ = write!(
            out,
            r##"<path d="{d}Z" fill="#ffffff" fill-opacity="0.55" stroke="#c0161b" stroke-width="500"/>"##
        );
        for p in &pts {
            let q = to([p.x, p.y]);
            let _ = write!(
                out,
                r##"<circle cx="{:.0}" cy="{:.0}" r="700" fill="#ffffff" stroke="#c0161b" stroke-width="300"/><text x="{:.0}" y="{:.0}" font-family="Barlow, sans-serif" font-size="2200" font-weight="600" fill="#c0161b">{}</text>"##,
                q[0],
                q[1],
                q[0] + 1_200.0,
                q[1] - 1_000.0,
                p.name.as_deref().unwrap_or("")
            );
        }
    }
    out.push_str("</svg>");
    Some(out)
}

/// The same made-up content as vectors for the PDF (`MapContent::Vector`): blocks and parcels,
/// buildings, the parcels' numbers and the blocks' names, and the sample parcel with its corners,
/// a layer each, on the ground.
pub fn map_vector(m: &MapPrim) -> Option<kentos_sheet::pdf::MapContent> {
    use kentos_sheet::pdf::*;
    m.view.center?;
    let ext = m.extent?;
    let pitch = [BLOCK_W + ROAD, BLOCK_H + ROAD];
    let i0 = ((ext[0] - BASE[0]) / pitch[0]).floor() as i64 - 1;
    let i1 = ((ext[2] - BASE[0]) / pitch[0]).ceil() as i64 + 1;
    let j0 = ((ext[1] - BASE[1]) / pitch[1]).floor() as i64 - 1;
    let j1 = ((ext[3] - BASE[1]) / pitch[1]).ceil() as i64 + 1;
    if (i1 - i0) * (j1 - j0) > 40_000 {
        return None;
    }
    let stroke = |color: &str, width: f64| MapStroke {
        color: color.into(),
        width,
        dash: Vec::new(),
        cap: LineCap::Round,
        join: LineJoin::Round,
    };
    let rect = |a: f64, b: f64, c: f64, d: f64| vec![[a, b], [c, b], [c, d], [a, d]];
    let label =
        |at: [f64; 2], text: String, size: f64, weight: u16, italic: bool, color: &str| MapText {
            at,
            text,
            size,
            rotation: 0.0,
            color: color.into(),
            font: "barlow".into(),
            weight,
            italic,
            anchor: TextAnchor::CenterMiddle,
            halo: Some("#ffffff".into()),
        };
    let layer = |id: &str, name: &str| MapLayerContent {
        id: id.into(),
        name: name.into(),
        paths: Vec::new(),
        texts: Vec::new(),
    };
    let (mut parcels, mut buildings, mut numbers, mut names) = (
        layer("parsel", "Parseller"),
        layer("yapi", "Yapılar"),
        layer("parsel-no", "Parsel numaraları"),
        layer("ada", "Ada adları"),
    );
    for i in i0..=i1 {
        for j in j0..=j1 {
            let x0 = BASE[0] + i as f64 * pitch[0];
            let y0 = BASE[1] + j as f64 * pitch[1];
            let fill = USES[block_use(i, j)].1;
            let (pw, ph) = (BLOCK_W / 3.0, BLOCK_H / 2.0);
            for pi in 0..3 {
                for pj in 0..2 {
                    let a = x0 + f64::from(pi) * pw;
                    let b = y0 + f64::from(pj) * ph;
                    parcels.paths.push(MapPath {
                        points: rect(a, b, a + pw, b + ph),
                        closed: true,
                        holes: Vec::new(),
                        stroke: Some(stroke("#2b2b2b", 0.22)),
                        fill: Some(fill.into()),
                    });
                    if block_use(i, j) != 4 {
                        let (ia, ib) = (a + 4.0, if pj == 0 { b + 5.0 } else { b + 9.0 });
                        buildings.paths.push(MapPath {
                            points: rect(ia, ib, a + pw - 4.0, ib + ph - 14.0),
                            closed: true,
                            holes: Vec::new(),
                            stroke: Some(stroke("#5f5c57", 0.14)),
                            fill: Some("#a6a39d".into()),
                        });
                    }
                    numbers.texts.push(label(
                        [a + pw / 2.0, b + ph / 2.0],
                        (pi + 3 * pj + 1).to_string(),
                        2.2,
                        500,
                        false,
                        "#1f1f1f",
                    ));
                }
            }
            names.texts.push(label(
                [x0 + BLOCK_W / 2.0, y0 + BLOCK_H + ROAD / 2.0],
                format!("{} ada", 1200 + (i - i0) * 10 + (j - j0)),
                2.0,
                400,
                true,
                "#6a6a6a",
            ));
        }
    }
    let mut corners_layer = layer("nirengi", "Parsel köşeleri");
    let pts = corners();
    corners_layer.paths.push(MapPath {
        points: pts.iter().map(|p| [p.x, p.y]).collect(),
        closed: true,
        holes: Vec::new(),
        stroke: Some(stroke("#c0161b", 0.5)),
        fill: Some("#ffffff8c".into()),
    });
    for p in &pts {
        let mut t = label(
            [p.x + 1.2, p.y + 1.0],
            p.name.clone().unwrap_or_default(),
            2.2,
            600,
            false,
            "#c0161b",
        );
        t.anchor = TextAnchor::LeftBaseline;
        corners_layer.texts.push(t);
    }
    Some(MapContent::Vector(VectorMap {
        layers: vec![parcels, buildings, numbers, names, corners_layer],
    }))
}

/// The drawing's TrueType faces (the desktop's copies), as the hosts pass them to the PDF.
pub fn pdf_fonts() -> Vec<kentos_sheet::pdf::PdfFont> {
    let dir = root().join("apps/desktop/assets/fonts/drawing");
    let files = [
        (
            "architects-daughter",
            400,
            false,
            "ArchitectsDaughter-400.ttf",
        ),
        ("arimo", 400, false, "Arimo-400.ttf"),
        ("arimo", 500, false, "Arimo-500.ttf"),
        ("arimo", 600, false, "Arimo-600.ttf"),
        ("arimo", 700, false, "Arimo-700.ttf"),
        ("barlow", 400, false, "Barlow-400.ttf"),
        ("barlow", 400, true, "Barlow-400-italic.ttf"),
        ("barlow", 500, false, "Barlow-500.ttf"),
        ("barlow", 600, false, "Barlow-600.ttf"),
        ("courier-prime", 400, false, "CourierPrime-400.ttf"),
        ("courier-prime", 400, true, "CourierPrime-400-italic.ttf"),
        ("courier-prime", 700, false, "CourierPrime-700.ttf"),
        ("overpass", 400, false, "Overpass-400.ttf"),
        ("overpass", 500, false, "Overpass-500.ttf"),
        ("overpass", 600, false, "Overpass-600.ttf"),
        ("overpass", 700, false, "Overpass-700.ttf"),
        ("plex-mono", 400, false, "IBMPlexMono-400.ttf"),
        ("plex-mono", 500, false, "IBMPlexMono-500.ttf"),
        ("quicksand", 400, false, "Quicksand-400.ttf"),
        ("quicksand", 500, false, "Quicksand-500.ttf"),
        ("quicksand", 600, false, "Quicksand-600.ttf"),
        ("quicksand", 700, false, "Quicksand-700.ttf"),
    ];
    files
        .iter()
        .map(|(font, weight, italic, file)| kentos_sheet::pdf::PdfFont {
            font: (*font).into(),
            weight: *weight,
            italic: *italic,
            data: std::fs::read(dir.join(file)).expect("the drawing's face"),
        })
        .collect()
}

fn b64(bytes: &[u8]) -> String {
    kentos_sheet::template::base64_encode(bytes)
}

/// The made-up content of every map frame, as data URLs.
pub fn map_images(list: &DisplayList) -> Vec<MapImage> {
    list.prims
        .iter()
        .filter_map(|p| match p {
            Prim::Map(m) => map_svg(m).map(|s| MapImage {
                item: m.item.clone(),
                href: format!("data:image/svg+xml;base64,{}", b64(s.as_bytes())),
            }),
            _ => None,
        })
        .collect()
}

/// An HTML page showing an SVG at 1 px per 0.1 mm with the drawing's own faces (the desktop's TrueType copies).
pub fn html(svg: &str, width_mm: f64, height_mm: f64) -> String {
    let fonts = root().join("apps/desktop/assets/fonts/drawing");
    let mut css = String::new();
    let faces = [
        ("Barlow", "Barlow", &[400, 500, 600][..]),
        ("Arimo", "Arimo", &[400, 500, 600, 700][..]),
        ("Overpass", "Overpass", &[400, 500, 600, 700][..]),
        ("Quicksand", "Quicksand", &[400, 500, 600, 700][..]),
        ("Courier Prime", "CourierPrime", &[400, 700][..]),
        ("IBM Plex Mono", "IBMPlexMono", &[400, 500][..]),
        ("Architects Daughter", "ArchitectsDaughter", &[400][..]),
    ];
    for (family, file, weights) in faces {
        for w in weights {
            let path = fonts.join(format!("{file}-{w}.ttf"));
            let _ = writeln!(
                css,
                "@font-face {{ font-family: '{family}'; font-weight: {w}; font-style: normal; src: url('file://{}'); }}",
                path.display()
            );
        }
    }
    let _ = writeln!(
        css,
        "@font-face {{ font-family: 'Barlow'; font-weight: 400; font-style: italic; src: url('file://{}'); }}",
        fonts.join("Barlow-400-italic.ttf").display()
    );
    let body = svg.split_once("?>").map_or(svg, |(_, b)| b);
    format!(
        "<!doctype html><html><head><meta charset=\"utf-8\"><style>{css}html,body{{margin:0;background:#8a8f96}} svg{{display:block;width:{}px;height:{}px;background:#fff;box-shadow:0 0 0 1px #6b7078}}</style></head><body>{body}</body></html>\n",
        (width_mm * 10.0).round(),
        (height_mm * 10.0).round()
    )
}

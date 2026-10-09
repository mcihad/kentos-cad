//! The drawing a new project starts as (the web's `model/newProject.ts` and
//! `model/standardLayers.ts`): an empty drawing with a CAD project's
//! technical drawing layers or the standard layer tree of a cadastral sheet,
//! the chosen coordinate system, plot scale, type and typeface, and the
//! default units. Its anchor is the zone's work-area
//! centre (the web's `workAreaCentre`), since no object exists yet to anchor
//! it. It opens on its start view, which stays its home view (`start_view`,
//! docs/adr/0165 §3).
//!
//! The web records the drawings its code builds for a few choices
//! (fixtures/project/v1/new-project.json); the test below builds the same
//! drawings here and compares them field by field.

use std::f64::consts::PI;

use kentos_contracts::{
    AngleUnit, AreaUnit, Bounds, DOCUMENT_FORMAT, DOCUMENT_VERSION, DocumentSnapshotV1,
    DrawingFont, DrawingUnit, LabelInk, LabelPlacement, LabelStyle, LayerNode, LayerNodeType,
    LayerStyle, LineType, PointStyle, PointSymbol, ProjectSettings, ProjectStyles, Vec2, Workspace,
};
use kentos_domain::default_style;

use crate::crs::{System, project_lat_lon, system};
use crate::provinces::{Province, province};

/// The name a new project is offered with.
pub const NEW_PROJECT_NAME: &str = "Yeni proje";
/// The layer new objects go to at first.
pub const STANDARD_ACTIVE_LAYER: &str = "taslak";
/// The layer a CAD project's new objects go to at first.
pub const CAD_ACTIVE_LAYER: &str = "cizim";
/// Plot scales offered (the web's `PLOT_SCALES`).
pub const PLOT_SCALES: [f64; 5] = [500.0, 1000.0, 2000.0, 5000.0, 25_000.0];

/// Where the work area of a zone lies (the web's `WORK_*`): the middle of
/// Türkiye's longitudes and latitudes, a northing near its centre.
const WORK_LON: f64 = 35.0;
const WORK_LAT: f64 = 39.0;
const WORK_NORTHING: f64 = 4_320_000.0;

/// What the Yeni proje window asks.
#[derive(Debug, Clone, PartialEq)]
pub struct NewProject {
    pub name: String,
    pub srid: u32,
    pub plot_scale: f64,
    pub workspace: Workspace,
    pub drawing_font: DrawingFont,
    /// The province (plate code) whose centre it opens on (docs/adr/0165 §3); none: the zone's work area.
    pub province: Option<u32>,
    /// A local project's drawing unit (docs/adr/0165 §2); none: metres.
    pub drawing_unit: Option<DrawingUnit>,
}

/// The default units of a project (the web's `PROJECT_SETTINGS_DEFAULTS`).
pub fn default_settings(srid: u32) -> ProjectSettings {
    ProjectSettings {
        srid,
        length_decimals: 3,
        area_decimals: 2,
        area_unit: AreaUnit::M2,
        angle_unit: AngleUnit::Grad,
        plot_scale: 1000.0,
        workspace: Some(Workspace::Gis),
        drawing_font: Some(DrawingFont::Barlow),
        drawing_unit: None,
        second_srid: None,
        custom_crs: None,
        second_custom_crs: None,
        datum_transforms: Vec::new(),
        layer_states: Vec::new(),
        survey: None,
        dimension_styles: Vec::new(),
        topology: None,
        annotation: None,
        text_styles: Vec::new(),
        connections: Vec::new(),
    }
}

/// The drawing a new project starts as.
pub fn new_project(o: &NewProject) -> Result<DocumentSnapshotV1, String> {
    let crs = system(o.srid).ok_or_else(|| {
        format!(
            "EPSG:{} bu sürümde tanımlı değil. Listedeki sistemlerden birini seçin.",
            o.srid
        )
    })?;
    let name = o.name.trim();
    let cad = o.workspace == Workspace::Cad;
    Ok(DocumentSnapshotV1 {
        format: DOCUMENT_FORMAT.to_owned(),
        version: DOCUMENT_VERSION,
        name: if name.is_empty() {
            NEW_PROJECT_NAME
        } else {
            name
        }
        .to_owned(),
        settings: ProjectSettings {
            plot_scale: o.plot_scale,
            workspace: Some(o.workspace),
            // A CAD project's angles are degrees, a CBS project's grads (docs/adr/0165 §4).
            angle_unit: if cad { AngleUnit::Deg } else { AngleUnit::Grad },
            drawing_font: Some(o.drawing_font),
            // Only a local project has a unit of its own; metres are not written.
            drawing_unit: o
                .drawing_unit
                .filter(|u| crs.is_local() && *u != DrawingUnit::M),
            ..default_settings(crs.srid)
        },
        origin: work_area_centre(crs),
        home_view: Some(start_view(crs, o.plot_scale, o.province.and_then(province))),
        // A CAD project starts with technical drawing layers, any other with a
        // cadastral sheet's (docs/adr/0165 §3).
        layers: if cad {
            cad_layers()
        } else {
            standard_layers(o.plot_scale)
        },
        active_layer: if cad {
            CAD_ACTIVE_LAYER
        } else {
            STANDARD_ACTIVE_LAYER
        }
        .to_owned(),
        entities: Vec::new(),
        styles: ProjectStyles::default(),
        blocks: Vec::new(),
    })
}

/// A layer of a new project's tree by its id (`parsel`), for a drawing that
/// lacks it: a tool that writes to a standard layer opens it as a new
/// project has it. A CAD project looks in its own tree first (its `yazi` is
/// “Yazı”, docs/adr/0175 §3), then in the CBS one (`parsel`, `kot`). None
/// for an id the trees do not have, or a group.
pub fn standard_layer(id: &str, plot_scale: f64, cad: bool) -> Option<LayerNode> {
    fn find(nodes: Vec<LayerNode>, id: &str) -> Option<LayerNode> {
        nodes.into_iter().find_map(|n| {
            if n.id == id && n.kind == LayerNodeType::Layer {
                Some(n)
            } else {
                find(n.children, id)
            }
        })
    }
    cad.then(|| find(cad_layers(), id))
        .flatten()
        .or_else(|| find(standard_layers(plot_scale), id))
}

/// The middle of the zone at Türkiye's centre latitude (the web's `workAreaCentre`).
pub fn work_area_centre(crs: &System) -> Vec2 {
    // A drawing in no coordinate system starts at 0,0 (AutoCAD's new drawing).
    if crs.is_local() {
        return Vec2 { x: 0.0, y: 0.0 };
    }
    if crs.kind == "geographic" {
        return Vec2 {
            x: WORK_LON,
            y: WORK_LAT,
        };
    }
    if crs.projection.as_deref() == Some("Pseudo-Mercator") {
        let r = 6_378_137.0;
        let x = r * (WORK_LON * PI / 180.0);
        let y = r * (PI / 4.0 + WORK_LAT * PI / 360.0).tan().ln();
        return Vec2 {
            x: (x / 1000.0).round() * 1000.0,
            y: (y / 1000.0).round() * 1000.0,
        };
    }
    Vec2 {
        x: crs.false_easting.unwrap_or(500_000.0),
        y: crs.false_northing.unwrap_or(0.0) + WORK_NORTHING,
    }
}

/// An A3 sheet across (landscape), metres on paper.
const A3: (f64, f64) = (0.42, 0.297);

/// Where a new project opens and comes back to, its home view (docs/adr/0165
/// §3): a local project on an A3 landscape sheet at its scale with 0,0 at the
/// bottom left, as AutoCAD's new drawing; any other on one sheet at its scale
/// around its province's centre, or around its zone's work area without one
/// (the web's `startView`).
pub fn start_view(crs: &System, plot_scale: f64, province: Option<&Province>) -> Bounds {
    if crs.is_local() {
        return Bounds {
            min_x: 0.0,
            min_y: 0.0,
            max_x: A3.0 * plot_scale,
            max_y: A3.1 * plot_scale,
        };
    }
    let centre = province
        .and_then(|p| project_lat_lon(crs, p.lat, p.lon))
        .unwrap_or_else(|| work_area_centre(crs));
    sheet_around(centre, plot_scale, crs.unit == "degree")
}

/// One paper sheet (50 × 37.5 cm) at the plot scale around `p`: what a new,
/// empty project shows first. In a geographic system the sheet's metres are
/// turned into degrees roughly; a start view needs no precision (the web's
/// `sheetAround`).
pub fn sheet_around(p: Vec2, plot_scale: f64, degrees: bool) -> Bounds {
    let k = if degrees { 1.0 / 111_320.0 } else { 1.0 };
    let w = 0.25 * plot_scale * k;
    let h = 0.1875 * plot_scale * k;
    Bounds {
        min_x: p.x - w,
        min_y: p.y - h,
        max_x: p.x + w,
        max_y: p.y + h,
    }
}

/// A style over the default one (the web's `{ ...defaultStyle, ...style }`).
fn style(color: &str, line_weight: Option<f64>) -> LayerStyle {
    LayerStyle {
        color: color.to_owned(),
        line_weight: line_weight.unwrap_or(default_style().line_weight),
        ..default_style()
    }
}

fn label(placement: LabelPlacement, size: f64) -> LabelStyle {
    LabelStyle {
        placement,
        size,
        grow: None,
        max_size: None,
        weight: None,
        template: None,
        min_feature_px: None,
        min_scale: None,
        max_scale: None,
        ink: None,
    }
}

fn node(id: &str, name: &str, style: LayerStyle) -> LayerNode {
    LayerNode {
        id: id.to_owned(),
        name: name.to_owned(),
        kind: LayerNodeType::Layer,
        visible: true,
        locked: false,
        expanded: true,
        style,
        children: Vec::new(),
        snap: None,
        fields: Vec::new(),
        service: None,
        feed: None,
    }
}

fn group(id: &str, name: &str, children: Vec<LayerNode>) -> LayerNode {
    LayerNode {
        kind: LayerNodeType::Group,
        children,
        ..node(id, name, default_style())
    }
}

/// The technical drawing layers a CAD project starts with (docs/adr/0165
/// §3): drawn edges in the drawing's ink, then dimensions, texts, hatches,
/// construction and centre lines, AutoCAD's usual set (the web's `cadLayers`).
pub fn cad_layers() -> Vec<LayerNode> {
    let lined = |line_type, color: &str, weight| LayerStyle {
        line_type,
        ..style(color, Some(weight))
    };
    vec![
        node("cizim", "Çizim", style("ink", Some(0.35))),
        node("olcu", "Ölçü", style("#56B6C2", Some(0.18))),
        node("yazi", "Yazı", style("fg", None)),
        node("tarama", "Tarama", style("#8C9AAA", Some(0.13))),
        node(
            "yardimci",
            "Yardımcı",
            lined(LineType::Dashed, "fg-dim", 0.13),
        ),
        node("eksen", "Eksen", lined(LineType::Dashdot, "#E06C75", 0.18)),
    ]
}

/// The standard layer tree of a cadastral sheet: what a new project starts
/// with. The map tools write to layers of this tree by id (`parsel`, `kot`),
/// so a new project carries them. `plot_scale` is written into the sheet
/// frame's label (“Pafta P-12   1:1000”).
pub fn standard_layers(plot_scale: f64) -> Vec<LayerNode> {
    let points = |symbol, size| PointStyle { symbol, size };
    vec![
        node("taslak", "Taslak", style("ink", None)),
        group(
            "g-kadastro",
            "Kadastro",
            vec![
                node(
                    "ada",
                    "Ada sınırı",
                    LayerStyle {
                        label: Some(LabelStyle {
                            grow: Some(3.0),
                            max_size: Some(22.0),
                            weight: Some(600),
                            template: Some("{label} ada".to_owned()),
                            min_feature_px: Some(90.0),
                            max_scale: Some(5.0),
                            ink: Some(LabelInk::Fg),
                            ..label(LabelPlacement::Center, 12.0)
                        }),
                        ..style("fg", Some(0.35))
                    },
                ),
                node(
                    "parsel",
                    "Parsel sınırı",
                    LayerStyle {
                        label: Some(LabelStyle {
                            grow: Some(1.2),
                            max_size: Some(14.0),
                            min_feature_px: Some(26.0),
                            ..label(LabelPlacement::Center, 9.0)
                        }),
                        ..style("fg-dim", Some(0.18))
                    },
                ),
                node(
                    "yapi",
                    "Yapı",
                    LayerStyle {
                        fill: Some("#7FB2E52E".to_owned()),
                        ..style("#7FB2E5", Some(0.25))
                    },
                ),
            ],
        ),
        group(
            "g-ulasim",
            "Ulaşım",
            vec![
                node(
                    "yol-ekseni",
                    "Yol ekseni",
                    LayerStyle {
                        line_type: LineType::Dashdot,
                        ..style("#E06C75", Some(0.13))
                    },
                ),
                node("kaldirim", "Kaldırım", style("#8C9AAA", Some(0.18))),
            ],
        ),
        group(
            "g-topo",
            "Topografya",
            vec![
                node("esyukselti", "Eşyükselti", style("#A87C54", Some(0.13))),
                node(
                    "ana-esyukselti",
                    "Ana eşyükselti",
                    LayerStyle {
                        label: Some(LabelStyle {
                            min_scale: Some(1.6),
                            ..label(LabelPlacement::Along, 10.0)
                        }),
                        ..style("#C9955F", Some(0.25))
                    },
                ),
                node(
                    "kot",
                    "Kot noktaları",
                    LayerStyle {
                        point: Some(points(PointSymbol::Cross, 7.0)),
                        label: Some(LabelStyle {
                            min_scale: Some(2.2),
                            ..label(LabelPlacement::Beside, 10.5)
                        }),
                        ..style("#9CCB7E", None)
                    },
                ),
            ],
        ),
        group(
            "g-jeodezi",
            "Jeodezi",
            vec![node(
                "poligon",
                "Poligon noktaları",
                LayerStyle {
                    point: Some(points(PointSymbol::Triangle, 11.0)),
                    label: Some(LabelStyle {
                        min_scale: Some(0.9),
                        ..label(LabelPlacement::Beside, 10.5)
                    }),
                    ..style("#56B6C2", None)
                },
            )],
        ),
        LayerNode {
            expanded: false,
            ..group(
                "g-pafta",
                "Pafta",
                vec![
                    node(
                        "pafta",
                        "Pafta çerçevesi",
                        LayerStyle {
                            pick_interior: Some(false),
                            label: Some(LabelStyle {
                                template: Some(format!(
                                    "Pafta {{label}}   1:{}",
                                    js_number(plot_scale)
                                )),
                                min_feature_px: Some(200.0),
                                ink: Some(LabelInk::FgDim),
                                ..label(LabelPlacement::Corner, 12.0)
                            }),
                            ..style("#6B7785", Some(0.5))
                        },
                    ),
                    node("karelaj", "Karelaj", style("#6B7785", Some(0.13))),
                    node("yazi", "Yazılar", style("fg-dim", None)),
                ],
            )
        },
    ]
}

/// A number as JavaScript writes it into a string (`${1000}` → “1000”,
/// `${0.5}` → “0.5”): whole numbers without a fraction.
pub fn js_number(v: f64) -> String {
    if v.fract() == 0.0 && v.abs() < 1e21 {
        format!("{}", v as i64)
    } else {
        format!("{v}")
    }
}

#[cfg(test)]
mod tests {
    use serde_json::Value;

    use super::*;

    const FIXTURE: &str = include_str!("../../../../fixtures/project/v1/new-project.json");

    #[test]
    fn a_new_project_is_the_drawing_the_web_records() {
        let file: Value = serde_json::from_str(FIXTURE).expect("the fixture reads");
        assert_eq!(file["format"], "kentos.new-project");
        let cases = file["cases"].as_array().expect("cases");
        assert!(cases.len() >= 5);
        for case in cases {
            let o = &case["options"];
            let workspace: Workspace = o
                .get("workspace")
                .map_or(Ok(Workspace::Gis), |w| serde_json::from_value(w.clone()))
                .expect("a mode");
            let drawing_font: DrawingFont = o
                .get("drawingFont")
                .map_or(Ok(DrawingFont::Barlow), |f| {
                    serde_json::from_value(f.clone())
                })
                .expect("a typeface");
            let options = NewProject {
                name: o["name"].as_str().expect("a name").to_owned(),
                srid: o["srid"].as_u64().expect("a system") as u32,
                plot_scale: o["plotScale"].as_f64().expect("a scale"),
                workspace,
                drawing_font,
                province: o.get("province").and_then(Value::as_u64).map(|c| c as u32),
                drawing_unit: o
                    .get("drawingUnit")
                    .map(|u| serde_json::from_value(u.clone()).expect("a unit")),
            };
            let want: DocumentSnapshotV1 =
                serde_json::from_value(case["snapshot"].clone()).expect("the web's drawing reads");
            let got = new_project(&options).expect("builds");
            assert_eq!(got, want, "{options:?}");
        }
    }

    #[test]
    fn an_unknown_system_is_refused_and_a_sheet_is_half_a_metre_wide_on_paper() {
        let refused = new_project(&NewProject {
            name: "x".into(),
            srid: 1234,
            plot_scale: 1000.0,
            workspace: Workspace::Gis,
            drawing_font: DrawingFont::Barlow,
            province: None,
            drawing_unit: None,
        });
        assert!(refused.expect_err("unknown").contains("EPSG:1234"));
        let b = sheet_around(Vec2 { x: 0.0, y: 0.0 }, 1000.0, false);
        assert_eq!((b.max_x - b.min_x, b.max_y - b.min_y), (500.0, 375.0));
    }
}

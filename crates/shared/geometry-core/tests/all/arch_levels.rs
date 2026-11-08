//! Columns, stairs, ramps, slab edges and furniture by the drawing's scale (docs/adr/3011): beyond 1:150 schematic the
//! simplest, at 1:100 as they were drawn, from 1:50 with a column's size and a stair's risers in words. Their tools'
//! previews and the reference cases keep the whole (`arch::pieces`).

use kentos_geometry_core::api::json::{Flat, Json};
use kentos_geometry_core::block::{self, Piece};
use kentos_geometry_core::entity::Shape;
use kentos_geometry_core::geom::arch::{self, ColumnSpec, RampSpec, StairSpec};
use kentos_geometry_core::geom::arrangement::{Area, Ring};
use kentos_geometry_core::geom::slab::{self, Edge, Look, Run};
use kentos_geometry_core::geom::wall::{Detail, View};
use kentos_geometry_core::text::{Font, TextAlign};
use kentos_geometry_core::vec2::Vec2;

const SIMPLE: View = View::new(Detail::Schematic, 200.0);
const SCHEMATIC: View = View::new(Detail::Schematic, 100.0);
const STANDARD: View = View::new(Detail::Standard, 50.0);
const DETAILED: View = View::new(Detail::Detailed, 20.0);

fn texts(pieces: &[Piece]) -> Vec<String> {
    pieces
        .iter()
        .filter_map(|p| match &p.shape {
            Shape::Text { text, .. } => Some(text.clone()),
            _ => None,
        })
        .collect()
}

fn column(json: &str) -> Shape {
    Shape::Column {
        at: Vec2::new(1.0, 1.0),
        column: ColumnSpec::read_flat(&Json::parse(json).unwrap()).unwrap(),
    }
}

fn stair(from: Vec2, to: Vec2) -> Shape {
    Shape::Stair {
        pts: vec![from, to],
        stair: StairSpec::read_flat(
            &Json::parse(
                r#"{"width":1.2,"risers":11,"riserHeight":0.175,"tread":0.28,"numbering":true,
                    "railings":"both","cut":6}"#,
            )
            .unwrap(),
        )
        .unwrap(),
    }
}

/// A column (Mimarlar Odası, Mimari Proje Çizim ve Sunuş Standartları): none of its words at 1:200, its name from
/// 1:100 (ön proje), its name and size from 1:50 (uygulama: "kolon isimleri ve ölçüleri yazılır").
#[test]
fn a_column_has_no_words_at_1_200_its_name_at_1_100_and_its_size_from_1_50() {
    let tagged = column(r#"{"width":0.3,"depth":0.5,"tag":"S1"}"#);
    assert!(texts(&arch::pieces_in(&tagged, SIMPLE)).is_empty());
    assert_eq!(texts(&arch::pieces_in(&tagged, SCHEMATIC)), ["S1"]);
    assert_eq!(texts(&arch::pieces_in(&tagged, STANDARD)), ["S1\n30/50"]);
    assert_eq!(texts(&arch::pieces_in(&tagged, DETAILED)), ["S1\n30/50"]);
    // Its tool's preview keeps the whole: its name at any scale, no size.
    assert_eq!(texts(&arch::pieces(&tagged, 200.0)), ["S1"]);
    // A round one its diameter; a square one twice its width; a size to the millimetre.
    let round = column(r#"{"shape":"round","width":0.4}"#);
    assert!(texts(&arch::pieces_in(&round, SCHEMATIC)).is_empty());
    assert_eq!(texts(&arch::pieces_in(&round, STANDARD)), ["Ø40"]);
    let square = column(r#"{"width":0.275}"#);
    assert_eq!(texts(&arch::pieces_in(&square, STANDARD)), ["27.5/27.5"]);
    // Its outline and fill at every scale.
    let filled = column(r#"{"width":0.3,"depth":0.5,"fill":true}"#);
    for v in [SIMPLE, SCHEMATIC, STANDARD] {
        let shapes = arch::pieces_in(&filled, v);
        assert!(
            shapes
                .iter()
                .any(|p| matches!(p.shape, Shape::Hatch { .. })),
            "{v:?}"
        );
        assert!(
            shapes
                .iter()
                .any(|p| matches!(p.shape, Shape::Polygon { .. } | Shape::Polyline { .. })),
            "{v:?}"
        );
    }
}

/// A stair: at 1:200 its band, treads, cut and arrow, no railings and no numbers (two millimetres of number in a tread
/// of 1.4 mm ran together); at 1:100 the railings and numbers; from 1:50 its risers and treads in words before its
/// first riser (Mimarlar Odası, uygulama: "basamak adedi … ve rıht yüksekliği yazılır").
#[test]
fn a_stair_drops_its_railings_and_numbers_at_1_200_and_writes_its_risers_from_1_50() {
    let s = stair(Vec2::new(0.0, 0.0), Vec2::new(2.8, 0.0));
    let (simple, schematic, standard) = (
        arch::pieces_in(&s, SIMPLE),
        arch::pieces_in(&s, SCHEMATIC),
        arch::pieces_in(&s, STANDARD),
    );
    let Shape::Stair { pts, stair: spec } = &s else {
        unreachable!()
    };
    let railings =
        arch::stair_railings(pts, spec).len() + arch::stair_railing_arcs(pts, spec).len();
    assert!(railings >= 2);
    assert!(texts(&simple).is_empty(), "{:?}", texts(&simple));
    assert_eq!(texts(&schematic), ["1", "2", "3", "4", "5", "6"]);
    assert_eq!(simple.len() + railings + 6, schematic.len());
    assert_eq!(texts(&standard)[0], "11 × 17.5/28");
    assert_eq!(standard.len(), schematic.len() + 1);
    // Whole for its tool: no words.
    assert_eq!(arch::pieces(&s, 50.0).len(), schematic.len());
    let words = |s: &Shape| {
        arch::pieces_in(s, STANDARD)
            .into_iter()
            .find_map(|p| match p.shape {
                Shape::Text {
                    p,
                    text,
                    rotation,
                    align,
                    ..
                } if text.contains('×') => Some((p, rotation, align)),
                _ => None,
            })
            .expect("its words")
    };
    // Before its first riser, along its run, read upright: a run to the right ends its words there, one to the left
    // starts them there, one down reads from below.
    let (p, rotation, align) = words(&s);
    assert!(
        p.x < 0.0 && p.y.abs() < 1e-9 && rotation.abs() < 1e-9,
        "{p:?} {rotation}"
    );
    assert_eq!(align, Some(TextAlign::MiddleRight));
    let (p, rotation, align) = words(&stair(Vec2::new(2.8, 0.0), Vec2::new(0.0, 0.0)));
    assert!(p.x > 2.8 && rotation.abs() < 1e-9, "{p:?} {rotation}");
    assert_eq!(align, Some(TextAlign::MiddleLeft));
    let (p, rotation, _) = words(&stair(Vec2::new(0.0, 2.8), Vec2::new(0.0, 0.0)));
    assert!(
        p.y > 2.8 && (rotation - 90.0).abs() < 1e-9,
        "{p:?} {rotation}"
    );
}

/// A ramp: no railings at 1:200, its slope and levels at every scale; its slope beside its band when its words do not
/// fit inside it (1:200), inside at 1:100.
#[test]
fn a_ramp_drops_its_railings_at_1_200_and_keeps_its_slope_off_its_edge() {
    let spec = RampSpec::read_flat(
        &Json::parse(r#"{"width":1.2,"rise":0.36,"slope":6,"railings":"both"}"#).unwrap(),
    )
    .unwrap();
    let pts = vec![Vec2::new(0.0, 0.0), Vec2::new(6.0, 0.0)];
    let railings = arch::ramp_railings(&pts, &spec).len();
    assert_eq!(railings, 2);
    let ramp = Shape::Ramp { pts, ramp: spec };
    let (simple, schematic, standard) = (
        arch::pieces_in(&ramp, SIMPLE),
        arch::pieces_in(&ramp, SCHEMATIC),
        arch::pieces_in(&ramp, STANDARD),
    );
    assert_eq!(simple.len() + railings, schematic.len());
    assert_eq!(standard.len(), schematic.len());
    for v in [&simple, &schematic] {
        assert!(texts(v).contains(&"%6.00".to_owned()) && texts(v).contains(&"+0.36".to_owned()));
    }
    let slope_y = |pieces: &[Piece]| {
        pieces
            .iter()
            .find_map(|p| match &p.shape {
                Shape::Text { p, text, .. } if text.starts_with('%') => Some(p.y),
                _ => None,
            })
            .unwrap()
    };
    assert!(
        slope_y(&simple) > 0.6 + 0.2,
        "beside the band at 1:200: {}",
        slope_y(&simple)
    );
    assert!(
        slope_y(&schematic) < 0.6 - 0.1,
        "inside at 1:100: {}",
        slope_y(&schematic)
    );
}

/// A slab's parapet: its outer line alone at 1:200, its inner line and ends from 1:100.
#[test]
fn a_slab_edge_is_its_outer_line_at_1_200() {
    let ring = |pts: &[(f64, f64)]| Ring {
        pts: pts.iter().map(|&(x, y)| Vec2::new(x, y)).collect(),
        bulges: None,
    };
    let parts = vec![Area {
        outer: ring(&[(0.0, 0.0), (4.0, 0.0), (4.0, 2.0), (0.0, 2.0)]),
        holes: Vec::new(),
    }];
    let look = Look {
        edge: Some(Edge {
            parapet: true,
            width: 0.2,
            runs: vec![Run {
                pts: vec![
                    Vec2::new(0.0, 2.0),
                    Vec2::new(0.0, 0.0),
                    Vec2::new(4.0, 0.0),
                ],
                bulges: None,
            }],
        }),
    };
    let whole = slab::pieces(&parts, &look).len();
    assert_eq!(slab::pieces_in(&parts, &look, SCHEMATIC).len(), whole);
    assert_eq!(slab::pieces_in(&parts, &look, STANDARD).len(), whole);
    // The ring and the run's outer line; the inner line and the two ends are left out.
    assert_eq!(slab::pieces_in(&parts, &look, SIMPLE).len(), 2);
    assert_eq!(whole, 5);
}

fn piece(shape: Shape) -> Piece {
    Piece {
        shape,
        color: None,
        line_weight: None,
        by_layer: Vec::new(),
        gates: Vec::new(),
        stroke: Default::default(),
        attribute: None,
        annotation_origin: None,
    }
}

fn rect(x0: f64, y0: f64, x1: f64, y1: f64) -> Shape {
    Shape::Polyline {
        pts: vec![
            Vec2::new(x0, y0),
            Vec2::new(x1, y0),
            Vec2::new(x1, y1),
            Vec2::new(x0, y1),
        ],
        bulges: None,
        holes: None,
        parts: None,
        closed: true,
        widths: None,
    }
}

/// Furniture's simplest view: the closed pieces round the whole (a bed's frame, a round table without its chairs);
/// one without a closed outline keeps its pieces.
#[test]
fn furniture_is_its_outline_beyond_1_150() {
    let font = Font::DEFAULT;
    // The library's double bed: its frame and two pillows.
    let bed = vec![
        piece(rect(0.0, 0.0, 1.6, 2.0)),
        piece(rect(0.1, 0.1, 0.75, 0.45)),
        piece(rect(0.85, 0.1, 1.5, 0.45)),
    ];
    let kept = block::outline_pieces(bed.clone(), font);
    assert_eq!(kept, bed[..1]);
    // A round table and four chairs about it.
    let mut table = vec![piece(Shape::Circle {
        c: Vec2::new(0.0, 0.0),
        r: 0.6,
    })];
    for (x, y) in [(-1.0, -0.2), (0.6, -0.2), (-0.2, -1.0), (-0.2, 0.6)] {
        table.push(piece(rect(x, y, x + 0.4, y + 0.4)));
    }
    assert_eq!(block::outline_pieces(table.clone(), font), table[..1]);
    // Four lines: no closed outline, all kept.
    let lines: Vec<Piece> = [
        ((0.0, 0.0), (1.0, 0.0)),
        ((1.0, 0.0), (1.0, 1.0)),
        ((1.0, 1.0), (0.0, 1.0)),
        ((0.0, 1.0), (0.0, 0.0)),
    ]
    .into_iter()
    .map(|(a, b)| {
        piece(Shape::Line {
            a: Vec2::new(a.0, a.1),
            b: Vec2::new(b.0, b.1),
        })
    })
    .collect();
    assert_eq!(block::outline_pieces(lines.clone(), font), lines);
}

//! A map cut to its atlas object (docs/adr/0638 §2): the object's outline on the ground becomes the map's clip, in
//! the content frame's millionths, cut to the frame and thinned to the clip's 256 corners.

use kentos_sheet::clip::{Clip, FULL};
use kentos_sheet::display::{self, AtlasFeature, Prim, RenderInputs};
use kentos_sheet::kinds::{ItemKind, MapView};
use kentos_sheet::model::SheetBook;
use kentos_sheet::model::{Atlas, SortKey};
use kentos_sheet::template::{InstanceIds, InstanceOptions, instantiate, system_template};

/// The system's atlas sheet, its atlas map cut to the object when `cut`.
fn book(cut: bool) -> SheetBook {
    let t = system_template("sys:gis-atlas").expect("the atlas template");
    let ids = InstanceIds {
        sheet: "atlas".into(),
        master: None,
        items: t.sheet.items.iter().map(|i| i.id.clone()).collect(),
        master_items: Vec::new(),
    };
    let inst = instantiate(t, &ids, &InstanceOptions::default()).expect("an instance");
    let mut sheet = inst.sheet;
    sheet.atlas = Some(Atlas {
        layer: "parsel".into(),
        filter: String::new(),
        sort: vec![SortKey {
            expression: "$id".into(),
            descending: false,
        }],
        page_name: "'p'".into(),
    });
    for it in &mut sheet.items {
        if let ItemKind::Map(m) = &mut it.kind
            && matches!(m.view, MapView::Atlas(_))
        {
            m.clip_to_atlas_feature = cut;
        }
    }
    SheetBook {
        sheets: vec![sheet],
        assets: inst.assets,
        ..SheetBook::default()
    }
}

/// The first atlas map's clip on the page of an object with `outline` (its box from it).
fn clip_of(cut: bool, outline: Vec<[f64; 2]>) -> Clip {
    let b = outline
        .iter()
        .fold([f64::MAX, f64::MAX, f64::MIN, f64::MIN], |b, p| {
            [
                b[0].min(p[0]),
                b[1].min(p[1]),
                b[2].max(p[0]),
                b[3].max(p[1]),
            ]
        });
    let book = book(cut);
    let feature = AtlasFeature {
        id: "p1".into(),
        bbox: b,
        attributes: Vec::new(),
        outline: Some(outline),
    };
    let plan = kentos_sheet::atlas::atlas_plan(&book, "atlas", &[feature]).expect("a plan");
    let inputs = RenderInputs {
        atlas: plan.pages.into_iter().next(),
        ..RenderInputs::default()
    };
    let list = display::display_list(&book, "atlas", &inputs).expect("a list");
    list.prims
        .iter()
        .find_map(|p| match p {
            Prim::Map(m) if m.clip_feature.is_some() || !cut => Some(m.clip_shape.clone()),
            _ => None,
        })
        .expect("the atlas map")
}

fn square(x: f64, y: f64, side: f64) -> Vec<[f64; 2]> {
    vec![[x, y], [x + side, y], [x + side, y + side], [x, y + side]]
}

#[test]
fn a_map_cut_to_its_object_clips_to_the_object_s_outline() {
    let Clip::Polygon { points } = clip_of(true, square(500_000.0, 4_400_000.0, 30.0)) else {
        panic!("a polygon clip");
    };
    assert_eq!(points.len(), 4, "{points:?}");
    // The page fits the object with a margin: its corners are inside the frame, not on its edges.
    for p in &points {
        assert!(p.iter().all(|v| *v > 0 && *v < FULL), "{points:?}");
    }
    // East right, north up: the square's south-west corner is the lowest left one on the paper.
    let left = points.iter().map(|p| p[0]).min().expect("a corner");
    let bottom = points.iter().map(|p| p[1]).max().expect("a corner");
    assert!(points.iter().any(|p| p[0] == left && p[1] == bottom));
    // Without the choice the map keeps its own shape.
    assert!(clip_of(false, square(500_000.0, 4_400_000.0, 30.0)).is_frame());
}

#[test]
fn an_outline_of_many_corners_is_thinned_to_the_clip_s_limit() {
    // A circle of 2000 corners.
    let ring: Vec<[f64; 2]> = (0..2000)
        .map(|k| {
            let t = f64::from(k) / 2000.0 * std::f64::consts::TAU;
            [500_000.0 + 40.0 * t.cos(), 4_400_000.0 + 40.0 * t.sin()]
        })
        .collect();
    let clip = clip_of(true, ring);
    let Clip::Polygon { points } = &clip else {
        panic!("a polygon clip");
    };
    assert!(
        points.len() <= 256 && points.len() >= 32,
        "{}",
        points.len()
    );
    assert!(clip.validate().is_ok());
}

#[test]
fn an_outline_past_the_frame_is_cut_to_it() {
    // An L whose long arm runs far past the page the box of its short one would make: the box is the L's whole,
    // so the page fits it; a thin arm stays thin.
    let ring = vec![
        [500_000.0, 4_400_000.0],
        [500_400.0, 4_400_000.0],
        [500_400.0, 4_400_010.0],
        [500_010.0, 4_400_010.0],
        [500_010.0, 4_400_400.0],
        [500_000.0, 4_400_400.0],
    ];
    let clip = clip_of(true, ring);
    let Clip::Polygon { points } = &clip else {
        panic!("a polygon clip");
    };
    assert_eq!(points.len(), 6, "{points:?}");
    assert!(points.iter().flatten().all(|v| (0..=FULL).contains(v)));
}

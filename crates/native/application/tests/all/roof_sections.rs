//! Çatı kesitte ve görünüşte (docs/adr/8044 §5) on the section building (fixtures/arch/v1/section-building.kcad): a hip
//! roof over its 1. kat, its eaves 50 cm out of the walls' outer faces (`roofs::eaves_from_walls`) at 33 %, its eaves at
//! the storey's top (+6.00). B-B runs north–south through the stair and the ridge: the two long faces cut down their fall
//! lines, hatched as wood (TS 5319 SN23) on MM-CATI-KIREMIT, "%33" over each; "+6.00 Saçak" and the ridge's top
//! "+7.69 Mahya" among the levels; the roof no longer listed as missing. A-A runs east–west along the south face: no
//! slope is written where the band lies level. The north facade sees the roof's fascia, its hips and its ridge.

use kentos_commands::{
    BuildingSectionLeft, BuildingSectionsEdit, BuildingSectionsOperation, CallOp, CommandResult,
    DocumentSnapshotV2, Entity, ViewMarkForm,
};
use kentos_domain::{Document, Uuid};
use kentos_native_application::{ExecutionContext, registry, roofs, sections, sections_edit};
use serde_json::json;

fn building() -> Document {
    let snap: DocumentSnapshotV2 = kentos_kcad::decode(include_bytes!(
        "../../../../../fixtures/arch/v1/section-building.kcad"
    ))
    .unwrap();
    Document::from_snapshot_v2(snap).unwrap()
}

/// The building with a hip roof over its 1. kat: its walls' outer outline 50 cm out, 33 % on every edge.
fn roofed(doc: &Document) -> Document {
    let mut doc = doc.clone();
    let eaves = roofs::eaves_from_walls(
        &doc,
        kentos_commands::Vec2 { x: 6.0, y: 4.5 },
        Some("k1"),
        0.5,
    )
    .expect("the walls enclose the point");
    let slopes = vec![33.0; eaves.len()];
    let answer = registry::run(
        &mut ExecutionContext::new(&mut doc),
        "cad.roofs.create",
        CallOp::Execute,
        json!({"layerId": "0", "roofs": [{"pts": eaves, "slopes": slopes, "overhang": 0.5,
                                          "arch": {"level": "k1"}}]}),
    )
    .expect("runs");
    assert_eq!(answer["status"], "completed", "{answer}");
    doc
}

fn mark(doc: &Document, form: ViewMarkForm, letter: &str) -> Uuid {
    doc.entities_with_uids()
        .find_map(|(u, e)| match e {
            Entity::ViewMark(v) if v.mark.form == form && v.mark.mark == letter => Some(u),
            _ => None,
        })
        .unwrap()
}

fn derive(doc: &mut Document, m: Uuid) -> kentos_commands::BuildingSectionsEdited {
    match sections_edit::execute(
        &mut ExecutionContext::new(doc),
        BuildingSectionsEdit {
            operation: BuildingSectionsOperation::Derive {
                mark: m.to_string(),
                at: Some(kentos_commands::Vec2 { x: 20.0, y: 0.0 }),
            },
            expected_revision: None,
        },
    ) {
        CommandResult::Completed { output, .. } => output,
        other => panic!("{other:?}"),
    }
}

fn texts(doc: &Document, m: Uuid) -> Vec<String> {
    doc.block(sections::block_id(m))
        .unwrap()
        .entities
        .iter()
        .filter_map(|e| match e {
            Entity::Text(t) => Some(t.text.clone()),
            _ => None,
        })
        .collect()
}

#[test]
fn b_b_cuts_the_roof_down_its_fall_lines_with_its_slope_and_levels() {
    let mut doc = roofed(&building());
    let b = mark(&doc, ViewMarkForm::Section, "B");
    let out = derive(&mut doc, b);
    assert!(
        !out.left_out
            .iter()
            .any(|l| l.why == BuildingSectionLeft::Roof),
        "the roof is drawn now: {:?}",
        out.left_out
    );
    let block = doc.block(sections::block_id(b)).unwrap();
    // Its band hatched as wood on its covering's layer, outlined heavy.
    let hatch = block
        .entities
        .iter()
        .find_map(|e| match e {
            Entity::Hatch(h) if h.base.layer_id == "mm-cati-kiremit-t" => Some(h),
            _ => None,
        })
        .expect("the roof's hatch");
    assert!(
        format!("{:?}", hatch.pattern).contains("SN23"),
        "{:?}",
        hatch.pattern
    );
    assert!(
        block
            .entities
            .iter()
            .any(|e| matches!(e, Entity::Polyline(p) if p.base.layer_id == "mm-cati-kiremit")),
        "its outline"
    );
    // The two long faces cut down their fall lines: their slope over each; the eaves' storey top and the ridge's top
    // among the levels (13.25 × 10.25 m at 33 %: 1.69 m over +6.00).
    let t = texts(&doc, b);
    assert_eq!(t.iter().filter(|x| *x == "%33").count(), 2, "{t:?}");
    for w in ["+6.00 Saçak", "+7.69 Mahya", "±0.00", "+3.00"] {
        assert!(t.iter().any(|x| x == w), "{w} in {t:?}");
    }
    // The storey top is the saçak seviyesi once: no bare "+6.00" beside it.
    assert!(!t.iter().any(|x| x == "+6.00"), "{t:?}");
}

#[test]
fn a_a_along_the_south_face_writes_no_slope_where_the_band_lies_level() {
    let mut doc = roofed(&building());
    let a = mark(&doc, ViewMarkForm::Section, "A");
    derive(&mut doc, a);
    let t = texts(&doc, a);
    // A-A at y 2.4 cuts the south face level (no slope written) and the east and west hip faces down their fall
    // lines, each over a metre: their slopes are written.
    assert_eq!(t.iter().filter(|x| *x == "%33").count(), 2, "{t:?}");
    assert!(t.iter().any(|x| x == "+7.69 Mahya"), "{t:?}");
}

#[test]
fn without_a_roof_the_section_still_says_it_is_missing() {
    let mut doc = building();
    let b = mark(&doc, ViewMarkForm::Section, "B");
    let out = derive(&mut doc, b);
    assert!(
        out.left_out
            .iter()
            .any(|l| l.why == BuildingSectionLeft::Roof)
    );
    let t = texts(&doc, b);
    assert!(t.iter().any(|x| x == "+6.00"), "{t:?}");
    assert!(!t.iter().any(|x| x.contains("Mahya")), "{t:?}");
}

/// The roof's cut outlines in a section: one for each band.
fn bands(doc: &Document, m: Uuid) -> usize {
    doc.block(sections::block_id(m))
        .unwrap()
        .entities
        .iter()
        .filter(|e| matches!(e, Entity::Polyline(p) if p.base.layer_id == "mm-cati-kiremit"))
        .count()
}

#[test]
fn b_b_cuts_the_chimney_on_its_line_and_writes_its_top() {
    let mut plain = roofed(&building());
    let b = mark(&plain, ViewMarkForm::Section, "B");
    derive(&mut plain, b);
    let without = bands(&plain, b);
    // A chimney on B-B's line (x 5.2) north of the ridge, a vent off it (docs/adr/8044 §6).
    let mut doc = roofed(&building());
    let roof = doc
        .entities_with_uids()
        .find_map(|(u, e)| matches!(e, Entity::Roof(_)).then_some(u))
        .unwrap();
    let answer = registry::run(
        &mut ExecutionContext::new(&mut doc),
        "cad.arch.edit",
        CallOp::Execute,
        json!({"uids": [roof.to_string()], "operation": {"kind": "set", "parts": [
            {"kind": "chimney", "at": {"x": 5.2, "y": 6.5}, "width": 0.4, "depth": 0.4},
            {"kind": "vent", "at": {"x": 9.5, "y": 2.0}, "width": 0.4, "depth": 0.4}
        ]}}),
    )
    .expect("runs");
    assert_eq!(answer["status"], "completed", "{answer}");
    let b = mark(&doc, ViewMarkForm::Section, "B");
    derive(&mut doc, b);
    let block = doc.block(sections::block_id(b)).unwrap();
    // Cut: hatched as masonry on EK-2's Baca.
    assert!(
        block
            .entities
            .iter()
            .any(|e| matches!(e, Entity::Hatch(h) if h.base.layer_id.starts_with("mm-baca-genel"))),
        "the chimney's hatch"
    );
    // Its top 80 cm over the ridge (+7.69): "+8.49 Baca" among the levels (Mimarlar Odası 14.3.4 g).
    let t = texts(&doc, b);
    assert!(t.iter().any(|x| x == "+8.49 Baca"), "{t:?}");
    // The north face's band stops at the chimney's sides: one band more than without it.
    assert_eq!(bands(&doc, b), without + 1);
    // The vent's top, 16 cm under the ridge, is written left of the ridge's level, not over it.
    let at = |prefix: &str| {
        doc.block(sections::block_id(b))
            .unwrap()
            .entities
            .iter()
            .find_map(|e| match e {
                Entity::Text(t) if t.text.ends_with(prefix) => {
                    Some((t.p.x, t.text.chars().count(), t.height))
                }
                _ => None,
            })
            .unwrap_or_else(|| panic!("{prefix} in {t:?}"))
    };
    let (vent, chars, height) = at("Hava bacası");
    let (ridge, _, _) = at("Mahya");
    assert!(
        vent + chars as f64 * height * 0.6 <= ridge,
        "{vent} {ridge}"
    );
    // The heights chain at the right stays the building's: the roof's 1,69, no link to the vent's or the chimney's top.
    assert!(t.iter().any(|x| x == "1,69"), "{t:?}");
    assert!(!t.iter().any(|x| x == "0,80" || x == "0,16"), "{t:?}");
}

/// The facade building (fixtures/arch/v1/facade-building.kcad): the section building with four elevation marks.
fn facades() -> Document {
    let snap: DocumentSnapshotV2 = kentos_kcad::decode(include_bytes!(
        "../../../../../fixtures/arch/v1/facade-building.kcad"
    ))
    .unwrap();
    Document::from_snapshot_v2(snap).unwrap()
}

#[test]
fn the_north_facade_sees_the_roofs_fascia_its_hips_and_its_ridge() {
    let mut doc = roofed(&facades());
    let north = mark(&doc, ViewMarkForm::Elevation, "1");
    derive(&mut doc, north);
    let block = doc.block(sections::block_id(north)).unwrap();
    let lines: Vec<&kentos_commands::LineEntity> = block
        .entities
        .iter()
        .filter_map(|e| match e {
            Entity::Line(l) if l.base.layer_id.starts_with("mm-cati-kiremit-") => Some(l),
            _ => None,
        })
        .collect();
    let level = |l: &&kentos_commands::LineEntity, z: f64| {
        (l.a.y - z).abs() < 1e-6 && (l.b.y - z).abs() < 1e-6
    };
    let long = |l: &&kentos_commands::LineEntity| (l.a.x - l.b.x).abs();
    // The ridge: 3 m level at its top, 1.69 m over the eaves (13.25 × 10.25 m at 33 %).
    let ridge = 6.0 + 0.33 * 10.25 / 2.0;
    assert!(
        lines
            .iter()
            .any(|l| level(l, ridge) && (long(l) - 3.0).abs() < 1e-6),
        "the ridge in {lines:?}"
    );
    // The fascia across the roof's width, its top at the eaves (+6.00) and its bottom 25 cm under it.
    for z in [6.0, 5.75] {
        assert!(
            lines
                .iter()
                .any(|l| level(l, z) && (long(l) - 13.25).abs() < 1e-6),
            "the fascia at {z} in {lines:?}"
        );
    }
    // Its two hips, slanting from the eaves' corners to the ridge's ends.
    let slanted = lines
        .iter()
        .filter(|l| (l.a.y - l.b.y).abs() > 1.0 && long(l) > 1.0)
        .count();
    assert_eq!(slanted, 2, "{lines:?}");
    // Its covering named among the facade's finishes (Mimarlar Odası 14.3.4 c: "çatı örtüleri kaplamaları belirtilir").
    let t = texts(&doc, north);
    for w in ["+6.00 Saçak", "+7.69 Mahya", "Kiremit"] {
        assert!(t.iter().any(|x| x == w), "{w} in {t:?}");
    }
}

/// The facade building's roof with a 15 cm gutter, a snow guard 30 cm in and a Ø100 downpipe at each end of its
/// south eaves, set where the İniş borusu tool sets them at a corner (0.58 m in from the eaves' ends, the gutter's
/// middle).
fn rained(doc: &Document) -> Document {
    let mut doc = doc.clone();
    let eaves = roofs::eaves_from_walls(
        &doc,
        kentos_commands::Vec2 { x: 6.0, y: 4.5 },
        Some("k1"),
        0.5,
    )
    .expect("the walls enclose the point");
    let y = eaves.iter().map(|p| p.y).fold(f64::INFINITY, f64::min);
    let x0 = eaves.iter().map(|p| p.x).fold(f64::INFINITY, f64::min);
    let x1 = eaves.iter().map(|p| p.x).fold(f64::NEG_INFINITY, f64::max);
    let pipe = |x: f64| json!({"kind": "downpipe", "at": {"x": x, "y": y - 0.075}, "width": 0.1, "depth": 0.1});
    let slopes = vec![33.0; eaves.len()];
    let answer = registry::run(
        &mut ExecutionContext::new(&mut doc),
        "cad.roofs.create",
        CallOp::Execute,
        json!({"layerId": "0", "roofs": [{"pts": eaves, "slopes": slopes, "overhang": 0.5, "gutter": 0.15,
                                          "snowGuard": 0.3, "parts": [pipe(x0 + 0.58), pipe(x1 - 0.58)],
                                          "arch": {"level": "k1"}}]}),
    )
    .expect("runs");
    assert_eq!(answer["status"], "completed", "{answer}");
    doc
}

/// The lines of an elevation's block on the layers starting with `layer`.
fn lines_on(doc: &Document, m: Uuid, layer: &str) -> Vec<kentos_commands::LineEntity> {
    doc.block(sections::block_id(m))
        .unwrap()
        .entities
        .iter()
        .filter_map(|e| match e {
            Entity::Line(l) if l.base.layer_id.starts_with(layer) => Some(l.clone()),
            _ => None,
        })
        .collect()
}

/// Görünüşte oluk ve iniş borusu (docs/adr/8044 §8; Mimarlık Hizmetleri Şartnamesi 14.3.5 e, uygulama projesi
/// görünüşleri h: "Yağmur inişleri ve olukları … gösterilir"). The south facade sees the gutter across the eaves, 7.5 cm
/// deep under +6.00 (half-round: half its width), the snow guard's line on the roof 30 cm in (+6.10), and each downpipe
/// down to ±0.00; the east facade sees a pipe beside the south wall, 8 cm off it, and its 45° swan neck back to the
/// gutter's outlet.
#[test]
fn the_facades_see_the_gutter_the_snow_guard_and_the_downpipes() {
    type Line = kentos_commands::LineEntity;
    let mut doc = rained(&facades());
    let level = |l: &Line, z: f64| (l.a.y - z).abs() < 1e-6 && (l.b.y - z).abs() < 1e-6;
    let long = |l: &Line| (l.a.x - l.b.x).abs();
    // The gutter's front at its bottom (7.5 cm under the eaves: half-round, half its width) and its left end.
    let front = |lines: &[Line], length: f64| {
        lines
            .iter()
            .find(|l| level(l, 5.925) && (long(l) - length).abs() < 1e-6)
            .map(|l| l.a.x.min(l.b.x))
    };
    // How far a pipe's upright side at `u` runs, the ground to its top, over all its pieces.
    let span = |lines: &[Line], u: f64| {
        let ys: Vec<f64> = lines
            .iter()
            .filter(|l| (l.a.x - u).abs() < 1e-6 && (l.b.x - u).abs() < 1e-6)
            .flat_map(|l| [l.a.y, l.b.y])
            .collect();
        (
            ys.iter().copied().fold(f64::INFINITY, f64::min),
            ys.iter().copied().fold(f64::NEG_INFINITY, f64::max),
        )
    };

    let south = mark(&doc, ViewMarkForm::Elevation, "2");
    derive(&mut doc, south);
    let gutter = lines_on(&doc, south, "mm-oluk-genel-");
    // Its front: 13.25 m of eaves and the two mitres' 15 cm, its top at the eaves (+6.00).
    let left = front(&gutter, 13.55).unwrap_or_else(|| panic!("the gutter's bottom in {gutter:?}"));
    assert!(
        gutter
            .iter()
            .any(|l| level(l, 6.0) && (long(l) - 13.55).abs() < 1e-6),
        "the gutter's top in {gutter:?}"
    );
    // The snow guard on the south face: 30 cm in at 33 % (+6.099), between the hips (13.25 − 2 × 0.30 m).
    let roof = lines_on(&doc, south, "mm-cati-kiremit-");
    assert!(
        roof.iter()
            .any(|l| level(l, 6.099) && (long(l) - 12.65).abs() < 1e-6),
        "the snow guard in {roof:?}"
    );
    // Each downpipe 0.58 m in from the eaves' ends (0.73 m from the gutter's end): its sides 10 cm apart, from the
    // graded ground (±0.00) up to the gutter's bottom.
    let pipes = lines_on(&doc, south, "mm-yibo-genel-");
    for u in [0.68, 0.78, 13.55 - 0.78, 13.55 - 0.68] {
        let (lo, hi) = span(&pipes, left + u);
        assert!(
            lo.abs() < 1e-6 && (hi - 5.925).abs() < 1e-6,
            "the pipe's side at {u}: {lo}…{hi} in {pipes:?}"
        );
    }

    // The east facade: the south-east pipe beside the south wall, from the ground up, 10 cm wide and 3 cm off the
    // wall's face as the drawing has it (its plaster included: the eaves were drawn 50 cm out of the unplastered face);
    // its swan neck at 45° from it out to the gutter's outlet.
    let east = mark(&doc, ViewMarkForm::Elevation, "3");
    derive(&mut doc, east);
    let upright = |l: &Line| (l.a.x - l.b.x).abs() < 1e-9 && (l.a.y - l.b.y).abs() > 1e-6;
    let pipes = lines_on(&doc, east, "mm-yibo-genel-");
    let mut sides: Vec<f64> = pipes
        .iter()
        .filter(|l| upright(l) && l.a.y.min(l.b.y).abs() < 1e-6)
        .map(|l| l.a.x)
        .collect();
    sides.sort_by(f64::total_cmp);
    sides.dedup_by(|a, b| (*a - *b).abs() < 1e-6);
    assert!(
        sides.len() == 2 && (sides[1] - sides[0] - 0.1).abs() < 1e-6,
        "the pipe's sides {sides:?} in {pipes:?}"
    );
    let face = lines_on(&doc, east, "mm-")
        .iter()
        .filter(|l| {
            !l.base.layer_id.starts_with("mm-yibo") && !l.base.layer_id.starts_with("mm-oluk")
        })
        .filter(|l| upright(l) && (l.a.y - l.b.y).abs() > 2.0 && l.a.x > sides[1] + 1e-9)
        .map(|l| l.a.x)
        .fold(f64::INFINITY, f64::min);
    assert!(
        (face - sides[1] - 0.03).abs() < 1e-6,
        "the wall's face at {face}, the pipe's side at {}",
        sides[1]
    );
    assert!(
        pipes.iter().any(|l| {
            let (du, dz) = ((l.b.x - l.a.x).abs(), (l.b.y - l.a.y).abs());
            du > 0.3 && (du - dz).abs() < 1e-6
        }),
        "the swan neck in {pipes:?}"
    );
}

/// Ruhsat takımı (docs/adr/8044 §7; PAİY 57(2)(a)): the roof over 1. kat has no storey over it whose plan would draw it
/// whole: the set gets "Çatı planı" after the storeys' plans, EK-2's PL and CAT (Tablo 2.13, "çatı planı ve
/// detayları"), its place NNN, its map the roof plan. Without a roof there is none.
#[test]
fn the_permit_set_gives_a_roof_no_storey_draws_whole_a_roof_plan() {
    use kentos_native_application::permit_set::{PermitSet, execute};
    let titles_of = |doc: &mut Document| {
        let (_, made, _) = execute(&mut ExecutionContext::new(doc), &PermitSet::default()).unwrap();
        made
    };
    let mut plain = building();
    assert!(
        !titles_of(&mut plain)
            .iter()
            .any(|m| m.title == "Çatı planı"),
        "no roof, no roof plan"
    );
    let mut doc = roofed(&building());
    let made = titles_of(&mut doc);
    let titles: Vec<&str> = made.iter().map(|m| m.title.as_str()).collect();
    let at = titles
        .iter()
        .position(|t| *t == "Çatı planı")
        .unwrap_or_else(|| panic!("{titles:?}"));
    assert_eq!(
        titles[at - 1],
        "1. kat planı",
        "after the storeys' plans: {titles:?}"
    );
    let (book, _) = kentos_sheet::project::read_project_sheets(doc.sheets().unwrap()).unwrap();
    let sheet = book.sheets.iter().find(|s| s.id == made[at].sheet).unwrap();
    let var = |n: &str| {
        sheet
            .variables
            .iter()
            .find(|v| v.name == n)
            .map(|v| v.value.clone())
    };
    let text = |t: &str| Some(kentos_sheet::model::VarValue::Text(t.into()));
    assert_eq!(var("dosya_turu"), text("PL"));
    assert_eq!(var("dosya_tipi"), text("CAT"));
    assert_eq!(var("kat_konum"), text("NNN"));
    let level = sheet
        .items
        .iter()
        .find_map(|i| match &i.kind {
            kentos_sheet::ItemKind::Map(m) => m.level.clone(),
            _ => None,
        })
        .expect("its map's level");
    assert_eq!(level.id, "k1");
    assert!(level.roof && !level.below, "{level:?}");
}

/// Denetim › Mevzuat, çatılar (docs/adr/8044 §9). On the facade building, a hip roof drawn from its walls with 1.20 m
/// eaves (14.65 × 11.65 m) at 75 %, 10 cm over its storey, no snow guard; a vent 0.90 m over the roof, a chimney 0.50 m
/// over the ridge; a 1.40 × 1.60 m window on the south face 2.50 m in from the eaves (its foot 1.86 m in: 0.66 m from the
/// facade line) and a 0.78 × 1.18 m one 1.50 m east of it. PAİY alone says the eaves, the vent and the chimney (the ridge,
/// 4.47 m over the saçak seviyesi, is within its 5.00 m); Sivas's rules read over it say the ridge (4.00 m), the slope,
/// the snow guard and the windows too, in the codes' order. The snow guard's fix gives the roof one and its finding
/// goes.
#[test]
fn the_regulation_measures_the_roof_its_chimneys_and_its_windows() {
    let mut doc = facades();
    let eaves = roofs::eaves_from_walls(
        &doc,
        kentos_commands::Vec2 { x: 6.0, y: 4.5 },
        Some("k1"),
        1.2,
    )
    .expect("the walls enclose the point");
    let x0 = eaves.iter().map(|p| p.x).fold(f64::INFINITY, f64::min);
    let y0 = eaves.iter().map(|p| p.y).fold(f64::INFINITY, f64::min);
    let at = |x: f64, y: f64| json!({"x": x0 + x, "y": y0 + y});
    let slopes = vec![75.0; eaves.len()];
    let answer = registry::run(
        &mut ExecutionContext::new(&mut doc),
        "cad.roofs.create",
        CallOp::Execute,
        json!({"layerId": "0", "roofs": [{"pts": eaves, "slopes": slopes, "overhang": 1.2, "offset": 0.1,
            "arch": {"level": "k1"}, "parts": [
                {"kind": "vent", "at": at(10.0, 8.0), "width": 0.4, "depth": 0.4, "rise": 0.9},
                {"kind": "chimney", "at": at(7.0, 8.5), "width": 0.4, "depth": 0.4, "rise": 0.5},
                {"kind": "window", "at": at(4.0, 2.5), "width": 1.4, "depth": 1.6},
                {"kind": "window", "at": at(6.59, 3.5), "width": 0.78, "depth": 1.18}
            ]}]}),
    )
    .expect("runs");
    assert_eq!(answer["status"], "completed", "{answer}");
    let roof_codes = [
        "paiy_ridge_high",
        "paiy_eaves_wide",
        "paiy_roof_vent_high",
        "bykhy_chimney_low",
        "local_roof_slope_steep",
        "local_snow_guard_missing",
        "local_roof_window_large",
        "local_roof_window_close",
    ];
    let regulation = |doc: &mut Document| -> Vec<serde_json::Value> {
        let answer = registry::run(
            &mut ExecutionContext::new(doc),
            "cad.arch.audit",
            CallOp::Execute,
            json!({"checks": ["regulation"]}),
        )
        .unwrap();
        answer["output"]["findings"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|f| roof_codes.contains(&f["code"].as_str().unwrap_or_default()))
            .cloned()
            .collect()
    };
    let said = |found: &[serde_json::Value]| -> Vec<String> {
        found
            .iter()
            .map(|f| f["message"].as_str().unwrap_or_default().to_owned())
            .collect()
    };
    assert_eq!(
        said(&regulation(&mut doc)),
        [
            "Saçak genişliği 1.20 m > 1.00 m — PAİY 42/1",
            "Hava bacasının çatı örtüsünü aşan yüksekliği 0.90 m > 0.60 m — PAİY 40/5",
            "Duman bacasının mahyayı aşan yüksekliği 0.50 m < 0.80 m — BYKHY 58/2",
        ]
    );
    let mut s = doc.settings().clone();
    s.arch.as_mut().expect("arch settings").local_rules = Some(kentos_commands::LocalRules::Sivas);
    doc.set_settings(s);
    let found = regulation(&mut doc);
    assert_eq!(
        said(&found),
        [
            "Mahya yüksekliği (saçak seviyesinden) 4.47 m > 4.00 m — Sivas 40/5-g; PAİY 40/4: 5.00 m",
            "Saçak genişliği 1.20 m > 1.00 m — PAİY 42/1",
            "Hava bacasının çatı örtüsünü aşan yüksekliği 0.90 m > 0.60 m — PAİY 40/5",
            "Duman bacasının mahyayı aşan yüksekliği 0.50 m < 0.80 m — BYKHY 58/2",
            "Çatının eğimi %75 > %40 — Sivas 40/5-g",
            "Eğimli çatının saçağında kar siperliği yok — Sivas 42/5",
            "Çatı penceresi 1.40 × 1.60 m, en çok 1.20 × 1.50 m (en/boy) — Sivas 40/5-e-2",
            "Çatı penceresinin bina cephe hattına uzaklığı 0.66 m < 1.00 m — Sivas 40/5-e-3",
            "İki çatı penceresi arası 1.50 m < 2.00 m — Sivas 40/5-e-3",
        ]
    );
    // Each names its article and gazette; the snow guard's fix is cad.arch.edit's on the roof.
    let guard = found
        .iter()
        .find(|f| f["code"] == "local_snow_guard_missing")
        .expect("the snow guard's finding");
    assert_eq!(guard["rule"], "Sivas İmar Yön. 42/5 · RG 10.11.2022/32009");
    let step = &guard["fix"]["step"];
    assert_eq!(step["command"], "cad.arch.edit", "{guard}");
    let answer = registry::run(
        &mut ExecutionContext::new(&mut doc),
        "cad.arch.edit",
        CallOp::Execute,
        step["input"].clone(),
    )
    .unwrap();
    assert_eq!(answer["status"], "completed", "{answer}");
    assert!(
        !regulation(&mut doc)
            .iter()
            .any(|f| f["code"] == "local_snow_guard_missing")
    );
}

/// Baca ve niş (docs/adr/8070 §3): a flue in the hall's west wall (x 4.5, İç duvar 15) on both storeys, 7 cm across
/// and centred (4 cm off each face), and a chimney on the roof over it are one IfcChimney: three bodies (the two
/// hollows and the stack from the roof's seat, +6.00), USERDEFINED "Duman bacası", its GlobalId the ground floor's flue.
/// Each hollow voids its wall, the chimney filling it. A niche in the east hall wall is a recess with no filler. The
/// section model takes the flue out of its wall as a box: 4.465 ≤ x ≤ 4.535 between its jambs (y 7.9 to 8.1, its
/// middle at 8.0, clear of the hall's door at 6.0 to 6.9).
#[test]
fn a_flue_in_its_walls_and_the_chimney_over_it_are_one_ifc_chimney_and_a_niche_a_recess() {
    use kentos_ifc::building::Class;
    let mut doc = roofed(&building());
    let roof = doc
        .entities_with_uids()
        .find_map(|(u, e)| matches!(e, Entity::Roof(_)).then_some(u))
        .unwrap();
    let run = |doc: &mut Document, name: &str, input: serde_json::Value| -> serde_json::Value {
        let answer = registry::run(&mut ExecutionContext::new(doc), name, CallOp::Execute, input).expect("runs");
        assert_eq!(answer["status"], "completed", "{answer}");
        answer
    };
    run(
        &mut doc,
        "cad.arch.edit",
        json!({"uids": [roof.to_string()], "operation": {"kind": "set", "parts": [
            {"kind": "chimney", "at": {"x": 4.5, "y": 8.0}, "width": 0.4, "depth": 0.4}
        ]}}),
    );
    // The hall's walls (x 4.5 west, x 7.5 east) from y 0 to 9 on each storey.
    let hall = |doc: &Document, x: f64| -> Vec<(String, Option<String>)> {
        doc.entities_with_uids()
            .filter_map(|(u, e)| match e {
                Entity::Wall(w)
                    if w.wall.pts.len() == 2
                        && w.wall.pts.iter().all(|q| (q.x - x).abs() < 1e-9)
                        && w.wall.pts[0].y.abs() < 1e-9 =>
                {
                    Some((u.to_string(), w.wall.arch.level.clone()))
                }
                _ => None,
            })
            .collect()
    };
    let west = hall(&doc, 4.5);
    assert_eq!(west.len(), 2, "{west:?}");
    let mut flues = Vec::new();
    for (wall, _) in &west {
        let out = run(
            &mut doc,
            "cad.openings.create",
            json!({"layerId": "0", "openings": [
                {"host": wall, "at": 8.0, "width": 0.2, "form": "flue", "operation": "fixed", "depth": 0.07}
            ]}),
        );
        flues.push(out["output"]["uids"][0].as_str().unwrap().to_owned());
    }
    let east = hall(&doc, 7.5);
    let zemin = east
        .iter()
        .find(|(_, l)| l.as_deref() == Some("zemin"))
        .expect("the ground floor's east hall wall");
    let out = run(
        &mut doc,
        "cad.openings.create",
        json!({"layerId": "0", "openings": [
            {"host": zemin.0, "at": 2.0, "width": 0.6, "form": "niche", "operation": "fixed", "side": "right",
             "depth": 0.05, "sill": 0.9, "head": 1.5}
        ]}),
    );
    let niche = out["output"]["uids"][0].as_str().unwrap().to_owned();
    // The section model: the flue a box out of every solid of its wall.
    let m = sections::model(&doc);
    let near = |a: f64, b: f64| (a - b).abs() < 1e-9;
    let ground = west
        .iter()
        .find(|(_, l)| l.as_deref() == Some("zemin"))
        .unwrap()
        .0
        .clone();
    let walls: Vec<&kentos_geometry_core::geom::section::Solid> = m
        .solids
        .iter()
        .zip(&m.owners)
        .filter(|(_, o)| o.as_deref() == Some(ground.as_str()))
        .map(|(s, _)| s)
        .collect();
    assert!(!walls.is_empty());
    for s in &walls {
        assert!(
            s.voids.iter().any(|v| {
                let xs = v.ring.iter().map(|q| q.x);
                let ys: Vec<f64> = v.ring.iter().map(|q| q.y).collect();
                xs.clone().any(|x| near(x, 4.465))
                    && xs.clone().any(|x| near(x, 4.535))
                    && ys.iter().any(|y| near(*y, 7.9))
                    && ys.iter().any(|y| near(*y, 8.1))
                    && near(v.bottom, 0.0)
            }),
            "{:?}",
            s.voids
        );
    }
    // The IFC model.
    let (b, written, _) = kentos_native_application::arch_ifc::model(&doc);
    let lost: Vec<&String> = written.left.iter().map(|l| &l.id).collect();
    for uid in flues.iter().chain([&niche]) {
        assert!(!lost.contains(&uid), "{uid} lost: {:?}", written.left);
    }
    let chimneys: Vec<(usize, &kentos_ifc::building::Element)> = b
        .elements
        .iter()
        .enumerate()
        .filter(|(_, e)| e.class == Class::Chimney)
        .collect();
    assert_eq!(chimneys.len(), 1, "{chimneys:?}");
    let (k, c) = chimneys[0];
    assert_eq!(c.predefined, "USERDEFINED");
    assert_eq!(c.object_type.as_deref(), Some("Duman bacası"));
    assert!(c.name.starts_with("MM-BAC-"), "{}", c.name);
    assert_eq!(c.shapes.len(), 3);
    let ground_flue = kentos_domain::Uuid::parse_str(&flues[0]).unwrap();
    let first = west.iter().position(|(w, _)| *w == ground).unwrap();
    let lowest = kentos_domain::Uuid::parse_str(&flues[first]).unwrap();
    assert_eq!(c.id, Some(lowest.as_u128()), "{ground_flue}");
    assert!(c.material.is_some());
    let bottoms: Vec<f64> = c
        .shapes
        .iter()
        .map(|s| match s {
            kentos_ifc::building::Shape::Prism { bottom, .. } => *bottom,
            other => panic!("{other:?}"),
        })
        .collect();
    assert!(near(bottoms[0], 0.0) && near(bottoms[1], 3.0) && near(bottoms[2], 6.0), "{bottoms:?}");
    let hollows: Vec<&kentos_ifc::building::Opening> =
        b.openings.iter().filter(|o| o.predefined == "OPENING" && o.filler == Some(k)).collect();
    assert_eq!(hollows.len(), 2, "{:?}", b.openings);
    let niche_id = kentos_domain::Uuid::parse_str(&niche).unwrap().as_u128();
    let recess: Vec<&kentos_ifc::building::Opening> =
        b.openings.iter().filter(|o| o.predefined == "RECESS").collect();
    assert_eq!(recess.len(), 1);
    assert_eq!((recess[0].id, recess[0].filler), (Some(niche_id), None));
    assert!(near(recess[0].bottom, 0.9) && near(recess[0].top, 1.5));
    assert_eq!(written.chimneys, 1);
    // The text: the chimney fills its hollows.
    let text = kentos_ifc::building::write(&b);
    assert!(text.contains("IFCCHIMNEY("), "the chimney");
    assert_eq!(text.matches("IFCRELFILLSELEMENT(").count(), b.openings.iter().filter(|o| o.filler.is_some()).count());
}

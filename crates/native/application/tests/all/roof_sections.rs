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

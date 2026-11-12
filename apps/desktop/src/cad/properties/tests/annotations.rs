//! Öznitelikler's tests: multi-part objects, block attributes, texts, leaders and dimensions, mixed
//! selections, shared attributes and the panel's speed and cache.

use super::*;

/// A multi-part area (docs/adr/0143): its corners, holes, perimeter and area
/// are every part's, and a row says how many parts it has. Parcel 4
/// (12 × 10 m) gets a 10 × 10 m part with a 2 × 2 m hole.
#[test]
fn a_multi_part_area_counts_every_part() {
    let mut app = objects();
    click(&mut app, -18.0, 9.0);
    let doc = app.document.as_mut().expect("open");
    let Some(Entity::Polygon(mut p)) = doc.model.get(Slot(4)).cloned() else {
        panic!("the parcel");
    };
    let at = |x: f64, y: f64| Wire { x: E + x, y: N + y };
    p.parts = Some(vec![kentos_commands::AreaPart {
        ms: None,
        pts: vec![at(30.0, 0.0), at(40.0, 0.0), at(40.0, 10.0), at(30.0, 10.0)],
        bulges: None,
        holes: Some(vec![kentos_commands::RingGeometry {
            pts: vec![at(34.0, 4.0), at(34.0, 6.0), at(36.0, 6.0), at(36.0, 4.0)],
            bulges: None,
            zs: None,
        }]),
        zs: None,
    }]);
    assert!(doc.model.update(Slot(4), Entity::Polygon(p)));
    assert_eq!(value(&app, "Geometri", "Köşe sayısı"), "8");
    assert_eq!(value(&app, "Geometri", "Parça sayısı"), "2");
    assert_eq!(value(&app, "Geometri", "Ada (delik)"), "1");
    // 44 + 40 + 8 m; 120 + 100 − 4 m².
    assert_eq!(value(&app, "Geometri", "Çevre"), "92.000 m");
    assert_eq!(value(&app, "Geometri", "Alan"), "216.00 m²");
}

/// A multi-part polyline counts every part and a multi-point object its
/// points (docs/adr/0174 §6): Köşe sayısı and Uzunluk are every part's, with
/// Parça sayısı; Nokta sayısı and one Kot row for every point.
#[test]
fn multi_part_lines_and_points_count_every_part() {
    let mut app = objects();
    let at = |x: f64, y: f64| Wire { x: E + x, y: N + y };
    let doc = app.document.as_mut().expect("open");
    let Some(Entity::Line(line)) = doc.model.get(Slot(1)).cloned() else {
        panic!("the line");
    };
    let road = Entity::Polyline(kentos_commands::PathEntity {
        space_separator: None,
        ms: None,
        base: line.base.clone(),
        pts: vec![at(-24.0, -12.0), at(-8.0, -12.0)],
        bulges: None,
        holes: None,
        zs: None,
        parts: Some(vec![kentos_commands::AreaPart {
            ms: None,
            pts: vec![at(0.0, -20.0), at(0.0, -10.0), at(5.0, -10.0)],
            bulges: None,
            holes: None,
            zs: None,
        }]),
        closed: false,
        widths: None,
    });
    assert!(doc.model.update(Slot(1), road));
    let Some(Entity::Point(mut marks)) = doc.model.get(Slot(5)).cloned() else {
        panic!("the point");
    };
    marks.z = Some(100.0);
    marks.parts = Some(vec![
        kentos_commands::PointPart {
            p: at(25.0, -8.0),
            z: Some(100.0),
        },
        kentos_commands::PointPart {
            p: at(30.0, -8.0),
            z: Some(102.5),
        },
    ]);
    assert!(doc.model.update(Slot(5), Entity::Point(marks)));
    select(&mut app, &[1]);
    assert_eq!(value(&app, "Geometri", "Köşe sayısı"), "5");
    assert_eq!(value(&app, "Geometri", "Parça sayısı"), "2");
    // 16 + 10 + 5 m.
    assert_eq!(value(&app, "Geometri", "Uzunluk"), "31.000 m");
    select(&mut app, &[5]);
    assert_eq!(value(&app, "Geometri", "Nokta sayısı"), "3");
    assert_eq!(value(&app, "Geometri", "Kot"), "100.000–102.500 m");
    // Typed into Kot, every point takes it.
    event(
        &mut app,
        Event::Commit(Field::Elevation(Slot(5), Spot::All), "99".into()),
    );
    let Entity::Point(p) = entity(&app, 5) else {
        panic!("a point");
    };
    assert_eq!(p.z, Some(99.0));
    assert!(p.parts.iter().flatten().all(|q| q.z == Some(99.0)));
}

/// The KCAD v2 blocks fixture (fixtures/kcad/v2/blocks.kcad): Rögar's attributes
/// are NO (default "R-1") and KOT (no default); insert 1 has both, insert 2 none.
pub(super) fn blocks_drawing() -> App {
    let (mut app, _) = App::boot(None);
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../fixtures/kcad/v2/blocks.kcad"
    );
    let doc = crate::document::Document::read(std::path::Path::new(path)).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    app
}

/// A section's labels in order.
pub(super) fn labels(app: &App, section: &str) -> Vec<String> {
    rows(app)
        .into_iter()
        .find(|(title, _)| title == section)
        .map(|(_, rows)| rows.into_iter().map(|r| r.label.into_owned()).collect())
        .unwrap_or_default()
}

/// An insert's block attributes (docs/adr/0144 §7): the definition's tags in its
/// order with what the insert shows, its own value else the default; they are
/// not repeated among its other attributes. A cell writes the insert's value.
#[test]
fn an_insert_shows_its_block_attributes_and_a_cell_writes_its_value() {
    let mut app = blocks_drawing();
    select(&mut app, &[1]);
    assert_eq!(labels(&app, "Blok öznitelikleri"), ["NO", "KOT"]);
    assert_eq!(value(&app, "Blok öznitelikleri", "NO"), "R-12");
    assert_eq!(value(&app, "Blok öznitelikleri", "KOT"), "101.35");
    assert!(labels(&app, "Öznitelik bilgileri").is_empty());
    // Without values: the default, and nothing for KOT.
    select(&mut app, &[2]);
    assert_eq!(value(&app, "Blok öznitelikleri", "NO"), "R-1");
    assert_eq!(value(&app, "Blok öznitelikleri", "KOT"), "");
    event(
        &mut app,
        Event::Commit(Field::Attribute(Slot(2), "NO".into()), "R-7".into()),
    );
    assert_eq!(
        entity(&app, 2).base().attrs.get("NO").map(String::as_str),
        Some("R-7")
    );
    assert_eq!(value(&app, "Blok öznitelikleri", "NO"), "R-7");
    // An empty value gives the default back.
    event(
        &mut app,
        Event::Commit(Field::Attribute(Slot(2), "NO".into()), String::new()),
    );
    assert_eq!(value(&app, "Blok öznitelikleri", "NO"), "R-1");
    // An attribute that is not the block's stays among the others.
    event(
        &mut app,
        Event::Commit(Field::Attribute(Slot(1), "Malzeme".into()), "Beton".into()),
    );
    select(&mut app, &[1]);
    assert_eq!(labels(&app, "Öznitelik bilgileri"), ["Malzeme"]);
    // The lighting pole has none: no section.
    select(&mut app, &[3]);
    assert!(labels(&app, "Blok öznitelikleri").is_empty());
}

/// Pictures of an insert's Blok öznitelikleri (docs/adr/0144 §7) over the
/// block attributes trace's drawing (fixtures/interaction/v1/block-attributes.kcad),
/// Genel and Geometri closed: the west manhole's own values, the middle one's
/// default, its NO cell typed into (the default wiped) and written with Enter.
/// `.run/shots/oznitelik-blok-*`.
/// Not run by default: `cargo test -p kentos-desktop properties::tests::block_attribute_screens -- --ignored --nocapture`.
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn block_attribute_screens() {
    use iced::Size;
    use kentos_ui::snapshot::{Input, Snapshot};

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/shots");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    let drawing = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../fixtures/interaction/v1/block-attributes.kcad"
    );
    for (mode, suffix) in [("dark", ""), ("light", "-acik")] {
        for (width, height) in [(1440.0, 900.0), (1100.0, 650.0)] {
            for (name, slot) in [
                ("kendi", 3),
                ("varsayilan", 4),
                ("yaziliyor", 4),
                ("yazildi", 4),
            ] {
                let (mut app, _) = App::boot(None);
                let _ = app
                    .settings
                    .choose(&[("appearance.theme", serde_json::Value::from(mode))]);
                app.apply_settings();
                let doc =
                    crate::document::Document::read(std::path::Path::new(drawing)).expect("opens");
                let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
                let mut snapshot = Snapshot::new(Size::new(width, height)).expect("a renderer");
                let mut update = |app: &mut App, message| {
                    let _ = app.update(message);
                };
                snapshot.settle(&mut app, App::view, &mut update);
                app.viewport.camera.center = kentos_interaction::Vec2::new(487015.0, 4420008.5);
                app.viewport.camera.scale = 1.0 / 0.03;
                select(&mut app, &[slot]);
                let _ = app.update(Message::Properties(Event::Toggle("general")));
                let _ = app.update(Message::Properties(Event::Toggle("geometry")));
                snapshot.settle(&mut app, App::view, &mut update);
                if name == "yaziliyor" || name == "yazildi" {
                    // The NO cell (read off the pictures); it opens with the default, R-?.
                    let at = if height > 800.0 {
                        iced::Point::new(1330.0, 709.0)
                    } else {
                        iced::Point::new(1000.0, 583.0)
                    };
                    snapshot.input(&mut app, App::view, &mut update, Input::Click(at));
                    for _ in 0..3 {
                        let back = Input::Key(iced::keyboard::key::Named::Backspace);
                        snapshot.input(&mut app, App::view, &mut update, back);
                    }
                    snapshot.input(&mut app, App::view, &mut update, Input::Type("R-7".into()));
                }
                if name == "yazildi" {
                    let enter = Input::Key(iced::keyboard::key::Named::Enter);
                    snapshot.input(&mut app, App::view, &mut update, enter);
                    let e = entity(&app, 4);
                    assert_eq!(e.base().attrs.get("NO").map(String::as_str), Some("R-7"));
                }
                snapshot.settle(&mut app, App::view, &mut update);
                let file = out.join(format!(
                    "oznitelik-blok-{name}-{width}x{height}{suffix}.png"
                ));
                snapshot
                    .render(app.view(), &app.theme())
                    .save(&file)
                    .expect("writes the picture");
                println!("{}", file.display());
            }
        }
    }
}

/// Two texts for the text rows (docs/adr/0145 §6): one plain, 30° turned;
/// one centred, narrowed and masked.
pub(super) fn two_texts(app: &mut App) -> (u32, u32) {
    let text = |p: Wire, text: &str, rotation: f64| TextEntity {
        annotative: None,
        base: base("cizim"),
        p,
        text: text.to_owned(),
        height: 2.0,
        rotation,
        align: None,
        width_factor: None,
        mask: false,
        label_of: None,
        label_scale: None,
        paragraph: Default::default(),
        face: Default::default(),
        path: None,
    };
    let a = add(
        app,
        Entity::Text(text(
            Wire {
                x: E + 10.0,
                y: N + 20.0,
            },
            "Ada 101",
            30.0,
        )),
    );
    let b = add(
        app,
        Entity::Text(TextEntity {
            align: Some(kentos_commands::TextAlign::MiddleCenter),
            width_factor: Some(0.8),
            mask: true,
            ..text(
                Wire {
                    x: E + 10.0,
                    y: N + 10.0,
                },
                "Ada 102",
                0.0,
            )
        }),
    );
    (a, b)
}

pub(super) fn text_of(app: &App, slot: u32) -> TextEntity {
    match entity(app, slot) {
        Entity::Text(t) => t,
        other => panic!("a text: {other:?}"),
    }
}

/// A text's box as it is drawn (the drawing's Barlow), to a micrometre.
pub(super) fn text_box(_app: &App, t: &TextEntity) -> Vec<[i64; 2]> {
    let font = kentos_geometry_core::text::Font::from_id("barlow");
    let place = kentos_geometry_core::entity::TextPlace::line(
        kentos_geometry_core::vec2::Vec2::new(t.p.x, t.p.y),
        &t.text,
        t.height,
        t.rotation,
        t.align
            .and_then(|a| kentos_geometry_core::text::TextAlign::from_name(a.name())),
        t.width_factor,
    );
    place
        .outline(font)
        .iter()
        .map(|q| [(q.x * 1e6).round() as i64, (q.y * 1e6).round() as i64])
        .collect()
}

/// Öznitelikler's Hiza, Genişlik çarpanı and Zemin (docs/adr/0145 §6): one
/// text's values, the twelve points with their pictures; the web's are
/// apps/web/src/ui/properties/textRows.test.ts.
#[test]
fn a_texts_rows_show_its_alignment_width_factor_and_mask() {
    let mut app = objects();
    let (_, b) = two_texts(&mut app);
    select(&mut app, &[b]);
    assert_eq!(value(&app, "Geometri", "Hiza"), "Orta");
    assert_eq!(value(&app, "Geometri", "Genişlik çarpanı"), "0.8");
    assert_eq!(value(&app, "Geometri", "Zemin"), "Açık");
    let rows = rows(&app);
    let hiza = rows
        .iter()
        .flat_map(|(_, rows)| rows)
        .find(|r| r.label == "Hiza")
        .expect("Hiza");
    let Some(Editor::Select { icon, items, .. }) = &hiza.editor else {
        panic!("a drop-down");
    };
    assert_eq!(*icon, Some("textAlignMiddleCenter"));
    let picks: Vec<(String, bool, Option<&str>)> = items
        .iter()
        .filter_map(|c| match c {
            Choice::Pick {
                label,
                chosen,
                icon,
                ..
            } => Some((label.clone(), *chosen, *icon)),
            _ => None,
        })
        .collect();
    assert_eq!(picks.len(), 12);
    assert_eq!(
        picks[3],
        ("Sol orta".to_owned(), false, Some("textAlignMiddleLeft"))
    );
    assert_eq!(
        picks[4],
        ("Orta".to_owned(), true, Some("textAlignMiddleCenter"))
    );
}

/// Over a selection the texts' rows say Çeşitli where they differ; a new
/// alignment keeps each where it is, in one step.
#[test]
fn a_new_alignment_keeps_each_text_where_it_is() {
    let mut app = objects();
    let (a, b) = two_texts(&mut app);
    select(&mut app, &[a, b]);
    for label in ["Hiza", "Genişlik çarpanı", "Zemin"] {
        assert_eq!(value(&app, "Yazı", label), "Çeşitli");
    }
    let before = [
        text_box(&app, &text_of(&app, a)),
        text_box(&app, &text_of(&app, b)),
    ];
    let (pa, pb) = (text_of(&app, a).p, text_of(&app, b).p);
    event(
        &mut app,
        Event::TextAlign(
            vec![Slot(a), Slot(b)],
            Some(kentos_commands::TextAlign::TopRight),
        ),
    );
    let (ta, tb) = (text_of(&app, a), text_of(&app, b));
    assert_eq!(
        (ta.align, tb.align),
        (
            Some(kentos_commands::TextAlign::TopRight),
            Some(kentos_commands::TextAlign::TopRight)
        )
    );
    assert_eq!([text_box(&app, &ta), text_box(&app, &tb)], before);
    assert_ne!(ta.p, pa);
    assert_eq!(value(&app, "Yazı", "Hiza"), "Sağ üst");
    assert_eq!(undo_label(&mut app).as_deref(), Some("Değiştir"));
    let (ta, tb) = (text_of(&app, a), text_of(&app, b));
    assert_eq!(
        (ta.p, tb.p, ta.align, tb.align),
        (
            pa,
            pb,
            None,
            Some(kentos_commands::TextAlign::MiddleCenter)
        )
    );
    // The left of the baseline takes the field away; the text still stays.
    let before = text_box(&app, &tb);
    event(&mut app, Event::TextAlign(vec![Slot(b)], None));
    let tb = text_of(&app, b);
    assert_eq!((tb.align, text_box(&app, &tb)), (None, before));
}

/// Genişlik çarpanı about each text's point, 1 as no field, a refusal said;
/// Zemin on and off, nothing written when nothing changes.
#[test]
fn width_factor_and_mask_are_written_to_the_texts() {
    let mut app = objects();
    let (a, b) = two_texts(&mut app);
    let (sa, sb) = (Slot(a), Slot(b));
    event(
        &mut app,
        Event::Commit(Field::TextWidth(vec![sa, sb]), "1,5".to_owned()),
    );
    let pb = text_of(&app, b).p;
    assert_eq!(
        (
            text_of(&app, a).width_factor,
            text_of(&app, b).width_factor,
            text_of(&app, b).p
        ),
        (Some(1.5), Some(1.5), pb)
    );
    event(
        &mut app,
        Event::Commit(Field::TextWidth(vec![sa]), "1".to_owned()),
    );
    assert_eq!(text_of(&app, a).width_factor, None);
    event(
        &mut app,
        Event::Commit(Field::TextWidth(vec![sb]), "0".to_owned()),
    );
    assert_eq!(text_of(&app, b).width_factor, Some(1.5));
    assert_eq!(
        warned(&app).as_deref(),
        Some(
            "Yazının genişlik çarpanı 0'dan büyük, en çok 100 olmalı; 0 verildi. Çarpanı bu aralıkta verin ya da alanı kaldırın (1)."
        )
    );
    // Zemin: b has one already, so turning it on writes nothing.
    let revision = |app: &App| app.document.as_ref().map_or(0, |d| d.model.revision());
    let before = revision(&app);
    event(&mut app, Event::TextMask(vec![sb], true));
    assert_eq!(revision(&app), before);
    event(&mut app, Event::TextMask(vec![sa, sb], true));
    assert_eq!((text_of(&app, a).mask, text_of(&app, b).mask), (true, true));
    event(&mut app, Event::TextMask(vec![sa, sb], false));
    assert_eq!(
        (text_of(&app, a).mask, text_of(&app, b).mask),
        (false, false)
    );
    assert_eq!(undo_label(&mut app).as_deref(), Some("Değiştir"));
    assert_eq!((text_of(&app, a).mask, text_of(&app, b).mask), (true, true));
}

/// Two leaders: a filled arrow with its note, an open one masked, on Çizim.
pub(super) fn two_leaders(app: &mut App) -> (u32, u32) {
    let leader = |x: f64, text: &str, arrow, mask| kentos_commands::LeaderEntity {
        tip: None,
        leader_style: None,
        blocks: Vec::new(),
        arms: Vec::new(),
        annotative: None,
        base: base("cizim"),
        pts: vec![
            Wire {
                x: 487000.0 + x,
                y: 4420020.0,
            },
            Wire {
                x: 487004.0 + x,
                y: 4420023.0,
            },
        ],
        text: Some(text.to_owned()),
        height: 2.0,
        rotation: 0.0,
        arrow,
        arrow_size: None,
        mask,
    };
    let a = add(app, Entity::Leader(leader(0.0, "Mevcut bina", None, false)));
    let b = add(
        app,
        Entity::Leader(leader(
            20.0,
            "Ø150 PVC",
            Some(kentos_commands::LeaderArrow::Open),
            true,
        )),
    );
    (a, b)
}

pub(super) fn leader_of(app: &App, slot: u32) -> kentos_commands::LeaderEntity {
    match entity(app, slot) {
        Entity::Leader(l) => l,
        other => panic!("a leader: {other:?}"),
    }
}

/// A leader's rows (docs/adr/0146 §7): its note, height, turn, arrowhead and
/// mask under Geometri; over a selection, Kılavuz with Çeşitli where they
/// differ, every change one step “Değiştir”, nothing written when nothing
/// changes, an emptied note the arrow alone, a height not over 0 not taken.
#[test]
fn a_leaders_rows_write_its_note_height_turn_arrowhead_and_mask() {
    let mut app = objects();
    let (a, b) = two_leaders(&mut app);
    select(&mut app, &[a]);
    assert_eq!(value(&app, "Geometri", "Not"), "Mevcut bina");
    assert_eq!(value(&app, "Geometri", "Yükseklik"), "2.000 m");
    assert_eq!(value(&app, "Geometri", "Dönüş"), "0.00 °");
    assert_eq!(value(&app, "Geometri", "Ok"), "Dolu üçgen");
    assert_eq!(value(&app, "Geometri", "Ok boyu"), "1.00 × yükseklik");
    assert_eq!(value(&app, "Geometri", "Zemin"), "Kapalı");
    assert_eq!(value(&app, "Geometri", "Köşe sayısı"), "2");
    assert_eq!(value(&app, "Geometri", "Uzunluk"), "5.000 m");
    select(&mut app, &[a, b]);
    for label in ["Not", "Ok", "Zemin"] {
        assert_eq!(value(&app, "Kılavuz", label), "Çeşitli");
    }
    assert_eq!(value(&app, "Kılavuz", "Yükseklik"), "2.000 m");
    let (sa, sb) = (Slot(a), Slot(b));
    event(
        &mut app,
        Event::LeaderArrow(vec![sa, sb], Some(kentos_commands::LeaderArrow::Dot)),
    );
    assert_eq!(
        (leader_of(&app, a).arrow, leader_of(&app, b).arrow),
        (
            Some(kentos_commands::LeaderArrow::Dot),
            Some(kentos_commands::LeaderArrow::Dot)
        )
    );
    assert_eq!(value(&app, "Kılavuz", "Ok"), "Dolu nokta");
    assert_eq!(undo_label(&mut app).as_deref(), Some("Değiştir"));
    // Ok boyu (docs/adr/0205 §7): outside 0.1 to 10 not taken; written to both in one step; 1 is no field.
    let size_before = app.document.as_ref().map_or(0, |d| d.model.revision());
    event(
        &mut app,
        Event::Commit(Field::LeaderArrowSize(vec![sa, sb]), "12".to_owned()),
    );
    assert_eq!(
        app.document.as_ref().map_or(0, |d| d.model.revision()),
        size_before
    );
    event(
        &mut app,
        Event::Commit(Field::LeaderArrowSize(vec![sa, sb]), "1,5".to_owned()),
    );
    assert_eq!(
        (leader_of(&app, a).arrow_size, leader_of(&app, b).arrow_size),
        (Some(1.5), Some(1.5))
    );
    assert_eq!(value(&app, "Kılavuz", "Ok boyu"), "1.50 × yükseklik");
    event(
        &mut app,
        Event::Commit(Field::LeaderArrowSize(vec![sa]), "1".to_owned()),
    );
    assert_eq!(leader_of(&app, a).arrow_size, None);
    assert_eq!(value(&app, "Kılavuz", "Ok boyu"), "Çeşitli");
    // Zemin on: b has it already, a alone is written.
    event(&mut app, Event::LeaderMask(vec![sa, sb], true));
    assert_eq!(
        (leader_of(&app, a).mask, leader_of(&app, b).mask),
        (true, true)
    );
    event(
        &mut app,
        Event::Commit(Field::LeaderTurn(vec![sa, sb]), "370".to_owned()),
    );
    assert_eq!(leader_of(&app, b).rotation, 10.0);
    let revision = |app: &App| app.document.as_ref().map_or(0, |d| d.model.revision());
    let before = revision(&app);
    event(
        &mut app,
        Event::Commit(Field::LeaderHeight(vec![sa]), "0".to_owned()),
    );
    assert_eq!(revision(&app), before, "a height not over 0 is not taken");
    event(
        &mut app,
        Event::Commit(Field::LeaderHeight(vec![sa]), "3,5".to_owned()),
    );
    assert_eq!(leader_of(&app, a).height, 3.5);
    event(
        &mut app,
        Event::Commit(Field::LeaderNote(vec![sb]), "  ".to_owned()),
    );
    assert_eq!(leader_of(&app, b).text, None, "the arrow alone");
    event(
        &mut app,
        Event::Commit(Field::LeaderNote(vec![sb]), " Ø200 PVC ".to_owned()),
    );
    assert_eq!(leader_of(&app, b).text.as_deref(), Some("Ø200 PVC"));
    assert_eq!(undo_label(&mut app).as_deref(), Some("Değiştir"));
    assert_eq!(leader_of(&app, b).text, None);
}

/// A dimension of `style` from (E + a, N) to (E + b, N), on Çizim.
pub(super) fn dimension(
    style: kentos_commands::DimensionStyle,
    a: [f64; 2],
    b: [f64; 2],
) -> DimensionEntity {
    DimensionEntity {
        annotative: None,
        text_shift: None,
        breaks: Vec::new(),
        base: base("cizim"),
        a: Wire {
            x: E + a[0],
            y: N + a[1],
        },
        b: Wire {
            x: E + b[0],
            y: N + b[1],
        },
        offset: 3.0,
        height: 2.5,
        text: None,
        style: Some(style),
        angle: None,
        c: None,
        mask: false,
        za: None,
        zb: None,
        refs: None,
        look: Default::default(),
        level: None,
    }
}

pub(super) fn dimension_of(app: &App, slot: u32) -> DimensionEntity {
    match entity(app, slot) {
        Entity::Dimension(d) => d,
        other => panic!("a dimension: {other:?}"),
    }
}

/// A dimension's rows (docs/adr/0147 §7): Zemin; an ordinate's Koordinat; a
/// slope's two elevations; an arc length's radius and angle, shown only.
/// Over a selection, Ölçü with Çeşitli where they differ, the kind's rows
/// only when all are of it; every change one step “Değiştir”, those that
/// have the value left out. The web's `dimensionRows.test.ts`.
#[test]
fn a_dimensions_rows_write_its_mask_axis_and_elevations() {
    use kentos_commands::DimensionStyle::{ArcLength, Ordinate, Slope};
    let mut app = objects();
    // A quarter arc of radius 10 about (E, N + 40), east to north, 3 m out.
    let arc = add(
        &mut app,
        Entity::Dimension(DimensionEntity {
            c: Some(Wire { x: E, y: N + 40.0 }),
            ..dimension(ArcLength, [10.0, 40.0], [0.0, 50.0])
        }),
    );
    select(&mut app, &[arc]);
    assert_eq!(value(&app, "Geometri", "Zemin"), "Kapalı");
    assert_eq!(value(&app, "Geometri", "Yarıçap"), "10.000 m");
    assert_eq!(value(&app, "Geometri", "Açı"), "100.0000 g");
    // Two ordinates, the Y's and the X's.
    let oy = add(
        &mut app,
        Entity::Dimension(DimensionEntity {
            angle: Some(0.0),
            ..dimension(Ordinate, [0.0, 60.0], [5.0, 70.0])
        }),
    );
    let ox = add(
        &mut app,
        Entity::Dimension(DimensionEntity {
            angle: Some(90.0),
            mask: true,
            ..dimension(Ordinate, [0.0, 80.0], [-10.0, 85.0])
        }),
    );
    select(&mut app, &[oy, ox]);
    assert_eq!(value(&app, "Ölçü", "Zemin"), "Çeşitli");
    assert_eq!(value(&app, "Ölçü", "Koordinat"), "Çeşitli");
    let revision = |app: &App| app.document.as_ref().map_or(0, |d| d.model.revision());
    event(
        &mut app,
        Event::DimensionAxis(vec![Slot(oy), Slot(ox)], 90.0),
    );
    assert_eq!(
        (dimension_of(&app, oy).angle, dimension_of(&app, ox).angle),
        (Some(90.0), Some(90.0))
    );
    assert_eq!(value(&app, "Ölçü", "Koordinat"), "X");
    let before = revision(&app);
    event(
        &mut app,
        Event::DimensionAxis(vec![Slot(oy), Slot(ox)], 90.0),
    );
    assert_eq!(revision(&app), before, "nothing written when both have it");
    // Zemin on: the X's has it already, the Y's alone is written.
    event(
        &mut app,
        Event::DimensionMask(vec![Slot(oy), Slot(ox)], true),
    );
    assert!(dimension_of(&app, oy).mask && dimension_of(&app, ox).mask);
    assert_eq!(undo_label(&mut app).as_deref(), Some("Değiştir"));
    assert!(!dimension_of(&app, oy).mask);
    // Two slopes: one first elevation, two second ones.
    let s1 = add(
        &mut app,
        Entity::Dimension(DimensionEntity {
            za: Some(100.0),
            zb: Some(99.0),
            ..dimension(Slope, [0.0, 100.0], [40.0, 100.0])
        }),
    );
    let s2 = add(
        &mut app,
        Entity::Dimension(DimensionEntity {
            za: Some(100.0),
            zb: Some(98.0),
            ..dimension(Slope, [0.0, 110.0], [40.0, 110.0])
        }),
    );
    select(&mut app, &[s1, s2]);
    assert_eq!(value(&app, "Ölçü", "Birinci kot"), "100.000 m");
    assert_eq!(value(&app, "Ölçü", "İkinci kot"), "Çeşitli");
    event(
        &mut app,
        Event::Commit(
            Field::DimensionZb(vec![Slot(s1), Slot(s2)]),
            "97,5".to_owned(),
        ),
    );
    assert_eq!(
        (dimension_of(&app, s1).zb, dimension_of(&app, s2).zb),
        (Some(97.5), Some(97.5))
    );
    assert_eq!(value(&app, "Ölçü", "İkinci kot"), "97.500 m");
    let before = revision(&app);
    event(
        &mut app,
        Event::Commit(Field::DimensionZa(vec![Slot(s1)]), "kot".to_owned()),
    );
    assert_eq!(revision(&app), before, "not a number: nothing written");
    // An ordinate and a slope: Zemin only; beside another object, counted.
    select(&mut app, &[oy, s1]);
    let titles: Vec<String> = rows(&app).into_iter().map(|(t, _)| t).collect();
    assert!(titles.contains(&"Ölçü".to_owned()), "{titles:?}");
    let section = rows(&app)
        .into_iter()
        .find(|(t, _)| t == "Ölçü")
        .expect("Ölçü");
    let labels: Vec<String> = section.1.iter().map(|r| r.label.to_string()).collect();
    // Both bind their measured points (docs/adr/0249 §3): free, as drawn here.
    assert_eq!(
        labels,
        [
            "İlişki",
            "Zemin",
            // Their options, value's place and breaks (docs/adr/0321).
            "Sığdırma",
            "Yuvarlama",
            "Sıfırlar",
            "Alternatif birim",
            "Tolerans",
            "Değerin yeri",
            "Kırmalar",
            "Çizgi rengi",
            "Çizgi kalınlığı",
            "Çizgi tipi",
            "Uzatma rengi",
            "Uzatma kalınlığı",
            "Uzatma tipi",
            "Değer rengi"
        ]
    );
    assert_eq!(value(&app, "Ölçü", "İlişki"), "Serbest");
    select(&mut app, &[oy, 1]);
    assert_eq!(value(&app, "Ölçüler (1)", "Zemin"), "Kapalı");
}

/// A dimension's lines' rows (docs/adr/0205 §6): common or “Çeşitli”, a colour
/// whatever its case and another program's by its value; a colour, a weight
/// and a type written in one step “Değiştir”, those that have it left out,
/// none taking it away. The web's `dimensionRows.test.ts`.
#[test]
fn a_dimensions_line_rows_write_its_colours_weights_and_types() {
    use kentos_commands::DimensionStyle::Aligned;
    use kentos_commands::LineType;
    let mut app = objects();
    let d1 = add(
        &mut app,
        Entity::Dimension(DimensionEntity {
            look: kentos_commands::DimensionLook {
                dim_line_color: Some("#E5484D".into()),
                ext_weight: Some(0.35),
                ext_line_type: Some(LineType::Dashed),
                ..Default::default()
            },
            ..dimension(Aligned, [0.0, 0.0], [10.0, 0.0])
        }),
    );
    let d2 = add(
        &mut app,
        Entity::Dimension(DimensionEntity {
            look: kentos_commands::DimensionLook {
                dim_line_color: Some("#e5484d".into()),
                text_color: Some("#123456".into()),
                ..Default::default()
            },
            ..dimension(Aligned, [0.0, 20.0], [10.0, 20.0])
        }),
    );
    select(&mut app, &[d1, d2]);
    let shown = |app: &App| {
        [
            "Çizgi rengi",
            "Uzatma kalınlığı",
            "Uzatma tipi",
            "Değer rengi",
        ]
        .map(|l| value(app, "Ölçü", l))
    };
    assert_eq!(shown(&app), ["Kırmızı", "Çeşitli", "Çeşitli", "Çeşitli"]);
    select(&mut app, &[d2]);
    assert_eq!(value(&app, "Geometri", "Değer rengi"), "#123456");
    assert_eq!(value(&app, "Geometri", "Çizgi tipi"), "Sürekli");
    let slots = vec![Slot(d1), Slot(d2)];
    let look = |app: &App, id| dimension_of(app, id).look.clone();
    event(
        &mut app,
        Event::DimensionLineColor(slots.clone(), Part::Ext, Some("#4F8EF7".into())),
    );
    assert_eq!(
        (look(&app, d1).ext_color, look(&app, d2).ext_color),
        (Some("#4F8EF7".into()), Some("#4F8EF7".into()))
    );
    assert_eq!(undo_label(&mut app).as_deref(), Some("Değiştir"));
    assert_eq!(
        (look(&app, d1).ext_color, look(&app, d2).ext_color),
        (None, None)
    );
    // A weight: the one that has it left out; Kılcal takes it away.
    event(
        &mut app,
        Event::DimensionLineWeight(slots.clone(), Part::Ext, Some(0.35)),
    );
    assert_eq!(
        (look(&app, d1).ext_weight, look(&app, d2).ext_weight),
        (Some(0.35), Some(0.35))
    );
    event(
        &mut app,
        Event::DimensionLineWeight(slots.clone(), Part::Ext, None),
    );
    assert_eq!(
        (look(&app, d1).ext_weight, look(&app, d2).ext_weight),
        (None, None)
    );
    // A type: Sürekli (none) takes it away.
    event(
        &mut app,
        Event::DimensionLineType(slots.clone(), Part::Line, Some(LineType::Dashdot)),
    );
    assert_eq!(
        (look(&app, d1).dim_line_type, look(&app, d2).dim_line_type),
        (Some(LineType::Dashdot), Some(LineType::Dashdot))
    );
    event(
        &mut app,
        Event::DimensionLineType(slots.clone(), Part::Ext, None),
    );
    assert_eq!(
        (look(&app, d1).ext_line_type, look(&app, d2).ext_line_type),
        (None, None)
    );
    // The value's colour back to the object's.
    event(
        &mut app,
        Event::DimensionLineColor(vec![Slot(d2)], Part::Value, None),
    );
    assert_eq!(look(&app, d2).text_color, None);
    let revision = |app: &App| app.document.as_ref().map_or(0, |d| d.model.revision());
    let before = revision(&app);
    event(
        &mut app,
        Event::DimensionLineColor(slots, Part::Line, Some("#E5484D".into())),
    );
    assert_eq!(
        revision(&app),
        before,
        "both have it already, whatever its case"
    );
}

/// A selection holding a locked object: Renk ▾ writes the others in one step, the locked one is skipped and
/// counted (docs/adr/0278, 0286).
#[test]
fn a_mixed_selection_changes_the_open_objects_and_counts_the_locked() {
    let mut app = objects();
    select(&mut app, &[1, 6]);
    let ids = vec![Slot(1), Slot(6)];
    event(&mut app, Event::Color(ids, Some("#E5484D".to_owned())));
    assert_eq!(entity(&app, 1).base().color.as_deref(), Some("#E5484D"));
    assert_eq!(entity(&app, 6).base().color, None);
    assert_eq!(
        warned(&app).as_deref(),
        Some(
            "1 nesne kilitli katmanda olduğu için atlandı. Değiştirmek için katmanın kilidini Katmanlar panelinden açın."
        )
    );
    assert_eq!(undo_label(&mut app).as_deref(), Some("Renk değiştir"));
}

/// Ortak öznitelikler: a value every object shares, else Çeşitli with how many have it; a cell writes them all,
/// one step (docs/adr/0286).
#[test]
fn several_objects_share_their_attributes_and_take_one_value() {
    let mut app = objects();
    let line = |a: [f64; 2], attrs: &[(&str, &str)]| {
        let mut b = base("0");
        b.attrs = attrs
            .iter()
            .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
            .collect();
        Entity::Line(LineEntity {
            base: b,
            a: Wire {
                x: E + a[0],
                y: N + a[1],
            },
            b: Wire {
                x: E + a[0] + 5.0,
                y: N + a[1],
            },
            za: None,
            zb: None,
            ma: None,
            mb: None,
        })
    };
    let first = add(
        &mut app,
        line([40.0, 0.0], &[("Tür", "Duvar"), ("Kat", "1")]),
    );
    let second = add(
        &mut app,
        line([40.0, 5.0], &[("Tür", "Duvar"), ("Kat", "2")]),
    );
    let third = add(&mut app, line([40.0, 10.0], &[("Tür", "Duvar")]));
    select(&mut app, &[first, second, third]);
    assert_eq!(value(&app, "Ortak öznitelikler", "Tür"), "Duvar");
    assert_eq!(
        value(&app, "Ortak öznitelikler", "Kat"),
        "Çeşitli (2/3 nesnede)"
    );
    event(
        &mut app,
        Event::Commit(
            Field::Attributes(
                vec![Slot(first), Slot(second), Slot(third)],
                "Kat".to_owned(),
            ),
            "3".to_owned(),
        ),
    );
    for slot in [first, second, third] {
        assert_eq!(
            entity(&app, slot)
                .base()
                .attrs
                .get("Kat")
                .map(String::as_str),
            Some("3")
        );
    }
    assert_eq!(value(&app, "Ortak öznitelikler", "Kat"), "3");
    assert_eq!(undo_label(&mut app).as_deref(), Some("Değiştir"));
}

/// docs/adr/0717 §2: a wall and a column selected together share Kat (Çeşitli while their levels differ); a level
/// chosen goes to both in one step “Kat ver”, Geri al takes it back.
#[test]
fn several_architectural_objects_take_one_level() {
    use serde_json::json;
    let mut app = objects();
    {
        let doc = app.document.as_mut().expect("open");
        let mut s = doc.model.settings().clone();
        s.arch = Some(
            serde_json::from_value(json!({"levels": [
                {"id": "zemin", "name": "Zemin kat", "elevation": 0.0},
                {"id": "kat-1", "name": "1. kat", "elevation": 3.0}
            ]}))
            .expect("levels"),
        );
        doc.model.set_settings(s);
    }
    let wall: Entity = serde_json::from_value(json!({
        "kind": "wall", "id": 0, "layerId": "0", "attrs": {},
        "pts": [{"x": E, "y": N}, {"x": E + 5.0, "y": N}], "width": 0.2, "justify": "center",
        "arch": {"level": "zemin"}
    }))
    .expect("a wall");
    let column: Entity = serde_json::from_value(json!({
        "kind": "column", "id": 0, "layerId": "0", "attrs": {},
        "at": {"x": E + 10.0, "y": N}, "width": 0.4, "depth": 0.4
    }))
    .expect("a column");
    let w = add(&mut app, wall);
    let c = add(&mut app, column);
    select(&mut app, &[w, c]);
    assert_eq!(value(&app, "Ortak özellikler", "Kat"), "Çeşitli");
    event(&mut app, Event::Levels(vec![Slot(w), Slot(c)], "kat-1".into()));
    for slot in [w, c] {
        assert_eq!(entity(&app, slot).own_level(), Some("kat-1"));
    }
    assert_eq!(value(&app, "Ortak özellikler", "Kat"), "1. kat");
    assert_eq!(undo_label(&mut app).as_deref(), Some("Kat ver"));
    assert_eq!(entity(&app, w).own_level(), Some("zemin"));
    assert_eq!(entity(&app, c).own_level(), None);
    // A line among them: no Kat row (it has no level of its own).
    let line = add(
        &mut app,
        Entity::Line(LineEntity {
            base: base("0"),
            a: Wire { x: E, y: N + 3.0 },
            b: Wire { x: E + 5.0, y: N + 3.0 },
            za: None,
            zb: None,
            ma: None,
            mb: None,
        }),
    );
    select(&mut app, &[w, c, line]);
    assert!(!has_row(&app, "Ortak özellikler", "Kat"));
}

/// The panel of a selection of 100 000 objects with attributes is built within a frame's budget (docs/adr/0286).
#[test]
#[ignore = "timing; run with --ignored --nocapture in release"]
fn a_hundred_thousand_objects_build_the_panel_quickly() {
    let mut app = objects();
    let mut slots = Vec::new();
    for i in 0..100_000u32 {
        let mut b = base("0");
        b.attrs = BTreeMap::from([
            ("Tür".to_owned(), "Duvar".to_owned()),
            ("No".to_owned(), i.to_string()),
        ]);
        let x = f64::from(i % 300) * 2.0;
        let y = f64::from(i / 300) * 2.0;
        slots.push(add(
            &mut app,
            Entity::Line(LineEntity {
                base: b,
                a: Wire { x: E + x, y: N + y },
                b: Wire {
                    x: E + x + 1.0,
                    y: N + y,
                },
                za: None,
                zb: None,
                ma: None,
                mb: None,
            }),
        ));
    }
    select(&mut app, &slots);
    let doc = app.document.as_ref().expect("open");
    let t = std::time::Instant::now();
    let panel = app.properties_panel(doc);
    let took = t.elapsed();
    println!("100 000 nesnenin paneli: {took:?}");
    assert!(
        panel
            .sections
            .iter()
            .any(|s| s.title == "Ortak öznitelikler")
    );
    assert!(took.as_millis() < 100, "{took:?}");
}

#[test]
fn a_single_objects_cached_properties_follow_edits_undo_and_other_drawings() {
    let mut app = objects();
    select(&mut app, &[4]);
    let before = value(&app, "Genel", "Renk");
    assert!(app.properties_cache.0.borrow().is_some(), "one object is cached too");
    event(&mut app, Event::Color(vec![Slot(4)], Some("#123456".into())));
    assert_ne!(value(&app, "Genel", "Renk"), before);
    let _ = app.update(Message::Run("edit.undo"));
    assert_eq!(value(&app, "Genel", "Renk"), before);
    let other = crate::files_testing::drawing(0);
    let _ = app.update(Message::Opened(Some(Ok(Box::new(other)))));
    select(&mut app, &[4]);
    let _ = rows(&app);
    assert_eq!(app.properties_cache.0.borrow().as_ref().expect("cached").0.0,
        app.document.as_ref().expect("open").session);
}

#[test]
#[ignore = "single complex CAD object timing; run explicitly without concurrent benchmarks"]
fn one_long_polylines_cached_panel_does_not_remeasure_on_every_keystroke() {
    let mut app = objects();
    let Entity::Polygon(mut p) = entity(&app, 4) else { panic!("area") };
    p.base = base("cizim");
    p.pts = (0..100_000).map(|i| Wire { x: E + f64::from(i)*0.1, y: N }).collect();
    let slot = add(&mut app, Entity::Polyline(p));
    select(&mut app, &[slot]);
    let doc = app.document.as_ref().expect("open");
    let started = std::time::Instant::now();
    let _ = app.properties_panel(doc);
    let cold = started.elapsed();
    let started = std::time::Instant::now();
    for _ in 0..100 { std::hint::black_box(app.properties_panel(doc)); }
    let warm = started.elapsed();
    println!("100.000 köşeli tek nesne: ilk panel {cold:?}, 100 tekrar {warm:?}");
    assert!(warm < cold*10 + std::time::Duration::from_millis(20), "panel remeasured: {cold:?} -> {warm:?}");
}

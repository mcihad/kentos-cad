//! Doğrama kodları (M28, docs/adr/3028): the acceptance apartment's doors and windows get Mimarlar Odası's joinery
//! codes, one for each type (K01 the four inner doors, GK01 the entrance, P01 the four 120 windows, P02 the 60 one),
//! and their tags write the code over the size, beside their walls and clear of the swings and of each other; one undo
//! takes it all back.

use std::collections::BTreeMap;

use crate::app::{App, Message};

/// A tag's frame's half size on the drawing at 1:`scale`, as the geometry draws it: its words' box and pad, a
/// hexagon's points, a rectangle's sides.
fn half(t: &kentos_commands::TagFields, scale: f64) -> (f64, f64) {
    use kentos_geometry_core::entity::Shape;
    use kentos_geometry_core::geom::tag::{Frame, PAD, half_box, height, radius};
    let Shape::Tag { tag: spec, .. } = kentos_formats::arch_elements::core_tag(t) else {
        return (0.0, 0.0);
    };
    let h = height(&spec, scale);
    let (w, b) = half_box(&spec, scale);
    let pad = PAD * h;
    match spec.frame {
        Frame::Circle => (radius(&spec, scale), radius(&spec, scale)),
        Frame::Hexagon => (w + pad + (b + pad) * 0.577, b + pad),
        _ => (w + pad, b + pad),
    }
}

#[cfg(unix)]
#[test]
fn the_apartment_s_doors_and_windows_get_their_joinery_codes_and_tags_in_one_step() {
    let (mut app, _dir) = crate::acceptance_flat_tests::opened("dograma-kodlari");
    let _ = crate::acceptance_flat_tests::drawn(&mut app);
    let tags = |app: &App| {
        let doc = &app.document.as_ref().expect("a drawing").model;
        doc.entities()
            .filter_map(|e| match e {
                kentos_commands::Entity::Tag(t) => Some(t.clone()),
                _ => None,
            })
            .collect::<Vec<_>>()
    };
    let before = tags(&app).len();
    let _ = app.update(Message::Run("tool.openingCodes"));
    assert_eq!(app.session.tool_id(), "openingCodes");
    let format = app.format();
    let card = app
        .session
        .preview(&format)
        .and_then(|p| p.tag)
        .map(|t| t.lines)
        .unwrap_or_default();
    assert_eq!(
        card.first().map(String::as_str),
        Some("4 tip · 10 açıklık · 4 yeni kod"),
        "{card:?}"
    );
    assert!(card.iter().any(|l| l == "K01  İç kapı 90 · 4"), "{card:?}");
    assert!(
        card.iter().any(|l| l == "GK01  Giriş kapısı · 1"),
        "{card:?}"
    );
    let _ = app.submit_line("");
    assert!(
        crate::files_testing::last_said(&app)
            .starts_with("Doğrama kodları verildi: K01 İç kapı 90, GK01 Giriş kapısı"),
        "{}",
        crate::files_testing::last_said(&app)
    );
    let doc = &app.document.as_ref().expect("a drawing").model;
    let codes: BTreeMap<String, Option<String>> = doc
        .opening_types()
        .iter()
        .map(|t| (t.name.clone(), t.code.clone()))
        .collect();
    assert_eq!(codes["İç kapı 90"].as_deref(), Some("K01"), "{codes:?}");
    assert_eq!(codes["Giriş kapısı"].as_deref(), Some("GK01"), "{codes:?}");
    assert_eq!(codes["Pencere 120"].as_deref(), Some("P01"), "{codes:?}");
    assert_eq!(codes["Pencere 60"].as_deref(), Some("P02"), "{codes:?}");
    // The openings' tags: the code over the size, 90/210.
    let made: Vec<_> = tags(&app)
        .into_iter()
        .filter(|t| {
            doc.tag_target(&t.tag.of)
                .is_some_and(|e| e.kind() == "opening")
        })
        .collect();
    assert_eq!(made.len(), 10, "every door and window tagged");
    let mut texts: Vec<&str> = made.iter().map(|t| t.tag.text.as_str()).collect();
    texts.sort_unstable();
    texts.dedup();
    assert_eq!(
        texts,
        ["GK01\n100/210", "K01\n90/210", "P01\n120/130", "P02\n60/70"],
        "{texts:?}"
    );
    let scale = doc.settings().plot_scale;
    for t in &made {
        let Some(kentos_commands::Entity::Opening(o)) = doc.tag_target(&t.tag.of) else {
            panic!("an opening");
        };
        // Beside its wall: across it farther than the wall's face.
        let p = &o.opening.place;
        let mid = |a: kentos_commands::Vec2, b: kentos_commands::Vec2| {
            ((a.x + b.x) / 2.0, (a.y + b.y) / 2.0)
        };
        let (l, r) = (mid(p[0], p[2]), mid(p[1], p[3]));
        let c = ((l.0 + r.0) / 2.0, (l.1 + r.1) / 2.0);
        let (nx, ny) = (l.0 - r.0, l.1 - r.1);
        let thick = nx.hypot(ny);
        let across = ((t.tag.at.x - c.0) * nx + (t.tag.at.y - c.1) * ny).abs() / thick;
        assert!(
            across > thick / 2.0,
            "{} on its wall: {across} ≤ {}",
            t.tag.text,
            thick / 2.0
        );
        // Near it: read as its own, not a neighbour's.
        let far = (t.tag.at.x - c.0).hypot(t.tag.at.y - c.1);
        assert!(far < 2.5, "{} {far:.2} m from its opening", t.tag.text);
    }
    // Clear of each other and of the rooms' tags.
    let all = tags(&app);
    for a in &made {
        let (aw, ah) = half(&a.tag, scale);
        for b in all.iter().filter(|b| b.tag.of != a.tag.of) {
            let (bw, bh) = half(&b.tag, scale);
            let apart = (a.tag.at.x - b.tag.at.x).abs() >= aw + bw - 1e-6
                || (a.tag.at.y - b.tag.at.y).abs() >= ah + bh - 1e-6;
            assert!(
                apart,
                "{:?} {:?} and {:?} {:?} overlap",
                a.tag.text, a.tag.at, b.tag.text, b.tag.at
            );
        }
    }
    let _ = app.update(Message::Run("edit.undo"));
    assert_eq!(tags(&app).len(), before, "one undo takes the tags");
    let doc = &app.document.as_ref().expect("a drawing").model;
    assert!(
        doc.opening_types().iter().all(|t| t.code.is_none()),
        "and the codes"
    );
}

/// An opening following no type gets one of its own for each make, named by its form and size; a door in the
/// building's outer wall (a room on one side only) is GK, and its tag says so.
#[test]
fn an_untyped_door_in_the_outer_wall_gets_a_type_and_gk01() {
    use kentos_commands::Entity;
    let (mut app, [_, door, _]) = crate::building_context_tests::building();
    let _ = app.update(Message::Run("tool.openingCodes"));
    let _ = app.submit_line("");
    let doc = &app.document.as_ref().expect("a drawing").model;
    let types: Vec<(String, Option<String>)> = doc
        .opening_types()
        .iter()
        .map(|t| (t.name.clone(), t.code.clone()))
        .collect();
    assert_eq!(types.len(), 1, "{types:?}");
    assert!(types[0].0.starts_with("Kapı 90/"), "{types:?}");
    assert_eq!(types[0].1.as_deref(), Some("GK01"), "{types:?}");
    let Some(Entity::Opening(o)) = doc.get(door) else {
        panic!("the door");
    };
    assert!(o.opening.follows_type(), "it follows its type");
    assert_eq!(o.opening.number.as_deref(), Some("K1"), "its number stays");
    let tag = doc
        .entities()
        .find_map(|e| match e {
            Entity::Tag(t)
                if doc
                    .tag_target(&t.tag.of)
                    .is_some_and(|x| x.kind() == "opening") =>
            {
                Some(t.tag.text.clone())
            }
            _ => None,
        })
        .expect("its tag");
    assert!(tag.starts_with("GK01\n90/"), "{tag}");
}

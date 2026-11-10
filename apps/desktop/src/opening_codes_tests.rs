//! Doğrama kodları (M28, docs/adr/8028): the acceptance apartment's doors and windows get Mimarlar Odası's joinery
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
    // The door and window schedule shows the codes: a Kod column after No.
    let doc = &app.document.as_ref().expect("a drawing").model;
    let table = doc
        .entities()
        .find_map(|e| match e {
            kentos_commands::Entity::Table(t)
                if matches!(
                    &t.source,
                    Some(kentos_commands::TableSource::Schedule {
                        of: kentos_commands::ScheduleOf::Openings,
                        ..
                    })
                ) =>
            {
                Some(t.clone())
            }
            _ => None,
        })
        .expect("the door and window schedule");
    assert_eq!(
        table.cells[0].get(1).map(String::as_str),
        Some("Kod"),
        "{:?}",
        table.cells[0]
    );
    let codes_in_table: Vec<&str> = table
        .cells
        .iter()
        .skip(1)
        .filter_map(|r| r.get(1).map(String::as_str))
        .collect();
    assert!(
        codes_in_table.contains(&"K01")
            && codes_in_table.contains(&"GK01")
            && codes_in_table.contains(&"P02"),
        "{codes_in_table:?}"
    );
    assert!(crate::files_testing::last_said(&app).contains("çizelgeye Kod sütunu eklendi"));
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

/// Mahale göre (M28, docs/adr/8028): Açıklıkları numarala gives each door its room's number (the room it opens into),
/// each window its room's, with -A, -B … where a room has more than one; K for the doors, P for the windows; the numbers
/// show before Enter; one undo takes them back.
#[cfg(unix)]
#[test]
fn numbering_by_room_gives_each_opening_its_room_s_number() {
    use kentos_commands::Entity;
    let (mut app, _dir) = crate::acceptance_flat_tests::opened("odaya-gore-numara");
    let _ = crate::acceptance_flat_tests::drawn(&mut app);
    let numbers = |app: &App| {
        let doc = &app.document.as_ref().expect("a drawing").model;
        let mut out: Vec<(String, String)> = doc
            .entities()
            .filter_map(|e| match e {
                Entity::Opening(o) => Some((
                    format!("{:?}", o.opening.form),
                    o.opening.number.clone().unwrap_or_default(),
                )),
                _ => None,
            })
            .collect();
        out.sort();
        out
    };
    let before = numbers(&app);
    let _ = app.update(Message::Run("tool.openingNumber"));
    assert_eq!(app.session.tool_id(), "openingNumber");
    assert_eq!(
        app.with_tool(|s, cx| s.input("M", cx)),
        Some(true),
        "M turns Mahale göre on"
    );
    let format = app.format();
    let shown: Vec<String> = app
        .session
        .preview(&format)
        .map(|p| p.labels.into_iter().map(|l| l.text).collect())
        .unwrap_or_default();
    let _ = app.submit_line("");
    let after = numbers(&app);
    // The scenario's doors open into the corridor; each is the room's it serves: the Salon's 01, the first bedroom's
    // 02, the entrance the corridor's 04 (outside on its other side), the bathroom's two (one from the corridor, one
    // from the second bedroom opening into the bathroom: between two rooms the one it opens into) 05-A and 05-B.
    let want: Vec<(String, String)> = [
        ("Door", "K01"),
        ("Door", "K02"),
        ("Door", "K04"),
        ("Door", "K05-A"),
        ("Door", "K05-B"),
        ("Window", "P01-A"),
        ("Window", "P01-B"),
        ("Window", "P02"),
        ("Window", "P03"),
        ("Window", "P05"),
    ]
    .into_iter()
    .map(|(f, n)| (f.to_owned(), n.to_owned()))
    .collect();
    assert_eq!(after, want);
    assert_eq!(
        crate::files_testing::last_said(&app),
        "10 açıklık mahalinin numarasını aldı."
    );
    let mut seen: Vec<&str> = after.iter().map(|(_, n)| n.as_str()).collect();
    seen.sort_unstable();
    let mut planned: Vec<&str> = shown.iter().map(String::as_str).collect();
    planned.sort_unstable();
    assert_eq!(seen, planned, "the preview showed the numbers written");
    let _ = app.update(Message::Run("edit.undo"));
    assert_eq!(numbers(&app), before, "one undo");
}

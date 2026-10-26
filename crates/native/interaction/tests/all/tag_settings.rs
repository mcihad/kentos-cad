//! Visible tag settings must produce semantic tags, never edit the selected room.
use crate::common::{Bench, E, N};
use kentos_commands::{Entity, TagShape, Vec2};
use kentos_domain::Slot;
use serde_json::json;

fn drawing() -> Bench {
    let mut b = Bench::new("tag");
    let room: Entity = serde_json::from_value(json!({
        "kind":"space", "id":0, "layerId":b.doc.layers().active(), "attrs":{},
        "number":"A-001", "name":"Salon", "seed":{"x":E+3.,"y":N+2.},
        "parts":[{"outer":{"pts":[{"x":E,"y":N},{"x":E+6.,"y":N},
            {"x":E+6.,"y":N+4.},{"x":E,"y":N+4.}]}}]
    }))
    .unwrap();
    let slot = b.doc.add(room).unwrap();
    b.selection.set([slot]);
    b.start("tag");
    b
}
fn set(b: &mut Bench, key: &str, value: &str) -> bool {
    b.run(|s, cx| s.configure(key, value, cx))
}
fn shown(b: &Bench, key: &str) -> String {
    b.session
        .settings(&Default::default())
        .into_iter()
        .find(|r| r.key == key)
        .unwrap_or_else(|| panic!("Missing visible setting {key}"))
        .value
}

#[test]
fn visible_settings_apply_twenty_combinations_with_one_write_and_unchanged_source() {
    for shape in TagShape::ALL {
        for (template, text) in [
            ("{number}", "A-001"),
            ("{name}", "Salon"),
            ("{number}\\n{name}", "A-001\nSalon"),
            ("Net {area}", "Net 24.00 m²"),
        ] {
            let mut b = drawing();
            let before = b.doc.to_snapshot_v2();
            assert_eq!(shown(&b, "B"), "Türe göre");
            assert!(set(&mut b, "Ş", template));
            assert!(set(&mut b, "B", shape.name()));
            assert!(set(&mut b, "K", "yes"));
            assert!(set(&mut b, "Y", "3,5 mm"));
            assert_eq!(
                b.session
                    .prompt()
                    .options
                    .iter()
                    .find(|o| o.key == "Ş")
                    .unwrap()
                    .value
                    .as_deref(),
                Some("Özel")
            );
            assert_eq!(b.doc.to_snapshot_v2(), before);
            assert_eq!(b.selection.ids(), &[Slot(1)]);
            b.click(3., 2.);
            assert_eq!(b.points(), 1);
            b.move_to(-5., 6.);
            let preview = b.session.preview(&Default::default()).unwrap();
            assert_eq!(
                preview
                    .texts
                    .iter()
                    .map(|t| t.text.as_str())
                    .collect::<Vec<_>>()
                    .join("\n"),
                text
            );
            assert_eq!(b.doc.to_snapshot_v2(), before);
            b.click(-5., 6.);
            assert_eq!(b.doc.len(), 2);
            let Entity::Tag(tag) = b.newest() else {
                panic!("tag")
            };
            assert_eq!(tag.tag.of, b.doc.uid(Slot(1)).unwrap().to_string());
            assert_eq!(
                tag.tag.at,
                Vec2 {
                    x: E - 5.,
                    y: N + 6.
                }
            );
            assert_eq!(tag.tag.text, text);
            assert_eq!(
                tag.tag.content.as_deref(),
                Some(template.replace("\\n", "\n").as_str())
            );
            assert_eq!(tag.tag.shape, shape);
            assert_eq!(tag.tag.size, Some(3.5));
            assert!(tag.tag.leader);
            assert_eq!(b.doc.get(Slot(1)), Some(&before.entities[0]));
            assert_eq!(
                b.doc.journal().last().unwrap().command.as_deref(),
                Some("cad.tags.create")
            );
            let after = b.doc.to_snapshot_v2();
            b.doc.undo().unwrap();
            assert_eq!(b.doc.to_snapshot_v2(), before);
            b.doc.redo().unwrap();
            assert_eq!(b.doc.to_snapshot_v2(), after);
        }
    }
}

#[test]
fn invalid_settings_keep_valid_values_and_target_and_empty_resets_defaults() {
    let mut b = drawing();
    assert!(set(&mut b, "Ş", "{number}"));
    assert!(set(&mut b, "Y", "4"));
    assert!(set(&mut b, "B", "rounded"));
    b.click(3., 2.);
    let before = b.doc.to_snapshot_v2();
    for (key, value) in [
        ("Ş", "{unknown}"),
        ("Ş", "unclosed {name"),
        ("Ş", "a\nb\nc\nd\ne\nf\ng"),
        ("Y", "0"),
        ("Y", "-1"),
        ("Y", "51"),
        ("Y", "NaN"),
        ("Y", "inf"),
        ("Y", "3 m"),
        ("B", "triangle"),
        ("K", "maybe"),
    ] {
        assert!(!set(&mut b, key, value), "{key}: {value}");
        assert_eq!(shown(&b, "Ş"), "{number}");
        assert_eq!(shown(&b, "Y"), "4");
        assert_eq!(shown(&b, "B"), "Yuvarlak köşeli");
        assert_eq!(b.points(), 1);
        assert_eq!(b.doc.to_snapshot_v2(), before);
    }
    assert!(!set(&mut b, "Ş", &"x".repeat(201)));
    assert!(set(&mut b, "Ş", ""));
    assert!(set(&mut b, "Y", ""));
    assert!(set(&mut b, "B", "default"));
    b.click(-5., 6.);
    let Entity::Tag(tag) = b.newest() else {
        panic!()
    };
    assert!(tag.tag.content.is_none() && tag.tag.size.is_none());
    assert_eq!(tag.tag.shape, TagShape::Rectangle);
    assert_eq!(tag.tag.text, "A-001\nSalon\n24.00 m²");
}

#[test]
fn visible_commit_closes_console_template_prompt_and_choice_menus_share_state() {
    let mut b = drawing();
    assert!(b.type_text("Ş"));
    assert!(set(&mut b, "Ş", "{name}"));
    assert!(
        b.type_text("K"),
        "Visible template commit must end the console ask"
    );
    assert_eq!(shown(&b, "K"), "Var");
    assert!(b.run(|s, cx| s.choose_option("B", "hexagon", cx)));
    assert_eq!(shown(&b, "B"), "Altıgen");
    assert!(b.run(|s, cx| s.choose_option("K", "no", cx)));
    assert_eq!(shown(&b, "K"), "Yok");
    assert!(b.type_text("B"));
    assert_eq!(
        shown(&b, "B"),
        "Dikdörtgen",
        "Legacy shape cycle is retained"
    );
    assert_eq!(b.doc.len(), 1);
}

#[test]
fn preview_settings_and_escape_do_not_write_or_erase_the_selection() {
    let mut b = drawing();
    let before = b.doc.to_snapshot_v2();
    b.click(3., 2.);
    b.move_to(-5., 6.);
    assert!(set(&mut b, "Ş", "{name}\\n{area}"));
    assert!(set(&mut b, "B", "none"));
    assert!(set(&mut b, "Y", "50 mm"));
    b.run(|s, cx| s.cancel(cx));
    assert_eq!(b.points(), 0);
    assert_eq!(b.doc.to_snapshot_v2(), before);
    assert_eq!(b.selection.ids(), &[Slot(1)]);
    b.run(|s, cx| s.cancel(cx));
    assert_ne!(b.session.tool_id(), "tag");
    assert_eq!(b.doc.to_snapshot_v2(), before);
}

#[test]
fn paper_height_is_independent_of_drawing_unit_for_twenty_unit_scale_pairs() {
    use kentos_commands::DrawingUnit;
    for unit in [
        DrawingUnit::Mm,
        DrawingUnit::Cm,
        DrawingUnit::M,
        DrawingUnit::In,
        DrawingUnit::Ft,
    ] {
        for scale in [25., 50., 100., 200.] {
            let mut b = drawing();
            let mut settings = b.doc.settings().clone();
            settings.drawing_unit = Some(unit);
            settings.plot_scale = scale;
            b.doc.set_settings(settings);
            let before = b.doc.to_snapshot_v2();
            assert!(set(&mut b, "Y", "3.5 mm"));
            b.click(3., 2.);
            b.move_to(-5., 6.);
            let preview = b.session.preview(&Default::default()).unwrap();
            assert!(!preview.texts.is_empty());
            for text in preview.texts {
                assert!(
                    (text.height - 3.5 * scale / 1000.).abs() < 1e-12,
                    "{unit:?}/{scale}"
                );
            }
            assert_eq!(b.doc.to_snapshot_v2(), before);
            b.click(-5., 6.);
            let Entity::Tag(tag) = b.newest() else {
                panic!()
            };
            assert_eq!(tag.tag.size, Some(3.5));
            assert_eq!(
                tag.tag.at,
                Vec2 {
                    x: E - 5.,
                    y: N + 6.
                }
            );
            assert_eq!(b.doc.settings(), &before.settings);
        }
    }
}

#[test]
fn bulk_keep_refresh_preview_and_one_confirm_preserve_manual_labels_and_one_undo() {
    use kentos_native_application::{ExecutionContext, tags_create};
    let mut b = drawing();
    let uid = b.doc.uid(Slot(1)).unwrap().to_string();
    let input = serde_json::from_value(json!({"layerId":b.doc.layers().active(),"tags":[
        {"of":uid,"at":{"x":E-4.,"y":N+8.},"content":"{name}","leader":true,"size":4.},
        {"of":uid,"at":{"x":E-4.,"y":N+10.},"content":"{area}","shape":"none","size":2.}
    ]}))
    .unwrap();
    assert!(matches!(
        tags_create::execute(&mut ExecutionContext::new(&mut b.doc), input),
        kentos_commands::CommandResult::Completed { .. }
    ));
    let before = b.doc.to_snapshot_v2();
    b.start("tagAll");
    assert_eq!(shown(&b, "E"), "Koru");
    assert!(set(&mut b, "Ş", "{number} / {name}"));
    assert!(
        b.session
            .preview(&Default::default())
            .unwrap()
            .texts
            .is_empty()
    );
    assert!(set(&mut b, "E", "refresh"));
    assert!(shown(&b, "N").contains("2 yenilenecek"));
    let preview = b.session.preview(&Default::default()).unwrap();
    assert_eq!(preview.texts.len(), 2);
    assert!(preview.texts.iter().all(|t| t.text == "A-001 / Salon"));
    assert_eq!(b.doc.to_snapshot_v2(), before);
    b.confirm();
    let after = b.doc.to_snapshot_v2();
    assert_eq!(after.uids, before.uids);
    assert_eq!(after.entities.len(), before.entities.len());
    assert_eq!(after.entities[0], before.entities[0]);
    for (a, p) in after.entities[1..].iter().zip(&before.entities[1..]) {
        let (Entity::Tag(a), Entity::Tag(p)) = (a, p) else {
            panic!()
        };
        assert_eq!(a.base, p.base);
        assert_eq!(a.tag.at, p.tag.at);
        assert_eq!(a.tag.leader, p.tag.leader);
        assert_eq!(a.tag.size, p.tag.size);
        assert_eq!(a.tag.text, "A-001 / Salon");
    }
    assert_eq!(
        b.doc.journal().last().unwrap().command.as_deref(),
        Some("cad.tags.create")
    );
    b.doc.undo().unwrap();
    assert_eq!(b.doc.to_snapshot_v2(), before);
    b.doc.redo().unwrap();
    assert_eq!(b.doc.to_snapshot_v2(), after);
}

#[test]
fn bulk_rejection_stays_open_escape_and_all_eight_kind_controls_are_read_only() {
    let mut b = drawing();
    b.start("tagAll");
    let before = b.doc.to_snapshot_v2();
    for (key, value) in [
        ("Ş", "{bad}"),
        ("Y", "0"),
        ("Y", "51"),
        ("Y", "NaN"),
        ("Y", "3 m"),
        ("E", "maybe"),
        ("B", "triangle"),
    ] {
        assert!(!set(&mut b, key, value));
        assert_eq!(b.doc.to_snapshot_v2(), before);
    }
    for key in ["A", "M", "K", "S", "D", "R", "P", "O"] {
        assert!(set(&mut b, key, "no"));
    }
    b.confirm();
    assert_eq!(b.session.prompt().tool, Some("Hepsini etiketle"));
    assert_eq!(b.doc.to_snapshot_v2(), before);
    assert!(set(&mut b, "M", "yes"));
    assert!(set(&mut b, "Y", "3,5 mm"));
    assert_eq!(shown(&b, "Y"), "3.5");
    assert!(set(&mut b, "Y", ""));
    assert_eq!(shown(&b, "Y"), "");
    b.run(|s, cx| s.cancel(cx));
    assert_eq!(b.doc.to_snapshot_v2(), before);
}

#[test]
fn bulk_target_changed_without_pointer_movement_requires_review_before_writing() {
    let mut b = drawing();
    let mut snapshot = b.doc.to_snapshot_v2();
    let mut layer = snapshot.layers[0].clone();
    layer.id = "etiket2".into();
    layer.name = "İkinci etiket".into();
    snapshot.layers.push(layer);
    b.doc = kentos_domain::Document::from_snapshot_v2(snapshot).unwrap();
    b.start("tagAll");
    assert_eq!(shown(&b, "L"), "Çizim");
    let revision = b.doc.revision();
    assert!(b.doc.set_active_layer("etiket2"));
    assert_eq!(b.doc.revision(), revision);
    // A keyboard layer change does not pass through the canvas pointer hook.
    let before = b.doc.to_snapshot_v2();
    b.confirm();
    assert_eq!(b.doc.to_snapshot_v2(), before);
    assert_eq!(b.session.prompt().tool, Some("Hepsini etiketle"));
    assert_eq!(shown(&b, "L"), "İkinci etiket");
    assert!(!b.doc.can_undo());
    b.confirm();
    let after = b.doc.to_snapshot_v2();
    assert_eq!(after.entities[0], before.entities[0]);
    assert_eq!(after.uids[0], before.uids[0]);
    assert_eq!(after.entities.len(), 2);
    assert_eq!(after.entities[1].base().layer_id, "etiket2");
    assert_eq!(b.session.prompt().tool, None);
    b.doc.undo().unwrap();
    assert_eq!(b.doc.to_snapshot_v2(), before);
    assert!(!b.doc.can_undo());
    b.doc.redo().unwrap();
    assert_eq!(b.doc.to_snapshot_v2(), after);
}

fn floors() -> Bench {
    let mut b = drawing();
    let mut snapshot = b.doc.to_snapshot_v2();
    snapshot.settings.arch = Some(
        serde_json::from_value(json!({"levels":[
        {"id":"ground","name":"Zemin kat","elevation":0.},
        {"id":"upper","name":"Üst kat","elevation":3.}]}))
        .unwrap(),
    );
    for (i, level) in [Some("ground"), Some("upper"), None]
        .into_iter()
        .enumerate()
    {
        let mut e = snapshot.entities[0].clone();
        e.base_mut().id = (i + 1) as u32;
        let Entity::Space(s) = &mut e else { panic!() };
        s.space.arch.level = level.map(str::to_owned);
        s.space.number = Some(format!("R-{i}"));
        if i == 0 {
            snapshot.entities[0] = e;
        } else {
            snapshot.entities.push(e);
            snapshot
                .uids
                .push(kentos_commands::EntityId((100 + i as u128).to_be_bytes()));
        }
    }
    b.doc = kentos_domain::Document::from_snapshot_v2(snapshot).unwrap();
    b.start("tagAll");
    b
}

#[test]
fn bulk_visible_source_floor_and_cycle_only_preview_until_one_confirmation() {
    for (value, name, source) in [
        ("id:ground", "Zemin kat", 0),
        ("id:upper", "Üst kat", 1),
        ("unassigned", "Atanmamış", 2),
    ] {
        let mut b = floors();
        let before = b.doc.to_snapshot_v2();
        assert_eq!(shown(&b, "F"), "Tüm katlar");
        assert!(shown(&b, "N").starts_with("3 yeni"));
        assert!(b.run(|s, cx| s.choose_option("F", value, cx)));
        assert_eq!(shown(&b, "F"), name);
        assert!(shown(&b, "N").starts_with("1 yeni"));
        assert_eq!(b.doc.to_snapshot_v2(), before);
        b.confirm();
        assert_eq!(b.session.prompt().tool, None);
        let after = b.doc.to_snapshot_v2();
        assert_eq!(after.entities[..3], before.entities[..]);
        let Entity::Tag(tag) = &after.entities[3] else {
            panic!()
        };
        assert_eq!(tag.tag.of, before.uids[source].to_string());
        b.doc.undo().unwrap();
        assert_eq!(b.doc.to_snapshot_v2(), before);
        b.doc.redo().unwrap();
        assert_eq!(b.doc.to_snapshot_v2(), after);
    }
    let mut b = floors();
    let before = b.doc.to_snapshot_v2();
    for label in ["Zemin kat", "Üst kat", "Atanmamış", "Tüm katlar"] {
        assert!(b.type_text("F"));
        assert_eq!(shown(&b, "F"), label);
    }
    assert!(!set(&mut b, "F", "id:unknown"));
    b.run(|s, cx| s.cancel(cx));
    assert_eq!(b.doc.to_snapshot_v2(), before);
}

#[test]
fn removed_source_floor_is_never_silently_replaced_by_all_or_unassigned() {
    let mut b = floors();
    assert!(set(&mut b, "F", "id:ground"));
    let mut settings = b.doc.settings().clone();
    settings.arch.as_mut().unwrap().levels.remove(0);
    b.doc.set_settings(settings);
    let before = b.doc.to_snapshot_v2();
    b.confirm();
    assert_eq!(b.doc.to_snapshot_v2(), before);
    assert_eq!(b.session.prompt().tool, Some("Hepsini etiketle"));
    assert!(shown(&b, "F").contains("Bulunamadı: ground"));
    assert!(shown(&b, "!").contains("kimlikli kat yok"));
    assert!(set(&mut b, "F", "unassigned"));
    assert!(shown(&b, "N").starts_with("1 yeni"));
    b.confirm();
    let after = b.doc.to_snapshot_v2();
    let Entity::Tag(t) = after.entities.last().unwrap() else {
        panic!()
    };
    assert_eq!(t.tag.of, before.uids[2].to_string());
}

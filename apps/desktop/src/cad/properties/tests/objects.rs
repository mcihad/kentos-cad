//! Öznitelikler's tests: one object's and the drawing's rows; where a value comes from and the type
//! scope; a parcel, a line, a point, a local project's unit, attributes, texts, dimensions, hatches and linked texts.

use super::*;

#[test]
fn cad_inspector_explains_inherited_and_own_model_appearance_without_writing() {
    let mut app = objects();
    crate::files_testing::make_cad(&mut app);
    let slot = add(&mut app, Entity::Line(LineEntity {
        base: base("parsel"),
        a: Wire { x: E, y: N },
        b: Wire { x: E + 1.0, y: N },
        za: None, zb: None, ma: None, mb: None,
    }));
    select(&mut app, &[slot]);
    let before = app.document.as_ref().unwrap().model.to_snapshot_v2();
    let source = value(&app, "Temel model görünüşü", "Renk");
    assert!(source.contains("Katman:"), "{source}");
    assert!(value(&app, "Temel model görünüşü", "Çizgi tipi").contains("Katman:"));
    assert!(value(&app, "Temel model görünüşü", "Çizgi kalınlığı").contains("Katman:"));
    assert_eq!(app.document.as_ref().unwrap().model.to_snapshot_v2(), before);

    event(&mut app, Event::Color(vec![Slot(slot)], Some("#E5484D".into())));
    assert_eq!(value(&app, "Temel model görünüşü", "Renk"), "#E5484D · Nesne");
    assert!(value(&app, "Temel model görünüşü", "Çizgi kalınlığı").contains("Katman:"));
    event(&mut app, Event::Color(vec![Slot(slot)], None));
    assert!(value(&app, "Temel model görünüşü", "Renk").contains("Katman:"));
}

/// docs/adr/1100 §5: “Bu değer nereden geliyor?” says the level each value comes from and what its reset reaches,
/// writing nothing.
#[test]
fn the_value_origin_section_names_the_level_and_the_reset_scope() {
    let mut app = objects();
    crate::files_testing::make_cad(&mut app);
    let slot = add(&mut app, Entity::Line(LineEntity {
        base: base("parsel"),
        a: Wire { x: E, y: N },
        b: Wire { x: E + 1.0, y: N },
        za: None, zb: None, ma: None, mb: None,
    }));
    select(&mut app, &[slot]);
    assert!(value(&app, "Bu değer nereden geliyor?", "Renk").ends_with("· Katman"));
    assert!(value(&app, "Bu değer nereden geliyor?", "Çizgi kalınlığı").ends_with("· Katman"));
    event(&mut app, Event::Color(vec![Slot(slot)], Some("#E5484D".into())));
    assert_eq!(value(&app, "Bu değer nereden geliyor?", "Renk"), "#E5484D · Nesne");
    let notes: Vec<String> = rows(&app)
        .into_iter()
        .find(|(title, _)| title == "Bu değer nereden geliyor?")
        .map(|(_, rows)| rows.into_iter().map(|r| r.value).collect())
        .unwrap_or_default();
    assert!(notes.iter().any(|n| n.starts_with("Sıfırla → ") && n.contains("· bu nesne (1)")), "{notes:?}");
    event(&mut app, Event::Color(vec![Slot(slot)], None));
    assert!(value(&app, "Bu değer nereden geliyor?", "Renk").ends_with("· Katman"));
}

#[test]
fn nothing_selected_shows_the_drawing() {
    let app = objects();
    let doc = app.document.as_ref().expect("open");
    let panel = app.properties_panel(doc);
    assert!(matches!(panel.summary, Summary::Empty));
    assert_eq!(panel.meta, None);
    assert_eq!(value(&app, "Çizim", "Dosya"), "Etkileşim izi: nesneler");
    assert_eq!(value(&app, "Çizim", "SRID"), "EPSG:5256");
    assert_eq!(value(&app, "Çizim", "Çizim ölçeği"), "1:1000");
    assert_eq!(value(&app, "Çizim", "Nesne sayısı"), "7");
    assert_eq!(value(&app, "Çizim", "Etkin katman"), "Çizim");
    assert!(!editable(&app));
}

pub(super) fn scoped_text(app: &mut App, layer: &str, height: f64) -> Slot {
    Slot(add(app, serde_json::from_value(serde_json::json!({
        "kind":"text", "id":0, "layerId":layer, "attrs":{}, "p":{"x":E,"y":N},
        "text":"Ölçülü metin", "height":height, "rotation":0
    })).unwrap()))
}

#[test]
fn type_scope_preserves_selection_counts_locked_targets_and_edits_only_its_texts() {
    let mut app = objects();
    crate::files_testing::make_cad(&mut app);
    let a = scoped_text(&mut app, "cizim", 0.2);
    let b = scoped_text(&mut app, "cizim", 0.3);
    let locked = scoped_text(&mut app, "kilitli", 0.4);
    app.selection.set([Slot(4), a, b, locked]);
    let before = app.document.as_ref().unwrap().model.to_snapshot_v2();
    event(&mut app, Event::Kind(Some("text".into())));
    let doc = app.document.as_ref().unwrap();
    let panel = app.properties_panel(doc);
    assert_eq!(panel.total, 4);
    assert_eq!(panel.editable, 2);
    assert!(matches!(panel.summary, Summary::Many { count:3, .. }));
    assert_eq!(app.selection.ids(), [Slot(4), a, b, locked]);
    assert_eq!(doc.model.to_snapshot_v2(), before);
    let height = panel.sections.into_iter().flat_map(|s| s.rows).find(|r| r.label == "Yükseklik").unwrap();
    assert_eq!(height.value, "Çeşitli");
    let Some(Editor::Number(Field::TextHeights(ids))) = height.editor else {panic!("bulk height")};
    assert_eq!(ids, [a, b]);
    event(&mut app, Event::Commit(Field::TextHeights(ids), "25 cm".into()));
    for slot in [a,b] {
        let Entity::Text(t) = entity(&app,slot.0) else {panic!("text")};
        assert_eq!(t.height, 0.25);
        assert_eq!(t.text, "Ölçülü metin");
    }
    let Entity::Text(t) = entity(&app,locked.0) else {panic!("text")};
    assert_eq!(t.height,0.4);
    assert_eq!(undo_label(&mut app).as_deref(),Some("Yazı yüksekliği"));
    assert_eq!(app.document.as_ref().unwrap().model.to_snapshot_v2(),before);
    app.selection.set([Slot(4)]);
    assert!(app.properties_kind(app.document.as_ref().unwrap()).is_none());
}

#[test]
fn an_queued_property_edit_cannot_write_after_selection_revision_or_scope_changes() {
    let mut app = objects();
    let a = scoped_text(&mut app,"cizim",0.2);
    let b = scoped_text(&mut app,"cizim",0.2);
    app.selection.set([a]);
    let key = app.properties_key(app.document.as_ref().unwrap());
    let old = Event::Guarded(key,Box::new(Event::Commit(Field::Text(a),"Yanlış hedef".into())));
    app.selection.set([b]);
    let before = app.document.as_ref().unwrap().model.to_snapshot_v2();
    event(&mut app,old);
    assert_eq!(app.document.as_ref().unwrap().model.to_snapshot_v2(),before);
    let key = app.properties_key(app.document.as_ref().unwrap());
    event(&mut app,Event::Kind(Some("text".into())));
    event(&mut app,Event::Guarded(key,Box::new(Event::Commit(Field::Text(b),"Eski kapsam".into()))));
    assert_eq!(app.document.as_ref().unwrap().model.to_snapshot_v2(),before);
    let key = app.properties_key(app.document.as_ref().unwrap());
    scoped_text(&mut app,"cizim",0.1);
    let before = app.document.as_ref().unwrap().model.to_snapshot_v2();
    event(&mut app,Event::Guarded(key,Box::new(Event::Commit(Field::Text(b),"Eski revizyon".into()))));
    assert_eq!(app.document.as_ref().unwrap().model.to_snapshot_v2(),before);
}

#[test]
fn one_parcel_its_kind_layer_measures_and_attributes() {
    let mut app = objects();
    click(&mut app, -18.0, 9.0);
    let doc = app.document.as_ref().expect("open");
    let panel = app.properties_panel(doc);
    assert_eq!(panel.meta.as_deref(), Some("#4"));
    let Summary::One { kind, label, layer } = panel.summary else {
        panic!("one object");
    };
    assert_eq!((kind, label), ("Kapalı alan", None));
    assert_eq!(layer, Some(("#E5484D".to_owned(), "Parsel".to_owned())));
    assert_eq!(value(&app, "Genel", "Tür"), "Kapalı alan");
    assert_eq!(value(&app, "Genel", "Renk"), "Katmana göre");
    assert_eq!(value(&app, "Genel", "Sembol"), "Katman stiline göre");
    // 12 × 10 m.
    assert_eq!(value(&app, "Geometri", "Köşe sayısı"), "4");
    assert_eq!(value(&app, "Geometri", "Çevre"), "44.000 m");
    assert_eq!(value(&app, "Geometri", "Alan"), "120.00 m²");
    assert_eq!(value(&app, "Öznitelik bilgileri", "Ada"), "104");
    assert_eq!(value(&app, "Öznitelik bilgileri", "Parsel"), "7");
    // Katman ▾ lists every layer by its path; the locked one cannot be chosen.
    let genel = rows(&app).remove(0).1;
    let Some(Editor::Select { text, items, .. }) = &genel[1].editor else {
        panic!("Katman ▾");
    };
    assert_eq!(text, "Parsel");
    let picks: Vec<(String, bool, bool)> = items
        .iter()
        .filter_map(|c| match c {
            Choice::Pick {
                label,
                chosen,
                enabled,
                ..
            } => Some((label.clone(), *chosen, *enabled)),
            _ => None,
        })
        .collect();
    assert_eq!(
        picks,
        [
            ("Çizim".to_owned(), false, true),
            ("Parsel".to_owned(), true, true),
            ("Nokta".to_owned(), false, true),
            ("Kilitli katman".to_owned(), false, false),
            ("Gizli katman".to_owned(), false, true),
        ]
    );
}

#[test]
fn layer_and_colour_are_one_step_each_and_a_hidden_layer_is_said() {
    let mut app = objects();
    select(&mut app, &[4]);
    event(&mut app, Event::Layer(vec![Slot(4)], "nokta".to_owned()));
    assert_eq!(entity(&app, 4).base().layer_id, "nokta");
    event(
        &mut app,
        Event::Color(vec![Slot(4)], Some("#E5484D".to_owned())),
    );
    assert_eq!(value(&app, "Genel", "Renk"), "Kırmızı");
    assert_eq!(undo_label(&mut app).as_deref(), Some("Renk değiştir"));
    assert_eq!(undo_label(&mut app).as_deref(), Some("Katman değiştir"));
    // The point to the hidden layer: moved, said, still selected.
    select(&mut app, &[5]);
    event(&mut app, Event::Layer(vec![Slot(5)], "gizli".to_owned()));
    assert_eq!(entity(&app, 5).base().layer_id, "gizli");
    assert_eq!(
        warned(&app).as_deref(),
        Some("“Gizli katman” katmanı gizli; taşınan nesneler görünmeyecek.")
    );
    assert_eq!(app.selection.ids(), [Slot(5)]);
}

#[test]
fn a_line_shows_its_own_weight_and_kalinlik_sets_it_or_gives_it_back_to_the_layer() {
    // docs/adr/0139: an imported weight (1.20 mm) is shown, and listed among the ordinary ones.
    let mut app = objects();
    let mut own = base("nokta");
    own.line_weight = Some(1.2);
    let line = add(
        &mut app,
        Entity::Line(LineEntity {
            ma: None,
            mb: None,
            base: own,
            a: Wire { x: E, y: N },
            b: Wire { x: E + 10.0, y: N },
            za: None,
            zb: None,
        }),
    );
    select(&mut app, &[line]);
    assert_eq!(value(&app, "Genel", "Çizgi kalınlığı"), "1.20 mm");
    let genel = rows(&app).remove(0).1;
    let Some(Editor::Select { items, .. }) = &genel
        .iter()
        .find(|r| r.label == "Çizgi kalınlığı")
        .expect("Çizgi kalınlığı ▾")
        .editor
    else {
        panic!("Çizgi kalınlığı ▾");
    };
    let picks: Vec<(String, bool)> = items
        .iter()
        .filter_map(|c| match c {
            Choice::Pick { label, chosen, .. } => Some((label.clone(), *chosen)),
            _ => None,
        })
        .collect();
    assert_eq!(picks[0], ("Katmana göre".to_owned(), false));
    assert!(picks.contains(&("1.20 mm".to_owned(), true)), "{picks:?}");
    // Çizgi kalınlığı ▾ → 0.35 mm, then Katmana göre: each its own step (its undo name is the shared contract's).
    event(&mut app, Event::Weight(vec![Slot(line)], Some(0.35)));
    assert_eq!(entity(&app, line).base().line_weight, Some(0.35));
    assert_eq!(value(&app, "Genel", "Çizgi kalınlığı"), "0.35 mm");
    event(&mut app, Event::Weight(vec![Slot(line)], None));
    assert_eq!(entity(&app, line).base().line_weight, None);
    assert_eq!(value(&app, "Genel", "Çizgi kalınlığı"), "Katmana göre");
    assert_eq!(undo_label(&mut app).as_deref(), Some("Kalınlık değiştir"));
    assert_eq!(entity(&app, line).base().line_weight, Some(0.35));
    // A point is not drawn with a line: it shows no weight.
    select(&mut app, &[5]);
    assert!(rows(&app)[0].1.iter().all(|r| r.label != "Çizgi kalınlığı"));
}

#[test]
fn a_point_is_moved_by_its_typed_coordinates_read_as_the_web_reads_them() {
    let mut app = objects();
    select(&mut app, &[5]);
    event(
        &mut app,
        Event::Commit(Field::PointX(Slot(5)), "487021,5".into()),
    );
    let Entity::Point(p) = entity(&app, 5) else {
        panic!("a point");
    };
    assert_eq!(p.p.x, E + 21.5);
    // What is not a number changes nothing; “12abc” is 12, as parseFloat reads it.
    event(
        &mut app,
        Event::Commit(Field::PointY(Slot(5)), "yok".into()),
    );
    event(
        &mut app,
        Event::Commit(Field::PointY(Slot(5)), "4419990abc".into()),
    );
    let Entity::Point(p) = entity(&app, 5) else {
        panic!("a point");
    };
    assert_eq!(p.p.y, N - 10.0);
    assert_eq!(undo_label(&mut app).as_deref(), Some("Değiştir"));
}

/// A local project's drawing unit (docs/adr/0165 §2): rows read in it, typed
/// values are taken in it (a trailing unit too), the drawing keeps metres.
#[test]
fn a_local_project_reads_and_takes_its_unit() {
    let mut app = objects();
    {
        let doc = app.document.as_mut().expect("open");
        let settings = kentos_commands::ProjectSettings {
            srid: 0,
            drawing_unit: Some(kentos_commands::DrawingUnit::Mm),
            ..doc.model.settings().clone()
        };
        doc.model.set_settings(settings);
    }
    select(&mut app, &[5]);
    event(
        &mut app,
        Event::Commit(Field::PointX(Slot(5)), "1500".into()),
    );
    let Entity::Point(p) = entity(&app, 5) else {
        panic!("a point");
    };
    assert_eq!(p.p.x, 1.5);
    assert_eq!(value(&app, "Geometri", "Y (sağa)"), "1500.000 mm");
    let line = line_z(&mut app, Some(0.1), None);
    select(&mut app, &[line]);
    assert_eq!(value(&app, "Geometri", "Kot (başlangıç)"), "100.000 mm");
    event(
        &mut app,
        Event::Commit(Field::Elevation(Slot(line), Spot::End), "250 mm".into()),
    );
    let Entity::Line(l) = entity(&app, line) else {
        panic!("a line");
    };
    assert_eq!(l.zb, Some(0.25));
}

#[test]
fn an_attribute_is_written_as_typed_and_the_parcel_number_follows() {
    let mut app = objects();
    select(&mut app, &[4]);
    // The drawn number equals the attribute: it follows the new value.
    if let Some(doc) = app.document.as_mut() {
        let mut e = doc.model.get(Slot(4)).cloned().expect("the parcel");
        e.base_mut().label = Some("7".to_owned());
        doc.model.update(Slot(4), e);
    }
    event(
        &mut app,
        Event::Commit(Field::Attribute(Slot(4), "Parsel".into()), " 8".into()),
    );
    let e = entity(&app, 4);
    assert_eq!(e.base().attrs.get("Parsel").map(String::as_str), Some(" 8"));
    assert_eq!(e.base().label.as_deref(), Some(" 8"));
    // An empty value is taken; Ada is not the label.
    event(
        &mut app,
        Event::Commit(Field::Attribute(Slot(4), "Ada".into()), String::new()),
    );
    let e = entity(&app, 4);
    assert_eq!(e.base().attrs.get("Ada").map(String::as_str), Some(""));
    assert_eq!(e.base().label.as_deref(), Some(" 8"));
}

#[test]
fn texts_dimensions_and_hatches_take_what_the_web_takes() {
    let mut app = objects();
    let text = add(
        &mut app,
        Entity::Text(TextEntity {
            annotative: None,
            base: base("cizim"),
            p: Wire { x: E, y: N },
            text: "Ada".to_owned(),
            height: 2.5,
            rotation: 0.0,
            align: None,
            width_factor: None,
            mask: false,
            label_of: None,
            label_scale: None,
            paragraph: Default::default(),
            face: Default::default(),
            path: None,
        }),
    );
    let s = Slot(text);
    for (field, typed) in [
        (Field::Text(s), "  Ada 5  "),
        (Field::Text(s), "   "),
        (Field::TextHeight(s), "0"),
        (Field::TextHeight(s), "3,5"),
        (Field::TextAngle(s), "-90"),
    ] {
        event(&mut app, Event::Commit(field, typed.to_owned()));
    }
    let Entity::Text(t) = entity(&app, text) else {
        panic!("a text");
    };
    assert_eq!(
        (t.text.as_str(), t.height, t.rotation),
        ("Ada 5", 3.5, 270.0)
    );

    let dim = add(
        &mut app,
        Entity::Dimension(DimensionEntity {
            annotative: None,
            text_shift: None,
            breaks: Vec::new(),
            base: base("cizim"),
            a: Wire { x: E, y: N },
            b: Wire { x: E + 10.0, y: N },
            offset: 2.0,
            height: 2.5,
            text: Some("özel".to_owned()),
            style: None,
            angle: None,
            c: None,
            mask: false,
            za: None,
            zb: None,
            refs: None,
            look: Default::default(),
            level: None,
        }),
    );
    let s = Slot(dim);
    for (field, typed) in [
        (Field::DimensionText(s), "  "),
        (Field::DimensionHeight(s), "-1"),
        (Field::DimensionOffset(s), "-3"),
    ] {
        event(&mut app, Event::Commit(field, typed.to_owned()));
    }
    let Entity::Dimension(d) = entity(&app, dim) else {
        panic!("a dimension");
    };
    assert_eq!((d.text, d.height, d.offset), (None, 2.5, -3.0));
    select(&mut app, &[dim]);
    assert_eq!(value(&app, "Geometri", "Ölçülen uzunluk"), "10.000 m");
    assert_eq!(value(&app, "Geometri", "Ötelenme"), "-3.000 m");

    let hatch = add(
        &mut app,
        Entity::Hatch(HatchEntity {
            boundary: None,
            base: base("cizim"),
            ring: vec![
                Wire { x: E, y: N },
                Wire { x: E + 4.0, y: N },
                Wire {
                    x: E + 4.0,
                    y: N + 5.0,
                },
            ],
            holes: None,
            pattern: HatchPattern::user(HatchPatternType::Lines, 45.0, 3.0),
            assoc: None,
            annotative: None,
        }),
    );
    let s = Slot(hatch);
    // Çapraz, Desen's second choice: its spacing and turn carried over (docs/adr/0186 §7).
    event(&mut app, Event::Pattern(s, 1));
    event(&mut app, Event::Commit(Field::HatchSpacing(s), "0".into()));
    event(&mut app, Event::Commit(Field::HatchAngle(s), "30".into()));
    let Entity::Hatch(h) = entity(&app, hatch) else {
        panic!("a hatch");
    };
    assert_eq!(
        (h.pattern.kind, h.pattern.angle, h.pattern.spacing),
        (HatchPatternType::Cross, 30.0, 3.0)
    );
    select(&mut app, &[hatch]);
    assert_eq!(value(&app, "Geometri", "Desen"), "Çapraz");
    assert_eq!(value(&app, "Geometri", "Açı"), "30.00 °");
    assert_eq!(value(&app, "Geometri", "Alan"), "10.00 m²");
    assert_eq!(value(&app, "Geometri", "İlişkili"), "Hayır");
    // ANSI31: a pattern of the library at its paper scale; then a sphere's gradient.
    event(
        &mut app,
        Event::Pattern(s, kentos_geometry_core::tools::hatch::DEFAULT_CHOICE),
    );
    select(&mut app, &[hatch]);
    assert_eq!(value(&app, "Geometri", "Desen"), "ANSI31");
    assert_eq!(value(&app, "Geometri", "Ölçek"), "1.000");
    let last = kentos_geometry_core::tools::hatch::choices().len() - 1;
    event(&mut app, Event::Pattern(s, last));
    event(&mut app, Event::GradientColour(s, "#E5484D".into()));
    event(&mut app, Event::GradientInverted(s, true));
    select(&mut app, &[hatch]);
    assert_eq!(value(&app, "Geometri", "Degrade biçimi"), "Küre");
    assert_eq!(value(&app, "Geometri", "İkinci renk"), "#E5484D");
    assert_eq!(value(&app, "Geometri", "Ters"), "Evet");
}

/// A linked text's Bağlı nesne row (docs/adr/0175 §4): its object's kind,
/// Nesneyi seç and Bağı kopar (one step, `cad.entities.set`), for several
/// texts how many are linked; on a locked layer only Nesneyi seç; an object
/// not in the drawing is said. The web's `textRows.test.ts` walks the same.
#[test]
fn a_linked_text_names_its_object_and_bagi_kopar_breaks_the_link() {
    let mut app = objects();
    let parcel = app
        .document
        .as_ref()
        .and_then(|d| d.model.uid(Slot(4)))
        .expect("the parcel's persistent id");
    let text = |label_of: Option<kentos_domain::Uuid>, words: &str| {
        Entity::Text(TextEntity {
            annotative: None,
            base: base("cizim"),
            p: Wire { x: E, y: N },
            text: words.to_owned(),
            height: 2.5,
            rotation: 0.0,
            align: None,
            width_factor: None,
            mask: false,
            label_of: label_of.map(|u| kentos_commands::EntityId(*u.as_bytes())),
            label_scale: label_of.map(|_| 1000.0),
            paragraph: Default::default(),
            face: Default::default(),
            path: None,
        })
    };
    let linked = add(&mut app, text(Some(parcel), "12"));
    let free = add(&mut app, text(None, "Not"));
    select(&mut app, &[free]);
    assert!(!has_row(&app, "Geometri", "Bağlı nesne"));
    select(&mut app, &[linked]);
    assert_eq!(value(&app, "Geometri", "Bağlı nesne"), "Kapalı alan");
    let labels = |app: &App| -> Vec<String> {
        rows(app)
            .into_iter()
            .flat_map(|(_, rows)| rows)
            .find(|r| r.label == "Bağlı nesne")
            .and_then(|r| match r.editor {
                Some(Editor::Select { items, .. }) => Some(
                    items
                        .into_iter()
                        .filter_map(|c| match c {
                            Choice::Pick { label, .. } => Some(label),
                            _ => None,
                        })
                        .collect(),
                ),
                _ => None,
            })
            .unwrap_or_default()
    };
    assert_eq!(labels(&app), ["Nesneyi seç", "Bağı kopar"]);
    event(&mut app, Event::SelectObject(Slot(4)));
    assert_eq!(app.selection.ids(), [Slot(4)]);
    select(&mut app, &[linked]);
    event(&mut app, Event::Unlink(vec![Slot(linked)]));
    let Entity::Text(t) = entity(&app, linked) else {
        panic!("a text")
    };
    assert_eq!((t.label_of, t.label_scale), (None, None));
    assert_eq!(undo_label(&mut app).as_deref(), Some("Bağı kopar"));
    // Several texts: how many are linked, Bağı kopar for them all.
    select(&mut app, &[linked, free]);
    assert_eq!(value(&app, "Yazı", "Bağlı nesne"), "1 yazı bağlı");
    assert_eq!(labels(&app), ["Bağı kopar"]);
    // Its object gone (with the text), a text naming it says so.
    let removed = app
        .document
        .as_mut()
        .map(|d| d.model.remove(&[Slot(4)]))
        .unwrap_or(0);
    assert_eq!(removed, 1);
    let orphan = add(&mut app, text(Some(parcel), "12"));
    select(&mut app, &[orphan]);
    assert_eq!(value(&app, "Geometri", "Bağlı nesne"), "Çizimde yok");
    assert_eq!(labels(&app), ["Bağı kopar"]);
}

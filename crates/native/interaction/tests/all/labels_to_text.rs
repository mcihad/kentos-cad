//! Etiketleri yazıya çevir (docs/adr/0175 §3) through the session, over the
//! native document: the labels taken when it starts, the finding shown
//! first, the scale and the three options, the texts written in one step on
//! the standard text layer (opened in that step) or the active one; as the
//! web's `apps/web/src/tools/labelsToTextTool.test.ts` walks it, the same
//! cases moved to (E, N). The places and sizes are the core's, checked
//! against the independent reference in the core's `tests/all/label_text.rs`.

use crate::common;

use common::{Bench, E, N};
use kentos_contracts::{Entity, PathEntity, PointEntity, TextAlign, TextEntity, Vec2, Workspace};
use kentos_domain::Slot;
use kentos_interaction::labels_to_text::read_scale;
use kentos_interaction::{LabelSpot, Level, Spatial};

const LABEL: &str = "Etiketleri yazıya çevir";

fn options(scale: u64, every: &str, mask: &str, layer: &str) -> String {
    linked_options(scale, every, mask, layer, "kapalı")
}

fn linked_options(scale: u64, every: &str, mask: &str, layer: &str, linked: &str) -> String {
    format!(
        "[Ölçek (Ö): 1:{scale} / Örtüşenler de (R): {every} / Zemin (Z): {mask} / Katman (K): {layer} / Nesneye bağlı (B): {linked} / Uygula (Enter)]"
    )
}

fn pt(x: f64, y: f64) -> Vec2 {
    Vec2 { x: E + x, y: N + y }
}

fn labelled(mut base: kentos_contracts::EntityBase, label: &str) -> kentos_contracts::EntityBase {
    base.label = Some(label.to_owned());
    base
}

/// Two parcels with numbers, a third whose label falls on the first's, a
/// parcel too small to label at 1:1000, a named point and a street, each
/// with its kind's default style (the drawing's).
fn drawing() -> Bench {
    let mut b = Bench::on(include_str!(
        "../../../../../fixtures/interaction/v1/empty.kcad"
    ));
    b.draft.snap = false;
    let square = |b: &mut Bench, x0: f64, y0: f64, x1: f64, y1: f64, label: &str| {
        b.doc
            .add(Entity::Polygon(PathEntity {
                base: labelled(common::base("cizim"), label),
                pts: vec![pt(x0, y0), pt(x1, y0), pt(x1, y1), pt(x0, y1)],
                bulges: None,
                holes: None,
                zs: None,
                parts: None,
            }))
            .expect("a slot")
    };
    square(&mut b, 0.0, 0.0, 20.0, 15.0, "101");
    square(&mut b, 20.0, 0.0, 40.0, 15.0, "102");
    square(&mut b, 2.0, 1.0, 18.0, 14.0, "104");
    square(&mut b, 8.0, 6.0, 11.0, 9.0, "103");
    b.doc
        .add(Entity::Point(PointEntity {
            base: labelled(common::base("cizim"), "P1"),
            p: pt(45.0, 5.0),
            z: None,
            parts: None,
        }))
        .expect("a slot");
    b.doc
        .add(Entity::Polyline(PathEntity {
            base: labelled(common::base("cizim"), "Cumhuriyet Cd."),
            pts: vec![pt(50.0, 20.0), pt(0.0, 20.0)],
            bulges: None,
            holes: None,
            zs: None,
            parts: None,
        }))
        .expect("a slot");
    b
}

fn texts(b: &Bench) -> Vec<TextEntity> {
    b.doc
        .entities()
        .filter_map(|e| match e {
            Entity::Text(t) => Some(t.clone()),
            _ => None,
        })
        .collect()
}

fn near(a: f64, b: f64, tol: f64) -> bool {
    (a - b).abs() <= tol
}

#[test]
fn writes_the_labels_as_a_sheet_would_in_one_step_on_the_text_layer_it_opens() {
    let mut b = drawing();
    let n0 = b.doc.len();
    let before = b.log.len();
    b.start("labelsToText");
    assert_eq!(
        b.said(before),
        [(
            Level::Info,
            format!("{LABEL}: bütün çizimde 6 etiket; 1:1000 ölçekte 4 yazı olacak, 1 örtüşen, 1 küçük etiket atlanacak. Enter ile yazın.").as_str()
        )]
    );
    assert_eq!(
        b.session.prompt().text(),
        format!(
            "{LABEL}: 1:1000 ölçekte 4 yazı olacak, 1 örtüşen, 1 küçük etiket atlanacak {}",
            options(1000, "kapalı", "kapalı", "Yazılar")
        )
    );
    assert_eq!(b.options(), ["Ö", "R", "Z", "K", "B", "Enter"]);
    b.move_to(60.0, 10.0);
    let preview = b.run(|s, cx| s.preview(&cx.format())).expect("a preview");
    assert_eq!(
        preview.tag.map(|t| t.lines),
        Some(vec![
            "4 yazı, 1:1000".to_owned(),
            "1 örtüşen atlanır".to_owned(),
            "1 küçük atlanır".to_owned(),
            "Enter: yaz".to_owned(),
        ])
    );
    assert_eq!(preview.texts.len(), 4);
    // Örtüşenler de: the third parcel's number too.
    assert!(b.type_text("R"));
    assert_eq!(
        b.last_text(),
        Some(format!("{LABEL}: 1:1000 ölçekte 5 yazı olacak, 1 küçük etiket atlanacak.").as_str())
    );
    // A typed scale: the parcels' labels grow to their 14 px cap.
    assert!(b.type_text("1:500"));
    assert_eq!(
        b.session.prompt().text(),
        format!(
            "{LABEL}: 1:500 ölçekte 5 yazı olacak, 1 küçük etiket atlanacak {}",
            options(500, "açık", "kapalı", "Yazılar")
        )
    );
    b.confirm();
    let made = texts(&b);
    assert_eq!(
        made.iter().map(|t| t.text.as_str()).collect::<Vec<_>>(),
        ["101", "102", "104", "P1", "Cumhuriyet Cd."]
    );
    assert!(made.iter().all(|t| t.base.layer_id == "yazi"));
    assert_eq!(
        b.doc.layers().get("yazi").map(|l| l.name.as_str()),
        Some("Yazılar")
    );
    let k = 96.0 / 0.0254 / 500.0;
    assert_eq!(made[0].p, pt(10.0, 7.5));
    assert_eq!(made[0].align, Some(TextAlign::MiddleCenter));
    assert_eq!(made[0].rotation, 0.0);
    assert!(near(made[0].height, 14.0 / k, 1e-12), "{}", made[0].height);
    assert_eq!(made[3].align, Some(TextAlign::MiddleLeft));
    assert!(near(made[3].p.x, E + 45.0 + 7.0 / k, 1e-9));
    // The street runs west: its name turns half round to read.
    assert_eq!(made[4].p, pt(25.0, 20.0));
    assert_eq!(made[4].rotation, 0.0);
    assert!(made.iter().all(|t| !t.mask));
    assert_eq!(
        b.selected(),
        made.iter().map(|t| t.base.id).collect::<Vec<_>>()
    );
    let n = b.log.len();
    assert_eq!(
        b.said(n - 2),
        [
            (
                Level::Info,
                "“Yazılar” katmanı çizimde yoktu; etiketlerin yazıları için açıldı."
            ),
            (
                Level::Success,
                format!("{LABEL}: 5 etiket yazıya çevrildi, 1 küçük etiket atlandı.").as_str()
            ),
        ]
    );
    assert_eq!(b.session.tool_id(), "select");
    assert_eq!(b.doc.undo().as_deref(), Some(LABEL));
    assert_eq!(b.doc.len(), n0);
    assert!(b.doc.layers().get("yazi").is_none());
}

#[test]
fn takes_a_selections_labels_only_writes_masked_texts_on_the_active_layer_and_keeps_its_options() {
    let mut b = drawing();
    let ids: Vec<Slot> = b
        .doc
        .entities()
        .filter(|e| matches!(e.base().label.as_deref(), Some("102" | "P1")))
        .map(|e| Slot(e.base().id))
        .collect();
    b.selection.set(ids);
    let before = b.log.len();
    b.start("labelsToText");
    assert_eq!(
        b.said(before),
        [(
            Level::Info,
            format!("{LABEL}: seçimde 2 etiket; 1:1000 ölçekte 2 yazı olacak. Enter ile yazın.")
                .as_str()
        )]
    );
    assert!(b.type_text("Z"));
    assert!(b.type_text("K"));
    assert_eq!(
        b.session.prompt().text(),
        format!(
            "{LABEL}: 1:1000 ölçekte 2 yazı olacak {}",
            options(1000, "kapalı", "açık", "Çizim")
        )
    );
    b.confirm();
    let made: Vec<(String, String, bool)> = texts(&b)
        .into_iter()
        .map(|t| (t.text, t.base.layer_id, t.mask))
        .collect();
    assert_eq!(
        made,
        [
            ("102".to_owned(), "cizim".to_owned(), true),
            ("P1".to_owned(), "cizim".to_owned(), true),
        ]
    );
    assert_eq!(b.doc.undo().as_deref(), Some(LABEL));
    assert!(b.memory.labels_mask && b.memory.labels_active);
}

#[test]
fn writes_texts_linked_to_their_objects_which_they_follow_and_whose_labels_leave_the_scope() {
    let mut b = drawing();
    b.start("labelsToText");
    assert!(b.type_text("B"));
    assert_eq!(
        b.session.prompt().text(),
        format!(
            "{LABEL}: 1:1000 ölçekte 4 yazı olacak, 1 örtüşen, 1 küçük etiket atlanacak {}",
            linked_options(1000, "kapalı", "kapalı", "Yazılar", "açık")
        )
    );
    b.confirm();
    let made = texts(&b);
    assert_eq!(
        made.iter().map(|t| t.text.as_str()).collect::<Vec<_>>(),
        ["101", "102", "P1", "Cumhuriyet Cd."]
    );
    // Each knows its object (the label it writes) and the scale.
    let object = |t: &TextEntity| {
        let uid = kentos_domain::Uuid::from_bytes(t.label_of.expect("a link").0);
        b.doc.slot_of(uid).expect("its object")
    };
    for t in &made {
        assert_eq!(
            b.doc.get(object(t)).and_then(|e| e.base().label.as_deref()),
            Some(t.text.as_str())
        );
        assert_eq!(t.label_scale, Some(1000.0));
    }
    // The parcel moves: its text follows in the same step, and one undo takes both back.
    let parcel = object(&made[1]);
    let Some(Entity::Polygon(mut moved)) = b.doc.get(parcel).cloned() else {
        panic!("a parcel")
    };
    for p in &mut moved.pts {
        p.y -= 10.0;
    }
    assert!(b.doc.update(parcel, Entity::Polygon(moved)));
    let Some(Entity::Text(text)) = b.doc.get(Slot(made[1].base.id)).cloned() else {
        panic!("a text")
    };
    assert!(near(text.p.x, made[1].p.x, 1e-9));
    assert!(near(text.p.y, made[1].p.y - 10.0, 1e-9));
    assert_eq!(text.label_of, made[1].label_of);
    assert!(b.doc.undo().is_some());
    assert_eq!(texts(&b)[1].p, made[1].p);
    // Run again on the whole drawing: the linked objects' labels are texts now; 104's and the small 103's are left.
    b.selection.set(Vec::<Slot>::new());
    let before = b.log.len();
    b.start("labelsToText");
    assert_eq!(
        b.said(before),
        [(
            Level::Info,
            format!("{LABEL}: bütün çizimde 2 etiket; 1:1000 ölçekte 1 yazı olacak, 1 küçük etiket atlanacak. Enter ile yazın.").as_str()
        )]
    );
}

/// An object whose label a text writes shows none of its own in the drawing
/// (docs/adr/0175 §4): the store hears it from the document as it syncs,
/// through undo and redo, the link's breaking and the object's removal (the
/// web's `apps/web/src/viewport/linkedLabels.test.ts`).
#[test]
fn an_object_whose_label_a_text_writes_shows_none_of_its_own() {
    let mut b = drawing();
    let mut spatial = Spatial::of(&b.doc);
    // The parcels whose own label the drawing places at 3 px/m (103 is too small).
    let centred = |spatial: &mut Spatial, b: &Bench| -> Vec<u32> {
        spatial.sync(&b.doc);
        spatial
            .labels(
                kentos_interaction::Vec2::new(E - 100.0, N - 100.0),
                kentos_interaction::Vec2::new(E + 200.0, N + 200.0),
                3.0,
            )
            .into_iter()
            .filter_map(|spot| match spot {
                LabelSpot::Center { slot, .. } => Some(slot.0),
                _ => None,
            })
            .collect()
    };
    assert_eq!(centred(&mut spatial, &b), [1, 2, 3]);
    let parcel = b.doc.uid(Slot(1)).expect("a persistent id");
    let text = b
        .doc
        .add(Entity::Text(TextEntity {
            base: common::base("cizim"),
            p: pt(10.0, 7.5),
            text: "101".into(),
            height: 2.0,
            rotation: 0.0,
            align: Some(TextAlign::MiddleCenter),
            width_factor: None,
            mask: false,
            label_of: Some(kentos_contracts::EntityId(*parcel.as_bytes())),
            label_scale: Some(1000.0),
            paragraph: Default::default(),
        }))
        .expect("a slot");
    assert_eq!(centred(&mut spatial, &b), [2, 3]);
    assert!(b.doc.undo().is_some());
    assert_eq!(centred(&mut spatial, &b), [1, 2, 3]);
    assert!(b.doc.redo().is_some());
    assert_eq!(centred(&mut spatial, &b), [2, 3]);
    // The text moved by hand loses its link: the object's own label is back.
    let Some(Entity::Text(mut moved)) = b.doc.get(text).cloned() else {
        panic!("a text")
    };
    moved.p = pt(11.0, 8.5);
    assert!(b.doc.update(text, Entity::Text(moved)));
    assert_eq!(centred(&mut spatial, &b), [1, 2, 3]);
    assert!(b.doc.undo().is_some());
    assert_eq!(centred(&mut spatial, &b), [2, 3]);
    // The object removed takes its text with it.
    assert_eq!(b.doc.remove(&[Slot(1)]), 1);
    assert!(b.doc.get(text).is_none());
    assert_eq!(centred(&mut spatial, &b), [2, 3]);
    assert!(b.doc.undo().is_some());
    assert!(b.doc.get(text).is_some());
    assert_eq!(centred(&mut spatial, &b), [2, 3]);
}

#[test]
fn asks_for_the_scale_says_a_wrong_one_and_leaves_at_once_when_there_is_no_label() {
    let mut b = drawing();
    b.start("labelsToText");
    assert!(b.type_text("Ö"));
    assert_eq!(
        b.session.prompt().text(),
        format!("{LABEL}: ölçeği 1:N ya da N olarak yazın (Enter: 1:1000)")
    );
    assert!(b.type_text("1:0"));
    assert_eq!(
        b.last_text(),
        Some("Ölçeği 1:N ya da N olarak, 1 ya da daha büyük bir tam sayıyla yazın (1:500, 1000).")
    );
    assert!(b.type_text("5000"));
    // At 1:5000 the point and the street are below their styles' scales, the parcels too small.
    assert_eq!(
        b.last_text(),
        Some(format!("{LABEL}: 1:5000 ölçekte yazı olacak etiket yok, 2 ölçek dışı, 4 küçük etiket atlanacak; başka bir ölçek yazın.").as_str())
    );
    assert!(!b.type_text("abc"));

    let mut empty = Bench::on(include_str!(
        "../../../../../fixtures/interaction/v1/empty.kcad"
    ));
    empty.add_line("cizim", [0.0, 0.0], [10.0, 0.0]);
    let before = empty.log.len();
    empty.start("labelsToText");
    assert_eq!(
        empty.said(before),
        [(
            Level::Warn,
            format!("{LABEL}: çizimde yazıya çevrilecek etiket yok.").as_str()
        )]
    );
    assert_eq!(empty.session.tool_id(), "select");
}

#[test]
fn names_a_cad_projects_text_layer_as_its_template_does() {
    let mut b = drawing();
    let mut settings = b.doc.settings().clone();
    settings.workspace = Some(Workspace::Cad);
    b.doc.set_settings(settings);
    b.start("labelsToText");
    assert!(
        b.session
            .prompt()
            .text()
            .ends_with(&options(1000, "kapalı", "kapalı", "Yazı"))
    );
    b.confirm();
    assert_eq!(
        b.doc.layers().get("yazi").map(|l| l.name.as_str()),
        Some("Yazı")
    );
}

#[test]
fn reads_a_scale_as_one_to_n_or_n() {
    assert_eq!(read_scale("1:500"), Some(500));
    assert_eq!(read_scale("1000"), Some(1000));
    assert_eq!(read_scale("1:0"), None);
}

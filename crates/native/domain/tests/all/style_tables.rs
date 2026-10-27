//! The style tables in the undo step (docs/adr/0699): a style edit is undone with its table, style by style; a
//! style another editor changed since is left as theirs, and a style that would come back under a taken name stays
//! away.

use kentos_domain::contracts::{DocumentSnapshotV1, ProjectSettings, WallStyleDef};
use kentos_domain::{Document, External, ExternalMeta};

fn style(id: &str, name: &str, width: f64) -> WallStyleDef {
    WallStyleDef {
        id: id.into(),
        name: name.into(),
        components: Vec::new(),
        caps: Default::default(),
        fill: None,
        width: Some(width),
    }
}

/// A drawing with three wall styles, no objects, saved.
fn drawing() -> Document {
    let text = r#"{"format":"kentos.document","version":1,"name":"Stiller","settings":{"srid":5256,"lengthDecimals":3,
        "areaDecimals":2,"areaUnit":"m2","angleUnit":"grad","plotScale":100},"origin":{"x":0,"y":0},
        "layers":[{"id":"a","name":"A","type":"layer","visible":true,"locked":false,"expanded":true,
        "style":{"color":"fg","lineType":"continuous","lineWeight":0.18},"children":[]}],
        "activeLayer":"a","entities":[],"styles":{"items":[],"categories":[]}}"#;
    let mut doc =
        Document::from_snapshot(DocumentSnapshotV1::from_json(text).expect("reads")).expect("opens");
    let mut settings = doc.settings().clone();
    settings.wall_styles = vec![
        style("s1", "Dış 25", 0.25),
        style("s2", "Bölme 10", 0.1),
        style("s3", "Perde 20", 0.2),
    ];
    doc.set_settings(settings);
    doc.mark_saved(doc.revision());
    doc
}

fn names(doc: &Document) -> Vec<&str> {
    doc.settings()
        .wall_styles
        .iter()
        .map(|s| s.name.as_str())
        .collect()
}

/// The table written as `cad.styles.edit` writes it: `set_settings` inside the recorded body.
fn edit(doc: &mut Document, change: impl FnOnce(&mut ProjectSettings)) {
    doc.with_style_tables("Duvar stili", |doc| {
        let mut settings = doc.settings().clone();
        change(&mut settings);
        doc.set_settings(settings);
    });
}

#[test]
fn a_name_alone_is_one_step_and_comes_back_on_undo() {
    let mut doc = drawing();
    assert!(!doc.can_undo());
    edit(&mut doc, |s| s.wall_styles[0].name = "Dış duvar 25 QA".into());
    assert_eq!(names(&doc), ["Dış duvar 25 QA", "Bölme 10", "Perde 20"]);
    assert!(doc.can_undo(), "a renamed style is an undo step");
    assert_eq!(doc.undo().as_deref(), Some("Duvar stili"));
    assert_eq!(names(&doc), ["Dış 25", "Bölme 10", "Perde 20"]);
    assert!(!doc.can_undo());
    assert_eq!(doc.redo().as_deref(), Some("Duvar stili"));
    assert_eq!(names(&doc), ["Dış duvar 25 QA", "Bölme 10", "Perde 20"]);
}

#[test]
fn a_removed_style_comes_back_in_its_place_and_a_new_one_goes() {
    let mut doc = drawing();
    edit(&mut doc, |s| {
        s.wall_styles.remove(1);
        s.wall_styles.push(style("s4", "Yalıtımlı 30", 0.3));
    });
    assert_eq!(names(&doc), ["Dış 25", "Perde 20", "Yalıtımlı 30"]);
    doc.undo();
    assert_eq!(names(&doc), ["Dış 25", "Bölme 10", "Perde 20"]);
    doc.redo();
    assert_eq!(names(&doc), ["Dış 25", "Perde 20", "Yalıtımlı 30"]);
}

#[test]
fn nothing_changed_is_no_step() {
    let mut doc = drawing();
    edit(&mut doc, |_| {});
    assert!(!doc.can_undo());
}

#[test]
fn another_editors_newer_style_stays_and_the_untouched_ones_too() {
    let mut doc = drawing();
    edit(&mut doc, |s| s.wall_styles[0].width = Some(0.3));
    // Another editor renames the same style and adds one, after this step.
    let mut theirs = doc.settings().clone();
    theirs.wall_styles[0].name = "Dış (onların)".into();
    theirs.wall_styles.push(style("s9", "Onların 40", 0.4));
    doc.apply_external(External {
        meta: Some(ExternalMeta {
            settings: Some(theirs),
            ..ExternalMeta::default()
        }),
        ..External::default()
    })
    .expect("takes their settings");
    doc.undo();
    // Their newer version of s1 is not taken back to the width before this step; their new style stays.
    assert_eq!(
        names(&doc),
        ["Dış (onların)", "Bölme 10", "Perde 20", "Onların 40"]
    );
    assert_eq!(doc.settings().wall_styles[0].width, Some(0.3));
}

#[test]
fn a_style_that_would_come_back_under_a_taken_name_stays_away() {
    let mut doc = drawing();
    edit(&mut doc, |s| {
        s.wall_styles.remove(1);
    });
    // Another editor makes a new style with the removed one's name.
    let mut theirs = doc.settings().clone();
    theirs.wall_styles.push(style("s7", "Bölme 10", 0.12));
    doc.apply_external(External {
        meta: Some(ExternalMeta {
            settings: Some(theirs),
            ..ExternalMeta::default()
        }),
        ..External::default()
    })
    .expect("takes their settings");
    doc.undo();
    // Two styles may not share a name: theirs stays, the removed one does not come back.
    assert_eq!(names(&doc), ["Dış 25", "Perde 20", "Bölme 10"]);
    assert_eq!(doc.settings().wall_styles[2].id, "s7");
}

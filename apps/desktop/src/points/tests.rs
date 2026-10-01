//! Noktalar's rules and the tab over `fixtures/interaction/v1/point-editor.kcad`
//! (docs/adr/0153 §2): the cases of the web's `ui/bottom/PointTable.test.ts`,
//! then the tab as the app runs it, and pictures for the owner.

use kentos_domain::Slot;

use super::{COLUMNS, Event, click_pick, next_sort, row_of};
use crate::app::{App, Message};
use crate::bottom::BottomTab;

fn app_with_points() -> App {
    let (mut app, _) = App::boot(None);
    let snapshot = kentos_contracts::DocumentSnapshotV1::from_json(include_str!(
        "../../../../fixtures/interaction/v1/point-editor.kcad"
    ))
    .expect("the drawing reads");
    let doc = crate::document::Document::new(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    app
}

/// The names of the rows shown, in their order.
fn names(app: &App) -> Vec<String> {
    let doc = app.document.as_ref().expect("a drawing");
    app.point_rows(doc)
        .shown
        .iter()
        .map(|&s| match doc.model.get(s) {
            Some(kentos_contracts::Entity::Point(p)) => p.base.label.clone().unwrap_or_default(),
            _ => String::new(),
        })
        .collect()
}

fn ev(app: &mut App, e: Event) {
    let _ = app.update(Message::Points(e));
}

#[test]
fn the_columns_are_the_adrs_sira_the_drawings_order() {
    let got: Vec<(&str, Option<&str>)> = COLUMNS.iter().map(|(l, s, _)| (*l, *s)).collect();
    assert_eq!(
        got,
        [
            ("Sıra", None),
            ("Ad", Some("name")),
            ("Y (sağa)", Some("east")),
            ("X (yukarı)", Some("north")),
            ("Z (kot)", Some("z")),
            ("Kod", Some("code")),
            ("Katman", Some("layer")),
        ]
    );
}

#[test]
fn a_column_sorts_ascending_then_descending_then_in_the_drawings_order() {
    let (mut sort, mut desc) = (None, false);
    (sort, desc) = next_sort(sort, desc, Some("name"));
    assert_eq!((sort, desc), (Some("name"), false));
    (sort, desc) = next_sort(sort, desc, Some("name"));
    assert_eq!((sort, desc), (Some("name"), true));
    (sort, desc) = next_sort(sort, desc, Some("name"));
    assert_eq!((sort, desc), (None, false));
    assert_eq!(next_sort(Some("name"), true, Some("z")), (Some("z"), false));
    assert_eq!(next_sort(Some("z"), false, None), (None, false));
}

#[test]
fn a_click_selects_the_row_ctrl_turns_one_over_shift_takes_a_run() {
    let s = |v: &[u32]| v.iter().map(|&i| Slot(i)).collect::<Vec<_>>();
    let shown = s(&[11, 12, 13, 14, 15]);
    assert_eq!(
        click_pick(&s(&[99]), &shown, 2, None, false, false),
        s(&[13])
    );
    assert_eq!(
        click_pick(&s(&[99, 13]), &shown, 2, Some(0), true, false),
        s(&[99])
    );
    assert_eq!(
        click_pick(&s(&[99]), &shown, 2, Some(0), true, false),
        s(&[99, 13])
    );
    assert_eq!(
        click_pick(&s(&[12]), &shown, 3, Some(1), false, true),
        s(&[12, 13, 14])
    );
    assert_eq!(
        click_pick(&s(&[15]), &shown, 1, Some(4), false, true),
        s(&[12, 13, 14, 15])
    );
    assert_eq!(click_pick(&[], &shown, 3, None, false, true), s(&[14]));
}

#[test]
fn a_point_reads_its_label_place_elevation_kod_layer_and_selection() {
    let p = kentos_contracts::PointEntity {
        base: kentos_contracts::EntityBase {
            id: 4,
            layer_id: "nokta".into(),
            color: None,
            attrs: [
                ("Kod".to_owned(), "SN".to_owned()),
                ("Tür".to_owned(), "x".to_owned()),
            ]
            .into(),
            label: Some("101".into()),
            symbol: None,
            line_weight: None,
        },
        p: kentos_contracts::Vec2 {
            x: 487001.5,
            y: 4420002.25,
        },
        z: Some(100.5),
    };
    let row = row_of(&p, "Nokta", true);
    assert_eq!(
        (
            row.name.as_deref(),
            row.east,
            row.north,
            row.z,
            row.code.as_deref(),
            row.layer.as_str(),
            row.selected
        ),
        (
            Some("101"),
            487001.5,
            4420002.25,
            Some(100.5),
            Some("SN"),
            "Nokta",
            true
        )
    );
}

#[test]
fn the_tab_lists_every_point_and_follows_its_query() {
    let mut app = app_with_points();
    let _ = app.update(Message::Run("point.editor"));
    assert!(app.command_expanded);
    assert_eq!(app.bottom_tab, BottomTab::Points);
    assert_eq!(names(&app).len(), 36);
    // Sorted by Ad: the natural order, unnamed last.
    ev(&mut app, Event::Sort(1));
    let sorted = names(&app);
    assert_eq!(
        &sorted[..8],
        [
            "101", "101/1", "101/4", "101/9", "101/10", "101/11", "101/12", "101/14"
        ]
    );
    assert_eq!(&sorted[34..], ["", ""]);
    // A search, caseless: the names holding “p1”.
    ev(&mut app, Event::Search("p1".into()));
    assert_eq!(names(&app), ["P13", "P15", "P16", "P18"]);
    // One layer; only the selected (none).
    ev(&mut app, Event::Search(String::new()));
    ev(&mut app, Event::Layer(Some("Kot".into())));
    assert_eq!(names(&app).len(), 17);
    ev(&mut app, Event::OnlySelected(true));
    assert!(names(&app).is_empty());
    ev(&mut app, Event::OnlySelected(false));
    ev(&mut app, Event::Layer(None));
    // Descending, then the drawing's order again.
    ev(&mut app, Event::Sort(1));
    assert_eq!(names(&app)[0], "S9");
    ev(&mut app, Event::Sort(1));
    assert_eq!(&names(&app)[..3], ["101", "102", "103"]);
}

#[test]
fn a_click_selects_the_point_and_a_double_click_on_its_number_zooms_to_it() {
    let mut app = app_with_points();
    let _ = app.update(Message::Run("point.editor"));
    let shown = {
        let doc = app.document.as_ref().expect("a drawing");
        app.point_rows(doc).shown
    };
    ev(&mut app, Event::Press(2));
    assert_eq!(app.selection.ids(), [shown[2]]);
    app.modifiers = iced::keyboard::Modifiers::SHIFT;
    ev(&mut app, Event::Press(4));
    assert_eq!(app.selection.ids(), &shown[2..=4]);
    app.modifiers = iced::keyboard::Modifiers::CTRL;
    ev(&mut app, Event::Press(3));
    assert_eq!(app.selection.ids(), [shown[2], shown[4]]);
    app.modifiers = iced::keyboard::Modifiers::empty();
    // The drawing's selection shows: its first row is the one revealed.
    {
        let doc = app.document.as_ref().expect("a drawing");
        assert_eq!(app.point_rows(doc).first_selected, Some(2));
    }
    let before = app.viewport.camera.center;
    ev(&mut app, Event::Zoom(10));
    assert_eq!(app.selection.ids(), [shown[10]]);
    assert_ne!(app.viewport.camera.center, before, "zoomed to the point");
}

/// Pictures of Noktalar for the owner, the scenes the web's
/// `node apps/web/scripts/e2e/shots.mjs pointeditor` takes: three points
/// selected in the drawing; sorted by Ad; a search; one layer. Dark and
/// light, at 1440×900 and 1100×650; `.run/shots/noktalar-*`:
///
/// ```text
/// cargo test -p kentos-desktop points::tests::screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/shots");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for (mode, suffix) in [("dark", ""), ("light", "-acik")] {
        for (width, height) in [(1440.0, 900.0), (1100.0, 650.0)] {
            for name in [
                "noktalar",
                "noktalar-ad-sirali",
                "noktalar-ara",
                "noktalar-katman",
            ] {
                let mut app = app_with_points();
                let _ = app
                    .settings
                    .choose(&[("appearance.theme", serde_json::Value::from(mode))]);
                app.apply_settings();
                let _ = app.update(Message::Run("point.editor"));
                let _ = app.update(Message::BottomResized(300.0));
                app.selection.set([Slot(4), Slot(5), Slot(6)]);
                match name {
                    "noktalar-ad-sirali" => ev(&mut app, Event::Sort(1)),
                    "noktalar-ara" => {
                        ev(&mut app, Event::Sort(1));
                        ev(&mut app, Event::Search("p1".into()));
                    }
                    "noktalar-katman" => ev(&mut app, Event::Layer(Some("Kot".into()))),
                    _ => {}
                }
                let mut snapshot = Snapshot::new(Size::new(width, height)).expect("a renderer");
                let mut update = |app: &mut App, message| {
                    let _ = app.update(message);
                };
                snapshot.settle(&mut app, App::view, &mut update);
                let file = out.join(format!("{name}-{width}x{height}{suffix}.png"));
                snapshot
                    .render(app.view(), &app.theme())
                    .save(&file)
                    .expect("writes the picture");
                println!("{}", file.display());
            }
        }
    }
}

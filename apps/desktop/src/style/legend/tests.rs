//! Lejant as the user drives it: the drawing's layers and their rows, the
//! choices, layers left out, the count, the picture's size and paper, and
//! the limit of a legend saved as PNG.

use kentos_native_style::legend::{LegendEntry, LegendGroup, legend_layout, texts};
use serde_json::json;

use super::sheet::{MAX_HEIGHT, Sheet, max_lines, png};
use super::{Event, Line, lines};
use crate::app::{App, Dialog, Message};
use crate::files_testing::{app_with_drawing, last_said};

fn lg(app: &mut App, e: Event) {
    let _ = app.update(Message::Legend(e));
}

fn window(app: &App) -> &super::LegendWindow {
    app.styles.legend.as_ref().expect("the window is open")
}

fn groups(app: &App) -> Vec<LegendGroup> {
    let doc = &app.document.as_ref().expect("open").model;
    window(app).groups(doc, &app.styles.library).to_vec()
}

#[test]
fn lists_the_drawing_s_layers_and_leaves_them_out() {
    let (mut app, _) = App::boot(None);
    let _ = app.update(Message::Run("style.legend"));
    assert_eq!(app.dialog, None, "no drawing, no legend");
    assert_eq!(last_said(&app), "Açık çizim yok.");

    let mut app = app_with_drawing();
    assert!(app.available("style.legend"));
    let _ = app.update(Message::Run("style.legend"));
    assert_eq!(app.dialog, Some(Dialog::Legend));
    let all = groups(&app);
    assert!(!all.is_empty());
    let rows: usize = all.iter().map(|g| g.entries.len()).sum();
    assert_eq!(window(&app).rows(&all), rows);
    // A layer left out leaves the count, and comes back.
    let first = all[0].layer_id.clone();
    lg(&mut app, Event::Layer(first.clone(), false));
    assert_eq!(window(&app).rows(&all), rows - all[0].entries.len());
    assert!(window(&app).shown(&all).iter().all(|g| g.layer_id != first));
    lg(&mut app, Event::Layer(first, true));
    assert_eq!(window(&app).rows(&all), rows);

    // A hidden layer shows only when every layer is asked for.
    let hidden = all[0].layer_id.clone();
    app.document
        .as_mut()
        .expect("open")
        .model
        .toggle_layer_visible(&hidden);
    assert!(groups(&app).iter().all(|g| g.layer_id != hidden));
    lg(&mut app, Event::VisibleOnly(false));
    assert!(groups(&app).iter().any(|g| g.layer_id == hidden));

    lg(&mut app, Event::Headings(false));
    assert!(!window(&app).headings);
    // Esc closes it.
    app.close_dialog();
    assert_eq!(app.dialog, None);
    assert!(app.styles.legend.is_none());
}

#[test]
fn the_list_is_headings_then_rows() {
    let group = |id: &str, n: usize| LegendGroup {
        layer_id: id.to_owned(),
        layer_name: id.to_owned(),
        entries: (0..n)
            .map(|i| LegendEntry {
                label: format!("{id}{i}"),
                symbol: None,
                px_per_mm: None,
            })
            .collect(),
    };
    assert_eq!(
        lines(&[group("a", 2), group("b", 1)]),
        vec![
            Line::Head(0),
            Line::Entry(0, 0),
            Line::Entry(0, 1),
            Line::Head(1),
            Line::Entry(1, 0)
        ]
    );
}

#[test]
fn the_picture_is_the_layout_s_on_white_paper() {
    let app = app_with_drawing();
    let groups = vec![LegendGroup {
        layer_id: "a".into(),
        layer_name: "Yapılar".into(),
        entries: vec![
            LegendEntry {
                label: "Konut".into(),
                symbol: Some(
                    json!({ "type": "fill", "layers": [{ "id": "f", "type": "simpleFill", "color": "#E15759" }] }),
                ),
                px_per_mm: None,
            },
            LegendEntry {
                label: "Resimsiz".into(),
                symbol: None,
                px_per_mm: None,
            },
        ],
    }];
    let layout = legend_layout(&groups, true, "Ada 101");
    let (w, h) = (layout.width * 2, layout.height * 2);
    let sheet = Sheet {
        symbols: groups
            .iter()
            .flat_map(|g| g.entries.iter().map(|e| e.symbol.clone()))
            .collect(),
        scales: vec![None; 2],
        layout,
        library: app.styles.library.clone(),
        images: app.styles.images.clone(),
    };
    let bytes = png(&sheet).expect("a picture");
    let decoder = png::Decoder::new(std::io::Cursor::new(bytes));
    let mut reader = decoder.read_info().expect("a PNG");
    let mut buf = vec![0; reader.output_buffer_size().expect("a size")];
    let info = reader.next_frame(&mut buf).expect("its pixels");
    assert_eq!((i64::from(info.width), i64::from(info.height)), (w, h));
    // White paper in the corner; ink where “LEJANT” is written.
    assert_eq!(&buf[..4], &[255, 255, 255, 255]);
    let row = |y: usize| &buf[y * info.width as usize * 4..(y + 1) * info.width as usize * 4];
    let inked = (40..70).any(|y| row(y).chunks(4).take(200).any(|p| p[0] < 128));
    assert!(inked, "the heading is written in black");
}

#[test]
fn a_legend_saved_as_png_has_a_limit() {
    assert_eq!(max_lines(), 543);
    let many = |n: usize| LegendGroup {
        layer_id: "a".into(),
        layer_name: "A".into(),
        entries: (0..n)
            .map(|i| LegendEntry {
                label: i.to_string(),
                symbol: None,
                px_per_mm: None,
            })
            .collect(),
    };
    assert!(legend_layout(&[many(542)], true, "").height <= MAX_HEIGHT);
    assert!(legend_layout(&[many(543)], true, "").height > MAX_HEIGHT);
    assert_eq!(texts::rows(3), "3 satır");
}

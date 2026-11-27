//! Etiketler on a data layer and its example in the style (docs/adr/9186): the rows are in their source, so the window
//! reads the table's first rows for its examples (off the interface's thread; while read it says so), Denetle counts the
//! labels on the rows the view read (before it holds them it says so, the examples' rows giving the candidates), and the
//! first text is drawn as the map draws it, the current style's look.

use crate::app::{App, Message};
use crate::data_layers_tests::parcels;
use crate::files_testing::{app_with_drawing, drive};
use crate::gis;
use crate::labelling::{Event, PREVIEW, form::Key};

fn lines(app: &App) -> Vec<String> {
    app.gis.labelling.as_ref().expect("the window").lines.iter().map(|(_, l)| l.clone()).collect()
}

fn send(app: &mut App, e: Event) {
    let task = app.update(Message::Gis(gis::Event::Labelling(e)));
    drive(app, task);
}

/// A CBS drawing with a GeoPackage's six parcels as a data layer (Konut, Park … by turns), the view over them.
fn bound(tag: &str) -> (App, String) {
    crate::data_layers::register();
    let path = parcels(tag, 6);
    let mut app = app_with_drawing();
    let id = app
        .add_data_layers(vec![crate::data_layers::describe(&path.display().to_string(), "parseller").expect("described")])
        .expect("added")
        .remove(0);
    let area = iced::Rectangle::new(iced::Point::ORIGIN, iced::Size::new(1000.0, 700.0));
    let _ = app.update(Message::Viewport(crate::viewport::Event::Resized(area)));
    app.viewport.camera.center = kentos_render_wgpu::Vec2::new(486_590.0, 4_420_235.0);
    app.viewport.camera.scale = 4.0;
    (app, id)
}

#[test]
fn a_data_layers_examples_come_from_its_sources_first_rows() {
    let (mut app, id) = bound("etiket-ornek");
    // Opened: the rows asked, said while read.
    let task = app.open_labelling(Some(id.clone()));
    let said = lines(&app);
    assert!(said.iter().any(|l| l.contains("veri katmanı, satırları kaynağında")), "{said:#?}");
    // The layer labels nothing until its labelling is turned on (docs/adr/9311): it opens on Yok; Tek etiket.
    send(&mut app, Event::Mode(kentos_commands::LabelsMode::Single));
    let said = lines(&app);
    assert!(said.iter().any(|l| l == "Örnekler için kaynağın ilk satırları okunuyor…"), "{said:#?}");
    drive(&mut app, task);
    send(&mut app, Event::Type(Key::Template, "{kullanim}".into()));
    let said = lines(&app);
    let has = |words: &str| said.iter().any(|l| l.contains(words));
    assert!(has("Örnekler tablonun ilk 6 satırından."), "{said:#?}");
    assert!(has("Örnek metin (ilk nesne): “Konut”."), "{said:#?}");
    assert!(!has("nesne,"), "a data layer is not counted as the drawing's objects: {said:#?}");
    let w = app.gis.labelling.as_ref().expect("the window");
    assert_eq!(w.sample.as_deref(), Some("Konut"));
    // A field the table has none of is said as before; the table's own are the picker's.
    assert!(w.fields.iter().any(|f| f == "kullanim") && w.fields.iter().any(|f| f == "ada"));
    send(&mut app, Event::Type(Key::Template, "{malik}".into()));
    assert!(lines(&app).iter().any(|l| l.contains("malik")), "{:#?}", lines(&app));
}

#[test]
fn denetle_counts_a_data_layers_labels_on_the_rows_the_view_read() {
    let (mut app, id) = bound("etiket-denetle");
    let task = app.open_labelling(Some(id.clone()));
    drive(&mut app, task);
    // The layer labels nothing until its labelling is turned on (docs/adr/9311): Tek etiket.
    send(&mut app, Event::Mode(kentos_commands::LabelsMode::Single));
    send(&mut app, Event::Type(Key::Template, "{kullanim}".into()));
    // No read of the view yet: said so, the examples' rows giving the candidates.
    send(&mut app, Event::Report);
    let said = lines(&app);
    assert!(said[0].starts_with("Veri katmanının etiketleri görünümün okuduğu satırlara yerleşir"), "{said:#?}");
    assert!(said.iter().any(|l| l.contains("Konut")), "the candidates' sample: {said:#?}");
    // The view's read held (its rows' document): counted there as the drawing's labels are.
    let rows = match &app.gis.labelling.as_ref().expect("the window").rows {
        Some(Ok(doc)) => (**doc).clone(),
        other => panic!("the rows: {other:?}"),
    };
    app.gis.data_labels.push(crate::data_labels::DataLabels::of(&id, rows));
    send(&mut app, Event::Report);
    let said = lines(&app);
    let placed: usize = said[0]
        .strip_prefix("Görünümde ")
        .and_then(|rest| rest.split(' ').next())
        .and_then(|n| n.parse().ok())
        .unwrap_or_else(|| panic!("{said:#?}"));
    assert!(placed > 0, "{said:#?}");
}

#[test]
fn the_first_text_is_drawn_in_the_styles_look() {
    let (mut app, id) = bound("etiket-onizleme");
    let task = app.open_labelling(Some(id));
    drive(&mut app, task);
    // The layer labels nothing until its labelling is turned on (docs/adr/9311): Tek etiket.
    send(&mut app, Event::Mode(kentos_commands::LabelsMode::Single));
    send(&mut app, Event::Type(Key::Template, "{kullanim}".into()));
    let mut shot = crate::files_testing::offscreen(iced::Size::new(1440.0, 900.0));
    let mut update = |app: &mut App, message: Message| {
        let _ = app.update(message);
    };
    shot.settle(&mut app, App::view, &mut update);
    assert!(crate::files_testing::find_text(&mut shot, &app, PREVIEW).is_some(), "the example's strip");
    // Yok: no label, no example.
    send(&mut app, Event::Mode(kentos_commands::LabelsMode::Off));
    shot.settle(&mut app, App::view, &mut update);
    assert!(crate::files_testing::find_text(&mut shot, &app, PREVIEW).is_none());
}

/// Pictures for the owner (`.run/gui-evidence/kullanim-9186/`): Etiketler on the data layer with its first row's text
/// drawn in the style, and with a halo and a background.
#[test]
#[ignore = "pictures for the owner"]
fn screens() {
    let prepared = |halo: bool| {
        move || {
            let (mut app, id) = bound(if halo { "etiket-resim-bicim" } else { "etiket-resim" });
            let task = app.open_labelling(Some(id));
            drive(&mut app, task);
            send(&mut app, Event::Type(Key::Template, "{kullanim} · ada {ada}".into()));
            if halo {
                // A larger, bold label on a round background, with a shadow: the strip draws it so.
                send(&mut app, Event::Type(Key::Size, "16".into()));
                send(&mut app, Event::Edit(crate::labelling::form::Edit::Bold(true)));
                send(&mut app, Event::Edit(crate::labelling::form::Edit::Background(Some(
                    kentos_commands::LabelShape::Round,
                ))));
                send(&mut app, Event::Edit(crate::labelling::form::Edit::Shadow(true)));
                send(&mut app, Event::Tab(crate::labelling::Tab::Look));
            }
            app
        }
    };
    crate::files_testing::usage_shots("9186", "1-veri-katmani-ornek", prepared(false));
    crate::files_testing::usage_shots("9186", "2-bicim", prepared(true));
}

/// The label tools on a data layer's label (docs/adr/9341; the CBS review of 27 November 2026, item 10): the label is
/// found under the click, and the tool's “click a label” is followed by why a data layer's label is not taken alone and
/// what to use.
#[test]
fn a_label_tools_click_on_a_data_label_says_why_and_what_to_use() {
    let (mut app, id) = bound("etiket-arac");
    let task = app.open_labelling(Some(id.clone()));
    drive(&mut app, task);
    send(&mut app, Event::Mode(kentos_commands::LabelsMode::Single));
    send(&mut app, Event::Type(Key::Template, "{kullanim}".into()));
    let rows = match &app.gis.labelling.as_ref().expect("the window").rows {
        Some(Ok(doc)) => (**doc).clone(),
        other => panic!("the rows: {other:?}"),
    };
    send(&mut app, Event::Save);
    app.gis.data_labels.push(crate::data_labels::DataLabels::of(&id, rows));
    // A parcel's middle, where its label lies.
    let at = *app.gis.data_labels[0].row_points().first().expect("a parcel");
    let name = app.document.as_ref().and_then(|d| d.model.layers().get(&id).map(|n| n.name.clone())).expect("the layer");
    assert_eq!(app.data_label_under(at, app.draft.pick_aperture), Some(name.clone()));
    for tool in ["tool.labelMove", "tool.labelPin", "tool.labelHide"] {
        let task = app.update(Message::Run(tool));
        drive(&mut app, task);
        let [sx, sy] = app.viewport.camera.world_to_screen(at);
        let p = iced::Point::new(sx as f32, sy as f32);
        for ev in [crate::viewport::Event::Moved(p), crate::viewport::Event::Pressed(p), crate::viewport::Event::Released(p)] {
            let task = app.update(Message::Viewport(ev));
            drive(&mut app, task);
        }
        let said = crate::files_testing::last_said(&app);
        assert_eq!(said, App::data_label_words(&name), "{tool}");
        assert!(said.contains("Etiketler › Yerleşim"), "{said}");
    }
}

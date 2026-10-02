//! The SVG editor as the user drives it, without a window: opening it from
//! the command, Stil yöneticisi and Sembol tasarımcısı; drawing with the
//! pointer (snapped to the grid), moving, scaling and turning; the keys;
//! one undo step per change; path and node operations; guides from the
//! rulers; the measure's readout; import, export, document properties and
//! the XML source; tracing a picture; saving to Kitaplığım (a system
//! drawing as the user's copy) and the question on closing.

use iced::keyboard::{Key, Modifiers, key::Named};
use kentos_native_style::library::{ItemKind, Source};

use super::actions::{Action, PathOp};
use super::files::{self, FileCmd, FileDialog};
use super::hit::Hit;
use super::stage::{Button, Input};
use super::state::{After, Opening, Question, SvgEditor};
use super::{Event, ToolId};
use crate::app::{App, Dialog, Message};
use crate::files_testing::{app_with_drawing, scratch};

fn send(app: &mut App, e: Event) {
    let _ = app.update(Message::SvgEdit(Box::new(e)));
}

fn ed(app: &App) -> &SvgEditor {
    app.styles.svg_editor.as_ref().expect("the editor is open")
}

fn ed_mut(app: &mut App) -> &mut SvgEditor {
    app.styles.svg_editor.as_mut().expect("the editor is open")
}

/// An app with Kitaplığım in a scratch folder and the editor open on a new drawing, sized 600 × 500.
fn open(name: &str) -> (App, std::path::PathBuf) {
    let mut app = app_with_drawing();
    let dir = scratch(name);
    assert_eq!(app.styles.open_user_library(&dir), None);
    let _ = app.update(Message::Run("style.svgEditor"));
    assert_eq!(app.dialog, Some(Dialog::SvgEditor));
    send(&mut app, Event::Stage(Input::Resized(600.0, 500.0)));
    (app, dir)
}

/// A drawing point on the screen.
fn screen(app: &App, p: [f64; 2]) -> [f64; 2] {
    ed(app).camera.to_screen(p)
}

fn drag(app: &mut App, from: [f64; 2], to: [f64; 2], shift: bool) {
    let (a, b) = (screen(app, from), screen(app, to));
    send(
        app,
        Event::Stage(Input::Down {
            s: a,
            button: Button::Left,
            shift,
            space: false,
        }),
    );
    let mid = [(a[0] + b[0]) / 2.0, (a[1] + b[1]) / 2.0];
    send(
        app,
        Event::Stage(Input::Move {
            s: mid,
            shift,
            alt: false,
        }),
    );
    send(
        app,
        Event::Stage(Input::Move {
            s: b,
            shift,
            alt: false,
        }),
    );
    send(app, Event::Stage(Input::Up { alt: false }));
}

fn click(app: &mut App, p: [f64; 2], shift: bool) {
    let s = screen(app, p);
    send(
        app,
        Event::Stage(Input::Down {
            s,
            button: Button::Left,
            shift,
            space: false,
        }),
    );
    send(app, Event::Stage(Input::Up { alt: false }));
}

fn key(app: &mut App, key: Key, modifiers: Modifiers, text: Option<&str>) {
    use iced::keyboard::key::{NativeCode, Physical};
    let press = crate::keys::KeyPress {
        key,
        physical: Physical::Unidentified(NativeCode::Unidentified),
        modifiers,
        text: text.map(str::to_owned),
        repeat: false,
    };
    // As the focus query answers when no field holds the keyboard.
    if press.named() == Some(Named::Escape) {
        let _ = app.update(Message::Key(press));
    } else {
        send(
            app,
            Event::Key(press, crate::style::designer::Focus::default()),
        );
    }
}

fn letter(app: &mut App, c: &str) {
    key(app, Key::Character(c.into()), Modifiers::empty(), Some(c));
}

fn ctrl(app: &mut App, c: &str) {
    key(app, Key::Character(c.into()), Modifiers::CTRL, None);
}

#[test]
fn a_rectangle_dragged_out_snaps_to_the_grid_and_is_one_undo_step() {
    let (mut app, _dir) = open("dikdortgen");
    assert_eq!(ed(&app).options.grid, 5.0);
    letter(&mut app, "r");
    assert_eq!(ed(&app).tool, ToolId::Rect);
    drag(&mut app, [10.4, 9.8], [44.6, 40.3], false);
    let e = ed(&app);
    assert_eq!(e.doc.shapes.len(), 1);
    let s = &e.doc.shapes[0];
    assert_eq!(s.kind(), "rect");
    assert_eq!(
        (s.num("x"), s.num("y"), s.num("w"), s.num("h")),
        (10.0, 10.0, 35.0, 30.0)
    );
    // Fields in the web's order (the history and the unsaved check compare texts).
    let keys: Vec<&str> = s.0.iter().map(|(k, _)| k.as_str()).collect();
    assert_eq!(
        keys,
        [
            "id",
            "fill",
            "stroke",
            "strokeWidth",
            "kind",
            "x",
            "y",
            "w",
            "h"
        ]
    );
    assert_eq!(e.tool, ToolId::Select, "back to Seç after a shape");
    assert_eq!(e.selection.len(), 1);
    assert!(e.dirty());
    assert!(e.title().ends_with('•'));
    ctrl(&mut app, "z");
    assert!(ed(&app).doc.shapes.is_empty());
    ctrl(&mut app, "y");
    assert_eq!(ed(&app).doc.shapes.len(), 1);
}

#[test]
fn shapes_are_picked_moved_scaled_and_turned_with_the_pointer() {
    let (mut app, _dir) = open("tasi");
    {
        let e = ed_mut(&mut app);
        e.set_tool(ToolId::Rect);
        e.place([10.0, 10.0], [30.0, 30.0], false);
    }
    let id = ed(&app).selection[0].clone();
    // A click on empty paper clears the choice, a click on the shape chooses it.
    click(&mut app, [80.0, 80.0], false);
    assert!(ed(&app).selection.is_empty());
    click(&mut app, [20.0, 20.0], false);
    assert_eq!(ed(&app).selection, vec![id.clone()]);
    // A drag moves it, snapped to the grid, as one step.
    drag(&mut app, [20.0, 20.0], [31.0, 26.0], false);
    let s = ed(&app).doc.shape(&id).expect("the shape").clone();
    assert_eq!((s.num("x"), s.num("y")), (20.0, 15.0));
    // The bottom-right handle scales it.
    let [a, c] = [[20.0, 15.0], [40.0, 35.0]];
    let (sa, sc) = (screen(&app, a), screen(&app, c));
    assert_eq!(ed(&app).hit_at(sc), Hit::Handle(4));
    assert_eq!(
        ed(&app).hit_at([(sa[0] + sc[0]) / 2.0, sa[1] - 26.0]),
        Hit::Rot
    );
    drag(&mut app, c, [50.0, 45.0], false);
    let s = ed(&app).doc.shape(&id).expect("the shape").clone();
    assert_eq!((s.num("w"), s.num("h")), (30.0, 30.0));
    // Esc during a turn puts it back.
    let knob = [(sa[0] + screen(&app, [50.0, 45.0])[0]) / 2.0, sa[1] - 26.0];
    send(
        &mut app,
        Event::Stage(Input::Down {
            s: knob,
            button: Button::Left,
            shift: false,
            space: false,
        }),
    );
    send(
        &mut app,
        Event::Stage(Input::Move {
            s: [knob[0] + 80.0, knob[1] + 60.0],
            shift: false,
            alt: false,
        }),
    );
    assert!(ed(&app).doc.shape(&id).expect("the shape").is("rotate"));
    key(
        &mut app,
        Key::Named(Named::Escape),
        Modifiers::empty(),
        None,
    );
    assert!(!ed(&app).doc.shape(&id).expect("the shape").is("rotate"));
    assert_eq!(app.dialog, Some(Dialog::SvgEditor));
}

#[test]
fn arrows_nudge_by_the_grid_and_delete_keeps_a_locked_shape() {
    let (mut app, _dir) = open("oklar");
    {
        let e = ed_mut(&mut app);
        e.set_tool(ToolId::Rect);
        e.place([10.0, 10.0], [30.0, 30.0], false);
        e.set_tool(ToolId::Ellipse);
        e.place([50.0, 50.0], [70.0, 70.0], false);
    }
    ctrl(&mut app, "a");
    assert_eq!(ed(&app).selection.len(), 2);
    key(
        &mut app,
        Key::Named(Named::ArrowRight),
        Modifiers::empty(),
        None,
    );
    key(
        &mut app,
        Key::Named(Named::ArrowDown),
        Modifiers::SHIFT,
        None,
    );
    let rect = ed(&app).doc.shapes[0].clone();
    assert_eq!((rect.num("x"), rect.num("y")), (15.0, 35.0));
    // Nudges within a second are one step.
    ctrl(&mut app, "z");
    assert_eq!(ed(&app).doc.shapes[0].num("x"), 10.0);
    // A locked shape stays when the chosen ones are deleted.
    let first = super::doc::id_of(&ed(&app).doc.shapes[0]).to_owned();
    ed_mut(&mut app).flip_flag(&first, "locked");
    ctrl(&mut app, "a");
    assert_eq!(ed(&app).selection.len(), 1, "a locked shape is not chosen");
    let second = super::doc::id_of(&ed(&app).doc.shapes[1]).to_owned();
    ed_mut(&mut app).select(vec![first.clone(), second]);
    key(
        &mut app,
        Key::Named(Named::Delete),
        Modifiers::empty(),
        None,
    );
    assert_eq!(ed(&app).doc.shapes.len(), 1);
    assert_eq!(super::doc::id_of(&ed(&app).doc.shapes[0]), first);
}

#[test]
fn a_polyline_is_clicked_node_by_node_and_ends_on_enter() {
    let (mut app, _dir) = open("cizgi");
    letter(&mut app, "l");
    for p in [[10.0, 10.0], [50.0, 10.0], [50.0, 50.0]] {
        click(&mut app, p, false);
        // Clicks far apart in time are not a double click.
        ed_mut(&mut app).last_click = None;
    }
    assert_eq!(ed(&app).draw.draft.len(), 3);
    key(&mut app, Key::Named(Named::Enter), Modifiers::empty(), None);
    let e = ed(&app);
    assert_eq!(e.doc.shapes.len(), 1);
    let s = &e.doc.shapes[0];
    assert_eq!(
        (s.text("fill"), s.text("stroke")),
        (Some("none"), Some("fill"))
    );
    assert_eq!(s.num("strokeWidth"), 4.0);
    let subs = s.subs().expect("sub-paths");
    assert!(!subs[0].closed);
    assert_eq!(subs[0].nodes.len(), 3);
    // Esc gives up a draft first, then the choice, then asks to close.
    for p in [[20.0, 70.0], [40.0, 80.0]] {
        click(&mut app, p, false);
        ed_mut(&mut app).last_click = None;
    }
    key(
        &mut app,
        Key::Named(Named::Escape),
        Modifiers::empty(),
        None,
    );
    assert!(!ed(&app).draw.drafting());
    assert_eq!(ed(&app).selection.len(), 1);
    key(
        &mut app,
        Key::Named(Named::Escape),
        Modifiers::empty(),
        None,
    );
    assert!(ed(&app).selection.is_empty());
    key(
        &mut app,
        Key::Named(Named::Escape),
        Modifiers::empty(),
        None,
    );
    assert!(matches!(ed(&app).question, Some(Question::Close)));
    send(&mut app, Event::Stay);
    assert!(ed(&app).question.is_none());
}

#[test]
fn path_operations_join_and_the_node_tool_edits_nodes() {
    let (mut app, _dir) = open("yol");
    {
        let e = ed_mut(&mut app);
        e.set_tool(ToolId::Rect);
        e.place([10.0, 10.0], [50.0, 50.0], false);
        e.set_tool(ToolId::Rect);
        e.place([30.0, 30.0], [70.0, 70.0], false);
        e.select_all();
        e.path_op(PathOp::Union);
    }
    let e = ed(&app);
    assert_eq!(e.doc.shapes.len(), 1);
    assert_eq!(e.doc.shapes[0].kind(), "path");
    assert_eq!(e.said.as_ref().map(|s| s.0.as_str()), Some("Birleşim."));
    let id = e.selection[0].clone();
    // Node editing: a double click on a node makes it smooth.
    ed_mut(&mut app).edit_nodes(Some(id.clone()));
    let corner = [10.0, 10.0];
    let s = screen(&app, corner);
    assert!(matches!(
        ed(&app).hit_at(s),
        Hit::Node(_, _, super::node_tool::Part::Node)
    ));
    click(&mut app, corner, false);
    click(&mut app, corner, false);
    let subs = ed(&app)
        .doc
        .shape(&id)
        .expect("the path")
        .subs()
        .expect("sub-paths");
    let n = subs[0]
        .nodes
        .iter()
        .find(|n| (n.x, n.y) == (10.0, 10.0))
        .expect("the corner");
    assert_eq!(n.ty.as_deref(), Some("smooth"));
    // Ctrl+Z takes it back as one step.
    ctrl(&mut app, "z");
    let subs = ed(&app)
        .doc
        .shape(&id)
        .expect("the path")
        .subs()
        .expect("sub-paths");
    assert!(subs[0].nodes.iter().all(|n| n.in_.is_none()));
}

#[test]
fn a_guide_comes_from_the_ruler_and_goes_back_to_it() {
    let (mut app, _dir) = open("kilavuz");
    let r = super::camera::ruler();
    let at = [300.0, r / 2.0];
    assert_eq!(ed(&app).hit_at(at), Hit::Ruler(super::hit::Axis::H));
    send(
        &mut app,
        Event::Stage(Input::Down {
            s: at,
            button: Button::Left,
            shift: false,
            space: false,
        }),
    );
    let below = screen(&app, [50.0, 42.0]);
    send(
        &mut app,
        Event::Stage(Input::Move {
            s: below,
            shift: false,
            alt: false,
        }),
    );
    send(&mut app, Event::Stage(Input::Up { alt: false }));
    let g = ed(&app).doc.guides.first().cloned().expect("a guide");
    assert_eq!((g.angle, g.y), (0.0, 40.0));
    assert!(ed(&app).can_undo());
    // Dragged back onto the ruler it goes.
    let on = screen(&app, [50.0, 40.0]);
    send(
        &mut app,
        Event::Stage(Input::Down {
            s: on,
            button: Button::Left,
            shift: false,
            space: false,
        }),
    );
    send(
        &mut app,
        Event::Stage(Input::Move {
            s: [on[0], r / 2.0],
            shift: false,
            alt: false,
        }),
    );
    send(&mut app, Event::Stage(Input::Up { alt: false }));
    assert!(ed(&app).doc.guides.is_empty());
}

#[test]
fn the_measure_reads_as_the_drawing_counts() {
    let (main, more) = super::measure::readout([0.0, 0.0], [30.0, -40.0], 100.0, Some(24.0));
    assert_eq!(main, "50 birim · 12 mm · açı -53.13°");
    assert_eq!(more, "ΔX 30, ΔY -40");
    let (main, _) = super::measure::readout([0.0, 0.0], [0.0, 10.0], 100.0, None);
    assert_eq!(main, "10 birim · açı 90°");
}

#[test]
fn kaydet_writes_the_drawing_to_kitapligim_and_a_system_drawing_as_a_copy() {
    let (mut app, _dir) = open("kaydet");
    // Nothing drawn: Kaydet refuses and says why.
    send(&mut app, Event::Save);
    assert_eq!(
        ed(&app).said.as_ref().map(|s| s.0.as_str()),
        Some("Boş çizim kaydedilmez: önce bir şekil çizin.")
    );
    {
        let e = ed_mut(&mut app);
        e.set_tool(ToolId::Ellipse);
        e.place([20.0, 20.0], [80.0, 80.0], false);
    }
    send(&mut app, Event::Name("Daire".into()));
    send(&mut app, Event::Save);
    let e = ed(&app);
    assert!(!e.dirty());
    let id = e.original.as_ref().expect("saved").id.clone();
    let (item, source) = app.styles.library.get(&id).expect("in the library");
    assert_eq!(source, Source::User);
    assert_eq!(item.kind(), ItemKind::Asset);
    assert_eq!(item.name(), "Daire");
    assert_eq!(item.path(), vec!["Çizimlerim"]);
    let data = item.data().expect("its SVG");
    assert!(
        data.contains("<ellipse") && data.contains("currentColor"),
        "{data}"
    );
    // A system drawing opens as its copy and is saved as a new user drawing.
    let system = app
        .styles
        .library
        .items(Some(Source::System))
        .into_iter()
        .find(|(i, _)| i.format() == Some("svg"))
        .map(|(i, _)| i.id().to_owned())
        .expect("a system drawing");
    app.styles.svg_editor = None;
    app.open_svg_editor(Opening {
        id: Some(system.clone()),
        path: None,
        after: After::Nothing,
    });
    let e = ed(&app);
    assert!(e.name.ends_with("(kopya)"));
    assert!(!e.original.as_ref().expect("the original").editable);
    ed_mut(&mut app).action(Action::FlipH);
    send(&mut app, Event::Save);
    let saved = ed(&app).original.as_ref().expect("saved").id.clone();
    assert_ne!(saved, system);
    assert_eq!(
        app.styles.library.get(&saved).map(|(_, s)| s),
        Some(Source::User)
    );
}

#[test]
fn closing_with_changes_asks_and_the_answer_is_kept() {
    let (mut app, _dir) = open("soru");
    {
        let e = ed_mut(&mut app);
        e.set_tool(ToolId::Rect);
        e.place([10.0, 10.0], [30.0, 30.0], false);
    }
    send(&mut app, Event::Close);
    assert!(matches!(ed(&app).question, Some(Question::Close)));
    assert_eq!(app.dialog, Some(Dialog::SvgEditor));
    send(&mut app, Event::SaveAndClose);
    assert!(app.styles.svg_editor.is_none());
    assert_eq!(app.dialog, None);
    let saved = app
        .styles
        .library
        .items(Some(Source::User))
        .into_iter()
        .any(|(i, _)| i.name() == "Yeni çizim");
    assert!(saved);
}

#[test]
fn an_svg_is_imported_as_new_or_into_the_drawing_with_its_colours_chosen() {
    let (mut app, _dir) = open("icealim");
    let svg = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 50 50"><rect width="50" height="25" fill="#1d1d1b"/><circle cx="25" cy="37" r="10" fill="#e30613"/></svg>"##;
    files::import::open(ed_mut(&mut app), svg, "iki");
    let Some(FileDialog::Import(d)) = &ed(&app).files.dialog else {
        panic!("the import window");
    };
    assert!(d.new, "an empty drawing opens the file as new");
    assert_eq!(d.colors.len(), 2);
    let mapped = d.mapped();
    assert_eq!(
        mapped.shapes[0].text("fill"),
        Some("fill"),
        "near black is the symbol's colour"
    );
    assert_eq!(mapped.shapes[1].text("fill"), Some("#E30613"));
    // Added to a drawing: fitted into its canvas.
    {
        let e = ed_mut(&mut app);
        e.files.dialog = None;
        e.set_tool(ToolId::Rect);
        e.place([0.0, 0.0], [10.0, 10.0], false);
    }
    files::import::open(ed_mut(&mut app), svg, "iki");
    let Some(FileDialog::Import(d)) = &mut ed_mut(&mut app).files.dialog else {
        panic!("the import window");
    };
    assert!(!d.new);
    d.second = Some("#E30613".into());
    let text = d.mapped();
    assert_eq!(text.shapes[1].text("fill"), Some("stroke"));
    let e = ed_mut(&mut app);
    let d = match e.files.dialog.take() {
        Some(FileDialog::Import(d)) => d,
        _ => panic!("the import window"),
    };
    let ids = e.add_shapes(&d.mapped());
    assert_eq!(ids.len(), 2);
    let r = e.doc.shape(&ids[0]).expect("the rectangle");
    assert_eq!(
        (r.num("w"), r.num("h")),
        (100.0, 50.0),
        "fitted: 50 units become 100"
    );
    assert_eq!(
        r.num("strokeWidth"),
        2.0,
        "the stroke scales with the drawing"
    );
}

#[test]
fn a_dropped_svg_opens_the_import_window_and_a_picture_asks_what_it_is_for() {
    let (mut app, dir) = open("birak");
    let svg = dir.join("isaret.svg");
    std::fs::write(&svg, r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 10 10"><rect width="10" height="10"/></svg>"#)
        .expect("a file");
    send(&mut app, Event::File(files::Event::Dropped(svg)));
    let Some(FileDialog::Import(d)) = &ed(&app).files.dialog else {
        panic!("the import window");
    };
    assert_eq!(d.name, "isaret");
    ed_mut(&mut app).files.dialog = None;
    let png = dir.join("resim.png");
    std::fs::write(&png, super::screens::sample_png()).expect("a file");
    send(&mut app, Event::File(files::Event::Dropped(png)));
    assert!(
        matches!(&ed(&app).files.dialog, Some(FileDialog::Dropped(name, _)) if name == "resim.png")
    );
    // Anything else is refused with the reason.
    let txt = dir.join("not.txt");
    std::fs::write(&txt, "merhaba").expect("a file");
    ed_mut(&mut app).files.dialog = None;
    send(&mut app, Event::File(files::Event::Dropped(txt)));
    assert!(
        ed(&app)
            .said
            .as_ref()
            .is_some_and(|s| s.1 && s.0.contains("SVG, PNG ya da JPEG"))
    );
}

#[test]
fn the_source_is_edited_and_read_back_as_one_step() {
    let (mut app, _dir) = open("kaynak");
    {
        let e = ed_mut(&mut app);
        e.set_tool(ToolId::Rect);
        e.place([10.0, 10.0], [30.0, 30.0], false);
    }
    send(
        &mut app,
        Event::File(files::Event::Cmd(FileCmd::ToggleSource)),
    );
    let text = ed(&app).files.source.as_ref().expect("the source").text();
    assert!(text.contains("<rect"), "{text}");
    // A broken edit names its line.
    {
        let e = ed_mut(&mut app);
        let s = e.files.source.as_mut().expect("the source");
        s.content = iced::widget::text_editor::Content::with_text("<svg><rect></svg>");
        s.edited = true;
    }
    send(
        &mut app,
        Event::Do(super::Change(std::sync::Arc::new(|ed| ed.source_apply()))),
    );
    let note = ed(&app)
        .files
        .source
        .as_ref()
        .and_then(|s| s.note.clone())
        .expect("a note");
    assert!(note.0.starts_with("Satır 1"), "{}", note.0);
    assert!(note.1);
    // A good one changes the drawing, and Ctrl+Z takes it back.
    let good = text.replace("x=\"10\"", "x=\"40\"");
    {
        let e = ed_mut(&mut app);
        let s = e.files.source.as_mut().expect("the source");
        s.content = iced::widget::text_editor::Content::with_text(&good);
        s.edited = true;
    }
    send(
        &mut app,
        Event::Do(super::Change(std::sync::Arc::new(|ed| ed.source_apply()))),
    );
    assert_eq!(ed(&app).doc.shapes[0].num("x"), 40.0);
    ctrl(&mut app, "z");
    assert_eq!(ed(&app).doc.shapes[0].num("x"), 10.0);
}

#[test]
fn the_sources_top_edge_is_dragged_and_a_double_click_gives_the_first_share() {
    use iced::{Point, Size};
    use kentos_ui::snapshot::{Input as Gesture, Snapshot};

    let (mut app, _dir) = open("kaynak-pay");
    send(
        &mut app,
        Event::File(files::Event::Cmd(FileCmd::ToggleSource)),
    );
    let mut snapshot = crate::files_testing::offscreen(Size::new(1440.0, 900.0));
    let mut update = |app: &mut App, message| {
        let _ = app.update(message);
    };
    // Twice: the canvas tells its size, the edge knows the source's height.
    snapshot.settle(&mut app, App::view, &mut update);
    snapshot.settle(&mut app, App::view, &mut update);
    let head = |snapshot: &mut Snapshot, app: &App| {
        crate::point_calc::texts(snapshot, app)
            .into_iter()
            .find(|(t, _)| t == "SVG kaynağı")
            .map(|(_, at)| at)
            .expect("the source's head")
    };
    let first = head(&mut snapshot, &app);
    // The edge lies over the panel's padding and the head's button row.
    let edge = Point::new(first.x + 40.0, first.y - 13.0);
    let up = Point::new(edge.x, edge.y - 120.0);
    snapshot.input(&mut app, App::view, &mut update, Gesture::Drag(edge, up));
    snapshot.settle(&mut app, App::view, &mut update);
    assert!(
        ed(&app)
            .files
            .source
            .as_ref()
            .expect("open")
            .height
            .is_some()
    );
    // The panel grew by the drag, from where it was: the head went up with the edge.
    let grown = head(&mut snapshot, &app);
    assert!(
        (first.y - grown.y - 120.0).abs() <= 3.0,
        "{first:?} → {grown:?}"
    );
    // Nothing of the drawing changed: no undo step.
    assert!(!ed(&app).can_undo());
    // A double click on the edge gives the first share back.
    snapshot.input(&mut app, App::view, &mut update, Gesture::Click(up));
    snapshot.input(&mut app, App::view, &mut update, Gesture::Click(up));
    snapshot.settle(&mut app, App::view, &mut update);
    assert_eq!(ed(&app).files.source.as_ref().expect("open").height, None);
    let back = head(&mut snapshot, &app);
    assert!((back.y - first.y).abs() <= 1.0, "{first:?} → {back:?}");
}

#[test]
fn document_properties_crop_scale_and_carry_the_guides() {
    let (mut app, _dir) = open("belge");
    {
        let e = ed_mut(&mut app);
        e.set_tool(ToolId::Rect);
        e.place([20.0, 20.0], [60.0, 50.0], false);
        e.doc.guides.push(super::doc::GuideLine {
            id: "k".into(),
            x: 40.0,
            y: 0.0,
            angle: 90.0,
        });
    }
    send(&mut app, Event::File(files::Event::Cmd(FileCmd::DocProps)));
    {
        let e = ed_mut(&mut app);
        let Some(FileDialog::DocProps(d)) = &mut e.files.dialog else {
            panic!("the window");
        };
        d.x = 20.0;
        d.y = 20.0;
        d.w = 40.0;
        d.h = 30.0;
        d.scale = 2.0;
        d.mm = 12.0;
    }
    send(
        &mut app,
        Event::Do(super::Change(std::sync::Arc::new(|ed| {
            files::docprops::apply(ed)
        }))),
    );
    let e = ed(&app);
    assert_eq!(
        (e.doc.width, e.doc.height, e.doc.size_mm),
        (80.0, 60.0, Some(12.0))
    );
    let r = &e.doc.shapes[0];
    assert_eq!(
        (r.num("x"), r.num("y"), r.num("w"), r.num("h")),
        (0.0, 0.0, 80.0, 60.0)
    );
    assert_eq!(e.doc.guides[0].x, 40.0, "the guide moved with the drawing");
    assert!(e.can_undo());
}

#[test]
fn a_picture_is_traced_into_paths_in_one_step() {
    let (mut app, _dir) = open("izle");
    let images = app.styles.images.clone();
    files::reference::load(
        ed_mut(&mut app),
        "ornek.png",
        &super::screens::sample_png(),
        &images,
    );
    let r = ed(&app).files.reference.as_ref().expect("a reference");
    assert!(r.spec.locked && !r.keep);
    assert_eq!(r.spec.width, 100.0);
    send(&mut app, Event::File(files::Event::Cmd(FileCmd::Trace)));
    files::trace::run_now(ed_mut(&mut app));
    let Some(FileDialog::Trace(d)) = &ed(&app).files.dialog else {
        panic!("the trace window");
    };
    let result = d.result.clone().expect("traced");
    assert_eq!(result.shapes.len(), 2, "a ring and a bar");
    assert_eq!(result.holes, 1);
    files::trace::add(ed_mut(&mut app));
    let e = ed(&app);
    assert_eq!(e.doc.shapes.len(), 1);
    assert_eq!(e.doc.shapes[0].text("name"), Some("İz"));
    assert!(e.files.dialog.is_none());
}

#[test]
fn stil_yoneticisi_and_the_designer_open_the_editor_and_get_its_drawing() {
    let mut app = app_with_drawing();
    let dir = scratch("acanlar");
    assert_eq!(app.styles.open_user_library(&dir), None);
    let _ = app.update(Message::Run("style.manager"));
    let _ = app.update(Message::StyleManager(Box::new(
        crate::style::manager::Event::NewDrawing,
    )));
    assert_eq!(app.dialog, Some(Dialog::SvgEditor));
    assert_eq!(ed(&app).under, Some(Dialog::StyleManager));
    {
        let e = ed_mut(&mut app);
        e.set_tool(ToolId::Rect);
        e.place([10.0, 10.0], [30.0, 30.0], false);
    }
    send(&mut app, Event::Save);
    let id = ed(&app).original.as_ref().expect("saved").id.clone();
    send(&mut app, Event::Close);
    assert_eq!(
        app.dialog,
        Some(Dialog::StyleManager),
        "back to the manager"
    );
    assert_eq!(
        app.styles.manager.as_ref().and_then(|m| m.selected.clone()),
        Some(id.clone()),
        "the saved drawing is shown"
    );
    // The designer's image field: Yeni çizim… and Kaydet give the field the drawing.
    let _ = app.update(Message::StyleManager(Box::new(
        crate::style::manager::Event::NewSymbol("marker"),
    )));
    let _ = app.update(Message::Designer(Box::new(
        crate::style::designer::Event::Add("svg", None),
    )));
    let _ = app.update(Message::Designer(Box::new(
        crate::style::designer::Event::DrawSvg("asset".into(), None),
    )));
    assert_eq!(app.dialog, Some(Dialog::SvgEditor));
    {
        let e = ed_mut(&mut app);
        e.set_tool(ToolId::Ellipse);
        e.place([20.0, 20.0], [80.0, 80.0], false);
    }
    send(&mut app, Event::Save);
    let drawing = ed(&app).original.as_ref().expect("saved").id.clone();
    send(&mut app, Event::Close);
    assert_eq!(app.dialog, Some(Dialog::SymbolDesigner));
    let d = app.styles.designer.as_ref().expect("the designer");
    let layer =
        kentos_native_style::designer::layer_at(&d.draft.symbol, d.selected).expect("the layer");
    assert_eq!(layer["asset"], serde_json::Value::from(drawing));
}

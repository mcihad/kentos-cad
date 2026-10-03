//! The sheet mode driven as the desktop drives it: messages in, the book,
//! the chosen items and the effects out. Every change is the core's
//! operation, so these check what the interface asks of it: a press chooses,
//! a drag moves with snapping, a handle resizes, a box chooses what it
//! holds, a tool draws a new item, the inspector, the page, guides, the
//! tree, the gallery, the preflight's fixes, the store and the exports.

use iced::{Point, Size};
use kentos_sheet::kinds::{GroundPoint, ItemKind};
use kentos_sheet::model::{HConstraint, Item, Orientation, Paper};
use kentos_sheet::profile::Capabilities;
use kentos_sheet_ui::gallery::{Asking, GalleryMessage, Source};
use kentos_sheet_ui::library::TemplateAction;
use kentos_sheet_ui::message::{FrameField, Side};
use kentos_sheet_ui::questions::QuestionsMessage;
use kentos_sheet_ui::save_template::SaveMessage;
use kentos_sheet_ui::{
    Context, DemoMaps, Designer, Effect, ExportKind, InspectorTab, Message, Say, StageEvent, Store,
    Tool, lend,
};
use kentos_ui::widget::rulers;

const STAGE: Size = Size::new(1000.0, 700.0);

fn context() -> Context {
    Context {
        capabilities: Capabilities {
            georeferenced: true,
            attribute_layers: true,
            plot_scale: Some(1000),
        },
        center: Some(GroundPoint {
            x: 500_000.0,
            y: 4_420_000.0,
        }),
        ..Context::default()
    }
}

/// A designer with a sheet of the mode's default template (genel A3 yatay) in front.
fn designer() -> Designer {
    let mut d = Designer::new(context());
    let effects = new_sheet(&mut d);
    assert_eq!(effects, [Effect::ShowSheet]);
    assert!(d.is_active());
    d
}

/// Yeni pafta: the template's questions are asked first; answered with its own values.
fn new_sheet(d: &mut Designer) -> Vec<Effect> {
    assert!(
        d.update(Message::NewSheet).is_empty(),
        "the questions first"
    );
    assert!(d.questions().is_some());
    d.update(Message::Questions(QuestionsMessage::Make))
}

/// Kullan: the template's questions, when it has any, answered with its own values.
fn use_picked(d: &mut Designer) -> Vec<Effect> {
    let effects = d.update(Message::Gallery(GalleryMessage::Use));
    if d.questions().is_some() {
        assert!(effects.is_empty(), "{effects:?}");
        return d.update(Message::Questions(QuestionsMessage::Make));
    }
    effects
}

fn item<'a>(d: &'a Designer, name: &str) -> &'a Item {
    d.open_sheet()
        .unwrap()
        .items
        .iter()
        .find(|i| i.name == name)
        .unwrap_or_else(|| panic!("{name} yok"))
}

fn center(i: &Item) -> [f64; 2] {
    i.frame.center()
}

fn press(d: &mut Designer, at: [f64; 2], shift: bool) {
    d.update(Message::Stage(StageEvent::Press {
        at,
        px: Point::ORIGIN,
        size: STAGE,
        shift,
        ctrl: false,
        alt: false,
    }));
}

/// A drag in four steps; `free` holds Ctrl (no snapping).
fn drag(d: &mut Designer, from: [f64; 2], to: [f64; 2], free: bool) {
    press(d, from, false);
    for k in 1..=4 {
        let t = f64::from(k) / 4.0;
        let at = [
            from[0] + (to[0] - from[0]) * t,
            from[1] + (to[1] - from[1]) * t,
        ];
        d.update(Message::Stage(StageEvent::Move {
            at,
            size: STAGE,
            shift: false,
            ctrl: free,
            alt: false,
        }));
    }
    d.update(Message::Stage(StageEvent::Release {
        at: to,
        shift: false,
        ctrl: free,
        alt: false,
    }));
}

#[test]
fn the_tabs_open_a_sheet_and_go_back_to_the_model() {
    let mut d = designer();
    assert_eq!(d.book().sheets.len(), 1);
    assert_eq!(d.open_sheet().unwrap().page.paper, Paper::A3);
    // The new map looks where the drawing area does, at the plot scale.
    let map = item(&d, "Harita");
    match &map.kind {
        ItemKind::Map(m) => match &m.view {
            kentos_sheet::kinds::MapView::Fixed(f) => {
                assert_eq!(
                    f.center,
                    Some(GroundPoint {
                        x: 500_000.0,
                        y: 4_420_000.0
                    })
                );
                assert_eq!(f.scale, 1000);
            }
            v => panic!("{v:?}"),
        },
        k => panic!("{k:?}"),
    }
    assert_eq!(d.update(Message::Tab(0)), [Effect::ShowModel]);
    assert!(!d.is_active());
    assert_eq!(d.update(Message::Tab(1)), [Effect::ShowSheet]);
    // A second sheet, then closing it: the book loses it, and the undo brings it back.
    new_sheet(&mut d);
    assert_eq!(d.book().sheets.len(), 2);
    let effects = d.update(Message::CloseTab(2));
    assert!(matches!(effects.last(), Some(Effect::Notice(_))));
    assert_eq!(d.book().sheets.len(), 1);
    d.update(Message::Undo);
    assert_eq!(d.book().sheets.len(), 2);
    // Moving a tab moves the sheet.
    let first = d.book().sheets[0].id.clone();
    d.update(Message::MoveTab(1, 2));
    assert_eq!(d.book().sheets[1].id, first);
}

#[test]
fn a_press_chooses_and_a_drag_moves_with_snapping_and_one_undo_step() {
    let mut d = designer();
    let title = item(&d, "Başlık").clone();
    // A press on the title chooses it.
    press(&mut d, center(&title), false);
    d.update(Message::Stage(StageEvent::Release {
        at: center(&title),
        shift: false,
        ctrl: false,
        alt: false,
    }));
    assert_eq!(d.selection(), std::slice::from_ref(&title.id));
    // A drag of 10.3 mm to the left, snapping off: it moves exactly that much.
    let c = center(&title);
    drag(&mut d, c, [c[0] - 10_300.0, c[1]], true);
    assert_eq!(item(&d, "Başlık").frame.left, title.frame.left - 10_300);
    assert_eq!(
        d.undo_label().map(str::to_owned),
        Some(format!("Taşı: {}", title.name))
    );
    // With snapping, a drag that ends near its old place lands back on it (the other items' edges hold it).
    let moved = item(&d, "Başlık").clone();
    let c = center(&moved);
    drag(&mut d, c, [c[0] + 10_300.0 - 400.0, c[1]], false);
    assert_eq!(
        item(&d, "Başlık").frame.left,
        title.frame.left,
        "snapped back to the column"
    );
    // Undo twice: where it was.
    d.update(Message::Undo);
    d.update(Message::Undo);
    assert_eq!(item(&d, "Başlık").frame, title.frame);
    d.update(Message::Redo);
    assert_eq!(item(&d, "Başlık").frame.left, title.frame.left - 10_300);
}

#[test]
fn a_handle_resizes_and_a_box_chooses_what_it_holds() {
    let mut d = designer();
    let map = item(&d, "Harita").clone();
    d.update(Message::Select(vec![map.id.clone()]));
    // The south-east corner dragged 20 mm in and up, snapping off.
    let corner = [map.frame.right() as f64, map.frame.bottom() as f64];
    drag(
        &mut d,
        corner,
        [corner[0] - 20_000.0, corner[1] - 20_000.0],
        true,
    );
    let after = item(&d, "Harita").frame;
    assert_eq!(
        (after.width, after.height),
        (map.frame.width - 20_000, map.frame.height - 20_000)
    );
    assert_eq!((after.left, after.top), (map.frame.left, map.frame.top));
    // A box drawn to the right over the strip's top: the items wholly in it.
    d.update(Message::Select(Vec::new()));
    let title = item(&d, "Başlık").clone();
    let sub = item(&d, "Alt başlık").clone();
    let from = [
        f64::from(title.frame.left) - 1_000.0,
        f64::from(title.frame.top) - 1_000.0,
    ];
    let to = [
        sub.frame.right() as f64 + 1_000.0,
        sub.frame.bottom() as f64 + 1_000.0,
    ];
    // The press must fall on empty paper: the strip's shape is under it, so start the box from the margin.
    let start = [f64::from(title.frame.left) - 1_000.0, 2_000.0];
    let _ = from;
    drag(&mut d, start, to, false);
    let mut chosen = d.selection().to_vec();
    chosen.sort();
    let mut want = vec![title.id.clone(), sub.id.clone()];
    want.sort();
    assert_eq!(chosen, want);
}

#[test]
fn a_tool_draws_a_new_item_and_hands_back_the_choosing_tool() {
    let mut d = designer();
    d.update(Message::Tool(Tool::Add {
        tool: "text".into(),
        preset: None,
    }));
    // A click: the kind's own size where it was clicked.
    press(&mut d, [40_000.0, 40_000.0], false);
    d.update(Message::Stage(StageEvent::Release {
        at: [40_000.0, 40_000.0],
        shift: false,
        ctrl: false,
        alt: false,
    }));
    let added = d.open_sheet().unwrap().items.last().unwrap().clone();
    assert!(matches!(added.kind, ItemKind::Text(_)));
    assert_eq!(
        (added.frame.left, added.frame.top, added.frame.width),
        (40_000, 40_000, 60_000)
    );
    assert_eq!(d.selection(), std::slice::from_ref(&added.id));
    assert_eq!(d.tool(), &Tool::Select);
    // A box drawn: that size.
    d.update(Message::Tool(Tool::Add {
        tool: "scaleBar".into(),
        preset: Some("numeric".into()),
    }));
    drag(&mut d, [30_000.0, 200_000.0], [90_000.0, 215_000.0], false);
    let bar = d.open_sheet().unwrap().items.last().unwrap().clone();
    assert_eq!((bar.frame.width, bar.frame.height), (60_000, 15_000));
    match &bar.kind {
        ItemKind::ScaleBar(s) => {
            assert_eq!(s.style, kentos_sheet::kinds::ScaleBarStyle::Numeric);
            // It reads the sheet's map.
            assert_eq!(s.map.as_deref(), Some(item(&d, "Harita").id.as_str()));
        }
        k => panic!("{k:?}"),
    }
}

/// Seçim: the chosen item fills the stage less 72 pixels a side, on the stage
/// as large as it last said it was; Sığdır brings the page back.
#[test]
fn the_chosen_item_fills_the_stage_at_its_size() {
    let mut d = designer();
    d.update(Message::Stage(StageEvent::Resize {
        size: Size::new(800.0, 600.0),
    }));
    let fitted = d.zoom_percent();
    let title = item(&d, "Başlık").clone();
    d.update(Message::Select(vec![title.id.clone()]));
    d.update(Message::ZoomSelection);
    let (w, h) = (
        f64::from(title.frame.width) / 1000.0,
        f64::from(title.frame.height) / 1000.0,
    );
    let want = ((800.0 - 144.0) / w)
        .min((600.0 - 144.0) / h)
        .min(96.0 / 25.4 * 16.0);
    assert_eq!(
        d.zoom_percent(),
        Some((want / (96.0 / 25.4) * 100.0).round() as u32)
    );
    d.update(Message::ZoomPage);
    assert_eq!(d.zoom_percent(), fitted);
    // Nothing chosen: nothing to come close to.
    d.update(Message::Select(Vec::new()));
    d.update(Message::ZoomSelection);
    assert_eq!(d.zoom_percent(), fitted);
}

/// The stage drawn at a size tells the mode that size (a redraw is enough), so
/// Sığdır and Seçim fit the stage as it is, not as the pointer last saw it.
#[test]
fn a_stage_drawn_at_a_new_size_says_so() {
    let mut d = designer();
    let before = d.zoom_percent();
    let mut snapshot =
        kentos_ui::snapshot::Snapshot::software(Size::new(640.0, 420.0)).expect("a renderer");
    let mut update = |d: &mut Designer, m: Message| {
        let _ = d.update(m);
    };
    snapshot.settle(&mut d, |d| d.stage(std::rc::Rc::new(DemoMaps)), &mut update);
    let ruler = rulers::thickness();
    let page = d.open_sheet().unwrap().page.size;
    let (w, h) = (
        f64::from(page.width) / 1000.0,
        f64::from(page.height) / 1000.0,
    );
    // Sığdır: the page in the stage less the rulers, 28 pixels around it.
    let fit = ((640.0 - f64::from(ruler) - 56.0) / w).min((420.0 - f64::from(ruler) - 56.0) / h);
    assert_eq!(
        d.zoom_percent(),
        Some((fit / (96.0 / 25.4) * 100.0).round() as u32)
    );
    assert_ne!(d.zoom_percent(), before);
}

/// A small PNG, its colours running across it (a logo's stand-in).
fn png_of(width: u32, height: u32) -> Vec<u8> {
    let mut out = Vec::new();
    {
        let mut e = png::Encoder::new(&mut out, width, height);
        e.set_color(png::ColorType::Rgba);
        e.set_depth(png::BitDepth::Eight);
        let mut w = e.write_header().expect("a header");
        let px: Vec<u8> = (0..width * height)
            .flat_map(|i| [(i % width * 255 / width) as u8, 80, 160, 255])
            .collect();
        w.write_image_data(&px).expect("the pixels");
    }
    out
}

/// “Resim seç…” (the web's): the host is asked for a file; a PNG (or an SVG)
/// is kept with the book and shown by the chosen frame in one step; anything
/// else is refused with the web's words, sizes written with a point.
#[test]
fn a_picture_file_is_kept_and_shown_in_one_step() {
    let mut d = designer();
    d.update(Message::Tool(Tool::Add {
        tool: "picture".into(),
        preset: None,
    }));
    press(&mut d, [40_000.0, 40_000.0], false);
    d.update(Message::Stage(StageEvent::Release {
        at: [40_000.0, 40_000.0],
        shift: false,
        ctrl: false,
        alt: false,
    }));
    let frame = d.open_sheet().unwrap().items.last().unwrap().clone();
    assert!(matches!(&frame.kind, ItemKind::Picture(p) if p.asset.is_none()));
    assert_eq!(d.update(Message::ChoosePicture), [Effect::AskPicturePath]);
    let assets = d.book().assets.len();
    let bytes = png_of(64, 32);
    assert!(
        d.update(Message::PictureFile("logo.png".into(), bytes.clone()))
            .is_empty()
    );
    let shown = |d: &Designer| {
        d.open_sheet()
            .unwrap()
            .items
            .iter()
            .find(|i| i.id == frame.id)
            .and_then(|i| match &i.kind {
                ItemKind::Picture(p) => p.asset.clone(),
                _ => None,
            })
    };
    let sha = shown(&d).expect("the frame shows the picture");
    let meta = d
        .book()
        .assets
        .iter()
        .find(|a| a.sha256 == sha)
        .expect("kept");
    assert_eq!(
        (
            meta.name.as_str(),
            meta.width,
            meta.height,
            meta.kind,
            meta.bytes as usize
        ),
        (
            "logo.png",
            64,
            32,
            kentos_sheet::model::AssetKind::Png,
            bytes.len()
        )
    );
    assert!(!d.findings().iter().any(|f| f.code == "asset_missing"));
    // One step back: no picture, no asset.
    d.update(Message::Undo);
    assert_eq!((shown(&d), d.book().assets.len()), (None, assets));
    // Refused, as the web says it.
    let warned = |e: Vec<Effect>| match e.as_slice() {
        [Effect::Say(Say::Warn, why)] => why.clone(),
        other => panic!("{other:?}"),
    };
    assert_eq!(
        warned(d.update(Message::PictureFile(
            "notlar.txt".into(),
            b"merhaba".to_vec()
        ))),
        "“notlar.txt” resim değil: PNG, JPEG ya da SVG seçin."
    );
    // An SVG is a picture too (resvg draws it), measured as the file says.
    assert!(
        d.update(Message::PictureFile(
            "logo.svg".into(),
            b"<svg xmlns='http://www.w3.org/2000/svg' width='120' height='40'/>".to_vec()
        ))
        .is_empty()
    );
    let svg = shown(&d).expect("the frame shows the SVG");
    let meta = d
        .book()
        .assets
        .iter()
        .find(|a| a.sha256 == svg)
        .expect("kept");
    assert_eq!(
        (meta.kind, meta.width, meta.height),
        (kentos_sheet::model::AssetKind::Svg, 120, 40)
    );
    d.update(Message::Undo);
    let mut big = b"\x89PNG".to_vec();
    big.resize(4 * 1024 * 1024 + 1, 0);
    assert_eq!(
        warned(d.update(Message::PictureFile("tarama.png".into(), big))),
        "“tarama.png” 4.0 MB: bir resim en çok 4 MB olabilir. Küçültüp yeniden seçin."
    );
    assert!(
        warned(d.update(Message::PictureFile(
            "bozuk.png".into(),
            b"\x89PNG bozuk".to_vec()
        )))
        .contains("okunamadı")
    );
    assert_eq!(shown(&d), None, "nothing refused reaches the sheet");
    let _ = assets;
}

#[test]
fn the_inspector_changes_the_frame_the_constraints_and_the_kind() {
    let mut d = designer();
    let title = item(&d, "Başlık").clone();
    d.update(Message::Select(vec![title.id.clone()]));
    d.update(Message::Frame(FrameField::Width, 70.0));
    assert_eq!(item(&d, "Başlık").frame.width, 70_000);
    // Typed as Turkish numbers with an expression.
    d.update(Message::FrameText(FrameField::Top, "10+6,5".into()));
    assert_eq!(item(&d, "Başlık").frame.top, 16_500);
    d.update(Message::ConstraintH(HConstraint::LeftRight));
    assert_eq!(item(&d, "Başlık").constraints.h, HConstraint::LeftRight);
    // The kind's own: the text's size, through the KentOS inspector's event.
    let size_field = 2;
    d.update(Message::Inspector(
        kentos_ui::widget::inspector::Event::Set {
            id: size_field,
            value: kentos_ui::attribute::Value::Real(5.0),
        },
    ));
    match &item(&d, "Başlık").kind {
        ItemKind::Text(t) => assert_eq!(t.style.size, 5_000),
        k => panic!("{k:?}"),
    }
    // Several items: what they share; a change goes to all.
    let sub = item(&d, "Alt başlık").clone();
    d.update(Message::Select(vec![title.id.clone(), sub.id.clone()]));
    d.update(Message::Opacity(50.0));
    assert_eq!(
        (item(&d, "Başlık").opacity, item(&d, "Alt başlık").opacity),
        (50, 50)
    );
    d.update(Message::Printable(false));
    assert!(!item(&d, "Alt başlık").printable);
}

#[test]
fn the_page_changes_paper_and_the_layout_variant_follows() {
    let mut d = designer();
    d.update(Message::InspectorTab(InspectorTab::Page));
    d.update(Message::Orientation(Orientation::Portrait));
    let s = d.open_sheet().unwrap();
    assert_eq!(
        (s.page.orientation, s.page.size.width),
        (Orientation::Portrait, 297_000)
    );
    // genel-a3-yatay has a portrait layout: in force now.
    assert_eq!(s.active_variant.as_deref(), Some("dikey"));
    d.update(Message::Paper(Paper::A4));
    assert_eq!(d.open_sheet().unwrap().page.size.width, 210_000);
    d.update(Message::Margin(Side::Left, 25.0));
    assert_eq!(d.open_sheet().unwrap().page.margins.left, 25_000);
    // Back to A3 landscape through undo: the book as it was.
    for _ in 0..3 {
        d.update(Message::Undo);
    }
    assert_eq!(
        d.open_sheet().unwrap().active_variant.as_deref(),
        Some("yatay")
    );
}

#[test]
fn guides_from_the_rulers_live_in_the_sheet() {
    let mut d = designer();
    d.update(Message::Guide(rulers::Event::Added(
        rulers::Guide::vertical(100.0),
    )));
    d.update(Message::Guide(rulers::Event::Added(
        rulers::Guide::horizontal(50.5),
    )));
    let g = &d.open_sheet().unwrap().guides;
    assert_eq!(
        g.iter().map(|g| (g.axis, g.at)).collect::<Vec<_>>(),
        [
            (kentos_sheet::model::Axis::X, 100_000),
            (kentos_sheet::model::Axis::Y, 50_500)
        ]
    );
    d.update(Message::Guide(rulers::Event::Moved(0, 120.0)));
    assert_eq!(d.open_sheet().unwrap().guides[0].at, 120_000);
    d.update(Message::Guide(rulers::Event::Removed(1)));
    assert_eq!(d.open_sheet().unwrap().guides.len(), 1);
}

#[test]
fn the_tree_hides_locks_reorders_and_groups() {
    let mut d = designer();
    let title = item(&d, "Başlık").clone();
    let sub = item(&d, "Alt başlık").clone();
    d.update(Message::Hidden(title.id.clone(), true));
    assert!(item(&d, "Başlık").hidden);
    d.update(Message::Locked(title.id.clone(), true));
    assert!(item(&d, "Başlık").locked);
    // A locked item does not move with the arrows.
    d.update(Message::Select(vec![title.id.clone()]));
    d.update(Message::Nudge(1_000, 0));
    assert_eq!(item(&d, "Başlık").frame, title.frame);
    d.update(Message::Locked(title.id.clone(), false));
    d.update(Message::Nudge(1_000, 0));
    assert_eq!(item(&d, "Başlık").frame.left, title.frame.left + 1_000);
    // Grouped and ungrouped.
    d.update(Message::Select(vec![title.id.clone(), sub.id.clone()]));
    d.update(Message::Group);
    let group = d.selection()[0].clone();
    assert!(matches!(
        d.book().item(&group).unwrap().kind,
        ItemKind::Group(_)
    ));
    assert_eq!(item(&d, "Başlık").group.as_deref(), Some(group.as_str()));
    d.update(Message::Ungroup);
    assert!(item(&d, "Başlık").group.is_none());
    // Dragged in the tree: before the frame item, the topmost row.
    let order = |d: &Designer| {
        d.open_sheet()
            .unwrap()
            .items
            .iter()
            .map(|i| i.name.clone())
            .collect::<Vec<_>>()
    };
    let before = order(&d);
    let rows = |d: &Designer| {
        d.open_sheet()
            .unwrap()
            .items
            .iter()
            .rev()
            .filter(|i| i.group.is_none())
            .map(|i| i.name.clone())
            .collect::<Vec<_>>()
    };
    let tree = rows(&d);
    let from = tree.iter().position(|n| n == "Pafta çerçevesi").unwrap();
    d.update(Message::TreeMove(
        from,
        0,
        kentos_ui::widget::tree_view::Place::Before,
    ));
    let after = order(&d);
    assert_ne!(before, after);
    assert_eq!(
        after.last().map(String::as_str),
        Some("Pafta çerçevesi"),
        "on top of the drawing"
    );
}

#[test]
fn duplicate_delete_align_and_keys() {
    let mut d = designer();
    let title = item(&d, "Başlık").clone();
    d.update(Message::Select(vec![title.id.clone()]));
    d.update(Message::Duplicate);
    let copy = d.selection()[0].clone();
    let c = d.book().item(&copy).unwrap().clone();
    assert_eq!(
        (c.frame.left, c.name.as_str()),
        (title.frame.left + 5_000, "Başlık (kopya)")
    );
    d.update(Message::Delete);
    assert!(d.book().item(&copy).is_none());
    // Two items aligned on their left edges.
    let sub = item(&d, "Alt başlık").clone();
    d.update(Message::Nudge(0, 0));
    d.update(Message::Select(vec![title.id.clone(), sub.id.clone()]));
    d.update(Message::Align(kentos_sheet::ops::AlignEdge::Right));
    assert_eq!(
        item(&d, "Başlık").frame.right(),
        item(&d, "Alt başlık").frame.right()
    );
    // The keys of the mode.
    use iced::keyboard::{Key, Modifiers, key::Named};
    assert!(matches!(
        d.key(&Key::Character("z".into()), Modifiers::CTRL),
        Some(Message::Undo)
    ));
    assert!(matches!(
        d.key(&Key::Named(Named::ArrowLeft), Modifiers::SHIFT),
        Some(Message::Nudge(-10_000, 0))
    ));
    assert!(matches!(
        d.key(&Key::Character("t".into()), Modifiers::empty()),
        Some(Message::Tool(Tool::Add { .. }))
    ));
    assert!(matches!(
        d.key(&Key::Character("0".into()), Modifiers::CTRL),
        Some(Message::ZoomPage)
    ));
    assert!(
        d.key(&Key::Character("q".into()), Modifiers::empty())
            .is_none()
    );
}

#[test]
fn the_preflight_lists_findings_and_a_fix_is_applied() {
    let mut d = designer();
    // An item pushed off the paper: the preflight says so and offers to bring it back.
    let title = item(&d, "Başlık").clone();
    d.update(Message::Select(vec![title.id.clone()]));
    d.update(Message::Frame(FrameField::Left, 500.0));
    let at = d
        .findings()
        .iter()
        .position(|f| f.item.as_deref() == Some(title.id.as_str()) && !f.fixes.is_empty())
        .expect("a finding with a fix");
    let fixes = d.findings()[at].fixes.len();
    assert!(fixes > 0);
    d.update(Message::Fix(at, 0));
    assert!(item(&d, "Başlık").frame.right() <= i64::from(d.open_sheet().unwrap().page.size.width));
}

#[test]
fn the_gallery_ranks_filters_and_uses_a_template_on_another_paper() {
    let mut d = Designer::new(Context {
        workspace: Some(kentos_contracts::Workspace::Gis),
        project_type: Some(kentos_contracts::ProjectType::Gis),
        ..context()
    });
    d.update(Message::Gallery(GalleryMessage::Open));
    // A GIS project: its own templates first.
    d.update(Message::Gallery(GalleryMessage::Pick(
        "sys:gis-tematik".into(),
    )));
    d.update(Message::Gallery(GalleryMessage::UsePaper(Some(
        kentos_sheet::template::PaperChoice {
            paper: Paper::A4,
            orientation: Orientation::Landscape,
        },
    ))));
    let effects = use_picked(&mut d);
    assert_eq!(effects, [Effect::ShowSheet]);
    let s = d.open_sheet().unwrap();
    assert_eq!(
        (s.page.paper, s.page.orientation),
        (Paper::A4, Orientation::Landscape)
    );
    assert_eq!(s.active_variant.as_deref(), Some("yatay-a4"));
    assert_eq!(
        s.origin.as_ref().map(|o| o.template_id.as_str()),
        Some("sys:gis-tematik")
    );
}

/// A template that lists a layer's objects asks first in a project without
/// attributes; once the host says the project has them, the card is worked
/// out again and Kullan makes the sheet at once.
#[test]
fn the_gallery_asks_for_what_the_project_lacks_until_the_host_says_it_has_it() {
    let bare = Context {
        capabilities: Capabilities {
            attribute_layers: false,
            ..context().capabilities
        },
        ..context()
    };
    let mut d = Designer::new(bare.clone());
    d.update(Message::Gallery(GalleryMessage::Open));
    d.update(Message::Gallery(GalleryMessage::AllModes(true)));
    d.update(Message::Gallery(GalleryMessage::Pick(
        "sys:ifraz-paftasi".into(),
    )));
    assert!(d.update(Message::Gallery(GalleryMessage::Use)).is_empty());
    match d.gallery_asking() {
        Some(Asking::Needs { needs, .. }) => assert!(
            needs
                .iter()
                .any(|n| n.contains("öznitelikli bir katman ister")),
            "{needs:?}"
        ),
        other => panic!("the needs question: {other:?}"),
    }
    d.update(Message::Gallery(GalleryMessage::Answer(false)));
    // The drawing has attributes after all: the same card, no question.
    d.set_context(context());
    assert_eq!(use_picked(&mut d), [Effect::ShowSheet]);
    assert!(d.gallery_asking().is_none());
}

#[test]
fn a_sheet_saved_as_a_template_is_kept_and_found_again() {
    let dir = std::env::temp_dir().join(format!("kentos-sheet-ui-test-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let mut d = Designer::new(context());
    d.attach(Store::open(&dir).unwrap(), "dosya/ada.kcad");
    new_sheet(&mut d);
    let title = item(&d, "Başlık").clone();
    d.update(Message::Select(vec![title.id.clone()]));
    d.update(Message::Nudge(2_000, 0));
    d.update(Message::SaveTemplate(SaveMessage::Open));
    d.update(Message::SaveTemplate(SaveMessage::Name("Bürom".into())));
    let effects = d.update(Message::SaveTemplate(SaveMessage::Save));
    assert!(
        matches!(effects.as_slice(), [Effect::Say(Say::Success, n)] if n.contains("Bürom") && n.contains("Benim")),
        "{effects:?}"
    );
    // Another designer on the same store: the book and the template are there.
    let mut again = Designer::new(context());
    again.attach(Store::open(&dir).unwrap(), "dosya/ada.kcad");
    assert_eq!(again.book(), d.book());
    again.update(Message::Gallery(GalleryMessage::Open));
    again.update(Message::Gallery(GalleryMessage::Source(Source::Mine)));
    again.update(Message::Gallery(GalleryMessage::Query("büro".into())));
    let ids: Vec<String> = Store::open(&dir)
        .unwrap()
        .templates()
        .iter()
        .map(|t| t.template.meta.name.clone())
        .collect();
    assert_eq!(ids, ["Bürom"]);
    // Duplicated and deleted in the gallery.
    let id = Store::open(&dir).unwrap().templates()[0].id.clone();
    again.update(Message::Gallery(GalleryMessage::Pick(id.clone())));
    again.update(Message::Gallery(GalleryMessage::Act(
        TemplateAction::Duplicate,
    )));
    assert_eq!(Store::open(&dir).unwrap().templates().len(), 2);
    // The copy is the one picked now: deleting it (asked first) leaves the original.
    again.update(Message::Gallery(GalleryMessage::Act(
        TemplateAction::Delete,
    )));
    assert_eq!(
        Store::open(&dir).unwrap().templates().len(),
        2,
        "asked first"
    );
    let said = again.update(Message::Gallery(GalleryMessage::Answer(true)));
    assert!(
        said.iter().any(
            |e| matches!(e, Effect::Say(Say::Success, n) if n.contains("bu cihazdan silindi"))
        ),
        "{said:?}"
    );
    let left: Vec<String> = Store::open(&dir)
        .unwrap()
        .templates()
        .iter()
        .map(|t| t.template.meta.name.clone())
        .collect();
    assert_eq!(left, ["Bürom"]);
    // A sheet made from it is saved back over it: the same id, the revision one higher.
    again.update(Message::Gallery(GalleryMessage::Pick(id.clone())));
    again.update(Message::Gallery(GalleryMessage::Act(TemplateAction::Edit)));
    again.update(Message::SaveTemplate(SaveMessage::Open));
    let effects = again.update(Message::SaveTemplate(SaveMessage::Save));
    assert!(
        matches!(effects.as_slice(), [Effect::Say(Say::Success, n)] if n.contains("revizyon 2")),
        "{effects:?}"
    );
    let kept = Store::open(&dir).unwrap().template(&id).unwrap();
    assert_eq!(kept.template.meta.revision, 2);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn exports_write_svg_png_and_a_kpafta_that_reads_back() {
    let mut d = designer();
    // SVG: the core's writer, the map as the painter's PNG.
    let svg = d.svg(&DemoMaps).unwrap();
    assert!(
        svg.starts_with("<svg") || svg.starts_with("<?xml"),
        "{}",
        &svg[..60]
    );
    assert!(
        svg.contains("data:image/png;base64,"),
        "the map's picture is in it"
    );
    // PNG at 96 dpi: an A3 landscape is 1587 × 1123 pixels.
    let mut png = Vec::new();
    let (w, h) = d.write_png(&DemoMaps, 96, &mut png).unwrap();
    assert_eq!((w, h), (1587, 1123));
    let decoder = png::Decoder::new(std::io::Cursor::new(&png));
    let mut reader = decoder.read_info().unwrap();
    let mut buf = vec![0; reader.output_buffer_size().unwrap()];
    let info = reader.next_frame(&mut buf).unwrap();
    assert_eq!((info.width, info.height), (1587, 1123));
    // The paper is white where nothing is drawn (the frame's corner outside the border).
    assert_eq!(&buf[..4], &[255, 255, 255, 255]);
    // The map's middle shows the painter's town, not the white paper.
    let map = item(&d, "Harita").frame;
    let (mx, my) = (
        (map.center()[0] / 25_400.0 * 96.0) as usize,
        (map.center()[1] / 25_400.0 * 96.0) as usize,
    );
    let i = (my * 1587 + mx) * 4;
    assert_ne!(&buf[i..i + 3], &[255, 255, 255]);
    // .kpafta: the book read back.
    let text = d.kpafta().unwrap();
    let mut other = Designer::new(context());
    other.read_kpafta(&text).unwrap();
    assert_eq!(other.book(), d.book());
    assert!(other.read_kpafta("{}").is_err());
    // Asking for an export while the preflight has errors (the title block's values are not given
    // here): a window says so first; “Yine de aktar” asks the host where, with the sheet's name.
    assert!(
        d.findings()
            .iter()
            .any(|f| f.severity == kentos_sheet::preflight::Severity::Error)
    );
    assert!(d.update(Message::Export(ExportKind::Svg)).is_empty());
    assert!(d.window(lend(&DemoMaps)).is_some());
    match d.update(Message::ExportAnyway).as_slice() {
        [
            Effect::AskExportPath {
                kind: ExportKind::Svg,
                suggested,
            },
        ] => assert!(suggested.ends_with(".svg"), "{suggested}"),
        e => panic!("{e:?}"),
    }
}

#[test]
fn the_variables_window_fills_the_values_the_preflight_misses() {
    use kentos_sheet_ui::variables::{Scope, VariablesMessage as V};
    let mut d = designer();
    let missing = |d: &Designer| {
        d.findings()
            .iter()
            .filter(|f| f.code == "value_missing")
            .count()
    };
    let before = missing(&d);
    assert!(before > 0);
    // The preflight's fix opens the window.
    let at = d.findings().iter().position(|f| {
        f.code == "value_missing"
            && f.fixes
                .iter()
                .any(|x| x.action.as_deref() == Some("sheet.variables"))
    });
    if let Some(at) = at {
        let fix = d.findings()[at]
            .fixes
            .iter()
            .position(|x| x.action.as_deref() == Some("sheet.variables"))
            .unwrap();
        assert!(d.update(Message::Fix(at, fix)).is_empty());
    } else {
        d.update(Message::Variables(V::Open));
    }
    assert!(d.window(lend(&DemoMaps)).is_some());
    d.update(Message::Variables(V::AddMissing));
    // Every missing name now has a field: give each a value.
    for i in 0..10 {
        d.update(Message::Variables(V::Text(
            Scope::Sheet,
            i,
            format!("değer {i}"),
        )));
    }
    // A project variable too, with a bad name first.
    d.update(Message::Variables(V::NewName(
        Scope::Project,
        "2idare".into(),
    )));
    d.update(Message::Variables(V::Add(Scope::Project)));
    d.update(Message::Variables(V::NewName(
        Scope::Project,
        "@idare".into(),
    )));
    d.update(Message::Variables(V::Add(Scope::Project)));
    d.update(Message::Variables(V::Save));
    assert!(d.window(lend(&DemoMaps)).is_none());
    assert_eq!(
        d.book()
            .variables
            .iter()
            .map(|v| v.name.as_str())
            .collect::<Vec<_>>(),
        ["idare"]
    );
    assert!(missing(&d) < before, "{} → {}", before, missing(&d));
    // One undo step for both lists.
    d.update(Message::Undo);
    assert!(d.book().variables.is_empty());
    assert_eq!(missing(&d), before);
}

#[test]
fn a_kpafta_is_imported_beside_or_in_place_of_the_sheets() {
    let mut d = designer();
    let file = d.kpafta().unwrap();
    // Beside: the file's sheet with new ids, after a question.
    d.update(Message::ImportText("ada.kpafta".into(), file.clone()));
    assert!(
        d.window(lend(&DemoMaps)).is_some(),
        "the project has sheets: beside or in place?"
    );
    let effects = d.update(Message::ImportChoose(false));
    assert!(
        matches!(effects.last(), Some(Effect::Notice(n)) if n.contains("eklendi")),
        "{effects:?}"
    );
    let ids: Vec<String> = d.book().sheets.iter().map(|s| s.id.clone()).collect();
    assert_eq!(ids.len(), 2);
    assert_ne!(ids[0], ids[1]);
    let items = |i: usize| {
        d.book().sheets[i]
            .items
            .iter()
            .map(|x| x.id.clone())
            .collect::<Vec<_>>()
    };
    assert!(
        items(0).iter().all(|id| !items(1).contains(id)),
        "no item id twice"
    );
    // In place: the file's sheet alone; the undo brings the two back.
    d.update(Message::ImportText("ada.kpafta".into(), file));
    d.update(Message::ImportChoose(true));
    assert_eq!(d.book().sheets.len(), 1);
    d.update(Message::Undo);
    assert_eq!(d.book().sheets.len(), 2);
    // Not a sheet file.
    d.update(Message::ImportText("x.kpafta".into(), "{}".into()));
    assert!(d.error().is_some_and(|e| e.contains("x.kpafta")));
}

#[test]
fn a_dragged_number_is_one_undo_step_and_typed_ones_are_one_each() {
    let mut d = designer();
    let title = item(&d, "Başlık").clone();
    d.update(Message::Select(vec![title.id.clone()]));
    let left = f64::from(title.frame.left) / 1000.0;
    // A drag of the field's label: every change joins the first one's step.
    d.update(Message::GestureStart);
    for k in 1..=5 {
        d.update(Message::Frame(FrameField::Left, left + f64::from(k)));
    }
    d.update(Message::GestureEnd);
    assert_eq!(item(&d, "Başlık").frame.left, title.frame.left + 5_000);
    d.update(Message::Undo);
    assert_eq!(item(&d, "Başlık").frame.left, title.frame.left, "one step");
    d.update(Message::Redo);
    assert_eq!(item(&d, "Başlık").frame.left, title.frame.left + 5_000);
    // Typed values are a step each.
    d.update(Message::Frame(FrameField::Left, left + 10.0));
    d.update(Message::Frame(FrameField::Left, left + 20.0));
    d.update(Message::Undo);
    assert_eq!(item(&d, "Başlık").frame.left, title.frame.left + 10_000);
    // The inspector's fields write numbers by the display rule (ADR 0149): a point, a half up.
    let f = kentos_ui::attribute::Field::real("Boyut", 1).point();
    assert_eq!(f.format(&kentos_ui::attribute::Value::Real(2.25)), "2.3");
}

#[test]
fn space_held_over_the_paper_is_the_hand_for_a_moment() {
    let mut d = designer();
    let before = item(&d, "Başlık").frame;
    d.update(Message::Space(true));
    // A drag on an item pans the paper instead of moving it.
    let from = center(item(&d, "Başlık"));
    drag(&mut d, from, [10_000.0, 10_000.0], true);
    assert_eq!(item(&d, "Başlık").frame, before);
    assert!(!d.can_undo() || d.undo_label() != Some("Taşı"));
    d.update(Message::Space(false));
    assert_eq!(*d.tool(), Tool::Select, "the tool was never changed");
}

#[test]
fn a_binding_is_written_checked_by_the_core_and_taken_off() {
    use kentos_sheet_ui::binding::BindingMessage as B;
    let mut d = designer();
    let title = item(&d, "Başlık").clone();
    d.update(Message::Select(vec![title.id.clone()]));
    d.update(Message::Binding(B::Open {
        item: title.id.clone(),
        property: "frame.left".into(),
    }));
    assert!(d.window(lend(&DemoMaps)).is_some());
    // A broken expression is not saved.
    d.update(Message::Binding(B::Text("@olcek_payda /".into())));
    d.update(Message::Binding(B::Save));
    assert!(item(&d, "Başlık").bindings.is_empty());
    d.update(Message::Binding(B::Text("20 + ".into())));
    d.update(Message::Binding(B::Insert("@sayfa".into())));
    d.update(Message::Binding(B::Save));
    assert!(d.window(lend(&DemoMaps)).is_none());
    let b = &item(&d, "Başlık").bindings;
    assert_eq!(
        b.iter()
            .map(|b| (b.property.as_str(), b.expression.as_str()))
            .collect::<Vec<_>>(),
        [("frame.left", "20 + @sayfa")]
    );
    assert_eq!(d.undo_label(), Some("Veriye bağla: Sol"));
    // Taken off: the property keeps its own value.
    d.update(Message::Binding(B::Open {
        item: title.id.clone(),
        property: "frame.left".into(),
    }));
    d.update(Message::Binding(B::Unbind));
    assert!(item(&d, "Başlık").bindings.is_empty());
    assert_eq!(d.undo_label(), Some("Bağı kaldır: Sol"));
}

#[test]
fn the_preflight_takes_a_map_s_place_from_the_drawing_area() {
    // A sheet made with no drawing open: its map has no place.
    let mut d = Designer::new(Context {
        center: None,
        ..context()
    });
    new_sheet(&mut d);
    let (at, fix) = d
        .findings()
        .iter()
        .enumerate()
        .find_map(|(i, f)| {
            f.fixes
                .iter()
                .position(|x| x.action.as_deref() == Some("map.placeFromView"))
                .map(|j| (i, j))
        })
        .expect("map_unplaced with its fix");
    // Without a view to take it from, it says so.
    assert!(d.update(Message::Fix(at, fix)).is_empty());
    assert!(d.error().is_some());
    // The drawing area looks somewhere now: “Görünümden al” puts the map's centre there, one step.
    d.set_context(context());
    let (at, fix) = d
        .findings()
        .iter()
        .enumerate()
        .find_map(|(i, f)| {
            f.fixes
                .iter()
                .position(|x| x.action.as_deref() == Some("map.placeFromView"))
                .map(|j| (i, j))
        })
        .expect("still unplaced");
    assert!(d.update(Message::Fix(at, fix)).is_empty());
    assert_eq!(d.undo_label(), Some("Harita merkezi: görünümden"));
    assert!(!d.findings().iter().any(|f| f.code == "map_unplaced"));
    // The project's coordinate system is the host's to choose.
    let crs = d.findings().iter().enumerate().find_map(|(i, f)| {
        f.fixes
            .iter()
            .position(|x| x.action.as_deref() == Some("project.crs"))
            .map(|j| (i, j))
    });
    if let Some((i, j)) = crs {
        assert_eq!(
            d.update(Message::Fix(i, j)),
            [Effect::HostAction("project.crs".into())]
        );
    }
}

/// Kullan and Yeni pafta ask the template's questions first (design §12, the web's
/// `TemplateQuestions`): filled with its own values, a text of several lines in an editor;
/// leaving goes back to the gallery without a sheet; the answers are the sheet's variables.
#[test]
fn a_template_s_questions_are_asked_before_its_sheet_is_made() {
    use kentos_sheet::model::VarValue;
    let mut d = Designer::new(context());
    // Yeni pafta: genel A3 yatay asks for the institution, the sheet's number and the checker.
    assert!(d.update(Message::NewSheet).is_empty());
    let q = d.questions().expect("the questions");
    let names: Vec<&str> = q
        .template_variables()
        .iter()
        .map(|v| v.name.as_str())
        .collect();
    assert_eq!(names, ["kurum", "pafta_no", "kontrol_eden"]);
    assert!(d.book().sheets.is_empty(), "no sheet before the answers");
    d.update(Message::Questions(QuestionsMessage::Text(
        0,
        "Suşehri Belediyesi".into(),
    )));
    assert_eq!(
        d.update(Message::Questions(QuestionsMessage::Make)),
        [Effect::ShowSheet]
    );
    let s = d.open_sheet().expect("the sheet");
    let value = |name: &str| {
        s.variables
            .iter()
            .find(|v| v.name == name)
            .map(|v| v.value.clone())
    };
    assert_eq!(
        value("kurum"),
        Some(VarValue::Text("Suşehri Belediyesi".into()))
    );
    assert_eq!(
        value("pafta_no"),
        Some(VarValue::Null),
        "left empty: ‹pafta_no?› on the paper"
    );
    // Kullan in the gallery: the questions over it; Vazgeç goes back to the gallery, no sheet.
    d.update(Message::Gallery(GalleryMessage::Open));
    d.update(Message::Gallery(GalleryMessage::AllModes(true)));
    d.update(Message::Gallery(GalleryMessage::Pick(
        "sys:cad-teknik".into(),
    )));
    assert!(d.update(Message::Gallery(GalleryMessage::Use)).is_empty());
    let q = d.questions().expect("the questions");
    let notes = q
        .template_variables()
        .iter()
        .position(|v| v.name == "notlar")
        .expect("the notes");
    assert!(q.is_area(notes), "the notes have several lines");
    assert!(q.texts[notes].starts_with("1. Ölçüler"));
    d.update(Message::CloseDialog);
    assert!(d.questions().is_none() && d.gallery_cards().iter().any(|c| c.id == "sys:cad-teknik"));
    assert_eq!(d.book().sheets.len(), 1);
    // Again, and made: the gallery closes, the notes kept with their lines.
    d.update(Message::Gallery(GalleryMessage::Use));
    assert_eq!(
        d.update(Message::Questions(QuestionsMessage::Make)),
        [Effect::ShowSheet]
    );
    assert_eq!(d.book().sheets.len(), 2);
    assert!(d.gallery_cards().is_empty(), "the gallery closed");
    let s = d.open_sheet().expect("the sheet");
    let notes = s
        .variables
        .iter()
        .find(|v| v.name == "notlar")
        .expect("the notes");
    assert!(
        matches!(&notes.value, VarValue::Text(t) if t.lines().count() == 3),
        "{:?}",
        notes.value
    );
}

/// The names an export offers (the web's `fileName`, `pdfName`, `exportName`): SVG and PNG the
/// sheet's name; a PDF of one sheet its export name as the core writes it (`[% @pafta_adi %]`
/// by default, a fixed text as it is); a PDF of several “<çizim> paftaları”; `.kpafta` the
/// project's; the characters no file system takes as spaces.
#[test]
fn an_export_s_file_is_named_as_the_web_names_it() {
    use kentos_sheet::display::ProjectInfo;
    use kentos_sheet_ui::message::{PdfMessage, PdfScope};
    let ctx = Context {
        project: ProjectInfo {
            name: "Ada 101.kcad".into(),
            ..ProjectInfo::default()
        },
        ..context()
    };
    let mut d = Designer::new(ctx);
    new_sheet(&mut d);
    let suggested = |d: &mut Designer, m: Message| -> String {
        let mut effects = d.update(m);
        if effects.is_empty() {
            effects = d.update(Message::ExportAnyway);
        }
        match effects.as_slice() {
            [Effect::AskExportPath { suggested, .. }] => suggested.clone(),
            other => panic!("{other:?}"),
        }
    };
    let name = d.open_sheet().unwrap().name.clone();
    assert_eq!(
        suggested(&mut d, Message::Export(ExportKind::Svg)),
        format!("{name}.svg")
    );
    // The PDF: the export name `[% @pafta_adi %]` written, the sheet's name.
    d.update(Message::Export(ExportKind::Pdf));
    assert_eq!(
        suggested(&mut d, Message::Pdf(PdfMessage::Save)),
        format!("{name}.pdf")
    );
    // A fixed export name with a character no file system takes.
    let id = d.open_sheet().unwrap().id.clone();
    let mut export = d.open_sheet().unwrap().export.clone();
    export.file_name = "Ada 101: parsel/7".into();
    assert!(d.apply(
        vec![kentos_sheet::ops::Op::SetExport(
            kentos_sheet::ops::SetExport { sheet: id, export }
        )],
        "Dışa aktarma"
    ));
    d.update(Message::Export(ExportKind::Pdf));
    assert_eq!(
        suggested(&mut d, Message::Pdf(PdfMessage::Save)),
        "Ada 101 parsel 7.pdf"
    );
    // Several sheets in one PDF: the drawing's name.
    new_sheet(&mut d);
    d.update(Message::Export(ExportKind::Pdf));
    d.update(Message::Pdf(PdfMessage::Scope(PdfScope::All)));
    assert_eq!(
        suggested(&mut d, Message::Pdf(PdfMessage::Save)),
        "Ada 101 paftaları.pdf"
    );
    assert_eq!(
        suggested(&mut d, Message::Export(ExportKind::Kpafta)),
        "Ada 101.kcad.kpafta"
    );
}

/// Sığdır, Gerçek and Seçim draw the paper again at their zoom: the paper's layers are cached at
/// the zoom they were drawn at, and a ribbon's zoom (not only a wheel's) empties them.
#[test]
fn a_ribbon_zoom_draws_the_paper_again() {
    let mut d = designer();
    let title = item(&d, "Başlık").id.clone();
    d.update(Message::Select(vec![title]));
    for m in [Message::ZoomSelection, Message::ZoomReal, Message::ZoomPage] {
        let before = d.paper_resets();
        d.update(m.clone());
        assert!(
            d.paper_resets() > before,
            "{m:?} empties the paper's caches"
        );
    }
}

/// A north arrow's declination in the inspector (design §8a; the web's `northSection`): what the
/// paper writes and from what (the core's `north_info`), the date it is for and where that comes
/// from; “Sapmayı elle gir” starts from the model's value to the minute when nothing was typed,
/// and then the value and its year are fields of their own.
#[test]
fn the_inspector_shows_the_north_arrow_s_declination_and_types_it_by_hand() {
    use kentos_sheet::display::{CrsInfo, DateSource, DeclinationSource};
    use kentos_sheet::geodesy::TmParams;
    use kentos_sheet::kinds::{ItemKind, NorthKind};
    use kentos_sheet::ops::{Op, SetItemProps};
    use kentos_sheet_ui::message::NorthMessage;
    let mut ctx = context();
    ctx.crs = Some(CrsInfo {
        name: "TUREF / TM33".into(),
        tm: Some(TmParams {
            central_meridian: 33.0,
            scale_factor: 1.0,
            false_easting: 500_000.0,
            false_northing: 0.0,
            semi_major: 6_378_137.0,
            inverse_flattening: 298.257_222_101,
        }),
    });
    ctx.center = Some(GroundPoint {
        x: 487_150.0,
        y: 4_417_820.0,
    });
    ctx.project.date = "2026-10-03".into();
    let mut d = Designer::new(ctx);
    new_sheet(&mut d);
    let arrow = d
        .open_sheet()
        .unwrap()
        .items
        .iter()
        .find(|i| matches!(i.kind, ItemKind::NorthArrow(_)))
        .map(|i| i.id.clone())
        .expect("the template's north arrow");
    let kind = |d: &Designer| match &d.book().item(&arrow).unwrap().kind {
        ItemKind::NorthArrow(k) => k.clone(),
        _ => unreachable!(),
    };
    d.update(Message::Select(vec![arrow.clone()]));
    // Grid north: nothing of the declination.
    assert!(kind(&d).north != NorthKind::Magnetic && d.north_info().is_none());
    assert!(d.apply(
        vec![Op::SetItemProps(SetItemProps {
            id: arrow.clone(),
            patch: serde_json::json!({ "kind": { "type": "northArrow", "north": "magnetic" } }),
        })],
        "Kuzey",
    ));
    let info = d.north_info().expect("what it shows").clone();
    assert_eq!(
        (
            info.source,
            info.date_source,
            info.date.as_deref(),
            info.in_model
        ),
        (
            Some(DeclinationSource::Model),
            Some(DateSource::Today),
            Some("2026-10-03"),
            Some(true)
        )
    );
    let model = info.declination.expect("the model's value");
    assert!((4.0..7.0).contains(&model), "Çankaya, 2026: {model}");
    // The value typed and its year are not fields while it comes from the model.
    assert_eq!(
        d.kind_field_names(),
        ["Harita", "Kuzey", "Biçim", "Yakınsama notu", "Renk"]
    );
    // By hand: from the model's value to the minute (nothing was typed), then its fields.
    d.update(Message::North(NorthMessage::Hand(true)));
    let k = kind(&d);
    assert_eq!(k.declination_hand, Some(true));
    assert_eq!(
        k.declination,
        ((model * 60.0).round() / 60.0 * 1000.0).round() as i32
    );
    assert_eq!(
        d.kind_field_names(),
        [
            "Harita",
            "Kuzey",
            "Biçim",
            "Elle sapma",
            "Sapmanın yılı",
            "Yakınsama notu",
            "Renk"
        ]
    );
    assert_eq!(
        d.north_info().map(|i| i.source),
        Some(Some(DeclinationSource::Hand))
    );
    d.update(Message::North(NorthMessage::Declination(-2.17)));
    d.update(Message::North(NorthMessage::Year(2024.0)));
    let k = kind(&d);
    assert_eq!((k.declination, k.declination_year), (-2170, Some(2024)));
    assert_eq!(d.north_info().and_then(|i| i.hand_year), Some(2024));
    // Off and on again: what was typed is kept.
    d.update(Message::North(NorthMessage::Hand(false)));
    assert_eq!(kind(&d).declination_hand, Some(false));
    d.update(Message::North(NorthMessage::Hand(true)));
    assert_eq!(kind(&d).declination, -2170);
}

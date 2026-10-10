//! The label engine of docs/adr/0212 in a GIS project (`label-engine.kcad`,
//! written by `scripts/fixtures/label_engine_scene.py`): a quarter's name
//! along its boundary, blocks, parcels with rule-based classes (the number,
//! the owner's name shortened when needed, a sliver's number outside with a
//! callout, one pinned by hand), buildings as obstacles, a street in three
//! pieces named along it, contours read uphill and survey points named
//! around them. The web's are `shots.mjs labels`. `tools_screens` takes them
//! in the dark and the light theme at 1440×900 and 1100×650:
//!
//! ```text
//! KENTOS_SHOTS_ONLY=etiket-sahne,etiket-yakin cargo test -p kentos-desktop tools_screens -- --ignored --nocapture
//! ```
//!
//! Etiketler's window (`etiket-pencere-*`: Kurallı's classes and the five
//! tabs, a layer that only blocks) and the label tools over the scene
//! (`etiket-tasi`, `etiket-dondur`, `etiket-gizle`, `etiket-yerlesmeyen`,
//! `etiket-sabit`).
//!
//! Test code only.

use iced::Point;
use kentos_domain::Slot;
use kentos_ui::snapshot::Snapshot;

use crate::app::{App, Message};
use crate::document::Document;
use crate::labelling::{Event as Window, Tab};
use crate::tools_screens::Pointed;
use crate::viewport::Event;

pub(crate) const DRAWING: &str = include_str!("../../../fixtures/interaction/v1/label-engine.kcad");
const X0: f64 = 487_000.0;
const Y0: f64 = 4_420_000.0;

/// The drawing open, the CBS ribbon's Harita on.
pub(crate) fn opened(app: &mut App) {
    let snapshot =
        kentos_contracts::DocumentSnapshotV1::from_json(DRAWING).expect("the drawing reads");
    let doc = Document::new(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    app.tab = "map";
    app.command_expanded = false;
}

/// The view on the box (relative to the scene's origin), once the drawing area has its size.
pub(crate) fn fit(s: &mut Snapshot, app: &mut App, b: [f64; 4]) {
    let mut update = |app: &mut App, message: Message| {
        let _ = app.update(message);
    };
    s.settle(app, App::view, &mut update);
    app.viewport.camera.fit(
        &kentos_render_wgpu::Bounds {
            min_x: X0 + b[0],
            min_y: Y0 + b[1],
            max_x: X0 + b[2],
            max_y: Y0 + b[3],
        },
        24.0,
    );
    s.settle(app, App::view, &mut update);
}

/// The whole scene.
pub(crate) fn whole(s: &mut Snapshot, app: &mut App) {
    fit(s, app, [-15.0, -15.0, 295.0, 180.0]);
}

/// Closer in: the first block and the street over it.
pub(crate) fn close(s: &mut Snapshot, app: &mut App) {
    fit(s, app, [0.0, 0.0, 150.0, 110.0]);
}

/// Etiketler over a layer, a class chosen and a tab open.
fn window(app: &mut App, layer: &str, class: usize, tab: Tab) {
    opened(app);
    app.open_labelling(Some(layer.to_owned()));
    let _ = app.update(Message::Labelling(Window::Class(class)));
    let _ = app.update(Message::Labelling(Window::Tab(tab)));
}

/// Parsel's rules: the owner's class chosen, its condition and text.
fn window_rules(app: &mut App) {
    window(app, "parsel", 1, Tab::Text);
}

fn window_place(app: &mut App) {
    window(app, "parsel", 0, Tab::Place);
}

fn window_look(app: &mut App) {
    window(app, "parsel", 0, Tab::Look);
}

fn window_fit(app: &mut App) {
    window(app, "parsel", 1, Tab::Fit);
}

/// Eşyükselti's single label: what goes first, overlaps, duplicates.
fn window_order(app: &mut App) {
    window(app, "esyukselti", 0, Tab::Order);
}

/// Bina: no labels of its own, its objects an obstacle to the others.
fn window_obstacle(app: &mut App) {
    window(app, "bina", 0, Tab::Text);
}

/// Where a world point is on the drawing area.
fn at(app: &App, x: f64, y: f64) -> Point {
    let [sx, sy] = app
        .viewport
        .camera
        .world_to_screen(kentos_render_wgpu::Vec2::new(x, y));
    Point::new(sx as f32, sy as f32)
}

fn hover(app: &mut App, p: Point) {
    let _ = app.update(Message::Viewport(Event::Moved(p)));
}

fn click(app: &mut App, p: Point) {
    hover(app, p);
    let _ = app.update(Message::Viewport(Event::Pressed(p)));
    let _ = app.update(Message::Viewport(Event::Released(p)));
}

/// The middle of the widest placed label of a layer's objects on screen, as the main view last drew it.
fn label_on(app: &App, layer: &str) -> (f64, f64) {
    let doc = app.document.as_ref().expect("a drawing");
    // Those on screen, a little in from its edges.
    let b = app.viewport.camera.visible_bounds();
    let (dx, dy) = ((b.max_x - b.min_x) * 0.1, (b.max_y - b.min_y) * 0.1);
    let all = app.spatial.labels_in(
        kentos_interaction::Vec2::new(b.min_x + dx, b.min_y + dy),
        kentos_interaction::Vec2::new(b.max_x - dx, b.max_y - dy),
        false,
    );
    let hit = all
        .iter()
        .filter(|h| {
            doc.model
                .get(Slot(h.id as u32))
                .is_some_and(|e| e.base().layer_id == layer)
        })
        .max_by(|a, b| a.w.total_cmp(&b.w))
        .unwrap_or_else(|| panic!("a label on {layer}"));
    (hit.at.x, hit.at.y)
}

fn settle(s: &mut Snapshot, app: &mut App) {
    let mut update = |app: &mut App, message: Message| {
        let _ = app.update(message);
    };
    s.settle(app, App::view, &mut update);
}

/// Etiketi taşı: a parcel's number taken, the pointer to the right with it.
fn moving(s: &mut Snapshot, app: &mut App) {
    close(s, app);
    let _ = app.update(Message::Run("tool.labelMove"));
    let (x, y) = label_on(app, "parsel");
    click(app, at(app, x, y));
    hover(app, at(app, x + 9.0, y - 6.0));
    settle(s, app);
}

/// Etiketi döndür: the street's name taken, the pointer turning it.
fn turning(s: &mut Snapshot, app: &mut App) {
    close(s, app);
    let _ = app.update(Message::Run("tool.labelRotate"));
    let (x, y) = label_on(app, "yol");
    click(app, at(app, x, y));
    hover(app, at(app, x + 14.0, y + 9.0));
    settle(s, app);
}

/// Etiketi gizle: a parcel's number hidden, then Göster (G): the hidden ones drawn faded.
fn hiding(s: &mut Snapshot, app: &mut App) {
    close(s, app);
    let _ = app.update(Message::Run("tool.labelHide"));
    let (x, y) = label_on(app, "parsel");
    click(app, at(app, x, y));
    settle(s, app);
    let _ = app.update(Message::PromptOption("G"));
    hover(app, at(app, x + 20.0, y + 20.0));
    settle(s, app);
}

/// Yerleşmeyen etiketleri göster: farther out, those that found no room red where they would go.
fn unplaced(s: &mut Snapshot, app: &mut App) {
    let _ = app.update(Message::Run("view.unplacedLabels"));
    fit(s, app, [-60.0, -45.0, 350.0, 230.0]);
}

/// Sabit etiketleri vurgula: the hand-pinned one framed.
fn pinned(s: &mut Snapshot, app: &mut App) {
    let _ = app.update(Message::Run("view.pinnedLabels"));
    close(s, app);
}

pub(crate) fn pointed() -> Vec<Pointed> {
    vec![
        // The whole scene: every mode at work.
        ("etiket-sahne", opened, whole),
        // Closer in: the pinned number and its callout, the owners' names, the sliver's number.
        ("etiket-yakin", opened, close),
        // Etiketler: Kurallı's classes, the owner's condition and text.
        ("etiket-pencere", window_rules, whole),
        ("etiket-pencere-yerlesim", window_place, whole),
        ("etiket-pencere-bicim", window_look, whole),
        ("etiket-pencere-sigdirma", window_fit, whole),
        ("etiket-pencere-oncelik", window_order, whole),
        ("etiket-pencere-engel", window_obstacle, whole),
        // The tools.
        ("etiket-tasi", opened, moving),
        ("etiket-dondur", opened, turning),
        ("etiket-gizle", opened, hiding),
        ("etiket-yerlesmeyen", opened, unplaced),
        ("etiket-sabit", opened, pinned),
    ]
}

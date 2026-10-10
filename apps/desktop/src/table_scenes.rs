//! The pictures of docs/adr/0184 (Tablo) in a CAD project: a parcel and its
//! corner points beside their coordinate schedule (written by the core's
//! rule, its heading bold and centred, its numbers right) and an area
//! schedule under a merged title; Tablo ekle with each source, its placement
//! hanging from the pointer, Tabloyu düzenle with a range chosen, Öznitelikler's
//! rows. The web's are `shots.mjs tables`.
//! `tools_screens` takes them in the dark and the light theme at 1440×900
//! and 1100×650:
//!
//! ```text
//! KENTOS_SHOTS_ONLY=tablo-cizim cargo test -p kentos-desktop tools_screens -- --ignored --nocapture
//! ```
//!
//! Test code only.

use kentos_contracts::{EntityGeometry, TextFace};
use kentos_domain::Slot;
use kentos_geometry_core::ops::table::ScheduleKind;
use kentos_interaction::table;

use crate::app::{App, Message};
use crate::files_testing::make_cad;
use crate::tables::editor::Event as Edit;
use crate::tables::insert::{Event as Insert, Lines, Source};
use crate::tools_scenes::{E, N, Objects, hover, open, run};
use crate::tools_screens::Scene;

/// The parcel's corners, and the neighbour's.
const PARCEL: [[f64; 2]; 5] = [
    [0.0, 0.0],
    [42.5, -3.25],
    [47.0, 24.0],
    [18.0, 31.5],
    [-2.0, 22.0],
];
const NEIGHBOUR: [[f64; 2]; 4] = [[42.5, -3.25], [71.0, -6.0], [74.5, 19.0], [47.0, 24.0]];

/// The parcels, their numbered corners (101 …) with elevations, a road.
fn ground() -> Objects {
    let mut o = Objects::new();
    let a = o.path("parsel", &PARCEL, true);
    o.data(
        a,
        &[("Ada", "1043"), ("Parsel", "7"), ("Nitelik", "Arsa")],
        Some("7"),
    );
    let b = o.path("parsel", &NEIGHBOUR, true);
    o.data(
        b,
        &[("Ada", "1043"), ("Parsel", "8"), ("Nitelik", "Bahçe")],
        Some("8"),
    );
    for (k, p) in PARCEL.iter().enumerate() {
        let id = o.point("cizim", *p, Some(812.4 + k as f64 * 0.35));
        o.data(id, &[], Some(&format!("{}", 101 + k)));
    }
    o.line("yol", [-8.0, -12.0], [80.0, -18.0]);
    o
}

/// A table of the core's schedule of `kind` over `slots`, at `p` (east, north),
/// its frame `frame` wide when given.
fn place(
    app: &mut App,
    kind: ScheduleKind,
    slots: &[u32],
    p: [f64; 2],
    height: f64,
    frame: Option<f64>,
) {
    let doc = app.document.as_mut().expect("a drawing");
    let format = kentos_interaction::Format::of(doc.settings());
    let slots: Vec<Slot> = slots.iter().map(|s| Slot(*s)).collect();
    let cells = table::schedule(&doc.model, kind, &slots, &format);
    assert!(cells.problem.is_none(), "{:?}", cells.problem);
    let source = table::source_of(&doc.model, kind, &slots);
    let look = table::Look {
        header: true,
        height,
        face: TextFace::default(),
        grid: None,
        frame,
    };
    let geometry = table::new_table(
        &doc.model,
        &cells,
        &look,
        kentos_interaction::Vec2::new(E + p[0], N + p[1]),
        Some(source),
    );
    let entity = kentos_native_application::geometry::entity_of(
        &geometry,
        kentos_contracts::EntityBase {
            id: 0,
            layer_id: "cizim".into(),
            color: None,
            attrs: Default::default(),
            label: None,
            symbol: None,
            line_weight: None,
            label_pins: Vec::new(),
        },
    );
    let _ = doc.model.add_many(vec![entity], "Tablo");
}

/// An area schedule under a merged title row: the title spans its columns.
fn titled(app: &mut App, p: [f64; 2], height: f64) {
    place(app, ScheduleKind::Areas, &[1, 2], p, height, Some(0.3));
    let doc = app.document.as_mut().expect("a drawing");
    let slot = Slot(u32::try_from(doc.model.next_slot() - 1).expect("a slot"));
    let Some(kentos_contracts::Entity::Table(mut t)) = doc.model.get(slot).cloned() else {
        panic!("the area table");
    };
    let m = t.columns.len();
    t.cells.insert(0, {
        let mut row = vec![String::new(); m];
        row[0] = "1043 ada parselleri".into();
        row
    });
    t.rows.insert(0, t.rows[0]);
    t.merges = vec![kentos_contracts::CellRange {
        row: 0,
        col: 0,
        rows: 1,
        cols: m as u32,
    }];
    let base = t.base.clone();
    let geometry = EntityGeometry::Table {
        p: t.p,
        rotation: t.rotation,
        height: t.height,
        rows: t.rows,
        columns: t.columns,
        cells: t.cells,
        merges: t.merges,
        aligns: t.aligns,
        header: false,
        grid: None,
        frame: t.frame,
        face: t.face,
        source: t.source,
    };
    let entity = kentos_native_application::geometry::entity_of(&geometry, base);
    let _ = doc.model.update(slot, entity);
}

/// The ground in a CAD project at 1:500, its schedules beside it.
fn drawn(app: &mut App) {
    open(app, ground());
    make_cad(app);
    {
        let doc = app.document.as_mut().expect("a drawing");
        let mut settings = doc.settings().clone();
        settings.plot_scale = 500.0;
        doc.model.set_settings(settings);
    }
    place(
        app,
        ScheduleKind::Coordinates,
        &[3, 4, 5, 6, 7],
        [84.0, 30.0],
        1.25,
        None,
    );
    titled(app, [84.0, 2.0], 1.25);
    // The geometry store follows the drawing, as an update leaves it.
    if let Some(doc) = app.document.as_ref() {
        app.spatial.sync(&doc.model);
    }
    app.tab = "annotate";
    app.command_expanded = false;
    app.viewport.camera.fit(
        &kentos_render_wgpu::Bounds {
            min_x: E - 10.0,
            min_y: N - 22.0,
            max_x: E + 132.0,
            max_y: N + 36.0,
        },
        24.0,
    );
}

/// Tablo ekle over the drawing with the two parcels and their corners
/// selected, `source` chosen.
fn insert(app: &mut App, source: Source) {
    drawn(app);
    app.selection.set([1, 2, 3, 4, 5, 6, 7].map(Slot));
    run(app, "table.insert");
    let _ = app.update(Message::TableInsert(Insert::Source(source)));
}

fn insert_areas(app: &mut App) {
    insert(app, Source::Areas);
    let _ = app.update(Message::TableInsert(Insert::Frame(true)));
}

fn insert_coordinates(app: &mut App) {
    insert(app, Source::Coordinates);
    let _ = app.update(Message::TableInsert(Insert::Lines(Lines::Rows)));
}

fn insert_attributes(app: &mut App) {
    insert(app, Source::Attributes);
    let _ = app.update(Message::TableInsert(Insert::Lines(Lines::All)));
}

fn insert_blank(app: &mut App) {
    insert(app, Source::Blank);
}

/// Yerleştir: the area schedule hangs from the pointer above the parcels.
fn placing(app: &mut App) {
    insert_areas(app);
    let _ = app.update(Message::TableInsert(Insert::Place));
    hover(app, [50.0, 35.0]);
}

/// Tabloyu düzenle on the coordinate schedule, B2:C3 chosen.
fn editing(app: &mut App) {
    drawn(app);
    let slot = table_slot(app);
    let _ = app.open_table_editor(slot);
    let _ = app.update(Message::TableEditor(Edit::Cell(1, 1)));
    let _ = app.update(Message::TableEditor(Edit::Release));
    app.modifiers = iced::keyboard::Modifiers::SHIFT;
    let _ = app.update(Message::TableEditor(Edit::Cell(2, 2)));
    let _ = app.update(Message::TableEditor(Edit::Release));
    app.modifiers = iced::keyboard::Modifiers::default();
}

/// The coordinate schedule selected: Öznitelikler's rows.
fn properties(app: &mut App) {
    drawn(app);
    let slot = table_slot(app);
    app.selection.set([slot]);
}

/// The first table of the drawing.
fn table_slot(app: &App) -> Slot {
    let doc = app.document.as_ref().expect("a drawing");
    doc.model
        .entities()
        .find(|e| matches!(e, kentos_contracts::Entity::Table(_)))
        .map(|e| Slot(e.base().id))
        .expect("a table")
}

pub(crate) fn scenes() -> Vec<Scene> {
    vec![
        ("tablo-cizim", drawn),
        ("tablo-ekle-alan", insert_areas),
        ("tablo-ekle-koordinat", insert_coordinates),
        ("tablo-ekle-oznitelik", insert_attributes),
        ("tablo-ekle-bos", insert_blank),
        ("tablo-yerlestir", placing),
        ("tablo-duzenle", editing),
        ("tablo-oznitelikler", properties),
    ]
}

//! The pictures of docs/adr/0728 (geçici ölçünün kalanı): a 6 × 4.5 m room of four 20 cm walls on grids A, C (x 0, 6)
//! and 1, 2 (y 0, 4.5), grid B at x 3 through it, a 10 cm partition at x 2.2, a column and a door. The partition
//! selected: each side's face with a square, a side switched to grid B, the partition moved 0.60 m from it; the south
//! wall's length, its start solid and a square at its end, then made 7 m (the east wall shifting with it); the column's
//! and the door's grids:
//!
//! ```text
//! KENTOS_SHOTS_ONLY=gecici-aks-adaylar,gecici-aks-secildi,gecici-aks-yazildi,gecici-boy,gecici-boy-yazildi,gecici-kolon-aks,gecici-acik-aks cargo test -p kentos-desktop --bin kentos-cad tools_screens::screens -- --ignored --nocapture
//! ```
//!
//! Test code only.

use kentos_domain::Slot;
use serde_json::{Value, json};

use crate::app::{App, Message};
use crate::files_testing::make_cad;
use crate::tools_scenes::{E, N, Objects, click, hover, layer, open_edited};
use crate::tools_screens::Scene;

fn xy(x: f64, y: f64) -> Value {
    json!({ "x": E + x, "y": N + y })
}

fn command(app: &mut App, name: &str, input: Value) -> Value {
    let doc = app.document.as_mut().expect("a drawing");
    let answer = kentos_native_application::registry::run(
        &mut kentos_native_application::ExecutionContext::new(&mut doc.model),
        name,
        kentos_commands::CallOp::Execute,
        input,
    )
    .expect("runs");
    assert_eq!(answer["status"], "completed", "{name}: {answer}");
    answer
}

/// The room: south (1), east (2), north (3), west (4) on their axes, the partition (5), grids A, B, C, 1, 2 (6–10) and
/// a 40 cm column (11) at (4.6, 2.2).
fn site(app: &mut App) {
    let mut o = Objects::new();
    for pts in [
        [(0.0, 0.0), (6.0, 0.0)],
        [(6.0, 0.0), (6.0, 4.5)],
        [(6.0, 4.5), (0.0, 4.5)],
        [(0.0, 4.5), (0.0, 0.0)],
    ] {
        o.push(
            "duvar",
            json!({"kind": "wall", "pts": pts.iter().map(|&(x, y)| xy(x, y)).collect::<Vec<_>>(), "width": 0.2,
                   "justify": "center"}),
        );
    }
    o.push(
        "duvar",
        json!({"kind": "wall", "pts": [xy(2.2, 0.0), xy(2.2, 4.5)], "width": 0.1, "justify": "center"}),
    );
    for (name, a, b) in [
        ("A", (0.0, -1.5), (0.0, 6.0)),
        ("B", (3.0, -1.5), (3.0, 6.0)),
        ("C", (6.0, -1.5), (6.0, 6.0)),
        ("1", (-1.5, 0.0), (7.5, 0.0)),
        ("2", (-1.5, 4.5), (7.5, 4.5)),
    ] {
        o.push("aks", json!({"kind": "grid", "pts": [xy(a.0, a.1), xy(b.0, b.1)], "name": name}));
    }
    o.push("kolon", json!({"kind": "column", "at": xy(4.6, 2.2), "width": 0.4}));
    open_edited(app, o, |d| {
        d["layers"] = json!([
            layer("duvar", "Duvar", "fg", 0.5),
            layer("aks", "Aks", "#8E8C99", 0.18),
            layer("kolon", "Kolon", "fg", 0.35),
        ]);
        d["settings"]["plotScale"] = json!(50);
    });
    make_cad(app);
    app.draft.snap = false;
    // The tests run with no window laid out: the area the size the pictures give it, so that the temporary
    // dimensions' pixel offsets are what they are on screen.
    if app.viewport.camera.width <= 1.0 {
        let _ = app.update(Message::Viewport(crate::viewport::Event::Resized(iced::Rectangle::new(
            iced::Point::ORIGIN,
            iced::Size::new(1000.0, 700.0),
        ))));
    }
    app.viewport.camera.fit(
        &kentos_render_wgpu::Bounds {
            min_x: E - 1.8,
            min_y: N - 2.0,
            max_x: E + 7.8,
            max_y: N + 6.2,
        },
        24.0,
    );
}

fn screen_click(app: &mut App, p: kentos_interaction::Vec2) {
    click(app, [p.x - E, p.y - N]);
}

/// The partition selected, the pointer by it: each side's face, a square at each (a grid there too); its length, the
/// start solid.
fn candidates(app: &mut App) {
    site(app);
    app.selection.set([Slot(5)]);
    hover(app, [2.3, 2.6]);
}

/// The east side's square clicked: that side measured from the partition's axis to grid B.
fn chosen(app: &mut App) {
    candidates(app);
    let east = app
        .session
        .temporary_squares()
        .into_iter()
        .max_by(|a, b| a.x.total_cmp(&b.x))
        .expect("the east side's square");
    screen_click(app, east);
    hover(app, [2.3, 2.6]);
}

/// 0.60 typed on the grid's number: the partition's axis 60 cm west of grid B.
fn written(app: &mut App) {
    chosen(app);
    let east = app
        .session
        .temporary_numbers()
        .into_iter()
        .max_by(|a, b| a.x.total_cmp(&b.x))
        .expect("the grid's number");
    screen_click(app, east);
    let _ = app.update(Message::TextField(crate::text_field::Event::Input("0.60".into())));
    let _ = app.update(Message::TextField(crate::text_field::Event::Keep));
    hover(app, [2.5, 2.6]);
}

/// The south wall selected, the pointer above it: its length below it, the start solid, a square at its end.
fn length(app: &mut App) {
    site(app);
    app.selection.set([Slot(1)]);
    hover(app, [3.6, 0.4]);
}

/// 7 typed on its length: the end 1 m east, the east wall shifting with the corner, the north wall following.
fn lengthened(app: &mut App) {
    length(app);
    let at = app.session.temporary_length().expect("the length's number");
    screen_click(app, at);
    let _ = app.update(Message::TextField(crate::text_field::Event::Input("7".into())));
    let _ = app.update(Message::TextField(crate::text_field::Event::Keep));
    app.viewport.camera.fit(
        &kentos_render_wgpu::Bounds {
            min_x: E - 1.8,
            min_y: N - 2.0,
            max_x: E + 8.8,
            max_y: N + 6.2,
        },
        24.0,
    );
    hover(app, [3.6, 0.4]);
}

/// The column selected, its east side switched to grid C: from its middle to the grid.
fn column(app: &mut App) {
    site(app);
    app.selection.set([Slot(11)]);
    hover(app, [4.6, 2.2]);
    let east = app
        .session
        .temporary_squares()
        .into_iter()
        .max_by(|a, b| a.x.total_cmp(&b.x))
        .expect("the east side's square");
    screen_click(app, east);
    hover(app, [4.6, 2.2]);
}

/// A door in the south wall, its middle at 4.1 m; its start side switched to grid B: from its middle to the grid.
fn door(app: &mut App) {
    site(app);
    let model = &app.document.as_ref().expect("a drawing").model;
    let south = model.uid(Slot(1)).expect("the south wall").to_string();
    let made = command(
        app,
        "cad.openings.create",
        json!({"layerId": "duvar", "openings": [{"host": south, "at": 4.1, "number": "K1"}]}),
    );
    let slot = Slot(made["output"]["ids"][0].as_u64().expect("its slot") as u32);
    app.selection.set([slot]);
    hover(app, [4.1, 0.6]);
    let start = app
        .session
        .temporary_squares()
        .into_iter()
        .min_by(|a, b| a.x.total_cmp(&b.x))
        .expect("the start side's square");
    screen_click(app, start);
    hover(app, [4.1, 0.6]);
}

pub(crate) fn scenes() -> Vec<Scene> {
    vec![
        ("gecici-aks-adaylar", candidates),
        ("gecici-aks-secildi", chosen),
        ("gecici-aks-yazildi", written),
        ("gecici-boy", length),
        ("gecici-boy-yazildi", lengthened),
        ("gecici-kolon-aks", column),
        ("gecici-acik-aks", door),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::files_testing::app_with_drawing;
    use kentos_commands::Entity;

    fn wall_pts(app: &App, slot: Slot) -> Vec<(f64, f64)> {
        match app.document.as_ref().expect("a drawing").model.get(slot) {
            Some(Entity::Wall(w)) => w.wall.pts.iter().map(|p| (p.x - E, p.y - N)).collect(),
            _ => panic!("a wall"),
        }
    }

    fn texts(app: &App) -> Vec<String> {
        app.session
            .preview(&Default::default())
            .map(|p| p.texts.into_iter().map(|t| t.text).collect())
            .unwrap_or_default()
    }

    fn close(a: (f64, f64), x: f64, y: f64) -> bool {
        (a.0 - x).abs() < 1e-6 && (a.1 - y).abs() < 1e-6
    }

    /// The walkthrough as the pictures take it (docs/adr/0728): the partition's sides switch to grid B and it moves
    /// from it; the south wall made 7 m takes the east wall along; the column's and the door's grids.
    #[test]
    fn temporary_distances_go_to_grids_and_lengths_as_the_pictures_show() {
        let mut app = app_with_drawing();
        candidates(&mut app);
        let squares = app.session.temporary_squares();
        assert_eq!(
            squares.iter().filter(|q| (q.x - (E + 2.2)).abs() > 1.0).count(),
            2,
            "both sides have a face and a grid (the third square, at its end, the length's): {squares:?}"
        );

        let mut app = app_with_drawing();
        chosen(&mut app);
        assert!(texts(&app).iter().any(|t| t.contains("0.800")), "the axis to grid B: {:?}", texts(&app));

        let mut app = app_with_drawing();
        written(&mut app);
        let p = wall_pts(&app, Slot(5));
        assert!(
            close(p[0], 2.4, 0.0) && close(p[1], 2.4, 4.5),
            "the axis 0.60 from grid B: {p:?}; {:?}; {}",
            texts(&app),
            crate::files_testing::last_said(&app)
        );
        assert!(texts(&app).iter().any(|t| t.contains("0.600")), "grid B kept after the edit: {:?}", texts(&app));

        let mut app = app_with_drawing();
        length(&mut app);
        assert!(app.session.temporary_length().is_some(), "the south wall's length");
        assert!(texts(&app).iter().any(|t| t.contains("6.000")), "{:?}", texts(&app));
        let squares = app.session.temporary_squares();
        assert!(
            squares.iter().any(|q| (q.x - (E + 6.0)).abs() < 1e-6 && q.y < N),
            "a square at its end on the length's line, below the wall, the start solid: {squares:?}"
        );

        let mut app = app_with_drawing();
        lengthened(&mut app);
        assert!(close(wall_pts(&app, Slot(1))[1], 7.0, 0.0), "{:?}", wall_pts(&app, Slot(1)));
        let east = wall_pts(&app, Slot(2));
        assert!(close(east[0], 7.0, 0.0) && close(east[1], 7.0, 4.5), "the east wall shifted: {east:?}");
        assert!(close(wall_pts(&app, Slot(3))[0], 7.0, 4.5), "the north wall followed");
        assert!(close(wall_pts(&app, Slot(5))[0], 2.2, 0.0), "the partition stays");

        let mut app = app_with_drawing();
        column(&mut app);
        assert!(texts(&app).iter().any(|t| t.contains("1.400")), "middle to grid C: {:?}", texts(&app));

        let mut app = app_with_drawing();
        door(&mut app);
        assert!(texts(&app).iter().any(|t| t.contains("1.100")), "the door's middle to grid B: {:?}", texts(&app));
    }
}

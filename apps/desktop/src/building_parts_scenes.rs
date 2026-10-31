//! The pictures of docs/adr/0771 (Yapı bileşenleri ve bağlı sınırlar): a 7 × 5 m house of an insulated outer wall
//! whose plaster and insulation turn at the ends and the window's reveal, an inner wall teeing in, a garden wall with a
//! free end; a floor slab bound to the walls' outer faces and a drawn balcony with its railing. The plan; the east wall
//! moved 50 cm out (the bound floor grows, the balcony stays); the window and the tee at 1:20, 1:50 and 1:100; the bound
//! slab broken when a wall goes; Öznitelikler's Sınır, Kalınlıkta sabit and node rows; Duvar stilleri's Döner column.
//! `tools_screens` takes them in both themes at 1440×900 and 1100×650:
//!
//! ```text
//! KENTOS_SHOTS_ONLY=yapi-bilesen-plan,yapi-bilesen-kaydirildi,yapi-bilesen-1-20,yapi-bilesen-1-50,yapi-bilesen-1-100,yapi-bilesen-kopuk,yapi-bilesen-doseme-oznitelik,yapi-bilesen-duvar-oznitelik,yapi-bilesen-stiller cargo test -p kentos-desktop --lib tools_screens::screens -- --ignored --nocapture
//! ```
//!
//! Test code only.

use kentos_commands::Entity;
use kentos_domain::Slot;
use serde_json::{Value, json};

use crate::app::App;
use crate::tools_scenes::{E, N, Objects, hover, run};
use crate::tools_screens::Scene;
use crate::wall_scenes::opened;

const STYLE: &str = "019a0000-0000-7000-8000-0000000a0771";

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

fn p(x: f64, y: f64) -> Value {
    json!({"x": E + x, "y": N + y})
}

fn layer(app: &App) -> String {
    app.document
        .as_ref()
        .expect("a drawing")
        .model
        .layers()
        .active()
        .to_owned()
}

/// The persistent id of the `k`-th object of `kind` in the drawing's order.
fn uid_of(app: &App, kind: &str, k: usize) -> String {
    let model = &app.document.as_ref().expect("a drawing").model;
    let slot = model
        .entities()
        .filter(|e| e.kind() == kind)
        .nth(k)
        .map(|e| Slot(e.base().id))
        .expect("the object");
    model.uid(slot).expect("its id").to_string()
}

fn slot_of(app: &App, kind: &str, k: usize) -> Slot {
    let model = &app.document.as_ref().expect("a drawing").model;
    model
        .entities()
        .filter(|e| e.kind() == kind)
        .nth(k)
        .map(|e| Slot(e.base().id))
        .expect("the object")
}

/// The house's style, walls, window and slabs, written through the commands; `p` places a point of the plan.
fn build(app: &mut App, p: impl Fn(f64, f64) -> Value) {
    let comp = |name: &str,
                t: Option<f64>,
                hatch: Option<&str>,
                detail: &str,
                priority: u8,
                wrap: bool| {
        let mut c = json!({"name": name, "material": name, "detail": detail, "priority": priority});
        if let Some(t) = t {
            c["thickness"] = json!(t);
        }
        if let Some(h) = hatch {
            c["hatch"] = json!({"pattern": h, "scale": 1.0, "angle": 0.0});
        }
        if wrap {
            c["wrap"] = json!(true);
        }
        c
    };
    command(
        app,
        "cad.styles.edit",
        json!({"kind": "wall", "operation": "create", "wall": {
            "id": STYLE, "name": "Yalıtımlı dış duvar 30",
            "components": [
                comp("İç sıva", Some(0.02), None, "detailed", 5, true),
                comp("Tuğla", None, Some("ANSI31"), "standard", 1, false),
                comp("Isı yalıtımı", Some(0.05), Some("ANSI37"), "standard", 3, true),
                comp("Dış sıva", Some(0.03), None, "detailed", 5, true),
            ],
            "fill": "#5A5A5A", "width": 0.3}}),
    );
    let lay = layer(app);
    command(
        app,
        "cad.walls.create",
        json!({"layerId": lay, "walls": [
            {"pts": [p(0.0, 0.0), p(7.0, 0.0), p(7.0, 5.0), p(0.0, 5.0)], "closed": true, "style": STYLE},
            {"pts": [p(4.2, 0.0), p(4.2, 5.0)], "style": "İç duvar 15"},
            {"pts": [p(7.0, 3.5), p(9.0, 3.5)], "style": STYLE}
        ]}),
    );
    let ring = uid_of(app, "wall", 0);
    command(
        app,
        "cad.openings.create",
        json!({"layerId": lay, "openings": [{"host": ring, "at": 2.0, "type": "Pencere 120"}]}),
    );
    command(
        app,
        "cad.slabs.create",
        json!({"layerId": lay, "slabs": [
            {"seed": p(2.0, 2.0), "bound": true},
            {"parts": [{"outer": {"pts": [p(1.0, -1.65), p(3.6, -1.65), p(3.6, -0.15), p(1.0, -0.15)]}}],
             "role": "balcony", "edge": {"kind": "railing"}}
        ]}),
    );
}

/// The house at 1:`scale` with the walls drawn at `detail`.
fn house(app: &mut App, scale: f64, detail: &str) {
    opened(app, Objects::new(), scale);
    build(app, p);
    let doc = app.document.as_mut().expect("a drawing");
    let mut settings = doc.model.settings().clone();
    let mut arch = settings.arch.clone().unwrap_or_default();
    arch.detail = Some(match detail {
        "detailed" => kentos_commands::WallDetail::Detailed,
        "schematic" => kentos_commands::WallDetail::Schematic,
        _ => kentos_commands::WallDetail::Standard,
    });
    settings.arch = Some(arch);
    doc.model.set_settings(settings);
    app.tab = "arch";
    app.selection.clear();
}

fn fit(app: &mut App, x0: f64, y0: f64, x1: f64, y1: f64) {
    app.viewport.camera.fit(
        &kentos_render_wgpu::Bounds {
            min_x: E + x0,
            min_y: N + y0,
            max_x: E + x1,
            max_y: N + y1,
        },
        24.0,
    );
    hover(app, [x1 + 5.0, y0 - 5.0]);
}

fn plan(app: &mut App) {
    house(app, 50.0, "standard");
    fit(app, -1.0, -2.2, 10.0, 6.0);
}

/// Kaydır: the east wall's edge 50 cm out; the bound floor grows with it in the same step, the balcony stays.
fn moved(app: &mut App) {
    house(app, 50.0, "standard");
    let ring = uid_of(app, "wall", 0);
    command(
        app,
        "cad.walls.edit",
        json!({"uids": [ring], "operation": {"kind": "shift", "edge": 1, "by": -0.5}}),
    );
    fit(app, -1.0, -2.2, 10.0, 6.0);
}

/// The window's reveal and the inner wall's tee, near, at 1:20 (every layer), 1:50 and 1:100.
fn near(app: &mut App, scale: f64, detail: &str) {
    house(app, scale, detail);
    fit(app, 1.0, -0.8, 4.8, 1.2);
}

fn at_20(app: &mut App) {
    near(app, 20.0, "detailed");
}

fn at_50(app: &mut App) {
    near(app, 50.0, "standard");
}

fn at_100(app: &mut App) {
    near(app, 100.0, "schematic");
}

/// The garden wall deleted and the ring's west edge's wall… the ring deleted: the bound floor keeps its outline, broken,
/// its mark above it; Öznitelikler says why.
fn broken(app: &mut App) {
    house(app, 50.0, "standard");
    let ring = uid_of(app, "wall", 0);
    command(app, "cad.entities.delete", json!({"uids": [ring]}));
    let slab = slot_of(app, "slab", 0);
    app.selection.set([slab]);
    rows_tall(app);
    fit(app, -1.0, -2.2, 10.0, 6.0);
}

/// Katmanlar folded so Öznitelikler's rows show.
fn rows_tall(app: &mut App) {
    let upper = kentos_ui::widget::docking::Slot::Docked(kentos_ui::widget::docking::Side::Right, 0);
    let _ = app.update(crate::app::Message::Dock(kentos_ui::widget::docking::Event::Collapsed(upper, true)));
}

fn slab_rows(app: &mut App) {
    house(app, 50.0, "standard");
    let slab = slot_of(app, "slab", 0);
    app.selection.set([slab]);
    rows_tall(app);
    fit(app, -1.0, -2.2, 10.0, 6.0);
}

/// The inner wall selected, its end's node decided by hand: Kalınlıkta sabit and the node rows.
fn wall_rows(app: &mut App) {
    house(app, 50.0, "standard");
    let inner = uid_of(app, "wall", 1);
    command(
        app,
        "cad.walls.edit",
        json!({"uids": [inner], "operation": {"kind": "joins", "startPass": [0, 0, 0]}}),
    );
    crate::properties::wall_parts::set_keep(kentos_commands::WallReference::LeftFace);
    let slot = slot_of(app, "wall", 1);
    app.selection.set([slot]);
    rows_tall(app);
    fit(app, -1.0, -2.2, 10.0, 6.0);
}

/// Duvar stilleri on the insulated wall: the Döner column.
fn styles(app: &mut App) {
    house(app, 50.0, "standard");
    run(app, kentos_interaction::wall::WALL_STYLES);
}

pub(crate) fn scenes() -> Vec<Scene> {
    vec![
        ("yapi-bilesen-plan", plan),
        ("yapi-bilesen-kaydirildi", moved),
        ("yapi-bilesen-1-20", at_20),
        ("yapi-bilesen-1-50", at_50),
        ("yapi-bilesen-1-100", at_100),
        ("yapi-bilesen-kopuk", broken),
        ("yapi-bilesen-doseme-oznitelik", slab_rows),
        ("yapi-bilesen-duvar-oznitelik", wall_rows),
        ("yapi-bilesen-stiller", styles),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::files_testing::app_with_drawing;

    fn slab(app: &App, k: usize) -> kentos_commands::SlabFields {
        let model = &app.document.as_ref().expect("a drawing").model;
        match model.get(slot_of(app, "slab", k)) {
            Some(Entity::Slab(s)) => s.slab.clone(),
            _ => panic!("a slab"),
        }
    }

    /// The acceptance of docs/adr/0771: the outer wall 50 cm out grows the bound floor, the drawn balcony and its
    /// railing stay, one undo takes the floor back; a wall gone leaves the floor's outline, broken.
    #[test]
    fn the_bound_floor_follows_the_outer_wall_and_the_balcony_stays() {
        let mut app = app_with_drawing();
        house(&mut app, 50.0, "standard");
        let area =
            |s: &kentos_commands::SlabFields| kentos_native_application::spaces::area_of(&s.parts);
        let (floor, balcony) = (slab(&app, 0), slab(&app, 1));
        // The ring 7.3 × 5.3 m to its outer faces and the garden wall's part outside it (1.85 × 0.3 m).
        assert!((area(&floor) - (7.3 * 5.3 + 1.85 * 0.3)).abs() < 1e-5, "{}", area(&floor));
        let ring = uid_of(&app, "wall", 0);
        command(
            &mut app,
            "cad.walls.edit",
            json!({"uids": [ring], "operation": {"kind": "shift", "edge": 1, "by": -0.5}}),
        );
        let grown = slab(&app, 0);
        // 50 cm more of the ring, 50 cm less of the garden wall outside it.
        assert!((area(&grown) - (7.8 * 5.3 + 1.35 * 0.3)).abs() < 1e-5, "{}", area(&grown));
        assert_eq!(slab(&app, 1), balcony, "the drawn balcony is its own");
        assert!(balcony.edge.as_ref().is_some_and(|e| !e.runs.is_empty()));
        let doc = app.document.as_mut().expect("a drawing");
        doc.model.undo();
        assert_eq!(
            slab(&app, 0),
            floor,
            "one undo takes the floor back with the wall"
        );
        let ring = uid_of(&app, "wall", 0);
        command(&mut app, "cad.entities.delete", json!({"uids": [ring]}));
        let lost = slab(&app, 0);
        assert_eq!(lost.parts, floor.parts);
        assert_eq!(
            lost.bound.and_then(|b| b.broken),
            Some(kentos_commands::SlabBreak::LostEnclosure)
        );
    }

    /// The plaster and the insulation turn at the window's reveal and the garden wall's free end: closing lines drawn.
    #[test]
    fn the_layers_turn_at_the_reveal_and_the_free_end() {
        let mut app = app_with_drawing();
        house(&mut app, 20.0, "detailed");
        let model = &app.document.as_ref().expect("a drawing").model;
        let walls: Vec<_> = model
            .entities()
            .filter_map(kentos_domain::space_follow::core_wall)
            .collect();
        let holes: Vec<Vec<kentos_geometry_core::geom::opening::Hole>> =
            vec![Vec::new(); walls.len()];
        let ins: Vec<_> = walls
            .iter()
            .map(|w| kentos_geometry_core::geom::wall_join::WallIn {
                pts: &w.pts,
                bulges: w.bulges.as_deref(),
                spec: &w.spec,
            })
            .collect();
        let solved = kentos_geometry_core::geom::wall_join::solve_with(&ins, &holes, None);
        let garden = solved.drawn[2].as_ref().expect("drawn");
        assert!(!garden.closures.is_empty(), "the free end closes");
    }
}

/// The window's reveal and the inner wall's tee on paper (docs/adr/0771's acceptance): the house on three sheets whose
/// maps stand at 1:20, 1:50 and 1:100 round the window, printed through the sheet mode's PDF and drawn back by poppler's
/// `pdftoppm` at 200 dpi into `.run/shots/yapi-bilesen-pdf-N.png` for the eye. Ignored: it needs poppler.
///
/// ```text
/// cargo test -p kentos-desktop --lib building_parts_scenes::pdf::the_reveal_and_the_tee_on_paper -- --ignored --nocapture
/// ```
#[cfg(test)]
mod pdf {
    use serde_json::{Value, json};

    use super::*;
    use crate::app::Message;
    use crate::document::Document;
    use crate::files_testing::drive;

    fn sheet(scale: f64) -> Value {
        let id = format!("pafta-{scale}");
        json!({ "op": "addSheet", "sheet": {
            "id": id, "name": format!("Söve {scale}"),
            "page": { "paper": "a3", "orientation": "landscape", "size": { "width": 420000, "height": 297000 },
                "margins": { "top": 10000, "right": 10000, "bottom": 10000, "left": 20000 } },
            "items": [{ "id": format!("{id}-harita"), "name": "Harita",
                "frame": { "left": 20000, "top": 10000, "width": 390000, "height": 277000 },
                "kind": { "type": "map", "view": { "type": "fixed", "center": { "x": 3.0, "y": 0.2 },
                    "scale": scale as u32, "rotation": 0 } } }]
        }})
    }

    #[test]
    #[ignore = "poppler (pdftoppm) ve elle bakış ister"]
    fn the_reveal_and_the_tee_on_paper() {
        let dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/shots");
        std::fs::create_dir_all(&dir).expect("the shots directory");
        let project = json!({ "name": "Söve", "workspace": "cad", "coords": "local", "drawingUnit": "m", "plotScale": 50 });
        let choices = kentos_headless::new_drawing(&project).expect("the choices");
        let made = kentos_headless::Session::new(&choices).expect("a new project");
        let path = dir.join("yapi-bilesen-pdf.kcad");
        std::fs::write(&path, made.to_bytes().expect("its bytes")).expect("written");
        let (mut app, _) = App::boot(None);
        let doc = Document::read(&path).expect("opens");
        let task = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
        drive(&mut app, task);
        build(&mut app, |x, y| json!({"x": x, "y": y}));
        command(
            &mut app,
            "cad.sheets.edit",
            json!({ "ops": [sheet(20.0), sheet(50.0), sheet(100.0)] }),
        );
        app.follow_sheets();
        let pdf = dir.join("yapi-bilesen.pdf");
        {
            use kentos_sheet_ui::message::{PdfMessage, PdfScope};
            let _ = app.update(Message::Sheet(kentos_sheet_ui::Message::Export(
                kentos_sheet_ui::ExportKind::Pdf,
            )));
            let _ = app.update(Message::Sheet(kentos_sheet_ui::Message::Pdf(
                PdfMessage::Scope(PdfScope::All),
            )));
            app.sheet_export_to(kentos_sheet_ui::ExportKind::Pdf, Some(pdf.clone()));
        }
        assert!(pdf.exists(), "the PDF written");
        let ran = std::process::Command::new("pdftoppm")
            .args(["-r", "200", "-png"])
            .arg(&pdf)
            .arg(dir.join("yapi-bilesen-pdf"))
            .status()
            .expect("pdftoppm runs");
        assert!(ran.success());
    }
}

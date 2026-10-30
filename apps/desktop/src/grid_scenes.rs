//! The pictures of docs/adr/0725 (aks ve kolon yerleşimi): Aks ağı's Eğrisel under the cursor (four radial grids 30°
//! apart, arcs of 6, 10 and 14 m, a column at every crossing) and written, column B-2 selected; Kolon's L put by its
//! lower left near crossing B-2 of a 3 × 3 grid; L columns written at every crossing, one selected (its legs and Hiza);
//! a system's grids gapped, 12 mm ends on paper, one selected (Kesik uçlar); İki nokta with Eşit böl, the x gaps
//! splitting the second click's distance; Mimari denetim's Akslar with B given twice, then fixed. `tools_screens`
//! takes them in both themes at 1440×900 and 1100×650:
//!
//! ```text
//! KENTOS_SHOTS_ONLY=aks-egrisel,aks-egrisel-yazildi,kolon-l,kolon-l-yazildi,kolon-l-oznitelik,aks-kesik,aks-esit-bol,aks-denetim,aks-denetim-duzeltildi cargo test -p kentos-desktop --lib tools_screens::screens -- --ignored --nocapture
//! ```
//!
//! Test code only.

use kentos_domain::Slot;
use serde_json::{Value, json};

use crate::app::{App, Message};
use crate::architecture_settings::Commit;
use crate::tools_scenes::{E, N, Objects, click, hover, run, typed};
use crate::tools_screens::Scene;
use crate::wall_scenes::opened;

fn command(app: &mut App, name: &str, input: Value) {
    let doc = app.document.as_mut().expect("a drawing");
    let answer = kentos_native_application::registry::run(
        &mut kentos_native_application::ExecutionContext::new(&mut doc.model),
        name,
        kentos_commands::CallOp::Execute,
        input,
    )
    .expect("runs");
    assert_eq!(answer["status"], "completed", "{name}: {answer}");
}

/// The view on `[x0, y0]`–`[x1, y1]` (from the drawing's origin).
fn view_on(app: &mut App, a: [f64; 2], b: [f64; 2]) {
    app.viewport.camera.fit(
        &kentos_render_wgpu::Bounds { min_x: E + a[0], min_y: N + a[1], max_x: E + b[0], max_y: N + b[1] },
        24.0,
    );
}

/// A setting of the running tool, as its row in Etkin aracın ayarları commits it.
fn set(app: &mut App, tool: &'static str, key: &'static str, value: &str) {
    let _ = app.update(Message::ToolSetting(Commit { tool, key, value: value.into() }));
}

/// An empty plan on Zemin kat, the command line open.
fn plan(app: &mut App) {
    opened(app, Objects::new(), 100.0);
    crate::arch_scenes::on_level(app);
    app.draft.snap = false;
    app.command_expanded = true;
}

/// Eğrisel under the cursor at the centre: radial grids 1–4 30° apart, arcs A–C, columns S1 … at the crossings.
fn radial(app: &mut App) {
    plan(app);
    view_on(app, [-4.0, -4.0], [18.0, 18.0]);
    run(app, "tool.gridSystem");
    set(app, "gridSystem", "F", "radial");
    set(app, "gridSystem", "K", "yes");
    set(app, "gridSystem", "S", "S1");
    hover(app, [0.0, 0.0]);
}

/// Written: four radial grids and three arcs (slots 1–7), twelve columns (8–19, arc by arc); B-2 selected.
fn radial_written(app: &mut App) {
    radial(app);
    click(app, [0.0, 0.0]);
    typed(app, "");
    app.selection.set([Slot(13)]);
    hover(app, [17.0, -3.0]);
}

/// A 3 × 3 grid 6 m by 5 m (1–3, A–C; slots 1–6) written with Aks ağı.
fn plain_grid(app: &mut App) {
    plan(app);
    view_on(app, [-3.0, -3.0], [15.0, 13.0]);
    run(app, "tool.gridSystem");
    set(app, "gridSystem", "X", "2*6");
    set(app, "gridSystem", "Y", "2*5");
    click(app, [0.0, 0.0]);
    typed(app, "");
}

/// Kolon L, 0.60 × 0.60 with 0.25 legs, put by its lower left near crossing B-2: its corner on the crossing.
fn column_l(app: &mut App) {
    plain_grid(app);
    run(app, "tool.column");
    set(app, "column", "B", "lShape");
    set(app, "column", "E", "0.6");
    set(app, "column", "Y", "0.6");
    set(app, "column", "L", "0.25 0.25");
    set(app, "column", "H", "bottomLeft");
    hover(app, [6.1, 5.1]);
}

/// L columns at every crossing by their lower left (slots 7–15), B-2 selected: Alt kol, Sol kol, Hiza.
fn column_l_written(app: &mut App) {
    plan(app);
    view_on(app, [-3.0, -3.0], [15.0, 13.0]);
    run(app, "tool.gridSystem");
    set(app, "gridSystem", "X", "2*6");
    set(app, "gridSystem", "Y", "2*5");
    set(app, "gridSystem", "K", "yes");
    set(app, "gridSystem", "M", "lShape");
    set(app, "gridSystem", "E", "0.6");
    set(app, "gridSystem", "O", "0.6");
    set(app, "gridSystem", "V", "0.25 0.25");
    set(app, "gridSystem", "J", "bottomLeft");
    set(app, "gridSystem", "S", "S1");
    click(app, [0.0, 0.0]);
    typed(app, "");
    app.selection.set([Slot(11)]);
    hover(app, [14.0, 12.0]);
}

/// Kesik: the 3 × 3 grid's lines only at their ends, 12 mm on paper (1.2 m at 1:100); grid 2 selected.
fn gapped(app: &mut App) {
    plan(app);
    view_on(app, [-3.0, -3.0], [15.0, 13.0]);
    run(app, "tool.gridSystem");
    set(app, "gridSystem", "X", "2*6");
    set(app, "gridSystem", "Y", "2*5");
    set(app, "gridSystem", "C", "12");
    click(app, [0.0, 0.0]);
    typed(app, "");
    app.selection.set([Slot(2)]);
    hover(app, [14.0, 12.0]);
}

/// İki nokta, Eşit böl: the origin clicked, the cursor 20 m off at 20°: the three x gaps 6.667 m each.
fn equal(app: &mut App) {
    plan(app);
    view_on(app, [-3.0, -3.0], [22.0, 18.0]);
    run(app, "tool.gridSystem");
    set(app, "gridSystem", "P", "two");
    set(app, "gridSystem", "Q", "yes");
    click(app, [0.0, 0.0]);
    let t = 20f64.to_radians();
    hover(app, [20.0 * t.cos(), 20.0 * t.sin()]);
}

/// Mimari denetim's Akslar: B on the grid's B (deleted) and B again 2.5 m above (named C … the next free).
fn duplicates(app: &mut App) {
    plain_grid(app);
    let model = &app.document.as_ref().expect("a drawing").model;
    let layer = model.layers().active().to_owned();
    let b = model
        .entities()
        .find_map(|e| match e {
            kentos_commands::Entity::Grid(g) if g.grid.label == "B" => Some(g.grid.clone()),
            _ => None,
        })
        .expect("grid B");
    let above: Vec<Value> = b.pts.iter().map(|q| json!({"x": q.x, "y": q.y + 2.5})).collect();
    command(app, "cad.grids.create", json!({"layerId": layer, "grids": [
        {"pts": b.pts, "label": "B", "arch": b.arch},
        {"pts": above, "label": "B", "arch": b.arch}
    ]}));
    app.selection.clear();
    run(app, "arch.audit");
    view_on(app, [-3.0, -3.0], [15.0, 13.0]);
    let row = app
        .arch_audit_shown()
        .iter()
        .position(|&i| {
            let f = &app.arch_audit.audited.as_ref().expect("audited").findings[i];
            f.code == "arch_grid_label_duplicate"
        })
        .expect("the grid's finding");
    let _ = app.update(Message::ArchAudit(crate::arch_audit::Event::Press(row)));
}

/// Düzelt: the grid lying on B deleted in one step; checked again, the other B is left.
fn duplicates_fixed(app: &mut App) {
    duplicates(app);
    let _ = app.update(Message::ArchAudit(crate::arch_audit::Event::Fix));
}

/// An L column selected, Öznitelikler scrolled to its rows: Biçim L, Boy, Alt kol and Sol kol, Dönüş, Hiza.
pub(crate) fn pointed() -> Vec<crate::tools_screens::Pointed> {
    vec![("kolon-l-oznitelik", column_l_written, |s, app| {
        let mut update = |app: &mut App, message: Message| {
            let _ = app.update(message);
        };
        let at = crate::tools_screens::caption_at(s, app, "Genel", false);
        s.input(app, App::view, &mut update, kentos_ui::snapshot::Input::Scroll(at, -7.0));
    })]
}

pub(crate) fn scenes() -> Vec<Scene> {
    vec![
        ("aks-egrisel", radial),
        ("aks-egrisel-yazildi", radial_written),
        ("kolon-l", column_l),
        ("kolon-l-yazildi", column_l_written),
        ("aks-kesik", gapped),
        ("aks-esit-bol", equal),
        ("aks-denetim", duplicates),
        ("aks-denetim-duzeltildi", duplicates_fixed),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::files_testing::app_with_drawing;
    use kentos_commands::{ColumnAnchor, ColumnShape, Entity};

    fn objects(app: &App) -> Vec<Entity> {
        app.document.as_ref().expect("a drawing").model.entities().cloned().collect()
    }

    /// The walkthrough as the pictures take it (docs/adr/0725): Eğrisel writes four radial grids, three arcs and twelve
    /// columns tied to them, B-2 30° turned; L columns stand by their lower left on the crossings; Kesik writes every
    /// grid's ends; Eşit böl splits the second click's distance; Denetim finds B twice and Düzelt deletes the one lying
    /// on B.
    #[test]
    fn grids_and_columns_are_placed_as_the_pictures_show() {
        let mut app = app_with_drawing();
        radial_written(&mut app);
        let all = objects(&app);
        let grids: Vec<_> = all.iter().filter_map(|e| match e { Entity::Grid(g) => Some(&g.grid), _ => None }).collect();
        let columns: Vec<_> =
            all.iter().filter_map(|e| match e { Entity::Column(c) => Some(&c.column), _ => None }).collect();
        assert_eq!((grids.len(), columns.len()), (7, 12));
        assert_eq!(grids.iter().filter(|g| g.bulges.is_some()).count(), 3, "three arcs");
        let b2 = columns.iter().find(|c| c.place.as_deref() == Some("B-2")).expect("B-2");
        assert!((b2.rotation - 30f64.to_radians()).abs() < 1e-9);
        // The centre where the click's pixel fell, within a hundredth of a millimetre of the origin.
        let r = (b2.at.x - E).hypot(b2.at.y - N);
        assert!((r - 10.0).abs() < 1e-5, "on arc B: {r}");

        let mut app = app_with_drawing();
        column_l_written(&mut app);
        let all = objects(&app);
        let ls: Vec<_> = all.iter().filter_map(|e| match e { Entity::Column(c) => Some(&c.column), _ => None }).collect();
        assert_eq!(ls.len(), 9);
        assert!(ls.iter().all(|c| c.shape == ColumnShape::LShape
            && c.anchor == ColumnAnchor::BottomLeft
            && c.legs == Some([0.25, 0.25])));

        let mut app = app_with_drawing();
        gapped(&mut app);
        assert!(objects(&app).iter().all(|e| !matches!(e, Entity::Grid(g) if g.grid.ends != Some(12.0))));

        let mut app = app_with_drawing();
        equal(&mut app);
        click(&mut app, [20.0 * 20f64.to_radians().cos(), 20.0 * 20f64.to_radians().sin()]);
        let xs: Vec<_> = objects(&app)
            .into_iter()
            .filter_map(|e| match e { Entity::Grid(g) if g.grid.label.parse::<u32>().is_ok() => Some(g.grid.pts[0]), _ => None })
            .collect();
        assert_eq!(xs.len(), 4);
        let gap = (xs[1].x - xs[0].x).hypot(xs[1].y - xs[0].y);
        // Its distance where the clicks' pixels fell.
        assert!((gap - 20.0 / 3.0).abs() < 1e-5, "{gap}");

        let mut app = app_with_drawing();
        duplicates(&mut app);
        let codes = |app: &App| -> Vec<String> {
            app.arch_audit.audited.as_ref().unwrap().findings.iter().map(|f| f.code.clone()).collect()
        };
        assert_eq!(codes(&app).iter().filter(|c| *c == "arch_grid_label_duplicate").count(), 2);
        let before = objects(&app).len();
        duplicates_fixed(&mut app);
        assert_eq!(objects(&app).len(), before - 1, "the grid lying on B deleted");
        assert_eq!(codes(&app).iter().filter(|c| *c == "arch_grid_label_duplicate").count(), 1);
    }
}

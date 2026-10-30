//! The pictures of docs/adr/0726 (blok kitaplığı, gerçek boy ve birim): Bloklar › Kitaplık with the fixtures' library
//! (fixtures/blocks/v1/kitaplik) opened on yatak-odasi.dxf, its blocks with their pictures, real sizes and units; the
//! toilet's file naming no unit, its sizes waiting for one, then read in millimetres; the bed taken in and placed
//! with Blok ekle in a 4 × 3.5 m bedroom, its real width × depth and unit beside the cursor and in the pane; KentOS's
//! single bed with its “Genç 120 × 200” row chosen before the click; the same bed in a millimetre plan; a placed,
//! turned and mirrored bed's Öznitelikler. `tools_screens` takes them in both themes at 1440×900 and 1100×650:
//!
//! ```text
//! KENTOS_SHOTS_ONLY=kitaplik-klasor,kitaplik-birim,kitaplik-birim-secildi,kitaplik-yerlestir,blok-ekle-varyant,blok-ekle-mm,blok-oznitelik-boyut cargo test -p kentos-desktop --lib tools_screens::screens -- --ignored --nocapture
//! ```
//!
//! Test code only.

use std::path::PathBuf;
use std::sync::Arc;

use serde_json::{Value, json};

use crate::app::{App, Message};
use crate::architecture_settings::Commit;
use crate::blocks_library::{self, Event};
use crate::files_testing::make_cad;
use crate::tools_scenes::{E, N, Objects, click, hover, open_edited, run, typed};
use crate::tools_screens::Scene;

const LIBRARY: &str = include_str!("../../../catalog/v1/blocks/kentos-temel.kblok");

/// The fixtures' library folder.
fn root() -> PathBuf {
    PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/blocks/v1/kitaplik"))
        .canonicalize()
        .expect("the library folder")
}

fn library(app: &mut App, e: Event) {
    let _ = app.update(Message::BlocksPanel(crate::blocks_panel::Event::Library(e)));
}

/// A drawing of the library opened in Kitaplık, read here rather than off the thread, in `unit` when given.
fn opened_file(app: &mut App, file: &str, unit: Option<kentos_commands::DrawingUnit>) -> PathBuf {
    let path = root().join(file);
    library(app, Event::Open(path.clone()));
    if let Some(u) = unit {
        library(app, Event::Unit(path.clone(), u));
    }
    let read = blocks_library::read_file(&path, unit).map(Arc::new);
    library(app, Event::Read(path.clone(), unit, read));
    path
}

/// A 4 × 3.5 m bedroom of 20 cm walls in a metre plan at 1:50; the Bloklar panel's Kitaplık with the library.
fn bedroom(app: &mut App, unit: &str, blocks: Option<Value>) {
    let mut o = Objects::new();
    let pts: Vec<Value> = [[-0.1, -0.1], [4.1, -0.1], [4.1, 3.6], [-0.1, 3.6]]
        .iter()
        .map(|[x, y]: &[f64; 2]| json!({ "x": E + x, "y": N + y }))
        .collect();
    o.push(
        "cizim",
        json!({ "kind": "wall", "pts": pts, "width": 0.2, "justify": "center", "closed": true }),
    );
    open_edited(app, o, |d| {
        // A local plan: its drawing unit is its own (a coordinate system's is metres).
        d["settings"]["srid"] = json!(0);
        d["settings"]["plotScale"] = json!(50.0);
        if unit != "m" {
            d["settings"]["drawingUnit"] = json!(unit);
        }
        if let Some(blocks) = blocks {
            d["blocks"] = blocks;
        }
    });
    make_cad(app);
    app.draft.snap = false;
    app.command_expanded = true;
    app.viewport.camera.fit(
        &kentos_render_wgpu::Bounds { min_x: E - 0.8, min_y: N - 0.8, max_x: E + 4.8, max_y: N + 4.3 },
        24.0,
    );
    run(app, "block.panel");
    let _ = app.update(Message::BlocksPanel(crate::blocks_panel::Event::Source(true)));
    app.blocks_panel.library = blocks_library::Library::default();
    library(app, Event::RootChosen(Some(root())));
}

/// Bloklar › Kitaplık on yatak-odasi.dxf: the drawing as one block, the bed, the bedside table and the rug.
fn listed(app: &mut App) {
    bedroom(app, "m", None);
    library(app, Event::Go(Some(root().join("yatak-odasi"))));
    opened_file(app, "yatak-odasi/yatak-odasi.dxf", None);
}

/// klozet.dxf names no unit: the unit is asked and no size is said.
fn unit_asked(app: &mut App) {
    bedroom(app, "m", None);
    library(app, Event::Go(Some(root().join("islak-hacim"))));
    opened_file(app, "islak-hacim/klozet.dxf", None);
}

/// … read in millimetres: the toilet 0.38 × 0.55 m.
fn unit_chosen(app: &mut App) {
    bedroom(app, "m", None);
    library(app, Event::Go(Some(root().join("islak-hacim"))));
    opened_file(app, "islak-hacim/klozet.dxf", Some(kentos_commands::DrawingUnit::Mm));
}

/// The bed taken in from the library, Blok ekle under the cursor by the top wall, turned to face the room.
fn placed(app: &mut App) {
    listed(app);
    let path = root().join("yatak-odasi/yatak-odasi.dxf");
    library(app, Event::Place(path, Some("Yatak 90".into())));
    set(app, "turn", "180");
    hover(app, [2.0, 3.5]);
}

fn set(app: &mut App, key: &'static str, value: &str) {
    let _ = app.update(Message::ToolSetting(Commit { tool: "blockInsert", key, value: value.into() }));
}

fn kentos_blocks() -> Value {
    serde_json::from_str::<Value>(LIBRARY).expect("the library")["blocks"].clone()
}

fn kentos_bed(app: &App) -> kentos_commands::BlockId {
    app.document
        .as_ref()
        .and_then(|d| d.model.block_named("Tek kişilik yatak").map(|b| b.id))
        .expect("the library's bed")
}

/// KentOS's single bed with Genç 120 × 200 chosen before the click: 1.20 × 2.00 m beside the cursor.
fn variant(app: &mut App) {
    bedroom(app, "m", Some(kentos_blocks()));
    let _ = app.update(Message::BlocksPanel(crate::blocks_panel::Event::Source(false)));
    app.memory.block_insert = Some(kentos_bed(app));
    run(app, "tool.blockInsert");
    typed(app, "V");
    set(app, "p1", "Genç 120 × 200");
    set(app, "turn", "180");
    hover(app, [2.4, 3.5]);
}

/// The same bed in a millimetre plan: its real size in millimetres.
fn in_millimetres(app: &mut App) {
    bedroom(app, "mm", Some(kentos_blocks()));
    let _ = app.update(Message::BlocksPanel(crate::blocks_panel::Event::Source(false)));
    app.memory.block_insert = Some(kentos_bed(app));
    run(app, "tool.blockInsert");
    set(app, "turn", "180");
    hover(app, [2.4, 3.5]);
}

/// The library's bed placed turned and mirrored, selected: its Öznitelikler say its real size and its block's unit.
fn properties(app: &mut App) {
    placed(app);
    // Mirrored in its x axis and not turned, the bed lies along the top wall into the room.
    set(app, "turn", "0");
    set(app, "mirror", "yes");
    click(app, [2.0, 3.5]);
    run(app, "tool.select");
    let inserts: Vec<kentos_domain::Slot> = app
        .document
        .as_ref()
        .expect("a drawing")
        .model
        .entities()
        .filter(|e| matches!(e, kentos_commands::Entity::Insert(_)))
        .map(|e| kentos_domain::Slot(e.base().id))
        .collect();
    app.selection.set(inserts);
}

pub(crate) fn scenes() -> Vec<Scene> {
    vec![
        ("kitaplik-klasor", listed),
        ("kitaplik-birim", unit_asked),
        ("kitaplik-birim-secildi", unit_chosen),
        ("kitaplik-yerlestir", placed),
        ("blok-ekle-varyant", variant),
        ("blok-ekle-mm", in_millimetres),
    ]
}

/// The placed bed's Öznitelikler scrolled to its Geometri: En × boy and Bloğun birimi.
pub(crate) fn pointed() -> Vec<crate::tools_screens::Pointed> {
    vec![("blok-oznitelik-boyut", properties, |s, app| {
        let mut update = |app: &mut App, message: Message| {
            let _ = app.update(message);
        };
        let at = crate::tools_screens::caption_at(s, app, "Genel", false);
        s.input(app, App::view, &mut update, kentos_ui::snapshot::Input::Scroll(at, -9.0));
    })]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::files_testing::app_with_drawing;
    use kentos_commands::{BlockUnits, Entity};

    fn block<'a>(app: &'a App, name: &str) -> Option<&'a kentos_commands::BlockDefinition> {
        app.document.as_ref()?.model.block_named(name)
    }

    /// The library as the pictures take it (docs/adr/0726 §1): Yerleştir takes the bed in with the library's
    /// category and starts Blok ekle with it; a file naming no unit places nothing until its unit is chosen, and then
    /// at the size that unit gives; Dosyadan blok ekle sends such a file to Kitaplık.
    #[test]
    fn the_library_takes_blocks_in_with_their_category_and_unit() {
        let mut app = app_with_drawing();
        placed(&mut app);
        let bed = block(&app, "Yatak 90").expect("the bed taken in");
        assert_eq!(bed.units, Some(BlockUnits::Mm));
        assert_eq!(
            bed.card.as_ref().and_then(|c| c.category.as_deref()),
            Some("kitaplik/yatak-odasi/yatak-odasi")
        );
        assert_eq!(app.session.tool_id(), "blockInsert");
        assert_eq!(app.memory.block_insert, Some(bed.id));
        click(&mut app, [2.0, 3.5]);
        let doc = &app.document.as_ref().expect("a drawing").model;
        let Some(Entity::Insert(i)) = doc.entities().find(|e| matches!(e, Entity::Insert(_))) else {
            panic!("the bed placed");
        };
        let [x0, y0, x1, y1] =
            kentos_native_application::blocks::definition_box(doc, i.block, None).expect("its box");
        assert!(((x1 - x0) - 0.9).abs() < 1e-9 && ((y1 - y0) - 2.0).abs() < 1e-9, "the real bed");

        // The toilet: nothing comes in before its unit, then 0.38 × 0.55 m.
        let mut app = app_with_drawing();
        unit_asked(&mut app);
        let path = root().join("islak-hacim/klozet.dxf");
        library(&mut app, Event::Place(path.clone(), Some("Klozet".into())));
        assert!(block(&app, "Klozet").is_none(), "no unit, nothing taken in");
        unit_chosen(&mut app);
        library(&mut app, Event::Place(path.clone(), Some("Klozet".into())));
        let toilet = block(&app, "Klozet").expect("taken in millimetres");
        assert_eq!(toilet.units, Some(BlockUnits::Mm));
        let doc = &app.document.as_ref().expect("a drawing").model;
        let [x0, y0, x1, y1] =
            kentos_native_application::blocks::definition_box(doc, toilet.id, None).expect("its box");
        assert!(((x1 - x0) - 0.38).abs() < 1e-9 && ((y1 - y0) - 0.55).abs() < 1e-9);

        // Dosyadan blok ekle with it: refused, and opened in Kitaplık where its unit is chosen.
        let mut app = app_with_drawing();
        bedroom(&mut app, "m", None);
        let _ = app.update(Message::BlocksPanel(crate::blocks_panel::Event::Source(false)));
        let _ = app.insert_file_block(&path);
        assert!(app.blocks_panel.library_shown);
        assert_eq!(app.blocks_panel.library.open.as_deref(), Some(path.as_path()));
        assert!(block(&app, "klozet").is_none());
    }

    /// The category of a library's block: the library's name, its folders and the drawing's name; the drawing as one
    /// block goes in its folder's.
    #[test]
    fn a_library_names_its_categories_by_its_folders() {
        let mut lib = blocks_library::Library::default();
        assert!(lib.add(&root()));
        assert!(!lib.add(&root()), "once");
        let bed = root().join("yatak-odasi/yatak-odasi.dxf");
        assert_eq!(lib.category(&bed, false).as_deref(), Some("kitaplik/yatak-odasi/yatak-odasi"));
        assert_eq!(lib.category(&bed, true).as_deref(), Some("kitaplik/yatak-odasi"));
        assert_eq!(lib.category(std::path::Path::new("/başka/yer/a.dxf"), false), None, "outside every library");
        let (dirs, files) = blocks_library::listing(&root());
        assert_eq!(dirs, ["islak-hacim", "yatak-odasi", "yemek"]);
        assert!(files.is_empty());
        let (_, files) = blocks_library::listing(&root().join("islak-hacim"));
        assert_eq!(files, ["klozet.dxf", "lavabo.dxf"]);
    }
}

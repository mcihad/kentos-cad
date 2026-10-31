//! The pictures and the acceptance of docs/adr/0789 (kırpma çerçevesi): the 2+1 plan of docs/adr/0773 with Banyo detay
//! of docs/adr/0781 (1:50, detailed, its linked map cut to its area). Görünümler with Alanı düzenle; Kırpma çerçevesi on
//! the model with the views' frames and the chosen one's grips, its right edge being dragged; the area written and its
//! sheet following; the 1:20 sheet printed and read back.
//!
//!     KENTOS_SHOTS_ONLY=kirpma-pencere,kirpma-cerceve,kirpma-surukle,kirpma-pafta cargo test -p kentos-desktop --lib tools_screens::screens -- --ignored --nocapture

use iced::Point;
use serde_json::json;

use crate::app::{App, Message};
use crate::program_scenes::command;
use crate::tools_scenes::{E, N, at, hover};
use crate::tools_screens::Scene;
use crate::view_detail_scenes::{BATH, bath, bath_recipe};
use crate::viewport::Event;

/// The model in front, framed round the whole house, Kırpma çerçevesi on Banyo detay.
fn cropping(app: &mut App) {
    bath(app);
    let task = app.update(Message::Sheet(kentos_sheet_ui::Message::Tab(0)));
    crate::files_testing::drive(app, task);
    // A test's app has no window yet: the drawing area of a 1440 × 900 one (the pictures resize it again).
    if app.viewport.camera.width < 2.0 {
        let _ = app.update(Message::Viewport(Event::Resized(iced::Rectangle::new(
            Point::new(0.0, 0.0),
            iced::Size::new(1119.0, 641.0),
        ))));
    }
    app.viewport.camera.fit(
        &kentos_render_wgpu::Bounds {
            min_x: E - 1.0,
            min_y: N - 1.0,
            max_x: E + 15.0,
            max_y: N + 11.0,
        },
        24.0,
    );
    app.memory.view_crop = app.named_views().iter().position(|v| v.name == "Banyo detay");
    let _ = app.update(Message::Run("tool.viewCrop"));
}

/// The right edge's grip of Banyo detay, east and north of the drawing's origin.
fn right_grip() -> [f64; 2] {
    let (_, y0, x1, y1) = BATH;
    [x1, (y0 + y1) / 2.0]
}

/// The right edge pressed and pulled 1.5 m east, not let go.
fn pulled(app: &mut App) -> Point {
    cropping(app);
    let from = right_grip();
    hover(app, from);
    let _ = app.update(Message::Viewport(Event::Pressed(at(app, from))));
    let to = at(app, [from[0] + 1.5, from[1]]);
    let _ = app.update(Message::Viewport(Event::Moved(Point::new((at(app, from).x + to.x) / 2.0, to.y))));
    let _ = app.update(Message::Viewport(Event::Moved(to)));
    to
}

fn window(app: &mut App) {
    bath(app);
    let task = app.update(Message::Sheet(kentos_sheet_ui::Message::Tab(0)));
    crate::files_testing::drive(app, task);
    let _ = app.update(Message::Run("view.namedViews"));
    let _ = app.update(Message::Views(crate::views::Event::Choose("Banyo detay".into())));
}

fn frame(app: &mut App) {
    cropping(app);
    hover(app, right_grip());
}

fn drag(app: &mut App) {
    pulled(app);
}

/// The edge let go: the area written; the sheet in front draws the wider area.
fn sheet(app: &mut App) {
    let to = pulled(app);
    let _ = app.update(Message::Viewport(Event::Released(to)));
    let _ = app.update(Message::Run("tool.cancel"));
    let task = app.update(Message::Sheet(kentos_sheet_ui::Message::Tab(3)));
    crate::files_testing::drive(app, task);
}

/// Banyo detay at 1:20: its area runs past the map's frame; the sheet's Ön denetim says so (docs/adr/0794).
fn cut(app: &mut App) {
    crate::view_detail_scenes::bath(app);
    let mut r = crate::view_detail_scenes::bath_recipe("detailed", true);
    r["scale"] = serde_json::json!(20);
    crate::program_scenes::command(app, "cad.views.edit", serde_json::json!({"edit": {"kind": "recipe", "name": "Banyo detay", "recipe": r}}));
    app.follow_sheets();
    let _ = app.update(Message::Sheet(kentos_sheet_ui::Message::InspectorTab(kentos_sheet_ui::message::InspectorTab::Preflight)));
}

pub(crate) fn scenes() -> Vec<Scene> {
    vec![
        ("kirpma-pencere", window),
        ("kirpma-cerceve", frame),
        ("kirpma-surukle", drag),
        ("kirpma-pafta", sheet),
        ("kirpma-kesik-denetim", cut),
    ]
}

/// Banyo detay's area as the drawing keeps it: middle (east and north of the origin), width, height.
fn area(app: &App) -> (f64, f64, f64, f64) {
    let v = app.named_views().iter().find(|v| v.name == "Banyo detay").cloned().expect("the view");
    (v.center.x - E, v.center.y - N, v.width, v.height)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn near(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-6
    }

    /// The front sheet's (only) linked map as the display list draws it.
    fn the_map(app: &mut App) -> kentos_sheet::display::MapPrim {
        let ctx = app.sheet_context();
        app.sheets.set_context(ctx);
        app.sheets
            .export_list()
            .expect("a sheet in front")
            .prims
            .into_iter()
            .find_map(|p| match p {
                kentos_sheet::display::Prim::Map(m) => Some(m),
                _ => None,
            })
            .expect("its map")
    }

    /// docs/adr/0789 §1, §2: the right edge dragged 1.5 m east widens the view by 1.5 m and moves its middle 0.75 m, its
    /// left edge, height and recipe kept; one undo step gives the area back, redo the new one; the linked map follows.
    #[test]
    fn dragging_an_edge_writes_the_area_in_one_step() {
        let mut app = crate::files_testing::app_with_drawing();
        let to = pulled(&mut app);
        assert_eq!(app.session.point_count(), 2, "the view chosen and its grip held");
        let (x0, y0, x1, y1) = BATH;
        let before = ((x0 + x1) / 2.0, (y0 + y1) / 2.0, x1 - x0, y1 - y0);
        assert_eq!(area(&app), before, "nothing written while the grip moves");
        let _ = app.update(Message::Viewport(Event::Released(to)));
        let (cx, cy, w, h) = area(&app);
        assert!(near(w, before.2 + 1.5) && near(h, before.3), "{w} × {h}");
        assert!(near(cx, before.0 + 0.75) && near(cy, before.1), "({cx}, {cy})");
        let v = app.named_views().iter().find(|v| v.name == "Banyo detay").cloned().expect("kept");
        assert_eq!(v.recipe.expect("its recipe").version, 1, "the recipe untouched");
        let doc = &app.document.as_ref().expect("a drawing").model;
        assert_eq!(doc.undo_labels().first().map(String::as_str), Some(kentos_native_application::views_edit::AREA_STEP));
        let _ = app.update(Message::Run("edit.undo"));
        assert_eq!(area(&app), before, "undo gives the area back");
        let _ = app.update(Message::Run("edit.redo"));
        assert!(near(area(&app).2, before.2 + 1.5), "redo the new one");
        // The linked sheet's map follows the view: its middle is the view's.
        let _ = app.update(Message::Run("tool.cancel"));
        let task = app.update(Message::Sheet(kentos_sheet_ui::Message::Tab(3)));
        crate::files_testing::drive(&mut app, task);
        let map = the_map(&mut app);
        assert!(!map.clip_shape.is_frame(), "still cut to the view's area");
    }

    /// Esc with a grip held: the frame as it was, nothing written, the view still chosen; Esc again lets the view go,
    /// then the tool ends.
    #[test]
    fn esc_lets_the_grip_go_then_the_view_then_the_tool() {
        let mut app = crate::files_testing::app_with_drawing();
        pulled(&mut app);
        let before = area(&app);
        let _ = app.update(Message::Run("tool.cancel"));
        assert_eq!(area(&app), before);
        assert_eq!(app.session.tool_id(), "viewCrop");
        let _ = app.update(Message::Run("tool.cancel"));
        let _ = app.update(Message::Run("tool.cancel"));
        assert_ne!(app.session.tool_id(), "viewCrop");
        let doc = &app.document.as_ref().expect("a drawing").model;
        assert_ne!(doc.undo_labels().first().map(String::as_str), Some(kentos_native_application::views_edit::AREA_STEP));
    }

    /// A typed `6x4` gives the width and height about the middle; Görünümler's Alanı düzenle starts the tool on the
    /// chosen view.
    #[test]
    fn typed_size_and_the_window_s_button() {
        let mut app = crate::files_testing::app_with_drawing();
        window(&mut app);
        let _ = app.update(Message::Views(crate::views::Event::Crop));
        assert_eq!(app.session.tool_id(), "viewCrop");
        let before = area(&app);
        let _ = app.submit_line("6x4");
        let (cx, cy, w, h) = area(&app);
        assert!(near(w, 6.0) && near(h, 4.0), "{w} × {h}");
        assert!(near(cx, before.0) && near(cy, before.1));
    }

    /// The 1:20 review of docs/adr/0789 §3: Banyo detay at 1:20, detailed and cut to its area, printed through the sheet
    /// mode's PDF; poppler's `pdftotext` reads the sheet's words and `pdftoppm` draws it into
    /// `.run/shots/kirpma-1-20-pdf-1.png`. Ignored: it needs poppler.
    ///
    /// ```text
    /// cargo test -p kentos-desktop --lib view_crop_scenes::tests::a_cropped_detail_at_1_20_on_paper -- --ignored --nocapture
    /// ```
    #[test]
    #[ignore = "poppler (pdftoppm, pdftotext) ister"]
    fn a_cropped_detail_at_1_20_on_paper() {
        let dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/shots");
        std::fs::create_dir_all(&dir).expect("the shots directory");
        let mut app = crate::files_testing::app_with_drawing();
        bath(&mut app);
        let mut r = bath_recipe("detailed", true);
        r["scale"] = json!(20);
        command(&mut app, "cad.views.edit", json!({"edit": {"kind": "recipe", "name": "Banyo detay", "recipe": r}}));
        app.follow_sheets();
        let pdf = dir.join("kirpma-1-20.pdf");
        {
            use kentos_sheet_ui::message::{PdfMessage, PdfScope};
            let _ = app.update(Message::Sheet(kentos_sheet_ui::Message::Export(kentos_sheet_ui::ExportKind::Pdf)));
            let _ = app.update(Message::Sheet(kentos_sheet_ui::Message::Pdf(PdfMessage::Scope(PdfScope::This))));
            app.sheet_export_to(kentos_sheet_ui::ExportKind::Pdf, Some(pdf.clone()));
        }
        assert!(pdf.exists(), "the PDF written");
        let text = std::process::Command::new("pdftotext")
            .args(["-layout"])
            .arg(&pdf)
            .arg("-")
            .output()
            .expect("pdftotext runs");
        let words = String::from_utf8_lossy(&text.stdout).into_owned();
        println!("{words}");
        std::fs::write(dir.join("kirpma-1-20.txt"), &words).expect("the words");
        // The scale in the title block, the bathroom's tag; the room beside it is cut away, its words too.
        assert!(words.contains("1/20"), "the scale");
        assert!(words.contains("202 · 2 Daire 2") && words.contains("12.04 m²"), "the bathroom's tag");
        assert!(!words.contains("201 · 2 Daire 2"), "a tag cut away is not in the file");
        let ran = std::process::Command::new("pdftoppm")
            .args(["-r", "150", "-png"])
            .arg(&pdf)
            .arg(dir.join("kirpma-1-20-pdf"))
            .status()
            .expect("pdftoppm runs");
        assert!(ran.success());
    }
}

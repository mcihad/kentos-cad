//! The pictures of docs/adr/0348 (external references): Dış başvuru ekle over the architect's plan
//! (`fixtures/xref/v1/plan.kcad`), the plan attached in place with its layers under “Dış başvuru: Plan”, Dış başvurular
//! with a changed source, a nested reference whose source is not here and an overlay, and the warning a drawing whose
//! reference's source changed opens with. `tools_screens` takes them in both themes at 1440×900 and 1100×650:
//!
//! ```text
//! KENTOS_SHOTS_ONLY=dis-basvuru-ekle,dis-basvuru,dis-basvuru-pencere,dis-basvuru-degisti cargo test -p kentos-desktop tools_screens -- --ignored --nocapture
//! ```
//!
//! And docs/adr/0350's pictures: a linked picture not on this computer, Yeniden bağla and Göm
//! (`resim-bulunamadi`, `resim-yeniden-bagla`, `resim-gomuldu`).
//!
//! Test code only.

use std::path::PathBuf;

use crate::app::{App, Message};
use crate::tools_screens::Scene;
use crate::wall_scenes::opened;
use crate::xrefs_window::{Event, Mode, Purpose};

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/xref/v1").join(name)
}

/// A copy of `name` of the scenes' own, which a scene may change under the drawing.
fn copy(name: &str, as_: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("kentos-xref-shots-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    let to = dir.join(as_);
    let _ = std::fs::copy(fixture(name), &to);
    to
}

fn send(app: &mut App, e: Event) {
    let _ = app.update(Message::Xrefs(e));
}

/// Dış başvuru ekle with `path` chosen: its window.
fn asked(app: &mut App, path: PathBuf) {
    send(app, Event::Picked(Purpose::Attach, Some(path)));
}

/// `path` attached in place as `name`, `mode`.
fn attached(app: &mut App, path: PathBuf, name: &str, mode: Mode) {
    asked(app, path);
    send(app, Event::Name(name.to_owned()));
    send(app, Event::Mode(mode));
    send(app, Event::InPlace);
}

/// The view on the references' inserts.
fn on_inserts(app: &mut App) {
    let inserts: Vec<f64> = app
        .document
        .as_ref()
        .map(|d| {
            d.model
                .entities()
                .filter(|e| matches!(e, kentos_commands::Entity::Insert(_)))
                .map(|e| f64::from(e.base().id))
                .collect()
        })
        .unwrap_or_default();
    app.zoom_to_objects(&inserts);
}

fn attach_window(app: &mut App) {
    opened(app, crate::tools_scenes::Objects::new(), 50.0);
    app.tab = "insert";
    asked(app, fixture("plan.kcad"));
}

fn attached_plan(app: &mut App) {
    opened(app, crate::tools_scenes::Objects::new(), 50.0);
    app.tab = "insert";
    attached(app, fixture("plan.kcad"), "Plan", Mode::Attach);
    on_inserts(app);
}

/// The plan attached from a copy that then changed, and the structural plan overlaid from a file not here any more.
fn three(app: &mut App) {
    opened(app, crate::tools_scenes::Objects::new(), 50.0);
    app.tab = "insert";
    let plan = copy("plan.kcad", "plan.kcad");
    attached(app, plan.clone(), "Plan", Mode::Attach);
    let gone = copy("plan2.kcad", "tasiyici.kcad");
    attached(app, gone.clone(), "Taşıyıcı", Mode::Overlay);
    let _ = std::fs::remove_file(&gone);
    let _ = std::fs::copy(fixture("plan2.kcad"), &plan);
    on_inserts(app);
}

fn manager(app: &mut App) {
    three(app);
    let _ = app.run("xref.manager");
}

fn opened_changed(app: &mut App) {
    three(app);
    app.xrefs_check_opened();
}

/// The plan attached from a copy, a named view on it; the copy then changes and the drawing looks at its files.
fn notice(app: &mut App) {
    opened(app, crate::tools_scenes::Objects::new(), 50.0);
    let plan = copy("plan.kcad", "bildirim.kcad");
    attached(app, plan.clone(), "Plan", Mode::Attach);
    if let Some(doc) = app.document.as_mut() {
        let mut settings = doc.model.settings().clone();
        settings.named_views.push(kentos_commands::NamedView {
            id: String::new(),
            name: "Salon".into(),
            center: kentos_commands::Vec2 { x: 5.0, y: 4.0 },
            width: 14.0,
            height: 10.0,
            twist: 0.0,
        });
        doc.model.set_settings(settings);
    }
    on_inserts(app);
    // A moment later the architect saves the next version.
    std::thread::sleep(std::time::Duration::from_millis(20));
    let _ = std::fs::copy(fixture("plan2.kcad"), &plan);
    let _ = app.update(Message::XrefWatch(crate::xrefs_watch::Event::Tick));
}

/// Dış başvurular › Arama yolları with two folders.
fn paths(app: &mut App) {
    three(app);
    send(app, Event::PathPicked(Some(PathBuf::from("/proje/ortak/mimari"))));
    send(app, Event::PathPicked(Some(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/xref/v1"))));
    send(app, Event::Section(crate::xrefs_window::Section::Paths));
}

/// Dış başvurular › Diğer bağlı dosyalar: a linked photo not there, a raster and a point cloud found, a linked table.
fn others(app: &mut App) {
    three(app);
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures");
    let at = |rel: &str| root.join(rel).display().to_string();
    let objects = serde_json::json!([
        { "kind": "image", "id": 0, "layerId": "0", "attrs": {}, "p": { "x": 0.0, "y": 12.0 }, "width": 4.0, "height": 3.0,
          "rotation": 0.0, "file": "/proje/fotograflar/cephe.jpg" },
        { "kind": "raster", "id": 0, "layerId": "0", "attrs": {}, "affine": [0.0, 1.0, 0.0, 40.0, 0.0, -1.0], "width": 10,
          "height": 10, "bands": 1, "sample": "u8", "file": at("raster/v1/files/i8.tif"), "srid": 0,
          "style": { "render": "gray", "bands": [1] } },
        { "kind": "pointcloud", "id": 0, "layerId": "0", "attrs": {},
          "sources": [{ "file": at("pointcloud/v1/files/bulut.csv"), "format": "xyz", "count": 1000,
                        "bounds": [0.0, 0.0, 0.0, 3.0, 2.0, 1.0] }],
          "bounds": [0.0, 0.0, 0.0, 3.0, 2.0, 1.0], "count": 1000, "srid": 0, "style": { "render": "rgb", "size": 2 } },
        { "kind": "line", "id": 0, "layerId": "0", "attrs": {}, "a": { "x": 0.0, "y": 0.0 }, "b": { "x": 1.0, "y": 0.0 },
          "attachments": [{ "name": "Tapu", "file": at("interaction/v1/belgeler/tapu-101-5.pdf") },
                          { "name": "Ruhsat", "url": "https://belediye.example/ruhsat/42" }] }
    ]);
    if let (Some(doc), Ok(list)) = (app.document.as_mut(), serde_json::from_value::<Vec<kentos_commands::Entity>>(objects)) {
        for e in list {
            let _ = doc.model.add(e);
        }
    }
    let _ = app.run("xref.manager");
    send(app, Event::Section(crate::xrefs_window::Section::Others));
}

/// Two linked pictures beside the plan's insert, one of them not on this computer (docs/adr/0350 §3), their pixels
/// read as the drawing area reads them; the missing one selected. Its slot.
fn pictures(app: &mut App) -> kentos_domain::Slot {
    opened(app, crate::tools_scenes::Objects::new(), 50.0);
    app.tab = "insert";
    let logo = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/interaction/v1/logo.png");
    let layer = app.document.as_ref().map(|d| d.model.layers().active().to_owned()).unwrap_or_default();
    let objects = serde_json::json!([
        { "kind": "image", "id": 0, "layerId": layer, "attrs": {}, "p": { "x": 0.0, "y": 0.0 }, "width": 8.0, "height": 6.0,
          "rotation": 0.0, "file": logo.display().to_string() },
        { "kind": "image", "id": 0, "layerId": layer, "attrs": {}, "p": { "x": 10.0, "y": 0.0 }, "width": 8.0, "height": 6.0,
          "rotation": 0.2, "file": "/proje/fotograflar/cephe.png" }
    ]);
    let mut slots = Vec::new();
    if let (Some(doc), Ok(list)) = (app.document.as_mut(), serde_json::from_value::<Vec<kentos_commands::Entity>>(objects)) {
        for e in list {
            if let Ok(slot) = doc.model.add(e) {
                slots.push(slot);
            }
        }
    }
    if let Some(doc) = app.document.as_ref() {
        for file in [logo.display().to_string(), "/proje/fotograflar/cephe.png".to_owned()] {
            let key = format!("file:{file}");
            let bitmap = crate::pictures::fetch(&key, &doc.model, None);
            app.styles.images.put_bitmap(key, bitmap);
        }
    }
    let ids: Vec<f64> = slots.iter().map(|s| f64::from(s.0)).collect();
    app.zoom_to_objects(&ids);
    let missing = slots.last().copied().unwrap_or(kentos_domain::Slot(0));
    app.selection.set([missing]);
    missing
}

fn picture_missing(app: &mut App) {
    pictures(app);
}

/// Yeniden bağla… with another PNG chosen: the picture shows it, one step.
fn picture_relinked(app: &mut App) {
    let slot = pictures(app);
    let rgb = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/raster/v1/files/rgb.png");
    send(app, Event::Picked(crate::xrefs_window::Purpose::Image(slot), Some(rgb.clone())));
    if let Some(doc) = app.document.as_ref() {
        let key = format!("file:{}", rgb.display());
        let bitmap = crate::pictures::fetch(&key, &doc.model, None);
        app.styles.images.put_bitmap(key, bitmap);
    }
}

/// Göm on the found picture: Öznitelikler's Kaynak says it is embedded.
fn picture_embedded(app: &mut App) {
    let missing = pictures(app);
    let found = kentos_domain::Slot(missing.0 - 1);
    let _ = app.update(Message::Properties(crate::properties::Event::EmbedImage(found)));
    app.selection.set([found]);
}

pub(crate) fn scenes() -> Vec<Scene> {
    vec![
        ("resim-bulunamadi", picture_missing),
        ("resim-yeniden-bagla", picture_relinked),
        ("resim-gomuldu", picture_embedded),
        ("dis-basvuru-bildirim", notice),
        ("dis-basvuru-yollar", paths),
        ("bagli-dosyalar", others),
        ("dis-basvuru-ekle", attach_window),
        ("dis-basvuru", attached_plan),
        ("dis-basvuru-pencere", manager),
        ("dis-basvuru-degisti", opened_changed),
    ]
}

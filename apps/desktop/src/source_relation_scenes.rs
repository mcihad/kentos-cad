//! The pictures of docs/adr/0532 (many-to-many references in bound GeoPackages, layers written together): the building,
//! parcel and owner example of `fixtures/geopackage/v1/iliskili*.gpkg`, bound as three layers. İlişkiler says how a
//! relation is kept in its file and maps it; Formla düzenle's Kaynağa da yaz writes a record to its file in the same
//! step; Kaynak ▸ Kaynağa birlikte yaz writes a parent and the layers its relations tie to it in one transaction.
//! `tools_screens` takes them in the dark and the light theme at 1440×900 and 1100×650:
//!
//! ```text
//! KENTOS_SHOTS_ONLY=kaynak-iliski-metin,kaynak-iliski-eslendi,kaynak-iliski-yabanci,kaynak-form-commit,kaynak-form-yazildi,kaynak-birlikte,kaynak-birlikte-reddedildi cargo test -p kentos-desktop tools_screens -- --ignored --nocapture
//! ```
//!
//! The files are copies in a temporary folder: a picture writes nothing to the fixtures. Test code only.

use std::path::PathBuf;

use kentos_contracts::CallOp;
use kentos_domain::Slot;
use kentos_native_application::{ExecutionContext, registry};
use serde_json::{Value, json};

use crate::app::{App, Message};
use crate::document::Document;
use crate::files_testing::scratch;
use crate::forms::{entry, relations};
use crate::tools_scenes::run;
use crate::tools_screens::Scene;

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/geopackage/v1")
        .join(name)
}

fn command(doc: &mut kentos_domain::Document, command: &str, input: Value) -> Value {
    let answer = registry::run(
        &mut ExecutionContext::new(doc),
        command,
        CallOp::Execute,
        input,
    )
    .expect("runs");
    assert_eq!(answer["status"], "completed", "{command}: {answer}");
    answer["output"].clone()
}

/// The three tables of `name` bound to layers (`import-malik`, `import-parsel`, `import-bina`), the parcels' `malikler`
/// a many-to-many reference, the project's two relations, a form on the owners' layer.
fn world(name: &str, tag: &str) -> Document {
    let file = scratch(tag).join(name);
    std::fs::copy(fixture(name), &file).expect("a copy of the file");
    let setup = json!({
        "format": "kentos.document", "version": 1, "name": "Kadastro",
        "settings": { "srid": 5256, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad",
                      "plotScale": 1000, "workspace": "gis" },
        "origin": { "x": 487000.0, "y": 4420000.0 },
        "layers": [{ "id": "cizim", "name": "Çizim", "type": "layer", "visible": true, "locked": false, "expanded": true,
                     "style": { "color": "fg", "lineType": "continuous", "lineWeight": 0.25 }, "children": [] }],
        "activeLayer": "cizim", "entities": [], "styles": { "items": [], "categories": [] }
    });
    let snapshot =
        kentos_contracts::DocumentSnapshotV1::from_json(&setup.to_string()).expect("setup");
    let mut d = Document::new(snapshot, None).expect("opens");
    let path = file.display().to_string();
    for table in ["malik", "parsel", "bina"] {
        command(
            &mut d.model,
            "cad.sources.edit",
            json!({ "edit": { "kind": "link", "path": path, "table": table } }),
        );
    }
    command(
        &mut d.model,
        "cad.layers.edit",
        json!({ "operation": "fields", "layer": "import-parsel", "fields": [
            { "name": "ada", "kind": "text" }, { "name": "parsel_no", "kind": "text" },
            { "name": "malikler", "kind": "text", "multiple": true } ] }),
    );
    for relation in [
        json!({ "id": "parsel-malik", "name": "Parselin malikleri", "parent": "import-malik", "child": "import-parsel", "field": "malikler" }),
        json!({ "id": "bina-parsel", "name": "Parselin binaları", "parent": "import-parsel", "child": "import-bina", "field": "parsel" }),
    ] {
        command(
            &mut d.model,
            "cad.relations.edit",
            json!({ "edit": { "kind": "create", "relation": relation } }),
        );
    }
    command(
        &mut d.model,
        "cad.forms.edit",
        json!({ "edit": { "kind": "create", "id": "malik-formu", "name": "Malik formu", "layer": "import-malik" } }),
    );
    command(
        &mut d.model,
        "cad.forms.edit",
        json!({ "edit": { "kind": "publish", "id": "malik-formu" } }),
    );
    d
}

fn open(app: &mut App, d: Document) {
    let _ = app.update(Message::Opened(Some(Ok(Box::new(d)))));
}

fn uid(app: &App, layer: &str, key: &str, value: &str) -> String {
    let doc = &app.document.as_ref().expect("a drawing").model;
    doc.by_layer_with_uids(layer)
        .find(|(_, e)| e.base().attrs.get(key).is_some_and(|v| v == value))
        .map(|(u, _)| u.to_string())
        .expect("an object")
}

fn set(app: &mut App, uid: &str, attrs: Value) {
    let doc = &mut app.document.as_mut().expect("a drawing").model;
    command(
        doc,
        "cad.entities.set",
        json!({ "uids": [uid], "operation": "attributes", "attrs": attrs }),
    );
}

/// İlişkiler on the owners' relation, the file as the layers last matched it: a text column.
fn text(app: &mut App) {
    open(app, world("iliskili.gpkg", "iliski-metin"));
    run(app, "data.relations");
    let _ = app.update(Message::Relations(relations::Event::Choose(0)));
}

/// Dosyaya eşle pressed: the relation is the file's own now, its mapping table named and counted.
fn mapped(app: &mut App) {
    text(app);
    let _ = app.update(Message::Relations(relations::Event::Map));
}

/// A file that relates the two tables another way: said, not rewritten, not read as text.
fn foreign(app: &mut App) {
    open(app, world("iliskili-yabanci.gpkg", "iliski-yabanci"));
    run(app, "data.relations");
    let _ = app.update(Message::Relations(relations::Event::Choose(0)));
}

/// Formla düzenle on the first owner: Tür typed, Kaynağa da yaz on.
fn form(app: &mut App) {
    open(app, world("iliskili-eslenmis.gpkg", "iliski-form"));
    app.selection.set([Slot(2)]);
    run(app, "data.formEdit");
    let _ = app.update(Message::FormEntry(entry::Event::Value(
        "tur".into(),
        "Tüzel".into(),
    )));
    let _ = app.update(Message::FormEntry(entry::Event::Commit(true)));
}

/// Kaydet pressed: the record and its file's side said together.
fn form_written(app: &mut App) {
    form(app);
    let _ = app.update(Message::FormEntry(entry::Event::Save));
}

/// A parcel and an owner changed locally, written together from the parcels' Kaynak ▸ menu.
fn together(app: &mut App) {
    open(app, world("iliskili-eslenmis.gpkg", "iliski-birlikte"));
    let parcel = uid(app, "import-parsel", "parsel_no", "1");
    let owner = uid(app, "import-malik", "ad", "Ayşe Yıldız");
    set(app, &parcel, json!({ "malikler": owner, "ada": "103" }));
    set(app, &owner, json!({ "tur": "Tüzel" }));
    app.selected_layer = Some("import-parsel".into());
    let _ = app.update(Message::Bound(crate::bound::Event::WriteTogether(Some(
        "import-parsel".into(),
    ))));
}

/// One of them cannot be written (a polygon on the owners' table): nothing is.
fn together_refused(app: &mut App) {
    open(app, world("iliskili-eslenmis.gpkg", "iliski-reddedildi"));
    let parcel = uid(app, "import-parsel", "parsel_no", "1");
    set(app, &parcel, json!({ "ada": "103" }));
    let doc = &mut app.document.as_mut().expect("a drawing").model;
    command(
        doc,
        "cad.polygon.create",
        json!({ "layerId": "import-malik", "pts": [{ "x": 487000.0, "y": 4420100.0 },
                                                  { "x": 487010.0, "y": 4420100.0 }, { "x": 487010.0, "y": 4420110.0 }] }),
    );
    app.selected_layer = Some("import-parsel".into());
    let _ = app.update(Message::Bound(crate::bound::Event::WriteTogether(Some(
        "import-parsel".into(),
    ))));
}

pub(crate) fn scenes() -> Vec<Scene> {
    vec![
        ("kaynak-iliski-metin", text),
        ("kaynak-iliski-eslendi", mapped),
        ("kaynak-iliski-yabanci", foreign),
        ("kaynak-form-commit", form),
        ("kaynak-form-yazildi", form_written),
        ("kaynak-birlikte", together),
        ("kaynak-birlikte-reddedildi", together_refused),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::files_testing::app_with_drawing;

    fn said(app: &App, words: &str) -> bool {
        app.log.lines().any(|l| l.text.contains(words))
    }

    #[test]
    fn a_text_reference_is_mapped_from_the_window_and_unmapped_again() {
        let mut app = app_with_drawing();
        text(&mut app);
        let w = app.relations_window.as_ref().expect("the window");
        assert!(w.has_filed("parsel-malik", kentos_contracts::RelationState::Text));
        let _ = app.update(Message::Relations(relations::Event::Map));
        let w = app.relations_window.as_ref().expect("the window");
        assert!(w.has_filed("parsel-malik", kentos_contracts::RelationState::Mapped));
        assert!(said(&app, "GeoPackage ilişkisi olarak eşlendi"));
        let _ = app.update(Message::Relations(relations::Event::Unmap));
        let w = app.relations_window.as_ref().expect("the window");
        assert!(w.has_filed("parsel-malik", kentos_contracts::RelationState::Text));
    }

    #[test]
    fn a_foreign_relation_is_said_and_offers_nothing() {
        let mut app = app_with_drawing();
        foreign(&mut app);
        let w = app.relations_window.as_ref().expect("the window");
        assert!(w.has_filed("parsel-malik", kentos_contracts::RelationState::Foreign));
        // Pressing Map would be refused (the window shows no button for it).
        let _ = app.update(Message::Relations(relations::Event::Map));
        assert!(said(&app, "başka bir temsille ilişkili"));
    }

    #[test]
    fn a_form_with_kaynaga_da_yaz_writes_the_file_in_the_same_step() {
        let mut app = app_with_drawing();
        form_written(&mut app);
        assert!(said(&app, "Kayıt “Malik formu” formuyla yazıldı."));
        assert!(
            said(&app, "dosyasına tek işlemde yazıldı"),
            "{:?}",
            app.log.last().map(|l| l.text.clone())
        );
        let link = app.bound_link("import-malik").expect("bound");
        let state = app.document.as_ref().map(|d| {
            kentos_native_application::sources::layer_digest(&d.model, "import-malik")
                == link.digest
        });
        assert_eq!(state, Some(true), "the layer matches its file again");
    }

    #[test]
    fn layers_written_together_say_each_and_a_refused_one_stops_all() {
        let mut app = app_with_drawing();
        together(&mut app);
        assert!(said(&app, "tek işlemde yazıldı"));
        let mut app = app_with_drawing();
        together_refused(&mut app);
        assert!(said(&app, "Hiçbir katman yazılmadı"));
    }
}

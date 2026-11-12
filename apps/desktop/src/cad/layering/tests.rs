use crate::app::Message;
use crate::files_testing::{app_with_drawing, last_said};

#[test]
fn a_new_layer_goes_next_to_the_active_one_and_becomes_active() {
    let mut app = app_with_drawing();
    let model = &app.document.as_ref().expect("open").model;
    let active = model.layers().active().to_owned();
    let group = model.layers().parent(&active).map(|g| g.id.clone());
    let revision = model.revision();

    let _ = app.update(Message::Run("layer.new"));
    let model = &app.document.as_ref().expect("open").model;
    let id = model.layers().active().to_owned();
    let node = model.layers().get(&id).expect("added");
    assert_eq!(node.name, "Yeni katman");
    assert_eq!(model.layers().parent(&id).map(|g| g.id.clone()), group);
    assert!(model.is_dirty() && model.can_undo());
    assert_ne!(model.revision(), revision);
    assert_eq!(
        last_said(&app),
        "“Yeni katman” katmanı eklendi ve etkin yapıldı."
    );

    let _ = app.update(Message::Run("layer.new"));
    let model = &app.document.as_ref().expect("open").model;
    let second = model.layers().active();
    assert_eq!(
        model.layers().get(second).map(|n| n.name.as_str()),
        Some("Yeni katman 2")
    );
    // Each is one step, “Katman ekle”: undo takes it away and makes the
    // layer before active again (docs/adr/0076).
    let model = &mut app.document.as_mut().expect("open").model;
    assert_eq!(model.undo().as_deref(), Some("Katman ekle"));
    assert_eq!(model.layers().active(), id);
    assert_eq!(model.undo().as_deref(), Some("Katman ekle"));
    assert_eq!(model.layers().active(), active);
    assert!(model.layers().get(&id).is_none());
    let _ = app.update(Message::Run("layer.newGroup"));
    let model = &mut app.document.as_mut().expect("open").model;
    assert_eq!(model.undo().as_deref(), Some("Grup ekle"));
}

/// Katmanlar → Sil (task 12, fixtures/document-ops/v1/layer-remove.json
/// for the document's side): the active layer and a group holding it are
/// refused in their words; a layer with objects is asked about, then goes
/// with them in one step that undo brings back; an empty layer goes at once.
#[test]
fn a_layer_goes_with_its_objects_after_the_question() {
    use super::Event;
    use crate::app::Dialog;
    let mut app = app_with_drawing();
    let _ = app.update(Message::Layer(Event::Remove("parsel".into())));
    assert_eq!(
        last_said(&app),
        "“Parsel” etkin katman; silinemez. Önce başka bir katmanı etkinleştirin."
    );
    let _ = app.update(Message::Layer(Event::Remove("layer-g".into())));
    assert_eq!(
        last_said(&app),
        "“Kadastro” grubu etkin katmanı (“Parsel”) içeriyor; silinemez. Önce grubun dışındaki bir katmanı etkinleştirin."
    );
    let objects = app.document.as_ref().expect("open").model.count("cizim");
    assert!(objects > 0);
    let _ = app.update(Message::Layer(Event::Remove("cizim".into())));
    assert_eq!(app.dialog, Some(Dialog::RemoveLayer));
    assert_eq!(app.removing_layer.as_deref(), Some("cizim"));
    // Vazgeç leaves it.
    let _ = app.update(Message::DialogClosed);
    let layers =
        |app: &crate::app::App| app.document.as_ref().expect("open").model.layers().clone();
    assert!(layers(&app).get("cizim").is_some());
    let _ = app.update(Message::Layer(Event::Remove("cizim".into())));
    let _ = app.update(Message::DialogConfirmed);
    assert_eq!(app.dialog, None);
    assert_eq!(
        last_said(&app),
        format!("“Çizim” katmanı ve üzerindeki {objects} nesne silindi.")
    );
    let model = &mut app.document.as_mut().expect("open").model;
    assert!(model.layers().get("cizim").is_none());
    assert_eq!(model.undo().as_deref(), Some("Katman sil"));
    assert_eq!(model.count("cizim"), objects);
    let empty = model
        .add_layer(kentos_domain::NewLayer::layer("Boş"), None, false)
        .expect("a new layer");
    let _ = app.update(Message::Layer(Event::Remove(empty.clone())));
    assert_eq!(app.dialog, None, "nothing to ask");
    assert_eq!(last_said(&app), "“Boş” katmanı silindi.");
    assert!(layers(&app).get(&empty).is_none());
}

#[test]
fn the_tree_shows_the_selected_objects_layer_and_keeps_the_active_one() {
    let mut app = app_with_drawing();
    let model = &app.document.as_ref().expect("open").model;
    let active = model.layers().active().to_owned();
    // An object on another layer than the active one, inside a group.
    let (slot, layer) = model
        .entities()
        .find_map(|e| {
            let layer = &e.base().layer_id;
            (layer != &active && model.layers().parent(layer).is_some())
                .then(|| (kentos_domain::Slot(e.base().id), layer.clone()))
        })
        .expect("an object in a group, off the active layer");
    let group = model.layers().parent(&layer).expect("its group").id.clone();
    app.document
        .as_mut()
        .expect("open")
        .model
        .set_layer_expanded(&group, false);
    let revision = app.document.as_ref().expect("open").model.revision();

    app.selection.set(vec![slot]);
    let _ = app.update(Message::Run("view.zoomIn"));
    assert!(app.layer_row_selected(&layer));
    let model = &app.document.as_ref().expect("open").model;
    assert_eq!(
        model.layers().active(),
        active,
        "new objects still go there"
    );
    assert!(
        model.layers().get(&group).expect("group").expanded,
        "opened to show it"
    );
    assert_eq!(model.revision(), revision, "not an edit");

    // A click in the tree chooses the rows, until the selection changes.
    let _ = app.update(Message::LayerSelected(active.clone()));
    assert!(app.layer_row_selected(&active) && !app.layer_row_selected(&layer));
    let _ = app.update(Message::Run("edit.deselect"));
    assert!(app.layer_row_selected(&active));
    app.selection.set(vec![slot]);
    let _ = app.update(Message::Run("view.zoomIn"));
    assert!(app.layer_row_selected(&layer) && !app.layer_row_selected(&active));
    // An empty selection gives the rows back to the layer last clicked.
    let _ = app.update(Message::Run("edit.deselect"));
    assert!(app.layer_row_selected(&active) && !app.layer_row_selected(&layer));
}

#[test]
fn a_double_click_makes_a_layer_active_and_opens_or_closes_a_group() {
    let mut app = app_with_drawing();
    let press = |app: &mut crate::app::App, id: &str| {
        let _ = app.update(Message::LayerSelected(id.to_owned()));
    };
    press(&mut app, "cizim");
    let model = &app.document.as_ref().expect("open").model;
    assert_eq!(
        model.layers().active(),
        "parsel",
        "one click only chooses the row"
    );
    press(&mut app, "cizim");
    let model = &app.document.as_ref().expect("open").model;
    assert_eq!(model.layers().active(), "cizim");
    assert!(!model.is_dirty(), "not an edit");
    let open = model.layers().get("layer-g").expect("group").expanded;
    press(&mut app, "layer-g");
    press(&mut app, "layer-g");
    let model = &app.document.as_ref().expect("open").model;
    assert_eq!(
        model.layers().get("layer-g").expect("group").expanded,
        !open
    );
    // A third press starts over: one click.
    press(&mut app, "layer-g");
    let model = &app.document.as_ref().expect("open").model;
    assert_eq!(
        model.layers().get("layer-g").expect("group").expanded,
        !open
    );
}

#[test]
fn the_rows_menu_does_what_the_webs_does() {
    use super::Event;
    let mut app = app_with_drawing();
    let layer = |app: &mut crate::app::App, e: Event| {
        let _ = app.update(Message::Layer(e));
    };
    layer(&mut app, Event::Activate("bina".into()));
    let model = &app.document.as_ref().expect("open").model;
    assert_eq!(model.layers().active(), "bina");
    // Yalnızca bunu göster.
    layer(&mut app, Event::Isolate("bina".into()));
    let model = &app.document.as_ref().expect("open").model;
    assert!(model.layers().is_visible("bina") && !model.layers().is_visible("parsel"));
    let _ = app.update(Message::Run("layer.showAll"));
    // Nesnelerini seç: a group's are all its layers'.
    layer(&mut app, Event::SelectObjects("layer-g".into()));
    assert_eq!(app.selection.len(), 4);
    assert_eq!(last_said(&app), "Kadastro: 4 nesne seçildi.");
    // Colour, line type and weight: one undo step each.
    layer(&mut app, Event::Color("bina".into(), "#4F8EF7".into()));
    layer(
        &mut app,
        Event::LineType("bina".into(), kentos_commands::LineType::Dashed),
    );
    layer(&mut app, Event::LineWeight("bina".into(), 0.35));
    let model = &app.document.as_ref().expect("open").model;
    let style = &model.layers().get("bina").expect("layer").style;
    assert_eq!(style.color, "#4F8EF7");
    assert_eq!(style.line_type, kentos_commands::LineType::Dashed);
    assert_eq!(style.line_weight, 0.35);
    let _ = app.update(Message::Run("edit.undo"));
    let model = &app.document.as_ref().expect("open").model;
    assert_eq!(
        model.layers().get("bina").expect("layer").style.line_weight,
        0.25
    );
    // Yeniden adlandır: typed, then Enter; Esc leaves the name.
    layer(&mut app, Event::Rename("bina".into()));
    layer(&mut app, Event::RenameInput("Yapı".into()));
    layer(&mut app, Event::RenameDone);
    let model = &app.document.as_ref().expect("open").model;
    assert_eq!(model.layers().get("bina").expect("layer").name, "Yapı");
    layer(&mut app, Event::Rename("bina".into()));
    layer(&mut app, Event::RenameInput("Başka".into()));
    layer(&mut app, Event::RenameCancel);
    let model = &app.document.as_ref().expect("open").model;
    assert_eq!(model.layers().get("bina").expect("layer").name, "Yapı");
    // Yanına yeni katman: beside it, made active.
    layer(&mut app, Event::AddBeside("bina".into()));
    let model = &app.document.as_ref().expect("open").model;
    let new = model.layers().active().to_owned();
    assert_eq!(model.layers().get(&new).expect("new").name, "Yeni katman");
    assert_eq!(
        model.layers().parent(&new).map(|g| g.id.clone()),
        Some("layer-g".to_owned())
    );
}

#[test]
fn warnings_said_while_their_tab_is_open_are_seen() {
    let mut app = app_with_drawing();
    let _ = app.update(Message::BottomTab(crate::bottom::BottomTab::Messages));
    app.warn("Görülen uyarı.");
    let _ = app.update(Message::BottomTab(crate::bottom::BottomTab::History));
    app.warn("Görülmeyen uyarı.");
    assert_eq!(app.log.unseen(), 1);
}

#[test]
fn a_new_group_goes_to_the_top_of_the_tree() {
    let mut app = app_with_drawing();
    let active = app
        .document
        .as_ref()
        .expect("open")
        .model
        .layers()
        .active()
        .to_owned();
    let _ = app.update(Message::Run("layer.newGroup"));
    let model = &app.document.as_ref().expect("open").model;
    let last = model.layers().nodes().last().expect("a node");
    assert_eq!(last.name, "Yeni grup");
    assert!(last.children.is_empty());
    assert_eq!(model.layers().active(), active, "the active layer stays");
    assert_eq!(last_said(&app), "“Yeni grup” grubu eklendi.");
}

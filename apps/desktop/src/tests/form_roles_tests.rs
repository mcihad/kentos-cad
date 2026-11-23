//! Writing records through a published form by the cloud project's role (docs/adr/9125): a Veri girişi writes the
//! record (it has `forms.use`, not `project.edit`) while the form's design and publishing stay closed to it; a
//! Görüntüleyici opens the same form and changes nothing, told why with its role's name; an archived project is
//! read only; a locked layer says so. The buildings and their form are form_scenes'.

use kentos_commands::{ProjectPermission, ProjectRole, ProjectState, ProjectStorage};
use kentos_domain::Slot;

use crate::app::App;
use crate::form_scenes::{buildings, entry};
use crate::forms::entry as fe;
use crate::tools_scenes::run;

/// The open drawing as a cloud database project this account holds `role` in, with `permissions`.
fn as_role(app: &mut App, role: ProjectRole, permissions: Vec<ProjectPermission>, state: ProjectState) {
    let mut info = crate::cloud::tests::info(ProjectStorage::Database, false, state);
    info.access.role = role;
    info.access.permissions = permissions;
    let doc = app.document.as_mut().expect("a drawing");
    doc.source = crate::document::Source::Cloud(Box::new(crate::document::CloudSource {
        tenant: kentos_domain::Uuid::from_bytes([1; 16]),
        project: kentos_domain::Uuid::from_bytes([2; 16]),
        info,
        workspace: "Harita Bürosu".into(),
        revision: None,
    }));
}

/// The second building's storeys, as the drawing holds them.
fn storeys(app: &App) -> Option<String> {
    let doc = app.document.as_ref()?;
    doc.model.get(Slot(2))?.base().attrs.get("Kat").cloned()
}

fn said(app: &App) -> Vec<String> {
    app.log.lines().map(|l| l.text.clone()).collect()
}

/// Formla düzenle on the second building, its storeys typed as 5, Kaydet.
fn type_and_save(app: &mut App) {
    app.selection.set([Slot(2)]);
    run(app, "data.formEdit");
    assert!(app.gis.form_entry.is_some(), "the form opens");
    entry(app, fe::Event::Value("kat-sayisi".into(), "5".into()));
    entry(app, fe::Event::Save);
}

#[test]
fn a_veri_girisi_writes_the_record_and_may_not_design_the_form() {
    let mut app = App::boot(None).0;
    buildings(&mut app, true);
    as_role(&mut app, ProjectRole::Entry, vec![ProjectPermission::Read, ProjectPermission::FormsUse], ProjectState::Active);
    assert!(app.records_locked().is_none(), "forms.use writes records");
    assert!(app.forms_locked().is_some(), "but designs and publishes no form");
    type_and_save(&mut app);
    assert_eq!(storeys(&app).as_deref(), Some("5"), "{:#?}", said(&app));
    assert!(app.gis.form_entry.is_none(), "saved and closed");
}

#[test]
fn a_goruntuleyici_opens_the_form_and_changes_nothing() {
    let mut app = App::boot(None).0;
    buildings(&mut app, true);
    as_role(&mut app, ProjectRole::Viewer, vec![ProjectPermission::Read], ProjectState::Active);
    let why = app.records_locked().expect("a viewer writes no record");
    assert!(why.contains("rolünüz (Görüntüleyici) kayıt yazmaz"), "{why}");
    type_and_save(&mut app);
    assert_eq!(storeys(&app).as_deref(), Some("2"), "unchanged");
}

#[test]
fn an_archived_project_is_read_only_whatever_the_role() {
    let mut app = App::boot(None).0;
    buildings(&mut app, true);
    as_role(&mut app, ProjectRole::Entry, vec![ProjectPermission::Read, ProjectPermission::FormsUse], ProjectState::Archived);
    assert!(app.records_locked().is_some_and(|w| w.starts_with("Proje arşivde")));
}

#[test]
fn a_local_drawing_writes_its_records_as_before() {
    let mut app = App::boot(None).0;
    buildings(&mut app, true);
    assert!(app.records_locked().is_none() && app.forms_locked().is_none());
    type_and_save(&mut app);
    assert_eq!(storeys(&app).as_deref(), Some("5"));
}

/// Pictures (docs/adr/9125): the same form opened by a Veri girişi (writable) and by a Görüntüleyici (read only, its
/// role said); light at 1440×900 and dark at 1100×760.
#[test]
#[ignore = "pictures: cargo test -p kentos-desktop --lib form_roles_tests::screens -- --ignored --nocapture"]
fn screens() {
    for (name, role, permissions) in [
        ("01-veri-girisi", ProjectRole::Entry, vec![ProjectPermission::Read, ProjectPermission::FormsUse]),
        ("02-goruntuleyici", ProjectRole::Viewer, vec![ProjectPermission::Read]),
    ] {
        crate::files_testing::usage_shots("9125", name, || {
            let mut app = App::boot(None).0;
            buildings(&mut app, true);
            as_role(&mut app, role, permissions.clone(), ProjectState::Active);
            app.selection.set([Slot(2)]);
            run(&mut app, "data.formEdit");
            app
        });
    }
}

/// The 8th item's publisher (docs/adr/9174): a role that designs forms but may not publish them (`forms.design`
/// without `forms.publish`, as the server tells them apart) keeps its draft and is refused Yayınla with its role's
/// name, nothing published; given `forms.publish`, Yayınla is open.
#[test]
fn a_role_that_designs_but_does_not_publish_is_refused_yayinla_with_why() {
    use crate::forms::designer::Event as D;
    let mut app = App::boot(None).0;
    buildings(&mut app, true);
    let designs = vec![
        ProjectPermission::Read,
        ProjectPermission::Edit,
        ProjectPermission::FormsDesign,
        ProjectPermission::FormsUse,
    ];
    as_role(&mut app, ProjectRole::Manager, designs.clone(), ProjectState::Active);
    assert!(app.forms_locked().is_none(), "it designs");
    let why = app.forms_publish_locked().expect("it does not publish");
    assert!(why.contains("formu yayınlamaz") && why.contains("taslağı kaydedebilirsiniz"), "{why}");
    run(&mut app, "data.forms");
    let releases = |app: &App| app.chosen_form().map(|f| f.releases.len());
    let before = releases(&app);
    assert!(before.is_some(), "a form chosen");
    let _ = app.update(crate::app::Message::Gis(crate::gis::Event::Forms(D::Publish)));
    assert_eq!(releases(&app), before, "nothing published");
    assert!(said(&app).iter().any(|l| l.contains("formu yayınlamaz")), "{:#?}", said(&app));
    // Given forms.publish: open.
    let mut publishes = designs;
    publishes.push(ProjectPermission::FormsPublish);
    as_role(&mut app, ProjectRole::Manager, publishes, ProjectState::Active);
    assert!(app.forms_publish_locked().is_none());
}

/// The picture (`.run/gui-evidence/kullanim-9174/`): Formlar for a role that designs and does not publish.
#[test]
#[ignore = "pictures for the owner"]
fn publisher_screens() {
    crate::files_testing::usage_shots("9174", "01-yayinlamayan-rol", || {
        let mut app = App::boot(None).0;
        buildings(&mut app, true);
        as_role(
            &mut app,
            ProjectRole::Manager,
            vec![ProjectPermission::Read, ProjectPermission::Edit, ProjectPermission::FormsDesign, ProjectPermission::FormsUse],
            ProjectState::Active,
        );
        run(&mut app, "data.forms");
        app
    });
}

//! A program's cloud projects (kinds.rs, docs/adr/0802), message by message without a server: a module's program
//! lists, offers and opens only its own module's catalog types; a project of another module, by its row or by its
//! id alone (asked of the server first), is refused with the program it belongs to; this device's copies are listed
//! by their drawing's module. kentos-cad lists and opens every type as before.

use kentos_cloud::ApiFailure;
use kentos_commands::{CatalogView, ProjectState, ProjectStorage, ProjectType, Workspace};
use kentos_domain::Uuid;
use kentos_launch::Module;

use crate::app::{App, Dialog, Then};
use crate::cloud::catalog::List;
use crate::cloud::kinds::{module_of_type, types_of};
use crate::cloud::tests::{
    PROJECT, TENANT, cloud, database, info, last_said, page, signed_in, summary,
};
use crate::cloud::{Event, plan};
use crate::files_testing::{drive, scratch};

const SECOND: &str = "0199aaaa-0000-7000-8000-00000000000b";
const THIRD: &str = "0199aaaa-0000-7000-8000-00000000000c";
const FOURTH: &str = "0199aaaa-0000-7000-8000-00000000000d";

fn uuid(text: &str) -> Uuid {
    Uuid::parse_str(text).expect("a uuid")
}

/// The app of `program`, signed in.
fn signed_in_as(program: Option<Module>) -> App {
    let mut app = signed_in();
    app.program = program;
    app
}

fn catalog(app: &App) -> &crate::cloud::Catalog {
    app.cloud.catalog.as_ref().expect("the window")
}

fn row(id: &str, name: &str, kind: ProjectType) -> kentos_commands::ProjectSummary {
    let mut p = summary(id, name, ProjectStorage::Database);
    p.project_type = kind;
    p
}

/// The page on its way answered with `rows`, `total` and `next`.
fn answered(
    app: &mut App,
    more: bool,
    rows: Vec<kentos_commands::ProjectSummary>,
    total: u32,
    next: Option<&str>,
) {
    let id = catalog(app).request().expect("asked");
    cloud(
        app,
        Event::CatalogPage {
            id,
            more,
            result: Ok(page(rows, total, next)),
        },
    );
}

fn names(app: &App) -> Vec<String> {
    catalog(app)
        .projects
        .iter()
        .map(|p| p.name.clone())
        .collect()
}

#[test]
fn every_catalog_type_is_one_modules_and_a_program_offers_its_own() {
    // The server's seven types, each one module's: a building's KentOS Mimari's, the CBS type KentOS GIS's, every
    // other (the map's and the general CAD's) KentOS Harita's.
    for t in plan::PROJECT_TYPES {
        let owner = module_of_type(t).expect("a module");
        assert!(types_of(owner).contains(&t), "{t:?}");
    }
    assert_eq!(types_of(Module::Architecture), [ProjectType::Architecture]);
    assert_eq!(
        types_of(Module::Map),
        [
            ProjectType::Cad,
            ProjectType::LandReadjustment,
            ProjectType::ZoningPlan,
            ProjectType::Subdivision,
            ProjectType::Road,
        ]
    );
    assert_eq!(types_of(Module::Gis), [ProjectType::Gis]);
    assert_eq!(signed_in_as(None).cloud_types(), plan::PROJECT_TYPES);

    // What the catalog asks the server for and offers in its filter.
    for (program, types, asked, sifted) in [
        (None, plan::PROJECT_TYPES.to_vec(), None, false),
        (
            Some(Module::Architecture),
            vec![ProjectType::Architecture],
            Some(ProjectType::Architecture),
            false,
        ),
        (Some(Module::Map), types_of(Module::Map), None, true),
        (
            Some(Module::Gis),
            vec![ProjectType::Gis],
            Some(ProjectType::Gis),
            false,
        ),
    ] {
        let mut app = signed_in_as(program);
        let _ = app.open_catalog();
        let c = catalog(&app);
        assert_eq!(c.types, types, "{program:?}");
        assert_eq!(c.asked_type(), asked, "{program:?}");
        assert_eq!(c.sifted(), sifted, "{program:?}");
        assert_eq!(
            app.cloud_types(),
            types,
            "{program:?}: Buluta yükle and Proje bilgileri"
        );
    }

    // KentOS Harita's filter takes its own types and no other.
    let mut app = signed_in_as(Some(Module::Map));
    let _ = app.open_catalog();
    let before = catalog(&app).request();
    cloud(&mut app, Event::CatalogKind(Some(ProjectType::Gis)));
    assert_eq!(
        catalog(&app).kind,
        None,
        "another module's type is not chosen"
    );
    assert_eq!(catalog(&app).request(), before, "nor asked for");
    cloud(&mut app, Event::CatalogKind(Some(ProjectType::ZoningPlan)));
    assert_eq!(catalog(&app).kind, Some(ProjectType::ZoningPlan));
    assert_eq!(catalog(&app).asked_type(), Some(ProjectType::ZoningPlan));
    assert_ne!(catalog(&app).request(), before, "asked again with the type");
}

#[test]
fn harita_leaves_other_modules_rows_out_and_goes_on_past_a_page_of_none() {
    let mut app = signed_in_as(Some(Module::Map));
    let _ = app.open_catalog();
    cloud(&mut app, Event::CatalogView(List::View(CatalogView::Mine)));
    answered(
        &mut app,
        false,
        vec![
            row(PROJECT, "Ada 7", ProjectType::Cad),
            row(SECOND, "Bina", ProjectType::Architecture),
            row(THIRD, "Mahalle analizi", ProjectType::Gis),
            row(FOURTH, "İmar planı", ProjectType::ZoningPlan),
        ],
        4,
        None,
    );
    assert_eq!(names(&app), ["Ada 7", "İmar planı"]);
    assert_eq!(
        catalog(&app).total,
        2,
        "the list's own count, its last page in"
    );

    // A page of other modules' projects only: the next is asked at once, the count not known before the last page.
    cloud(
        &mut app,
        Event::CatalogView(List::View(CatalogView::Recent)),
    );
    answered(
        &mut app,
        false,
        vec![
            row(SECOND, "Bina", ProjectType::Architecture),
            row(THIRD, "Mahalle analizi", ProjectType::Gis),
        ],
        40,
        Some("sonraki"),
    );
    assert!(names(&app).is_empty());
    assert!(catalog(&app).loading(), "the next page is on its way");
    assert_eq!(catalog(&app).total, 0, "not every type's 40");
    answered(
        &mut app,
        true,
        vec![row(PROJECT, "Ada 7", ProjectType::Road)],
        40,
        Some("daha"),
    );
    assert_eq!(names(&app), ["Ada 7"]);
    assert!(
        !catalog(&app).loading(),
        "a page with one of its own stops there"
    );

    // Empty, it says whose projects are listed; narrowed to a type, to try without it.
    answered_empty_view(&mut app);
    let said = crate::cloud::catalog::empty_text(catalog(&app), 1);
    assert!(
        said.ends_with("Burada yalnız KentOS Harita projeleri listelenir; başka türdeki projeler kendi programlarında açılır."),
        "{said}"
    );
    cloud(&mut app, Event::CatalogKind(Some(ProjectType::Road)));
    answered(&mut app, false, Vec::new(), 0, None);
    assert_eq!(
        crate::cloud::catalog::empty_text(catalog(&app), 1),
        plan::EMPTY_SEARCH
    );

    // KentOS Mimari's one type is no filter to take off.
    let mut app = signed_in_as(Some(Module::Architecture));
    let _ = app.open_catalog();
    cloud(&mut app, Event::CatalogView(List::View(CatalogView::Mine)));
    answered(&mut app, false, Vec::new(), 0, None);
    let said = crate::cloud::catalog::empty_text(catalog(&app), 1);
    assert!(said.contains("yalnız KentOS Mimari projeleri"), "{said}");
    assert_ne!(said, plan::EMPTY_SEARCH);
}

/// Projelerim answered empty.
fn answered_empty_view(app: &mut App) {
    cloud(app, Event::CatalogView(List::View(CatalogView::Mine)));
    answered(app, false, Vec::new(), 0, None);
}

#[test]
fn another_modules_project_is_refused_with_the_program_it_belongs_to() {
    let project = uuid(PROJECT);
    let tenant = uuid(TENANT);
    let ask = |app: &mut App, name: &str, kind: ProjectType| {
        app.cloud.open_hint = Some((name.to_owned(), ProjectStorage::Database));
        app.cloud.open_kind = Some((project, Some(kind)));
        let task = app.leave(Then::OpenCloud { tenant, project });
        drop(task);
    };
    // KentOS Harita: a building is KentOS Mimari's, a CBS project KentOS GIS's.
    let mut app = signed_in_as(Some(Module::Map));
    ask(&mut app, "Bina", ProjectType::Architecture);
    assert!(app.cloud.opening.is_none(), "nothing is read");
    assert_eq!(
        last_said(&app),
        "“Bina” KentOS Mimari'nin projesi; onu KentOS Mimari'de açın."
    );
    assert!(app.cloud.open_hint.is_none() && app.cloud.open_kind.is_none());
    ask(&mut app, "Mahalle analizi", ProjectType::Gis);
    assert_eq!(
        last_said(&app),
        "“Mahalle analizi” KentOS GIS'in projesi; onu KentOS GIS'te açın."
    );
    // Its own types open.
    ask(&mut app, "Ada 7", ProjectType::Subdivision);
    assert_eq!(
        app.cloud.opening.as_ref().map(|o| o.name.as_str()),
        Some("Ada 7")
    );
    assert!(app.cloud.open_kind.is_none(), "spent");

    // KentOS GIS refuses a map's; KentOS Mimari a map's.
    let mut app = signed_in_as(Some(Module::Gis));
    ask(&mut app, "Ada 7", ProjectType::Cad);
    assert_eq!(
        last_said(&app),
        "“Ada 7” KentOS Harita'nın projesi; onu KentOS Harita'da açın."
    );
    let mut app = signed_in_as(Some(Module::Architecture));
    ask(&mut app, "İmar planı", ProjectType::ZoningPlan);
    assert!(app.cloud.opening.is_none());
    assert_eq!(
        last_said(&app),
        "“İmar planı” KentOS Harita'nın projesi; onu KentOS Harita'da açın."
    );
    // kentos-cad opens every type, asking nothing first.
    let mut app = signed_in_as(None);
    ask(&mut app, "Bina", ProjectType::Architecture);
    assert!(app.cloud.opening.is_some());
    let mut app = signed_in_as(None);
    app.cloud.open_hint = Some(("Ada 7".into(), ProjectStorage::Database));
    let _ = app.leave(Then::OpenCloud { tenant, project });
    assert!(app.cloud.opening.is_some() && app.cloud.open_check.is_none());

    // Over the catalog the refusal is the window's line.
    let mut app = signed_in_as(Some(Module::Map));
    let _ = app.open_catalog();
    assert_eq!(app.dialog, Some(Dialog::Catalog));
    ask(&mut app, "Bina", ProjectType::Architecture);
    let status = catalog(&app).status.as_ref().expect("said in the window");
    assert!(status.error);
    assert_eq!(
        status.text,
        "“Bina” KentOS Mimari'nin projesi; onu KentOS Mimari'de açın."
    );
}

#[test]
fn a_project_known_by_its_id_alone_is_asked_of_the_server_first() {
    let project = uuid(PROJECT);
    let tenant = uuid(TENANT);
    // The application menu's recent project: its id and name, not its type.
    let by_id = |app: &mut App| {
        app.cloud.open_hint = Some(("Mahalle analizi".into(), ProjectStorage::Database));
        let _ = app.leave(Then::OpenCloud { tenant, project });
        app.cloud
            .open_check
            .map(|(id, _)| id)
            .expect("asked of the server")
    };
    let mut app = signed_in_as(Some(Module::Map));
    let id = by_id(&mut app);
    assert!(
        app.cloud.opening.is_none(),
        "not read before its type is known"
    );
    cloud(
        &mut app,
        Event::OpenChecked {
            id,
            tenant,
            project,
            result: Ok((
                "Mahalle analizi".into(),
                ProjectType::Gis,
                ProjectStorage::Database,
            )),
        },
    );
    assert!(app.cloud.opening.is_none());
    assert_eq!(
        last_said(&app),
        "“Mahalle analizi” KentOS GIS'in projesi; onu KentOS GIS'te açın."
    );

    // One of its own opens, named as the server names it when the menu did not.
    let id = {
        let _ = app.leave(Then::OpenCloud { tenant, project });
        app.cloud.open_check.map(|(id, _)| id).expect("asked")
    };
    cloud(
        &mut app,
        Event::OpenChecked {
            id,
            tenant,
            project,
            result: Ok((
                "Ada 7".into(),
                ProjectType::ZoningPlan,
                ProjectStorage::File,
            )),
        },
    );
    let opening = app.cloud.opening.as_ref().expect("opening");
    assert_eq!(
        (opening.name.as_str(), opening.storage),
        ("Ada 7", Some(ProjectStorage::File))
    );

    // The server does not answer: it opens as it did before.
    let mut app = signed_in_as(Some(Module::Map));
    let id = by_id(&mut app);
    cloud(
        &mut app,
        Event::OpenChecked {
            id,
            tenant,
            project,
            result: Err(ApiFailure::new(0, "network", "Sunucuya ulaşılamadı.")),
        },
    );
    assert!(app.cloud.opening.is_some());

    // A late answer, after another question, is dropped; another drawing on screen meanwhile opens nothing.
    let mut app = signed_in_as(Some(Module::Map));
    let first = by_id(&mut app);
    let second = by_id(&mut app);
    assert_ne!(first, second);
    cloud(
        &mut app,
        Event::OpenChecked {
            id: first,
            tenant,
            project,
            result: Ok(("Ada 7".into(), ProjectType::Cad, ProjectStorage::Database)),
        },
    );
    assert!(app.cloud.opening.is_none() && app.cloud.open_check.is_some());
    app.document = Some(crate::files_testing::drawing(0));
    cloud(
        &mut app,
        Event::OpenChecked {
            id: second,
            tenant,
            project,
            result: Ok(("Ada 7".into(), ProjectType::Cad, ProjectStorage::Database)),
        },
    );
    assert!(app.cloud.opening.is_none());
    assert!(
        last_said(&app)
            .starts_with("“Mahalle analizi” açılmadı: açılış sürerken ekrandaki çizim değişti")
    );

    // Without a session nothing can be asked: this device's copy opens as before.
    let (mut app, _) = App::boot(None);
    app.program = Some(Module::Map);
    assert!(app.cloud_open_check(tenant, project).is_none());
}

#[test]
fn this_devices_copies_are_listed_by_their_drawings_module() {
    // The settings of a copy's drawing: a CBS project, a map, a building, a CAD drawing not asked its work.
    let settings = |kind: Option<Workspace>, work: Option<kentos_commands::CadWork>| {
        let mut i = info(ProjectStorage::Database, true, ProjectState::Active);
        i.settings.workspace = kind;
        i.settings.cad_work = work;
        i
    };
    let gis = settings(Some(Workspace::Gis), None);
    let map = settings(Some(Workspace::Cad), Some(kentos_commands::CadWork::Map));
    let building = settings(
        Some(Workspace::Cad),
        Some(kentos_commands::CadWork::Architecture),
    );
    let unasked = settings(Some(Workspace::Cad), None);
    for (program, shown) in [
        (None, [true, true, true, true]),
        (Some(Module::Architecture), [false, false, true, true]),
        (Some(Module::Map), [false, true, false, true]),
        (Some(Module::Gis), [true, false, false, true]),
    ] {
        let app = signed_in_as(program);
        let listed = [&gis, &map, &building, &unasked].map(|i| app.lists_copy(i));
        assert_eq!(listed, shown, "{program:?}");
    }

    // Through the catalog: a CBS project's copy is KentOS GIS's and kentos-cad's, not KentOS Harita's.
    let dir = scratch("bulut-program-kopya");
    let mut first = signed_in();
    first.cloud.replicas = Some(kentos_cloud::ReplicaStore::new(dir.join("kopya")));
    database(&mut first);
    let task = first.update(crate::app::Message::CloseRequested(
        iced::window::Id::unique(),
    ));
    drive(&mut first, task);
    drop(first);
    for (program, copies) in [(None, 1), (Some(Module::Gis), 1), (Some(Module::Map), 0)] {
        let (mut app, _) = App::boot(None);
        app.program = program;
        let _ = app.settings.choose(&[
            (
                "cloud.server",
                serde_json::Value::from("http://127.0.0.1:9"),
            ),
            (
                "cloud.account",
                serde_json::Value::from("0199aaaa-0000-7000-8000-00000000a1a1"),
            ),
        ]);
        app.cloud.replicas = Some(kentos_cloud::ReplicaStore::new(dir.join("kopya")));
        let _ = app.open_catalog();
        let c = catalog(&app);
        assert_eq!(c.list, List::Device, "{program:?}");
        assert_eq!(c.device.len(), copies, "{program:?}");
    }
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn the_upload_and_the_projects_form_offer_the_programs_types() {
    // A type of another module is not taken by the upload.
    let mut app = crate::files_testing::app_with_drawing();
    app.program = Some(Module::Architecture);
    app.cloud.client = signed_in().cloud.client.clone();
    app.cloud.me = signed_in().cloud.me.clone();
    app.open_upload(ProjectStorage::Database);
    let u = app.cloud.upload.as_ref().expect("Buluta yükle");
    assert_eq!(u.project_type, ProjectType::Architecture);
    cloud(&mut app, Event::UploadType(ProjectType::Gis));
    assert_eq!(
        app.cloud.upload.as_ref().map(|u| u.project_type),
        Some(ProjectType::Architecture),
        "not a building's type: not taken"
    );
    let hint = crate::cloud::kinds::type_hint(app.program);
    assert!(
        hint.starts_with(plan::TYPE_HINT) && hint.contains("KentOS Mimari'nin türleri"),
        "{hint}"
    );
    assert_eq!(crate::cloud::kinds::type_hint(None), plan::TYPE_HINT);
}

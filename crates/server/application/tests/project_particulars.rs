//! Künyenin kişisel bilgileri (docs/adr/8100; plan/bim-plan.md BIM-15, "yetkisiz kullanıcı künyeyi göremez"): the
//! permit set's künye variables that name people — and who delivered a package — go to whoever works on the project's
//! objects. An İzleyici reads them cut in the project's metadata and its snapshot, and gets no stored copy that carries
//! them (a checkpoint, a file project's revision, an exported copy, a file project added to a cloud portal dossier); a
//! stored copy without them is still hers. Real database (`KENTOS_TEST_DB=required` makes a missing one a failure).

mod common;

use std::time::Duration;

use common::{PASSWORD, account, blobs, envelope, member, new_project, open, share};
use kentos_application::access::ProjectAccess;
use kentos_application::blobs::Blobs;
use kentos_application::commands::{CatalogPolicy, run};
use kentos_application::jobs::{self, JobLimits};
use kentos_application::portal::{accounts, dossiers, files as dossier_files, platform};
use kentos_application::{
    AppError, admin, changes, checkpoints, files, identity, projects, snapshot,
};
use kentos_contracts::{
    ApplicationCreate, CUT_PARTICULAR, CommandEnvelope, DocumentSnapshotV1, DossierSite,
    FileUploadBegin, FromProject, GrantRole, NewOwner, OrganizationCategory, PROJECT_FILE_COMMIT,
    PlatformTenantCreate, Profession, ProfessionalReview, ProfessionalStatus, ProjectChanges,
    ProjectCreate, ProjectPatch, ProjectSettings, ProjectStorage, ServerJobCreate, ServerJobState,
    SignInMethod, Signup, TenantRole, VariableValue,
};
use kentos_postgres::testing::TestDb;
use serde_json::json;
use sha2::{Digest, Sha256};
use uuid::Uuid;

const SAMPLE: &str = include_str!("../../../../fixtures/document/v1/sample.json");
const DRAWING: &[u8] = include_bytes!("../../../../fixtures/kcad/v2/drawing.kcad");
const LEASE: Duration = Duration::from_secs(30);
const OWNER: &str = "Ayşe Yılmaz";
const ARCHITECT: &str = "Ali Kaya";

fn sha(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// `settings` with the künye given: the owner, the architect and his registration number, the project's number, and
/// a delivery by the architect.
fn with_kunye(settings: &ProjectSettings) -> ProjectSettings {
    let mut s = settings.clone();
    s.variables = serde_json::from_value(json!([
        {"name": "yapi_sahibi", "label": "Yapı sahibi", "value": OWNER},
        {"name": "mimar", "label": "Mimar (müellif) adı soyadı", "value": ARCHITECT},
        {"name": "sicil_no", "label": "Mimarlar Odası sicil no", "kind": "number", "value": 41187},
        {"name": "proje_no", "label": "Proje no (büronun)", "value": "2026/14"}
    ]))
    .unwrap();
    let mut arch = s.arch.clone().unwrap_or_default();
    arch.deliveries = serde_json::from_value(json!([{
        "id": "0192f5a1-8100-7000-8000-000000000001", "number": 1, "revision": "000",
        "package": {"manifest": "a".repeat(64), "files": 3}, "to": "Sivas Belediyesi", "channel": "epys",
        "at": "2026-11-02", "by": ARCHITECT
    }]))
    .unwrap();
    s.arch = Some(arch);
    s
}

/// The values of the künye's variables a settings carries, by name, and who delivered.
fn kunye(s: &ProjectSettings) -> (Vec<(String, VariableValue)>, Option<String>) {
    let vars = s
        .variables
        .iter()
        .map(|v| (v.name.clone(), v.value.clone()))
        .collect();
    let by = s
        .arch
        .as_ref()
        .and_then(|a| a.deliveries.first())
        .and_then(|d| d.by.clone());
    (vars, by)
}

fn whole() -> (Vec<(String, VariableValue)>, Option<String>) {
    (
        vec![
            ("yapi_sahibi".into(), VariableValue::Text(OWNER.into())),
            ("mimar".into(), VariableValue::Text(ARCHITECT.into())),
            ("sicil_no".into(), VariableValue::Number(41187.0)),
            ("proje_no".into(), VariableValue::Text("2026/14".into())),
        ],
        Some(ARCHITECT.into()),
    )
}

fn cut() -> (Vec<(String, VariableValue)>, Option<String>) {
    (
        vec![
            (
                "yapi_sahibi".into(),
                VariableValue::Text(CUT_PARTICULAR.into()),
            ),
            ("mimar".into(), VariableValue::Text(CUT_PARTICULAR.into())),
            ("sicil_no".into(), VariableValue::Null),
            ("proje_no".into(), VariableValue::Text("2026/14".into())),
        ],
        None,
    )
}

fn refused_for_the_kunye<T>(r: Result<T, AppError>) {
    match r {
        Err(AppError::Forbidden(m)) => assert!(m.contains("künyenin kişisel bilgilerini"), "{m}"),
        Err(e) => panic!("refused for another reason: {e}"),
        Ok(_) => panic!("given to an İzleyici"),
    }
}

#[tokio::test]
async fn a_viewer_reads_the_kunye_cut_and_gets_no_stored_copy_that_carries_it() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    admin::create_tenant(&db.owner, "buro", "Mimarlık Bürosu", 4)
        .await
        .unwrap();
    let ayse = member(&db, "buro", "ayse", TenantRole::ProjectManager).await;
    let dilek = member(&db, "buro", "dilek", TenantRole::Viewer).await;
    let emre = member(&db, "buro", "emre", TenantRole::Viewer).await;
    let project = new_project(&db, &ayse, "Konut").await;
    let by = open(&db, &ayse, project).await;
    let info = projects::info(&db.app, &by).await.unwrap();
    let patch = ProjectChanges {
        blocks: Vec::new(),
        features: Vec::new(),
        project: Some(ProjectPatch {
            settings: Some(with_kunye(&info.settings)),
            ..Default::default()
        }),
    };
    changes::commit(
        &db.app,
        &by,
        envelope(&ayse, project, patch, &[("@project", &info.meta_version)]),
    )
    .await
    .unwrap();
    share(&db, &by, &dilek.actor, GrantRole::Viewer).await;
    share(&db, &by, &emre.actor, GrantRole::Editor).await;
    let viewer = open(&db, &dilek, project).await;
    let editor = open(&db, &emre, project).await;
    assert!(!viewer.sees_particulars() && editor.sees_particulars() && by.sees_particulars());

    // The metadata: whole for the editor, cut for the viewer.
    assert_eq!(
        kunye(&projects::info(&db.app, &editor).await.unwrap().settings),
        whole()
    );
    assert_eq!(
        kunye(&projects::info(&db.app, &viewer).await.unwrap().settings),
        cut()
    );

    // The snapshot: the same.
    let read = |bytes: &[u8]| kentos_kcad::decode(bytes).unwrap().settings;
    assert_eq!(
        kunye(&read(
            &snapshot::snapshot(&db.app, &editor).await.unwrap().bytes
        )),
        whole()
    );
    assert_eq!(
        kunye(&read(
            &snapshot::snapshot(&db.app, &viewer).await.unwrap().bytes
        )),
        cut()
    );

    // A checkpoint is kept whole: the editor takes it, the viewer does not.
    let store = blobs();
    let limits = JobLimits::default();
    let job = |kind: &str, input: serde_json::Value, request: &str| ServerJobCreate {
        kind: kind.into(),
        input,
        request_id: request.into(),
        timeout_seconds: None,
        fresh: false,
    };
    jobs::create(
        &db.app,
        &by,
        job("project.checkpoint", json!({"name": "Teslim"}), "k"),
        &limits,
    )
    .await
    .unwrap();
    let done = jobs::work_once(&db.app, &store, Uuid::now_v7(), LEASE, &limits)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(done.state, ServerJobState::Succeeded);
    let id =
        Uuid::parse_str(&checkpoints::list(&db.app, &by).await.unwrap().checkpoints[0].id).unwrap();
    assert!(
        checkpoints::download(&db.app, &store, &editor, id)
            .await
            .is_ok()
    );
    refused_for_the_kunye(checkpoints::download(&db.app, &store, &viewer, id).await);

    // An export the owner asked for is made for her: the viewer does not take it.
    let export = jobs::create(&db.app, &by, job("project.export", json!({}), "e"), &limits)
        .await
        .unwrap();
    let done = jobs::work_once(&db.app, &store, Uuid::now_v7(), LEASE, &limits)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(done.state, ServerJobState::Succeeded);
    let export = Uuid::parse_str(&export.id).unwrap();
    assert!(
        jobs::artifact(&db.app, &store, &editor, export)
            .await
            .is_ok()
    );
    refused_for_the_kunye(jobs::artifact(&db.app, &store, &viewer, export).await);
    db.close().await;
}

async fn file_project(db: &TestDb, who: &kentos_application::tenancy::Access) -> Uuid {
    let s = DocumentSnapshotV1::from_json(SAMPLE).unwrap();
    let input = ProjectCreate {
        dimension_sets: Vec::new(),
        casework: Vec::new(),
        constraints: Default::default(),
        name: "Konut dosyası".into(),
        settings: s.settings,
        origin: s.origin,
        home_view: s.home_view,
        layers: s.layers,
        active_layer: s.active_layer,
        styles: s.styles,
        sheets: None,
        arrays: Vec::new(),
        description: None,
        project_type: None,
        tags: None,
        storage: Some(ProjectStorage::File),
    };
    Uuid::parse_str(
        &projects::create(&db.app, who, input, None)
            .await
            .unwrap()
            .id,
    )
    .unwrap()
}

/// Saves `bytes` as the file project's next revision after `expected`.
async fn save(db: &TestDb, store: &Blobs, by: &ProjectAccess, bytes: &[u8], expected: u32) {
    let begun = files::begin(
        &db.app,
        by,
        FileUploadBegin {
            size: bytes.len() as u32,
            sha256: sha(bytes),
        },
    )
    .await
    .unwrap();
    let id = Uuid::parse_str(&begun.id).unwrap();
    let mut receiving = files::start_receive(&db.app, store, by, id).await.unwrap();
    receiving.writer.write(bytes).await.unwrap();
    files::finish_receive(&db.app, store, by, receiving)
        .await
        .unwrap();
    let commit = CommandEnvelope {
        command_name: PROJECT_FILE_COMMIT.into(),
        version: 1,
        tenant_id: by.tenant.to_string(),
        project_id: by.project.to_string(),
        request_id: format!("istek-{}", Uuid::new_v4()),
        idempotency_key: Uuid::new_v4().to_string(),
        expected_versions: [("@file".to_string(), expected.to_string())]
            .into_iter()
            .collect(),
        input: json!({ "uploadId": id.to_string() }),
    };
    run(&db.app, store, &CatalogPolicy::default(), by, commit)
        .await
        .unwrap();
}

#[tokio::test]
async fn a_file_project_s_revision_that_carries_the_kunye_is_not_the_viewer_s() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    admin::create_tenant(&db.owner, "buro", "Mimarlık Bürosu", 3)
        .await
        .unwrap();
    let ayse = member(&db, "buro", "ayse", TenantRole::ProjectManager).await;
    let dilek = member(&db, "buro", "dilek", TenantRole::Viewer).await;
    let project = file_project(&db, &ayse).await;
    let by = open(&db, &ayse, project).await;
    share(&db, &by, &dilek.actor, GrantRole::Viewer).await;
    let viewer = open(&db, &dilek, project).await;
    let store = blobs();
    // The first revision has no künye; the second has it.
    let mut doc = kentos_kcad::decode(DRAWING).unwrap();
    doc.entities.truncate(13);
    doc.uids.truncate(13);
    let plain = kentos_kcad::encode(&doc).unwrap();
    doc.settings = with_kunye(&doc.settings);
    let given = kentos_kcad::encode(&doc).unwrap();
    save(&db, &store, &by, &plain, 0).await;
    save(&db, &store, &by, &given, 1).await;
    assert!(
        files::download(&db.app, &store, &viewer, 1).await.is_ok(),
        "no künye: the viewer's"
    );
    refused_for_the_kunye(files::download(&db.app, &store, &viewer, 2).await);
    let (_, mut file) = files::download(&db.app, &store, &by, 2).await.unwrap();
    let mut bytes = Vec::new();
    tokio::io::AsyncReadExt::read_to_end(&mut file, &mut bytes)
        .await
        .unwrap();
    assert_eq!(
        sha(&bytes),
        sha(&given),
        "the owner takes it as it was saved"
    );
    db.close().await;
}

/// A file project's newest revision goes into a cloud portal dossier as it was stored (docs/adr/9000 §7): Ayşe, a
/// verified architect, adds an office's project she is an İzleyici of to her application. The revision without the
/// künye goes in; the one with it does not until she is a Düzenleyici, and then as it was saved.
#[tokio::test]
async fn a_file_project_s_revision_that_carries_the_kunye_goes_into_a_dossier_only_from_its_editors()
 {
    let Some(db) = TestDb::create().await else {
        return;
    };
    let store = blobs();
    let root = account(&db, "sistem").await;
    admin::set_platform_admin(&db.owner, "sistem", true)
        .await
        .unwrap();
    let municipality = PlatformTenantCreate {
        slug: "ornek".into(),
        name: "Örnek Belediyesi".into(),
        category: OrganizationCategory::Municipality,
        province: Some("Örnek".into()),
        district: Some("Merkez".into()),
        seats: 3,
        owner: Some(NewOwner {
            user_id: None,
            login: Some("mehmet".into()),
            password: Some(PASSWORD.into()),
            display_name: Some("Mehmet Demir".into()),
            email: Some("mehmet@ornek.test".into()),
        }),
    };
    let ornek = platform::create_tenant(&db.app, &root, &municipality)
        .await
        .unwrap();
    let signup = Signup {
        login: "ayse".into(),
        password: PASSWORD.into(),
        display_name: OWNER.into(),
        email: "ayse@posta.test".into(),
        profession: Profession::Architect,
        chamber_no: "41187".into(),
        office_name: None,
        phone: None,
        province: None,
        ticket: None,
        website: None,
    };
    let id = accounts::sign_up(&db.app, &signup, Some(&root))
        .await
        .unwrap();
    let verified = ProfessionalReview {
        status: ProfessionalStatus::Verified,
        note: String::new(),
    };
    platform::review(&db.app, &root, id, &verified)
        .await
        .unwrap();
    let user = identity::check_local_login(&db.app, "ayse", PASSWORD)
        .await
        .unwrap()
        .expect("signs in");
    let ayse = identity::actor_of(&db.app, user, SignInMethod::Local)
        .await
        .unwrap()
        .unwrap();
    let application = ApplicationCreate {
        tenant_id: ornek.to_string(),
        kind: "architectural_project".into(),
        title: "412 ada 7 parsel konut".into(),
        description: String::new(),
        site: DossierSite {
            neighbourhood: Some("Cumhuriyet Mahallesi".into()),
            block: Some("412".into()),
            parcel: Some("7".into()),
            ..DossierSite::default()
        },
        owner_name: None,
        office_id: None,
    };
    let dossier = dossiers::create_application(&db.app, &ayse, &application)
        .await
        .unwrap();
    // An office's file project, shared with her as an İzleyici.
    admin::create_tenant(&db.owner, "buro", "Mimarlık Bürosu", 3)
        .await
        .unwrap();
    let mert = member(&db, "buro", "mert", TenantRole::ProjectManager).await;
    admin::set_membership(&db.owner, "buro", "ayse", TenantRole::Viewer, true)
        .await
        .unwrap();
    let project = file_project(&db, &mert).await;
    let by = open(&db, &mert, project).await;
    share(&db, &by, &ayse, GrantRole::Viewer).await;
    let mut doc = kentos_kcad::decode(DRAWING).unwrap();
    doc.entities.truncate(13);
    doc.uids.truncate(13);
    let plain = kentos_kcad::encode(&doc).unwrap();
    doc.settings = with_kunye(&doc.settings);
    let given = kentos_kcad::encode(&doc).unwrap();
    let from = FromProject {
        tenant_id: by.tenant.to_string(),
        project_id: project.to_string(),
        doc_type: None,
    };
    save(&db, &store, &by, &plain, 0).await;
    let added = dossier_files::from_project(&db.app, &store, &ayse, dossier, &from)
        .await
        .unwrap();
    assert_eq!(added.sha256, sha(&plain), "no künye: hers to add");
    save(&db, &store, &by, &given, 1).await;
    refused_for_the_kunye(
        dossier_files::from_project(&db.app, &store, &ayse, dossier, &from).await,
    );
    share(&db, &by, &ayse, GrantRole::Editor).await;
    let added = dossier_files::from_project(&db.app, &store, &ayse, dossier, &from)
        .await
        .unwrap();
    assert_eq!(
        added.sha256,
        sha(&given),
        "a Düzenleyici adds it as it was saved"
    );
    db.close().await;
}

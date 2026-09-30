//! A database project's block definitions against a real PostgreSQL/PostGIS
//! (docs/adr/0144 §5): made, replaced and removed in the objects' commit with
//! versions and conflicts; the rules over them in the documents' words; an
//! insert's geometry its block's objects placed, made again when a
//! definition it places changes; and the definitions travelling with the
//! project (its `.kcad` image, a copy, an import).

mod common;

use common::{blobs, catalog_envelope, envelope, member, new_project, open, run, share};
use kentos_application::commands::{self, CatalogPolicy, CommandOutcome};
use kentos_application::{AppError, admin, blocks, changes, events, projects, snapshot};
use kentos_contracts::{
    BlockChange, BlockDefinition, ConflictReason, Entity, FeatureChange, FeatureOp, GrantRole,
    PROJECT_CHECKPOINT_CREATE, PROJECT_CHECKPOINT_RESTORE, PROJECT_DUPLICATE, ProjectChanges,
    TenantRole, block_key,
};
use kentos_postgres::testing::TestDb;
use serde_json::json;
use uuid::Uuid;

const LAMBA: &str = "018f3a2b-0000-7000-8000-00000000000a";
const DIREK: &str = "018f3a2b-0000-7000-8000-00000000000b";

/// A lamp: a crossbar and its centre.
fn lamba(half: f64) -> BlockDefinition {
    serde_json::from_value(json!({
        "id": LAMBA, "name": "Lamba", "base": { "x": 0, "y": 0 },
        "entities": [
            { "kind": "line", "id": 1, "layerId": "", "attrs": {}, "a": { "x": -half, "y": 0 }, "b": { "x": half, "y": 0 } },
            { "kind": "point", "id": 2, "layerId": "", "attrs": {}, "p": { "x": 0, "y": 0 } }
        ]
    }))
    .unwrap()
}

/// A pole six metres high with the lamp on its top.
fn direk() -> BlockDefinition {
    serde_json::from_value(json!({
        "id": DIREK, "name": "Direk", "base": { "x": 0, "y": 0 },
        "entities": [
            { "kind": "line", "id": 1, "layerId": "", "attrs": {}, "a": { "x": 0, "y": 0 }, "b": { "x": 0, "y": 6 } },
            { "kind": "insert", "id": 2, "layerId": "", "attrs": {}, "block": LAMBA, "p": { "x": 0, "y": 6 }, "scale": 1, "rotation": 0 }
        ]
    }))
    .unwrap()
}

fn insert(block: &str) -> Entity {
    serde_json::from_value(
        json!({ "kind": "insert", "id": 1, "layerId": "cizim", "attrs": { "No": "7" },
        "block": block, "p": { "x": 486500, "y": 4420200 }, "scale": 2, "rotation": 0 }),
    )
    .unwrap()
}

fn with(blocks: Vec<BlockChange>, features: Vec<FeatureChange>) -> ProjectChanges {
    ProjectChanges {
        features,
        project: None,
        blocks,
    }
}

/// The insert's geometry as PostGIS writes it.
async fn geometry(db: &TestDb, id: Uuid) -> (String, i64) {
    sqlx::query_as("select public.st_astext(geom), version from kentos.feature where id = $1")
        .bind(id)
        .fetch_one(&db.owner)
        .await
        .unwrap()
}

#[tokio::test]
async fn blocks_go_in_with_their_inserts_and_follow_their_changes() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    admin::create_tenant(&db.owner, "buro", "Büro", 5)
        .await
        .unwrap();
    let pm = member(&db, "buro", "yonetici", TenantRole::ProjectManager).await;
    let project = new_project(&db, &pm, "Aydınlatma").await;
    let by = open(&db, &pm, project).await;
    let placed = Uuid::now_v7();

    // One commit: the lamp, the pole holding it, and the pole placed.
    let made = changes::commit(
        &db.app,
        &by,
        envelope(
            &pm,
            project,
            with(
                vec![
                    BlockChange::Create { block: lamba(0.5) },
                    BlockChange::Create { block: direk() },
                ],
                vec![FeatureChange::Create {
                    id: placed.to_string(),
                    entity: insert(DIREK),
                }],
            ),
            &[],
        ),
    )
    .await
    .unwrap();
    let (lamp, pole) = (block_key(lamba(0.5).id), block_key(direk().id));
    assert_eq!(
        [
            &made.versions[&lamp],
            &made.versions[&pole],
            &made.versions[&placed.to_string()]
        ],
        ["1", "1", "1"]
    );
    let info = projects::info(&db.app, &by).await.unwrap();
    let listed: Vec<(&str, &str)> = info
        .blocks
        .iter()
        .map(|r| (r.block.name.as_str(), r.version.as_str()))
        .collect();
    assert_eq!(listed, [("Lamba", "1"), ("Direk", "1")]);
    assert_eq!(
        blocks::list(&db.app, &by).await.unwrap().blocks,
        info.blocks
    );
    // The pole's line, then the lamp's crossbar and centre, twice the size, at the insert's point.
    assert_eq!(
        geometry(&db, placed).await,
        (
            "GEOMETRYCOLLECTION(LINESTRING(486500 4420200,486500 4420212),LINESTRING(486499 4420212,486501 4420212),POINT(486500 4420212))".into(),
            1
        )
    );
    let log = events::after(&db.app, &by, 0, 10).await.unwrap().events;
    let said: Vec<(&str, FeatureOp)> = log[0]
        .blocks
        .iter()
        .map(|b| (b.id.as_str(), b.op))
        .collect();
    assert_eq!(
        said,
        [(LAMBA, FeatureOp::Create), (DIREK, FeatureOp::Create)]
    );

    // A wider lamp: the pole's insert is placed anew, its own version as it was.
    let wider = changes::commit(
        &db.app,
        &by,
        envelope(
            &pm,
            project,
            with(vec![BlockChange::Update { block: lamba(1.0) }], vec![]),
            &[(lamp.as_str(), "1")],
        ),
    )
    .await
    .unwrap();
    assert_eq!(wider.versions[&lamp], "2");
    assert_eq!(
        geometry(&db, placed).await,
        (
            "GEOMETRYCOLLECTION(LINESTRING(486500 4420200,486500 4420212),LINESTRING(486498 4420212,486502 4420212),POINT(486500 4420212))".into(),
            1
        )
    );
    // Based on the old version: a conflict under the block's key, nothing written.
    let stale = changes::commit(
        &db.app,
        &by,
        envelope(
            &pm,
            project,
            with(vec![BlockChange::Update { block: lamba(3.0) }], vec![]),
            &[(lamp.as_str(), "1")],
        ),
    )
    .await;
    assert!(
        matches!(&stale, Err(AppError::Conflict { conflicts, .. }) if conflicts[0].id == lamp && conflicts[0].reason == ConflictReason::Changed),
        "{stale:?}"
    );
    let unguarded = changes::commit(
        &db.app,
        &by,
        envelope(
            &pm,
            project,
            with(vec![BlockChange::Update { block: lamba(3.0) }], vec![]),
            &[],
        ),
    )
    .await;
    assert!(
        matches!(&unguarded, Err(AppError::Invalid { path: Some(p), .. }) if *p == format!("expectedVersions[{lamp}]")),
        "{unguarded:?}"
    );

    // In use: the pole is placed, the lamp is in the pole.
    let pole_gone = changes::commit(
        &db.app,
        &by,
        envelope(
            &pm,
            project,
            with(vec![BlockChange::Delete { id: direk().id }], vec![]),
            &[(pole.as_str(), "1")],
        ),
    )
    .await;
    assert!(
        matches!(&pole_gone, Err(AppError::Conflict { message, conflicts, .. })
            if message.starts_with("“Direk” bloğu kullanılıyor (çizimde 1 yerleştirmesi") && conflicts[0].id == pole),
        "{pole_gone:?}"
    );
    let lamp_gone = changes::commit(
        &db.app,
        &by,
        envelope(
            &pm,
            project,
            with(vec![BlockChange::Delete { id: lamba(0.5).id }], vec![]),
            &[(lamp.as_str(), "2")],
        ),
    )
    .await;
    assert!(
        matches!(&lamp_gone, Err(AppError::Invalid { message, .. }) if message == "“Lamba” bloğu “Direk” bloğunun içinde kullanılıyor; silinemez."),
        "{lamp_gone:?}"
    );
    // The insert and the pole in one commit, then the lamp: nothing left.
    changes::commit(
        &db.app,
        &by,
        envelope(
            &pm,
            project,
            with(
                vec![BlockChange::Delete { id: direk().id }],
                vec![FeatureChange::Delete {
                    id: placed.to_string(),
                }],
            ),
            &[(pole.as_str(), "1"), (placed.to_string().as_str(), "1")],
        ),
    )
    .await
    .unwrap();
    let last = changes::commit(
        &db.app,
        &by,
        envelope(
            &pm,
            project,
            with(vec![BlockChange::Delete { id: lamba(0.5).id }], vec![]),
            &[(lamp.as_str(), "2")],
        ),
    )
    .await
    .unwrap();
    assert_eq!(last.deleted, [lamp]);
    assert!(
        projects::info(&db.app, &by)
            .await
            .unwrap()
            .blocks
            .is_empty()
    );
}

#[tokio::test]
async fn the_block_rules_are_the_documents() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    admin::create_tenant(&db.owner, "buro", "Büro", 5)
        .await
        .unwrap();
    let pm = member(&db, "buro", "yonetici", TenantRole::ProjectManager).await;
    let viewer = member(&db, "buro", "izleyici", TenantRole::Viewer).await;
    let project = new_project(&db, &pm, "Kurallar").await;
    let by = open(&db, &pm, project).await;
    changes::commit(
        &db.app,
        &by,
        envelope(
            &pm,
            project,
            with(vec![BlockChange::Create { block: lamba(0.5) }], vec![]),
            &[],
        ),
    )
    .await
    .unwrap();
    let refused = |r: Result<kentos_contracts::CommitResult, AppError>| match r {
        Err(AppError::Invalid { message, .. }) => message,
        other => panic!("not refused: {other:?}"),
    };
    // A name the drawing has, Turkish case folded.
    let mut again = direk();
    again.name = "LAMBA".into();
    again.entities.truncate(1);
    let taken = changes::commit(
        &db.app,
        &by,
        envelope(
            &pm,
            project,
            with(vec![BlockChange::Create { block: again }], vec![]),
            &[],
        ),
    )
    .await;
    assert_eq!(
        refused(taken),
        "Çizimde “Lamba” adında bir blok var; başka bir ad verin."
    );
    // A lamp holding itself.
    let mut itself = lamba(0.5);
    itself.entities.push(direk().entities[1].clone());
    let cycle = changes::commit(
        &db.app,
        &by,
        envelope(
            &pm,
            project,
            with(vec![BlockChange::Update { block: itself }], vec![]),
            &[(block_key(lamba(0.5).id).as_str(), "1")],
        ),
    )
    .await;
    assert_eq!(
        refused(cycle),
        "“Lamba” bloğu kendini içeremez (doğrudan ya da başka bloklar yoluyla)."
    );
    // An insert of a block the project does not have.
    let unknown = changes::commit(
        &db.app,
        &by,
        envelope(
            &pm,
            project,
            with(
                vec![],
                vec![FeatureChange::Create {
                    id: Uuid::now_v7().to_string(),
                    entity: insert(DIREK),
                }],
            ),
            &[],
        ),
    )
    .await;
    assert!(refused(unknown).ends_with("yerleştirilen blok projede tanımlı değil."));
    // A viewer changes no block: they are the drawing's content.
    share(&db, &by, &viewer.actor, GrantRole::Viewer).await;
    let seen = open(&db, &viewer, project).await;
    let viewing = changes::commit(
        &db.app,
        &seen,
        envelope(
            &viewer,
            project,
            with(vec![BlockChange::Create { block: direk() }], vec![]),
            &[],
        ),
    )
    .await;
    assert!(
        matches!(viewing, Err(AppError::Forbidden(_))),
        "{viewing:?}"
    );
}

#[tokio::test]
async fn blocks_travel_with_the_project() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    admin::create_tenant(&db.owner, "buro", "Büro", 5)
        .await
        .unwrap();
    let pm = member(&db, "buro", "yonetici", TenantRole::ProjectManager).await;
    let project = new_project(&db, &pm, "Taşınan").await;
    let by = open(&db, &pm, project).await;
    let heavy: Entity = serde_json::from_value(json!({ "kind": "line", "id": 1, "layerId": "cizim", "attrs": {},
        "lineWeight": 0.35, "a": { "x": 486500, "y": 4420200 }, "b": { "x": 486510, "y": 4420200 } }))
    .unwrap();
    changes::commit(
        &db.app,
        &by,
        envelope(
            &pm,
            project,
            with(
                vec![
                    BlockChange::Create { block: lamba(0.5) },
                    BlockChange::Create { block: direk() },
                ],
                vec![
                    FeatureChange::Create {
                        id: Uuid::now_v7().to_string(),
                        entity: insert(DIREK),
                    },
                    FeatureChange::Create {
                        id: Uuid::now_v7().to_string(),
                        entity: heavy,
                    },
                ],
            ),
            &[],
        ),
    )
    .await
    .unwrap();

    // Its .kcad image holds the definitions, in their order.
    let image =
        kentos_kcad::decode(&snapshot::snapshot(&db.app, &by).await.unwrap().bytes).unwrap();
    let names: Vec<&str> = image.blocks.iter().map(|b| b.name.as_str()).collect();
    assert_eq!(names, ["Lamba", "Direk"]);
    assert_eq!(image.blocks, [lamba(0.5), direk()]);

    // A copy takes the definitions, and the objects' own line weights.
    let copy = match run(&db, &by, PROJECT_DUPLICATE, json!({})).await {
        Ok(CommandOutcome::Duplicated(d)) => d,
        other => panic!("not a copy: {other:?}"),
    };
    let copied = open(&db, &pm, Uuid::parse_str(&copy.project.id).unwrap()).await;
    let info = projects::info(&db.app, &copied).await.unwrap();
    let listed: Vec<(&str, &str)> = info
        .blocks
        .iter()
        .map(|r| (r.block.name.as_str(), r.version.as_str()))
        .collect();
    assert_eq!(listed, [("Lamba", "1"), ("Direk", "1")]);
    let weights: Vec<Option<f64>> = sqlx::query_scalar(
        "select line_weight from kentos.feature where project_id = $1 and kind = 'line'",
    )
    .bind(copied.project)
    .fetch_all(&db.owner)
    .await
    .unwrap();
    assert_eq!(weights, [Some(0.35)]);

    // A checkpoint restored as a new project: its image read back, the same definitions and insert.
    let store = blobs();
    let point = match commands::run(
        &db.app,
        &store,
        &CatalogPolicy::default(),
        &by,
        catalog_envelope(
            &by,
            PROJECT_CHECKPOINT_CREATE,
            json!({ "name": "Teslim" }),
            &[],
        ),
    )
    .await
    .unwrap()
    {
        CommandOutcome::Checkpoint(c) => c.checkpoint.id,
        other => panic!("not a checkpoint: {other:?}"),
    };
    let restored = match commands::run(
        &db.app,
        &store,
        &CatalogPolicy::default(),
        &by,
        catalog_envelope(
            &by,
            PROJECT_CHECKPOINT_RESTORE,
            json!({ "checkpointId": point }),
            &[],
        ),
    )
    .await
    .unwrap()
    {
        CommandOutcome::Duplicated(d) => d,
        other => panic!("not a restore: {other:?}"),
    };
    let back = open(&db, &pm, Uuid::parse_str(&restored.project.id).unwrap()).await;
    let image =
        kentos_kcad::decode(&snapshot::snapshot(&db.app, &back).await.unwrap().bytes).unwrap();
    assert_eq!(image.blocks, [lamba(0.5), direk()]);
    // The insert as it was placed (its slot is the file's own).
    let placed: Vec<Entity> = image
        .entities
        .iter()
        .filter(|e| matches!(e, Entity::Insert(_)))
        .map(|e| {
            let mut e = e.clone();
            e.base_mut().id = 1;
            e
        })
        .collect();
    assert_eq!(placed, [insert(DIREK)]);
}

//! Recipe metadata and its ordinary dimension features commit, snapshot and duplicate together.
//! KENTOS_TEST_DB=required makes a missing real PostgreSQL test environment fail.

mod common;

use common::{envelope, member, open};
use kentos_application::{admin, changes, projects, snapshot};
use kentos_contracts::{
    DocumentSnapshotV2, FeatureChange, ProjectChanges, ProjectCreate, ProjectPatch, TenantRole,
};
use kentos_postgres::testing::TestDb;
use uuid::Uuid;

fn model() -> DocumentSnapshotV2 {
    serde_json::from_str(include_str!(
        "../../../../fixtures/kcad/v2/dimension-sets.json"
    ))
    .unwrap()
}

fn create(source: &DocumentSnapshotV2) -> ProjectCreate {
    ProjectCreate {
        name: "Kalıcı mimari ölçüler".into(),
        settings: source.settings.clone(),
        origin: source.origin,
        home_view: source.home_view,
        layers: source.layers.clone(),
        active_layer: source.active_layer.clone(),
        styles: source.styles.clone(),
        sheets: None,
        arrays: Vec::new(),
        dimension_sets: Vec::new(),
        casework: Vec::new(),
        constraints: Default::default(),
        description: None,
        project_type: None,
        tags: None,
        storage: None,
    }
}

#[tokio::test]
async fn v1_cannot_drop_metadata_and_v2_commits_members_in_one_revision() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    admin::create_tenant(&db.owner, "buro", "Büro", 3)
        .await
        .unwrap();
    let who = member(&db, "buro", "ayse", TenantRole::ProjectManager).await;
    let source = model();
    // A public create cannot bind active members before their objects exist.
    let mut premature = create(&source);
    premature.dimension_sets = source.dimension_sets.clone();
    let refused = projects::create(&db.app, &who, premature, None)
        .await
        .unwrap_err();
    assert_eq!(refused.path(), Some("dimensionSets"));
    let created = projects::create(&db.app, &who, create(&source), None)
        .await
        .unwrap();
    let project = Uuid::parse_str(&created.id).unwrap();
    let by = open(&db, &who, project).await;
    let changes = ProjectChanges {
        features: source
            .uids
            .iter()
            .zip(&source.entities)
            .map(|(id, entity)| FeatureChange::Create {
                id: id.to_text(),
                entity: entity.clone(),
            })
            .collect(),
        blocks: Vec::new(),
        project: Some(ProjectPatch {
            dimension_sets: Some(source.dimension_sets.clone()),
            ..Default::default()
        }),
    };
    let mut command = envelope(
        &who,
        project,
        changes,
        &[("@project", &created.meta_version)],
    );
    let refused = changes::commit(&db.app, &by, command.clone())
        .await
        .unwrap_err();
    assert_eq!(refused.path(), Some("project.dimensionSets"));
    let empty = projects::info(&db.app, &by).await.unwrap();
    assert_eq!(empty.feature_count, "0");
    assert_eq!(empty.data_revision, "0");
    command.version = 2;
    let committed = changes::commit(&db.app, &by, command).await.unwrap();
    assert_eq!(committed.data_revision, "1");
    let info = projects::info(&db.app, &by).await.unwrap();
    assert_eq!(info.dimension_sets, source.dimension_sets);
    assert_eq!(info.feature_count, "2");
    let back = kentos_kcad::decode(&snapshot::snapshot(&db.app, &by).await.unwrap().bytes).unwrap();
    assert_eq!(back.dimension_sets, source.dimension_sets);
    assert_eq!(back.uids.len(), 2);
    let mut mismatched = source.dimension_sets.clone();
    mismatched[0].members[0].a.place = kentos_contracts::SubPlace::Vertex {
        part: 0,
        ring: 0,
        index: 2,
    };
    let mut bad_refs = envelope(
        &who,
        project,
        ProjectChanges {
            features: Vec::new(),
            blocks: Vec::new(),
            project: Some(ProjectPatch {
                dimension_sets: Some(mismatched),
                ..Default::default()
            }),
        },
        &[("@project", &info.meta_version)],
    );
    bad_refs.version = 2;
    assert_eq!(
        changes::commit(&db.app, &by, bad_refs)
            .await
            .unwrap_err()
            .path(),
        Some("project.dimensionSets[0].members[0]")
    );
    assert_eq!(
        projects::info(&db.app, &by).await.unwrap().data_revision,
        "1"
    );
    // Deleting a child through an old request cannot leave an accepted dangling recipe.
    let uid = source.uids[1].to_text();
    let invalid = envelope(
        &who,
        project,
        ProjectChanges {
            features: vec![FeatureChange::Delete { id: uid.clone() }],
            project: None,
            blocks: Vec::new(),
        },
        &[(&uid, &committed.versions[&uid])],
    );
    let refused = changes::commit(&db.app, &by, invalid).await.unwrap_err();
    assert_eq!(refused.path(), Some("project.dimensionSets"));
    assert_eq!(
        projects::info(&db.app, &by).await.unwrap().data_revision,
        "1"
    );
    let wall_uid = source.uids[0].to_text();
    let mut moved_wall = source.entities[0].clone();
    let mut moved_dimension = source.entities[1].clone();
    if let kentos_contracts::Entity::Wall(wall) = &mut moved_wall {
        wall.wall.pts[1].x = 10.;
    }
    if let kentos_contracts::Entity::Dimension(dimension) = &mut moved_dimension {
        dimension.b.x = 10.;
    }
    let source_features = vec![
        FeatureChange::Update {
            id: wall_uid.clone(),
            entity: moved_wall,
        },
        FeatureChange::Update {
            id: uid.clone(),
            entity: moved_dimension,
        },
    ];
    let expected = [
        (&wall_uid[..], &committed.versions[&wall_uid][..]),
        (&uid[..], &committed.versions[&uid][..]),
    ];
    let old_client = envelope(
        &who,
        project,
        ProjectChanges {
            features: source_features.clone(),
            blocks: Vec::new(),
            project: None,
        },
        &expected,
    );
    assert_eq!(
        changes::commit(&db.app, &by, old_client)
            .await
            .unwrap_err()
            .path(),
        Some("project.dimensionSets")
    );
    assert_eq!(
        projects::info(&db.app, &by).await.unwrap().data_revision,
        "1"
    );
    let unchanged =
        kentos_kcad::decode(&snapshot::snapshot(&db.app, &by).await.unwrap().bytes).unwrap();
    assert_eq!(unchanged.entities, back.entities);
    let mut v2 = envelope(
        &who,
        project,
        ProjectChanges {
            features: source_features,
            blocks: Vec::new(),
            project: Some(ProjectPatch {
                dimension_sets: Some(source.dimension_sets.clone()),
                ..Default::default()
            }),
        },
        &expected,
    );
    v2.version = 2;
    v2.expected_versions
        .insert("@project".into(), info.meta_version.clone());
    let advanced = changes::commit(&db.app, &by, v2).await.unwrap();
    assert_eq!(advanced.data_revision, "2");
    let info = projects::info(&db.app, &by).await.unwrap();
    let moved =
        kentos_kcad::decode(&snapshot::snapshot(&db.app, &by).await.unwrap().bytes).unwrap();
    let d = moved
        .entities
        .iter()
        .find_map(|e| {
            if let kentos_contracts::Entity::Dimension(d) = e {
                Some(d)
            } else {
                None
            }
        })
        .unwrap();
    assert_eq!(d.b.x, 10.);
    // SQL duplication keeps the recipe and its UUID-bearing member features together.
    let copy = Uuid::now_v7();
    let mut tx = db.app.scoped(by.scope()).await.unwrap();
    let count: i64 = sqlx::query_scalar("select kentos.duplicate_project($1, $2, $1, $3, $4)")
        .bind(who.tenant)
        .bind(project)
        .bind(copy)
        .bind("Ölçü kopyası")
        .fetch_one(&mut *tx)
        .await
        .unwrap();
    assert_eq!(count, 2);
    tx.commit().await.unwrap();
    let copy_by = open(&db, &who, copy).await;
    assert_eq!(
        projects::info(&db.app, &copy_by)
            .await
            .unwrap()
            .dimension_sets,
        source.dimension_sets
    );
    // Explicit removal is still v2. V1 must reject an empty tombstone too.
    let mut clear = envelope(
        &who,
        project,
        ProjectChanges {
            features: Vec::new(),
            blocks: Vec::new(),
            project: Some(ProjectPatch {
                dimension_sets: Some(Vec::new()),
                ..Default::default()
            }),
        },
        &[("@project", &info.meta_version)],
    );
    assert_eq!(
        changes::commit(&db.app, &by, clear.clone())
            .await
            .unwrap_err()
            .path(),
        Some("project.dimensionSets")
    );
    clear.version = 2;
    changes::commit(&db.app, &by, clear).await.unwrap();
    assert!(
        projects::info(&db.app, &by)
            .await
            .unwrap()
            .dimension_sets
            .is_empty()
    );
    assert_eq!(
        projects::info(&db.app, &by).await.unwrap().feature_count,
        "2"
    );
    db.close().await;
}

fn dimension_numbers(source: &DocumentSnapshotV2) -> (f64, f64) {
    source
        .entities
        .iter()
        .find_map(|entity| match entity {
            kentos_contracts::Entity::Dimension(dimension) => {
                Some((dimension.offset, dimension.height))
            }
            _ => None,
        })
        .unwrap()
}

#[tokio::test]
async fn plot_scale_requires_an_atomic_correct_default_row_cache() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    admin::create_tenant(&db.owner, "buro", "Büro", 3)
        .await
        .unwrap();
    let who = member(&db, "buro", "ayse", TenantRole::ProjectManager).await;
    let mut source = model();
    source.dimension_sets[0].row_places.clear();
    // Independently calculated: y=1 + (7 mm * 50 / 1000) - left face y=.125.
    // A manual text height is deliberately independent of the recipe height.
    if let kentos_contracts::Entity::Dimension(dimension) = &mut source.entities[1] {
        dimension.offset = 1.225;
        dimension.height = 1.234;
    }
    let created = projects::create(&db.app, &who, create(&source), None)
        .await
        .unwrap();
    let project = Uuid::parse_str(&created.id).unwrap();
    let by = open(&db, &who, project).await;
    let mut initial = envelope(
        &who,
        project,
        ProjectChanges {
            features: source
                .uids
                .iter()
                .zip(&source.entities)
                .map(|(id, entity)| FeatureChange::Create {
                    id: id.to_text(),
                    entity: entity.clone(),
                })
                .collect(),
            blocks: Vec::new(),
            project: Some(ProjectPatch {
                dimension_sets: Some(source.dimension_sets.clone()),
                ..Default::default()
            }),
        },
        &[("@project", &created.meta_version)],
    );
    initial.version = 2;
    let installed = changes::commit(&db.app, &by, initial).await.unwrap();
    let info = projects::info(&db.app, &by).await.unwrap();
    let uid = source.uids[1].to_text();
    let mut settings = source.settings.clone();
    settings.plot_scale = 100.;
    let patch = |full_recipe: bool| ProjectPatch {
        settings: Some(settings.clone()),
        dimension_sets: full_recipe.then(|| source.dimension_sets.clone()),
        ..Default::default()
    };
    let command = |full_recipe: bool, features| {
        let mut command = envelope(
            &who,
            project,
            ProjectChanges {
                features,
                blocks: Vec::new(),
                project: Some(patch(full_recipe)),
            },
            &[
                ("@project", &info.meta_version),
                (&uid, &installed.versions[&uid]),
            ],
        );
        command.version = if full_recipe { 2 } else { 1 };
        command
    };
    // An old client's settings-only edit must not accept an old serialized row.
    let old_client = command(false, Vec::new());
    assert_eq!(
        changes::commit(&db.app, &by, old_client)
            .await
            .unwrap_err()
            .path(),
        Some("project.dimensionSets")
    );
    // Merely resending the recipe does not update the cached ordinary dimension.
    let recipe_only = command(true, Vec::new());
    assert_eq!(
        changes::commit(&db.app, &by, recipe_only)
            .await
            .unwrap_err()
            .path(),
        Some("project.dimensionSets[0].members[0].uid")
    );
    // A member write with the old geometry is rejected and rolled back too.
    let stale = command(
        true,
        vec![FeatureChange::Update {
            id: uid.clone(),
            entity: source.entities[1].clone(),
        }],
    );
    assert_eq!(
        changes::commit(&db.app, &by, stale)
            .await
            .unwrap_err()
            .path(),
        Some("features[0].entity.offset")
    );
    let unchanged = projects::info(&db.app, &by).await.unwrap();
    assert_eq!(unchanged.settings.plot_scale, 50.);
    assert_eq!(unchanged.data_revision, "1");
    assert_eq!(unchanged.meta_version, info.meta_version);
    let back = kentos_kcad::decode(&snapshot::snapshot(&db.app, &by).await.unwrap().bytes).unwrap();
    assert_eq!(dimension_numbers(&back), (1.225, 1.234));
    // New scale: y=1 + (7 mm * 100 / 1000) - .125 = 1.575.
    let mut moved = source.entities[1].clone();
    if let kentos_contracts::Entity::Dimension(dimension) = &mut moved {
        dimension.offset = 1.575;
    }
    let valid = command(
        true,
        vec![FeatureChange::Update {
            id: uid.clone(),
            entity: moved,
        }],
    );
    let committed = changes::commit(&db.app, &by, valid).await.unwrap();
    assert_eq!(committed.data_revision, "2");
    let info = projects::info(&db.app, &by).await.unwrap();
    assert_eq!(info.settings.plot_scale, 100.);
    let back = kentos_kcad::decode(&snapshot::snapshot(&db.app, &by).await.unwrap().bytes).unwrap();
    assert_eq!(dimension_numbers(&back), (1.575, 1.234));
    assert_eq!(back.uids, source.uids);
    assert_eq!(back.dimension_sets, source.dimension_sets);
    db.close().await;
}

#[tokio::test]
async fn plot_scale_keeps_explicit_first_and_broken_rows_usable_without_a_cache_write() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    admin::create_tenant(&db.owner, "buro", "Büro", 3)
        .await
        .unwrap();
    let who = member(&db, "buro", "ayse", TenantRole::ProjectManager).await;
    for case in ["explicit", "first", "missing_host"] {
        let mut source = model();
        if case == "first" {
            source.dimension_sets[0].rows = vec![kentos_contracts::DimensionSetRow::Overall];
            source.dimension_sets[0].row_places.clear();
            source.dimension_sets[0].suppressed.clear();
            if let kentos_contracts::Entity::Dimension(dimension) = &mut source.entities[1] {
                dimension.offset = 0.875;
            }
        } else if case == "missing_host" {
            source.dimension_sets[0].row_places.clear();
            source.dimension_sets[0].broken = Some(kentos_contracts::DimensionSetBreak::HostGone);
            source.entities.remove(0);
            source.uids.remove(0);
        }
        let original = dimension_numbers(&source);
        let created = projects::create(&db.app, &who, create(&source), None)
            .await
            .unwrap();
        let project = Uuid::parse_str(&created.id).unwrap();
        let by = open(&db, &who, project).await;
        let mut initial = envelope(
            &who,
            project,
            ProjectChanges {
                features: source
                    .uids
                    .iter()
                    .zip(&source.entities)
                    .map(|(id, entity)| FeatureChange::Create {
                        id: id.to_text(),
                        entity: entity.clone(),
                    })
                    .collect(),
                blocks: Vec::new(),
                project: Some(ProjectPatch {
                    dimension_sets: Some(source.dimension_sets.clone()),
                    ..Default::default()
                }),
            },
            &[("@project", &created.meta_version)],
        );
        initial.version = 2;
        changes::commit(&db.app, &by, initial).await.unwrap();
        let info = projects::info(&db.app, &by).await.unwrap();
        let mut settings = source.settings.clone();
        settings.plot_scale = 100.;
        let metadata_only = envelope(
            &who,
            project,
            ProjectChanges {
                features: Vec::new(),
                blocks: Vec::new(),
                project: Some(ProjectPatch {
                    settings: Some(settings),
                    ..Default::default()
                }),
            },
            &[("@project", &info.meta_version)],
        );
        let committed = changes::commit(&db.app, &by, metadata_only)
            .await
            .unwrap_or_else(|error| panic!("{case}: {error}"));
        assert_eq!(committed.data_revision, "2", "{case}");
        assert_eq!(
            projects::info(&db.app, &by)
                .await
                .unwrap()
                .settings
                .plot_scale,
            100.,
            "{case}"
        );
        let back =
            kentos_kcad::decode(&snapshot::snapshot(&db.app, &by).await.unwrap().bytes).unwrap();
        assert_eq!(dimension_numbers(&back), original, "{case}");
        assert_eq!(back.dimension_sets, source.dimension_sets, "{case}");
    }
    db.close().await;
}

#[tokio::test]
async fn committed_v1_source_retry_replays_after_v2_adds_recipe_ownership() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    admin::create_tenant(&db.owner, "buro", "Büro", 3)
        .await
        .unwrap();
    let who = member(&db, "buro", "ayse", TenantRole::ProjectManager).await;
    let source = model();
    let created = projects::create(&db.app, &who, create(&source), None)
        .await
        .unwrap();
    let project = Uuid::parse_str(&created.id).unwrap();
    let by = open(&db, &who, project).await;
    let wall = source.uids[0].to_text();
    let dimension = source.uids[1].to_text();
    // This v1 source request was valid and committed before any recipe owned the wall.
    let original = envelope(
        &who,
        project,
        ProjectChanges {
            features: vec![FeatureChange::Create {
                id: wall.clone(),
                entity: source.entities[0].clone(),
            }],
            blocks: Vec::new(),
            project: None,
        },
        &[],
    );
    let first = changes::commit(&db.app, &by, original.clone())
        .await
        .unwrap();
    assert_eq!(first.data_revision, "1");
    let mut attach = envelope(
        &who,
        project,
        ProjectChanges {
            features: vec![FeatureChange::Create {
                id: dimension.clone(),
                entity: source.entities[1].clone(),
            }],
            blocks: Vec::new(),
            project: Some(ProjectPatch {
                dimension_sets: Some(source.dimension_sets.clone()),
                ..Default::default()
            }),
        },
        &[("@project", &created.meta_version)],
    );
    attach.version = 2;
    let attached = changes::commit(&db.app, &by, attach).await.unwrap();
    assert_eq!(attached.data_revision, "2");
    let current = projects::info(&db.app, &by).await.unwrap();
    assert_eq!(current.feature_count, "2");
    assert_eq!(current.dimension_sets, source.dimension_sets);
    // The exact retry returns the original response, rather than judging it against later ownership.
    let replay = changes::commit(&db.app, &by, original.clone())
        .await
        .unwrap();
    let mut expected = first.clone();
    expected.replayed = true;
    assert_eq!(replay, expected);
    let mut moved = source.entities[0].clone();
    if let kentos_contracts::Entity::Wall(w) = &mut moved {
        w.wall.pts[1].x = 10.;
    }
    let changed = ProjectChanges {
        features: vec![FeatureChange::Update {
            id: wall.clone(),
            entity: moved,
        }],
        blocks: Vec::new(),
        project: None,
    };
    // A different request with the old key is still the idempotency error, with no ownership bypass.
    let mut reused = original;
    reused.input = serde_json::to_value(&changed).unwrap();
    assert!(
        matches!(changes::commit(&db.app,&by,reused).await,Err(kentos_application::AppError::Invalid{message,..}) if message.contains("idempotency"))
    );
    // A fresh v1 source edit is still subject to the recipe guard.
    let fresh = envelope(&who, project, changed, &[(&wall, &first.versions[&wall])]);
    assert_eq!(
        changes::commit(&db.app, &by, fresh)
            .await
            .unwrap_err()
            .path(),
        Some("project.dimensionSets")
    );
    let after = projects::info(&db.app, &by).await.unwrap();
    assert_eq!(after.data_revision, "2");
    assert_eq!(after.feature_count, "2");
    assert_eq!(after.dimension_sets, source.dimension_sets);
    db.close().await;
}

async fn install_source(
    db: &TestDb,
    source: &DocumentSnapshotV2,
) -> (
    kentos_application::tenancy::Access,
    kentos_application::access::ProjectAccess,
    kentos_contracts::CommitResult,
) {
    admin::create_tenant(&db.owner, "buro", "Büro", 3)
        .await
        .unwrap();
    let who = member(db, "buro", "ayse", TenantRole::ProjectManager).await;
    let made = projects::create(&db.app, &who, create(source), None)
        .await
        .unwrap();
    let project = Uuid::parse_str(&made.id).unwrap();
    let by = open(db, &who, project).await;
    let mut command = envelope(
        &who,
        project,
        ProjectChanges {
            features: source
                .uids
                .iter()
                .zip(&source.entities)
                .map(|(id, entity)| FeatureChange::Create {
                    id: id.to_text(),
                    entity: entity.clone(),
                })
                .collect(),
            blocks: Vec::new(),
            project: Some(ProjectPatch {
                dimension_sets: Some(source.dimension_sets.clone()),
                ..Default::default()
            }),
        },
        &[("@project", &made.meta_version)],
    );
    command.version = 2;
    let installed = changes::commit(&db.app, &by, command).await.unwrap();
    (who, by, installed)
}

#[tokio::test]
async fn same_recipe_v2_refuses_stale_healthy_source_cache_atomically() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    let source = model();
    let (who, by, installed) = install_source(&db, &source).await;
    let info = projects::info(&db.app, &by).await.unwrap();
    let wall = source.uids[0].to_text();
    let dimension = source.uids[1].to_text();
    let before = snapshot::snapshot(&db.app, &by).await.unwrap().bytes;
    let mut moved = source.entities[0].clone();
    if let kentos_contracts::Entity::Wall(w) = &mut moved {
        w.wall.pts[1].x = 10.;
    }
    for (stale_child, fake_broken, expected_path) in [
        (false, false, "project.dimensionSets[0].members[0].uid"),
        (true, false, "features[1].entity.b"),
        (false, true, "project.dimensionSets[0].members[0].uid"),
    ] {
        let mut sets = source.dimension_sets.clone();
        if fake_broken {
            sets[0].broken = Some(kentos_contracts::DimensionSetBreak::HostGone);
        }
        let mut features = vec![FeatureChange::Update {
            id: wall.clone(),
            entity: moved.clone(),
        }];
        if stale_child {
            features.push(FeatureChange::Update {
                id: dimension.clone(),
                entity: source.entities[1].clone(),
            });
        }
        let mut command = envelope(
            &who,
            by.project,
            ProjectChanges {
                features,
                blocks: Vec::new(),
                project: Some(ProjectPatch {
                    dimension_sets: Some(sets),
                    ..Default::default()
                }),
            },
            &[
                ("@project", &info.meta_version),
                (&wall, &installed.versions[&wall]),
                (&dimension, &installed.versions[&dimension]),
            ],
        );
        command.version = 2;
        let error = changes::commit(&db.app, &by, command).await.unwrap_err();
        assert_eq!(error.path(), Some(expected_path));
        let after = projects::info(&db.app, &by).await.unwrap();
        assert_eq!(after.data_revision, "1");
        assert_eq!(after.meta_version, info.meta_version);
        assert_eq!(
            snapshot::snapshot(&db.app, &by).await.unwrap().bytes,
            before
        );
    }
    let replacement = kentos_contracts::EntityId(Uuid::now_v7().into_bytes());
    let mut changed_id = source.dimension_sets.clone();
    changed_id[0].members[0].uid = replacement;
    let mut command = envelope(
        &who,
        by.project,
        ProjectChanges {
            features: vec![
                FeatureChange::Delete {
                    id: dimension.clone(),
                },
                FeatureChange::Create {
                    id: replacement.to_text(),
                    entity: source.entities[1].clone(),
                },
            ],
            blocks: Vec::new(),
            project: Some(ProjectPatch {
                dimension_sets: Some(changed_id),
                ..Default::default()
            }),
        },
        &[
            ("@project", &info.meta_version),
            (&dimension, &installed.versions[&dimension]),
        ],
    );
    command.version = 2;
    assert_eq!(
        changes::commit(&db.app, &by, command)
            .await
            .unwrap_err()
            .path(),
        Some("project.dimensionSets[0].members[0].uid")
    );
    assert_eq!(
        snapshot::snapshot(&db.app, &by).await.unwrap().bytes,
        before
    );
    let mut next_dimension = source.entities[1].clone();
    if let kentos_contracts::Entity::Dimension(d) = &mut next_dimension {
        d.b.x = 10.;
        d.height = 1.234;
        d.text_shift = Some(kentos_contracts::TextShift {
            along: 0.3,
            across: 0.4,
        });
        d.look.arrow_size = Some(1.7);
    }
    let mut valid = envelope(
        &who,
        by.project,
        ProjectChanges {
            features: vec![
                FeatureChange::Update {
                    id: wall.clone(),
                    entity: moved,
                },
                FeatureChange::Update {
                    id: dimension.clone(),
                    entity: next_dimension,
                },
            ],
            blocks: Vec::new(),
            project: Some(ProjectPatch {
                dimension_sets: Some(source.dimension_sets.clone()),
                ..Default::default()
            }),
        },
        &[
            ("@project", &info.meta_version),
            (&wall, &installed.versions[&wall]),
            (&dimension, &installed.versions[&dimension]),
        ],
    );
    valid.version = 2;
    let committed = changes::commit(&db.app, &by, valid).await.unwrap();
    let back = kentos_kcad::decode(&snapshot::snapshot(&db.app, &by).await.unwrap().bytes).unwrap();
    assert_eq!(back.dimension_sets, source.dimension_sets);
    assert_eq!(back.uids, source.uids);
    let kentos_contracts::Entity::Dimension(d) = &back.entities[1] else {
        panic!()
    };
    assert_eq!((d.a.x, d.b.x, d.offset, d.height), (0., 10., 1.875, 1.234));
    assert_eq!(
        d.text_shift,
        Some(kentos_contracts::TextShift {
            along: 0.3,
            across: 0.4
        })
    );
    assert_eq!(d.look.arrow_size, Some(1.7));
    // Appearance/text placement alone remains legal under the same recipe.
    let info = projects::info(&db.app, &by).await.unwrap();
    let mut appearance = back.entities[1].clone();
    if let kentos_contracts::Entity::Dimension(d) = &mut appearance {
        d.height = 0.222;
        d.text_shift = None;
    }
    let mut command = envelope(
        &who,
        by.project,
        ProjectChanges {
            features: vec![FeatureChange::Update {
                id: dimension.clone(),
                entity: appearance,
            }],
            blocks: Vec::new(),
            project: Some(ProjectPatch {
                dimension_sets: Some(source.dimension_sets.clone()),
                ..Default::default()
            }),
        },
        &[
            ("@project", &info.meta_version),
            (&dimension, &committed.versions[&dimension]),
        ],
    );
    command.version = 2;
    changes::commit(&db.app, &by, command).await.unwrap();
    let back = kentos_kcad::decode(&snapshot::snapshot(&db.app, &by).await.unwrap().bytes).unwrap();
    let kentos_contracts::Entity::Dimension(d) = &back.entities[1] else {
        panic!()
    };
    assert_eq!(
        (d.b.x, d.offset, d.height, d.text_shift),
        (10., 1.875, 0.222, None)
    );
    db.close().await;
}

#[tokio::test]
async fn new_and_deleted_opening_require_complete_interval_membership() {
    use kentos_contracts::{
        DimensionSetMember, DimensionSetRow, Entity, EntityId, SubPlace, SubRef, Vec2,
    };
    let Some(db) = TestDb::create().await else {
        return;
    };
    let source = model();
    let (who, by, installed) = install_source(&db, &source).await;
    let info = projects::info(&db.app, &by).await.unwrap();
    let opening = EntityId(Uuid::now_v7().into_bytes());
    let host = source.uids[0];
    let entity: Entity = serde_json::from_value(serde_json::json!({
        "kind":"opening", "id":3, "layerId":"0", "attrs":{},
        "host":host.to_text(), "at":3., "width":1.,
        "place":[{"x":2.5,"y":0.125},{"x":2.5,"y":-0.125},{"x":3.5,"y":0.125},{"x":3.5,"y":-0.125}]
    }))
    .unwrap();
    let mut missing = envelope(
        &who,
        by.project,
        ProjectChanges {
            features: vec![FeatureChange::Create {
                id: opening.to_text(),
                entity: entity.clone(),
            }],
            blocks: Vec::new(),
            project: Some(ProjectPatch {
                dimension_sets: Some(source.dimension_sets.clone()),
                ..Default::default()
            }),
        },
        &[("@project", &info.meta_version)],
    );
    missing.version = 2;
    assert_eq!(
        changes::commit(&db.app, &by, missing)
            .await
            .unwrap_err()
            .path(),
        Some("project.dimensionSets[0].members")
    );
    assert_eq!(
        projects::info(&db.app, &by).await.unwrap().feature_count,
        "2"
    );
    assert_eq!(
        projects::info(&db.app, &by).await.unwrap().data_revision,
        "1"
    );
    let original = &source.dimension_sets[0].members[0];
    let jamb = |index| SubRef {
        uid: opening,
        place: SubPlace::Vertex {
            part: 0,
            ring: 0,
            index,
        },
        path: Vec::new(),
        broken: None,
    };
    let nodes = [
        (0., original.a.clone()),
        (2.5, jamb(0)),
        (3.5, jamb(2)),
        (8., original.b.clone()),
    ];
    let mut sets = source.dimension_sets.clone();
    let mut features = vec![FeatureChange::Create {
        id: opening.to_text(),
        entity,
    }];
    let mut interval_ids = Vec::new();
    for pair in nodes.windows(2) {
        let uid = EntityId(Uuid::now_v7().into_bytes());
        let mut entity = source.entities[1].clone();
        let Entity::Dimension(d) = &mut entity else {
            panic!()
        };
        d.a = Vec2 {
            x: pair[0].0,
            y: 0.125,
        };
        d.b = Vec2 {
            x: pair[1].0,
            y: 0.125,
        };
        d.offset = 0.875; // Openings row y=1, physical left face y=.125.
        let refs = d.refs.as_mut().unwrap();
        refs.a = Some(pair[0].1.clone());
        refs.b = Some(pair[1].1.clone());
        sets[0].members.push(DimensionSetMember {
            uid,
            row: DimensionSetRow::Openings,
            a: pair[0].1.clone(),
            b: pair[1].1.clone(),
        });
        features.push(FeatureChange::Create {
            id: uid.to_text(),
            entity,
        });
        interval_ids.push(uid);
    }
    let mut valid = envelope(
        &who,
        by.project,
        ProjectChanges {
            features,
            blocks: Vec::new(),
            project: Some(ProjectPatch {
                dimension_sets: Some(sets.clone()),
                ..Default::default()
            }),
        },
        &[("@project", &info.meta_version)],
    );
    valid.version = 2;
    let added = changes::commit(&db.app, &by, valid).await.unwrap();
    let info = projects::info(&db.app, &by).await.unwrap();
    assert_eq!(info.dimension_sets[0].members[0].uid, original.uid);
    assert_eq!(info.dimension_sets[0].members.len(), 4);
    let mut stale_delete = envelope(
        &who,
        by.project,
        ProjectChanges {
            features: vec![FeatureChange::Delete {
                id: opening.to_text(),
            }],
            blocks: Vec::new(),
            project: Some(ProjectPatch {
                dimension_sets: Some(sets),
                ..Default::default()
            }),
        },
        &[
            ("@project", &info.meta_version),
            (&opening.to_text(), &added.versions[&opening.to_text()]),
        ],
    );
    stale_delete.version = 2;
    assert_eq!(
        changes::commit(&db.app, &by, stale_delete)
            .await
            .unwrap_err()
            .path(),
        Some("project.dimensionSets[0].members[1]")
    );
    assert_eq!(
        projects::info(&db.app, &by).await.unwrap().feature_count,
        "6"
    );
    assert_eq!(
        projects::info(&db.app, &by).await.unwrap().data_revision,
        "2"
    );
    let mut features = vec![FeatureChange::Delete {
        id: opening.to_text(),
    }];
    features.extend(
        interval_ids
            .iter()
            .map(|uid| FeatureChange::Delete { id: uid.to_text() }),
    );
    let mut expected = vec![
        ("@project".to_owned(), info.meta_version.clone()),
        (
            opening.to_text(),
            added.versions[&opening.to_text()].clone(),
        ),
    ];
    expected.extend(
        interval_ids
            .iter()
            .map(|uid| (uid.to_text(), added.versions[&uid.to_text()].clone())),
    );
    let expected: Vec<_> = expected
        .iter()
        .map(|(k, v)| (k.as_str(), v.as_str()))
        .collect();
    let mut correct = envelope(
        &who,
        by.project,
        ProjectChanges {
            features,
            blocks: Vec::new(),
            project: Some(ProjectPatch {
                dimension_sets: Some(source.dimension_sets.clone()),
                ..Default::default()
            }),
        },
        &expected,
    );
    correct.version = 2;
    changes::commit(&db.app, &by, correct).await.unwrap();
    let back = kentos_kcad::decode(&snapshot::snapshot(&db.app, &by).await.unwrap().bytes).unwrap();
    assert_eq!(back.dimension_sets, source.dimension_sets);
    assert_eq!(back.uids, source.uids);
    assert_eq!(back.entities, source.entities);
    assert_eq!(installed.data_revision, "1");
    db.close().await;
}

#[tokio::test]
async fn rotated_large_coordinate_centres_and_right_faces_keep_valid_cached_members() {
    use kentos_contracts::{
        DimensionSetFace, DimensionSetMember, DimensionSetOpenings, DimensionSetRow, Entity,
        EntityId, SubPlace, SubRef, Vec2,
    };
    let Some(db) = TestDb::create().await else {
        return;
    };
    admin::create_tenant(&db.owner, "buro", "Büro", 3)
        .await
        .unwrap();
    let who = member(&db, "buro", "ayse", TenantRole::ProjectManager).await;
    for (dx, dy, right, ox, oy) in [
        (0.6_f64, 0.8_f64, false, 500_000., 4_400_000.),
        (0.6, 0.8, true, 500_000., 4_400_000.),
        (0., 1., false, 0., 0.),
        (-0.6, -0.8, true, 500_000., 4_400_000.),
    ] {
        let q = |along: f64, across: f64| Vec2 {
            x: ox + dx * along - dy * across,
            y: oy + dy * along + dx * across,
        };
        let mut source = model();
        let opening = EntityId(Uuid::now_v7().into_bytes());
        let host = source.uids[0];
        let face = if right { -0.125 } else { 0.125 };
        let part = if right { 1 } else { 0 };
        // The stored large coordinates have already rounded the ideal direction.
        // Derive this independent expectation from the actual face endpoints;
        // every centre interval projects along that face, not its slanted chord.
        let face_start = q(0., face);
        let face_end = q(8., face);
        let angle = (face_end.y - face_start.y)
            .atan2(face_end.x - face_start.x)
            .to_degrees();
        let set = &mut source.dimension_sets[0];
        set.face = if right {
            DimensionSetFace::Right
        } else {
            DimensionSetFace::Left
        };
        set.openings = DimensionSetOpenings::Centers;
        set.at = q(0., 1.);
        set.row_places[0].at = q(2., 2.);
        for member in &mut set.members {
            for reference in [&mut member.a, &mut member.b] {
                let SubPlace::Vertex {
                    part: ref mut current,
                    ..
                } = reference.place
                else {
                    panic!()
                };
                *current = part;
            }
        }
        for segment in &mut set.suppressed {
            for reference in [&mut segment.a, &mut segment.b] {
                let SubPlace::Vertex {
                    part: ref mut current,
                    ..
                } = reference.place
                else {
                    panic!()
                };
                *current = part;
            }
        }
        let Entity::Wall(w) = &mut source.entities[0] else {
            panic!()
        };
        w.wall.pts = vec![q(0., 0.), q(8., 0.)];
        let Entity::Dimension(d) = &mut source.entities[1] else {
            panic!()
        };
        d.a = q(0., face);
        d.b = q(8., face);
        d.offset = 2. - face;
        d.angle = Some(angle);
        d.refs.as_mut().unwrap().a = Some(set.members[0].a.clone());
        d.refs.as_mut().unwrap().b = Some(set.members[0].b.clone());
        let original = set.members[0].clone();
        let source_opening:Entity=serde_json::from_value(serde_json::json!({
            "kind":"opening","id":3,"layerId":"0","attrs":{},"host":host.to_text(),"at":3.,"width":1.,
            "place":[q(2.5,0.125),q(2.5,-0.125),q(3.5,0.125),q(3.5,-0.125)]
        })).unwrap();
        source.entities.push(source_opening);
        source.uids.push(opening);
        let centre = SubRef {
            uid: opening,
            place: SubPlace::Center,
            path: Vec::new(),
            broken: None,
        };
        let nodes = [
            (q(0., face), original.a.clone(), 1. - face),
            (q(3., 0.), centre, 1.),
            (q(8., face), original.b.clone(), 0.),
        ];
        for pair in nodes.windows(2) {
            let uid = EntityId(Uuid::now_v7().into_bytes());
            let mut entity = source.entities[1].clone();
            let Entity::Dimension(d) = &mut entity else {
                panic!()
            };
            d.a = pair[0].0;
            d.b = pair[1].0;
            d.offset = pair[0].2;
            d.height = 1.234;
            d.refs.as_mut().unwrap().a = Some(pair[0].1.clone());
            d.refs.as_mut().unwrap().b = Some(pair[1].1.clone());
            source.dimension_sets[0].members.push(DimensionSetMember {
                uid,
                row: DimensionSetRow::Openings,
                a: pair[0].1.clone(),
                b: pair[1].1.clone(),
            });
            source.entities.push(entity);
            source.uids.push(uid);
        }
        let made = projects::create(&db.app, &who, create(&source), None)
            .await
            .unwrap();
        let project = Uuid::parse_str(&made.id).unwrap();
        let by = open(&db, &who, project).await;
        let mut command = envelope(
            &who,
            project,
            ProjectChanges {
                features: source
                    .uids
                    .iter()
                    .zip(&source.entities)
                    .map(|(uid, entity)| FeatureChange::Create {
                        id: uid.to_text(),
                        entity: entity.clone(),
                    })
                    .collect(),
                blocks: Vec::new(),
                project: Some(ProjectPatch {
                    dimension_sets: Some(source.dimension_sets.clone()),
                    ..Default::default()
                }),
            },
            &[("@project", &made.meta_version)],
        );
        command.version = 2;
        let installed = changes::commit(&db.app, &by, command).await.unwrap();
        let info = projects::info(&db.app, &by).await.unwrap();
        assert_eq!(info.dimension_sets, source.dimension_sets);
        // Widening around the same centre changes jambs but no centre/overall
        // interval, so no unchanged dimension write should be required.
        let mut wider = source.entities[2].clone();
        let Entity::Opening(o) = &mut wider else {
            panic!()
        };
        o.opening.width = 1.2;
        o.opening.place = vec![q(2.4, 0.125), q(2.4, -0.125), q(3.6, 0.125), q(3.6, -0.125)];
        let mut command = envelope(
            &who,
            project,
            ProjectChanges {
                features: vec![FeatureChange::Update {
                    id: opening.to_text(),
                    entity: wider,
                }],
                blocks: Vec::new(),
                project: Some(ProjectPatch {
                    dimension_sets: Some(source.dimension_sets.clone()),
                    ..Default::default()
                }),
            },
            &[
                ("@project", &info.meta_version),
                (&opening.to_text(), &installed.versions[&opening.to_text()]),
            ],
        );
        command.version = 2;
        changes::commit(&db.app, &by, command).await.unwrap();
        assert_eq!(
            projects::info(&db.app, &by).await.unwrap().dimension_sets,
            source.dimension_sets
        );
    }
    db.close().await;
}

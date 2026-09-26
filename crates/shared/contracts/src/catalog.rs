//! The product command catalog (docs/adr/0013): the typed, versioned
//! operations every host reaches the same way (the web app, the desktop app,
//! the server, the CLI, Python and AI). The web app's interface commands
//! (`core/commands.ts`: open a window, pick a tool) are not here; one may end
//! in a product command.
//!
//! Input and output schemas are JSON Schema derived from the contract types
//! themselves (`schemars`, the `schema` feature), so a type and its schema
//! cannot drift. A command is listed only once a host handles it.
//!
//! `cargo test -p kentos-contracts` compares the catalog with
//! `apps/web/src/contracts/generated/commandCatalog.json` and fails on a
//! difference; after a deliberate change, `KENTOS_WRITE_CATALOG=1` rewrites
//! the file (read the diff before committing).

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

pub const CATALOG_FORMAT: &str = "kentos.commands";
pub const CATALOG_VERSION: u32 = 1;

/// Every product command, as `catalog()` builds it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct CommandCatalog {
    #[cfg_attr(feature = "ts", ts(type = "\"kentos.commands\""))]
    pub format: String,
    #[cfg_attr(feature = "ts", ts(type = "1"))]
    pub version: u32,
    pub commands: Vec<CommandDescriptor>,
}

/// One product command (TODOS.md CMD-01).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct CommandDescriptor {
    /// Stable name `alan.nesne.eylem`, lowercase ASCII (`cad.polygon.create`); never reused.
    pub id: String,
    /// Version of the input and output schema; an incompatible change is a new version.
    pub version: u32,
    /// Turkish name (interface, help, AI tool title).
    pub title: String,
    /// One or two Turkish sentences: what it does and what it needs.
    pub summary: String,
    /// Command line and search names.
    pub aliases: Vec<String>,
    pub effect: CommandEffect,
    /// Where a handler exists.
    pub hosts: Vec<CommandHost>,
    /// Runs from explicit input alone, without an interface or its implicit state (CMD-07).
    pub headless: bool,
    pub requires: Vec<CommandRequirement>,
    /// Project permissions it needs (docs/adr/0015); `summary` says which applies when.
    pub permissions: Vec<String>,
    pub undo: CommandUndo,
    pub cost: CommandCost,
    /// JSON Schema of the input.
    #[cfg_attr(feature = "ts", ts(type = "unknown"))]
    pub input: serde_json::Value,
    /// JSON Schema of the result.
    #[cfg_attr(feature = "ts", ts(type = "unknown"))]
    pub output: serde_json::Value,
    pub examples: Vec<CommandExample>,
}

/// What a command changes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "lowercase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum CommandEffect {
    /// Nothing.
    Query,
    /// Only the local session: camera, selection, active tool.
    View,
    /// The open document; undoable.
    Document,
    /// Project state on the server: saving, sharing, cloud commits.
    Project,
    /// Tenants and users.
    Admin,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "lowercase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum CommandHost {
    Web,
    Desktop,
    Server,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum CommandRequirement {
    /// An open document.
    Document,
    /// An open cloud project.
    CloudProject,
    /// A signed-in account.
    SignedIn,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "lowercase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum CommandUndo {
    None,
    /// One local undo step.
    Step,
    /// Undone in the cloud by an inverse command, checked like any other (TX-03).
    Inverse,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "lowercase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum CommandCost {
    Instant,
    /// The user waits for it.
    Interactive,
    /// Queued; the answer is a job id (202).
    Job,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct CommandExample {
    pub title: String,
    #[cfg_attr(feature = "ts", ts(type = "unknown"))]
    pub input: serde_json::Value,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional, type = "unknown"))]
    pub output: Option<serde_json::Value>,
}

#[cfg(feature = "schema")]
fn schema<T: schemars::JsonSchema>() -> serde_json::Value {
    schemars::schema_for!(T).to_value()
}

/// The catalog, with schemas derived from the contract types.
#[cfg(feature = "schema")]
pub fn catalog() -> CommandCatalog {
    use crate::{
        CAD_LINE_CREATE, CAD_LINE_CREATE_VERSION, CAD_POLYGON_CREATE, CAD_POLYGON_CREATE_VERSION,
        CAD_POLYLINE_CREATE, CAD_POLYLINE_CREATE_VERSION, CommitResult, LineCreate, LineCreated,
        PROJECT_ACCESS_REVOKE, PROJECT_ACCESS_REVOKE_VERSION, PROJECT_CHANGES,
        PROJECT_CHANGES_VERSION, PROJECT_SHARE, PROJECT_SHARE_VERSION, PolygonCreate,
        PolygonCreated, PolylineCreate, PolylineCreated, ProjectAccessChange, ProjectAccessRevoke,
        ProjectChanges, ProjectPermission, ProjectShare,
    };
    use serde_json::json;
    CommandCatalog {
        format: CATALOG_FORMAT.into(),
        version: CATALOG_VERSION,
        commands: vec![CommandDescriptor {
            id: PROJECT_CHANGES.into(),
            version: PROJECT_CHANGES_VERSION,
            title: "Proje değişiklikleri".into(),
            summary: "Bir bulut projesine nesne ekler, değiştirir, siler ve proje bilgisini günceller; hepsi tek işlemde yazılır ya da hiçbiri yazılmaz. \
                      Her değişen nesnenin ve proje bilgisinin beklenen sürümü zarfın expectedVersions alanındadır; biri değişmişse 409 döner, hiçbir şey yazılmaz. \
                      Nesneler feature.write, proje bilgisi project.edit ister. Aynı idempotency anahtarıyla tekrar aynı sonucu verir."
                .into(),
            aliases: vec![],
            effect: CommandEffect::Project,
            hosts: vec![CommandHost::Server],
            headless: true,
            requires: vec![CommandRequirement::SignedIn, CommandRequirement::CloudProject],
            permissions: vec![
                ProjectPermission::FeatureWrite.name().into(),
                ProjectPermission::Edit.name().into(),
            ],
            undo: CommandUndo::None,
            cost: CommandCost::Interactive,
            input: schema::<ProjectChanges>(),
            output: schema::<CommitResult>(),
            examples: vec![CommandExample {
                title: "Yeni bir nokta ekle".into(),
                input: json!({
                    "features": [{
                        "op": "create",
                        "id": "01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f",
                        "entity": {
                            "kind": "point",
                            "id": 1,
                            "layerId": "kot",
                            "attrs": { "ad": "P1" },
                            "p": { "x": 423510.25, "y": 4512300.5 }
                        }
                    }]
                }),
                output: None,
            }],
        },
        CommandDescriptor {
            id: PROJECT_SHARE.into(),
            version: PROJECT_SHARE_VERSION,
            title: "Projeyi paylaş".into(),
            summary: "Bir kişiye projede rol verir ya da rolünü değiştirir: görüntüleyici, yorumcu, düzenleyici ya da yönetici; istenirse bir bitiş zamanıyla. \
                      Kurum projesinde kişi kurumun üyesi olmalıdır. project.share ister; değişiklik denetime ve projenin olay günlüğüne yazılır."
                .into(),
            aliases: vec![],
            effect: CommandEffect::Project,
            hosts: vec![CommandHost::Server],
            headless: true,
            requires: vec![CommandRequirement::SignedIn, CommandRequirement::CloudProject],
            permissions: vec![ProjectPermission::Share.name().into()],
            undo: CommandUndo::None,
            cost: CommandCost::Interactive,
            input: schema::<ProjectShare>(),
            output: schema::<ProjectAccessChange>(),
            examples: vec![CommandExample {
                title: "Bir meslektaşı düzenleyici yap".into(),
                input: json!({ "userId": "01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f", "role": "editor" }),
                output: None,
            }],
        },
        CommandDescriptor {
            id: PROJECT_ACCESS_REVOKE.into(),
            version: PROJECT_ACCESS_REVOKE_VERSION,
            title: "Proje erişimini kaldır".into(),
            summary: "Bir kişinin projedeki paylaşımını kaldırır; açık bağlantıları kapanır ve sonraki istekleri reddedilir. \
                      Önceden indirilmiş kopyalar geri alınamaz. project.share ister."
                .into(),
            aliases: vec![],
            effect: CommandEffect::Project,
            hosts: vec![CommandHost::Server],
            headless: true,
            requires: vec![CommandRequirement::SignedIn, CommandRequirement::CloudProject],
            permissions: vec![ProjectPermission::Share.name().into()],
            undo: CommandUndo::None,
            cost: CommandCost::Interactive,
            input: schema::<ProjectAccessRevoke>(),
            output: schema::<ProjectAccessChange>(),
            examples: vec![CommandExample {
                title: "Paylaşımı kaldır".into(),
                input: json!({ "userId": "01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f" }),
                output: None,
            }],
        },
        // The first document command (docs/adr/0022): the web's and the desktop's own
        // handlers, held together by fixtures/commands/v1/cad.polygon.create.json.
        CommandDescriptor {
            id: CAD_POLYGON_CREATE.into(),
            version: CAD_POLYGON_CREATE_VERSION,
            title: "Kapalı alan oluştur".into(),
            summary: "Açık çizimde verilen katmana köşeleri, isteğe bağlı yay değerleri ve delikleriyle bir kapalı alan ekler; tek geri alma adımıdır. \
                      Katman girdide açıkça verilir: kilitli katmana yazılmaz, gizli katmana uyarıyla yazılır. \
                      expectedRevision verilmişse ve çizim o sürümde değilse hiçbir şey yazılmaz, sonuç conflict olur. \
                      Yerel çizim izin istemez; bulut projesine değişiklik project.changes ile gider."
                .into(),
            aliases: vec![],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Web, CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Instant,
            input: schema::<PolygonCreate>(),
            output: schema::<PolygonCreated>(),
            examples: vec![
                CommandExample {
                    title: "Dikdörtgen bir alan".into(),
                    input: json!({
                        "layerId": "yapi",
                        "pts": [
                            { "x": 423500.0, "y": 4512300.0 },
                            { "x": 423520.0, "y": 4512300.0 },
                            { "x": 423520.0, "y": 4512312.5 },
                            { "x": 423500.0, "y": 4512312.5 }
                        ]
                    }),
                    output: Some(json!({
                        "uid": "01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f",
                        "id": 12,
                        "revision": "38"
                    })),
                },
                CommandExample {
                    title: "Doğu kenarı yarım daire, ortasında delik olan renkli alan; yalnız planlandığı sürümde yazılır".into(),
                    input: json!({
                        "layerId": "yapi",
                        "pts": [
                            { "x": 423500.0, "y": 4512300.0 },
                            { "x": 423520.0, "y": 4512300.0 },
                            { "x": 423520.0, "y": 4512312.5 },
                            { "x": 423500.0, "y": 4512312.5 }
                        ],
                        "bulges": [0.0, 1.0, 0.0, 0.0],
                        "holes": [{
                            "pts": [
                                { "x": 423505.0, "y": 4512304.0 },
                                { "x": 423509.0, "y": 4512304.0 },
                                { "x": 423509.0, "y": 4512308.0 },
                                { "x": 423505.0, "y": 4512308.0 }
                            ]
                        }],
                        "color": "#E5484D",
                        "attrs": { "Ad": "Avlu" },
                        "expectedRevision": "37"
                    }),
                    output: None,
                },
            ],
        },
        // The line and polyline tools' commands (docs/adr/0027), held together by
        // fixtures/commands/v1/cad.line.create.json and cad.polyline.create.json.
        CommandDescriptor {
            id: CAD_LINE_CREATE.into(),
            version: CAD_LINE_CREATE_VERSION,
            title: "Çizgi oluştur".into(),
            summary: "Açık çizimde verilen katmana iki uç noktası arasında düz bir çizgi ekler; tek geri alma adımıdır. \
                      Çizgi aracı zincirin her parçası için bu komutu bir kez çalıştırır: her parça ayrı nesne ve ayrı adımdır. \
                      Katman girdide açıkça verilir: kilitli katmana yazılmaz, gizli katmana uyarıyla yazılır. \
                      expectedRevision verilmişse ve çizim o sürümde değilse hiçbir şey yazılmaz, sonuç conflict olur. \
                      Yerel çizim izin istemez; bulut projesine değişiklik project.changes ile gider."
                .into(),
            aliases: vec![],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Web, CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Instant,
            input: schema::<LineCreate>(),
            output: schema::<LineCreated>(),
            examples: vec![
                CommandExample {
                    title: "Bir çizgi".into(),
                    input: json!({
                        "layerId": "yapi",
                        "a": { "x": 423500.0, "y": 4512300.0 },
                        "b": { "x": 423520.0, "y": 4512312.5 }
                    }),
                    output: Some(json!({
                        "uid": "01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f",
                        "id": 12,
                        "revision": "38"
                    })),
                },
                CommandExample {
                    title: "Renkli ve öznitelikli bir çizgi; yalnız planlandığı sürümde yazılır".into(),
                    input: json!({
                        "layerId": "yapi",
                        "a": { "x": 423500.0, "y": 4512300.0 },
                        "b": { "x": 423500.0, "y": 4512330.0 },
                        "color": "#E5484D",
                        "attrs": { "Tür": "İstinat duvarı" },
                        "expectedRevision": "37"
                    }),
                    output: None,
                },
            ],
        },
        CommandDescriptor {
            id: CAD_POLYLINE_CREATE.into(),
            version: CAD_POLYLINE_CREATE_VERSION,
            title: "Çoklu çizgi oluştur".into(),
            summary: "Açık çizimde verilen katmana noktaları ve isteğe bağlı yay değerleriyle (kenar başına bir tane) açık bir çoklu çizgi ekler; tek geri alma adımıdır. \
                      Katman girdide açıkça verilir: kilitli katmana yazılmaz, gizli katmana uyarıyla yazılır. \
                      expectedRevision verilmişse ve çizim o sürümde değilse hiçbir şey yazılmaz, sonuç conflict olur. \
                      Yerel çizim izin istemez; bulut projesine değişiklik project.changes ile gider."
                .into(),
            aliases: vec![],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Web, CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Instant,
            input: schema::<PolylineCreate>(),
            output: schema::<PolylineCreated>(),
            examples: vec![
                CommandExample {
                    title: "Üç noktalı çoklu çizgi".into(),
                    input: json!({
                        "layerId": "yapi",
                        "pts": [
                            { "x": 423500.0, "y": 4512300.0 },
                            { "x": 423520.0, "y": 4512300.0 },
                            { "x": 423520.0, "y": 4512312.5 }
                        ]
                    }),
                    output: Some(json!({
                        "uid": "01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f",
                        "id": 12,
                        "revision": "38"
                    })),
                },
                CommandExample {
                    title: "İkinci kenarı çeyrek daire yayı olan renkli çoklu çizgi; yalnız planlandığı sürümde yazılır".into(),
                    input: json!({
                        "layerId": "yapi",
                        "pts": [
                            { "x": 423500.0, "y": 4512300.0 },
                            { "x": 423520.0, "y": 4512300.0 },
                            { "x": 423530.0, "y": 4512310.0 }
                        ],
                        "bulges": [0.0, 0.41421356237309503],
                        "color": "#E5484D",
                        "expectedRevision": "37"
                    }),
                    output: None,
                },
            ],
        }],
    }
}

#[cfg(all(test, feature = "schema"))]
mod tests {
    use super::*;
    use crate::{
        LineCreate, LineCreated, PolygonCreate, PolygonCreated, PolylineCreate, PolylineCreated,
        ProjectAccessRevoke, ProjectChanges, ProjectPermission, ProjectShare,
    };
    use std::path::PathBuf;

    fn catalog_file() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../../apps/web/src/contracts/generated/commandCatalog.json")
    }

    #[test]
    fn ids_are_unique_lowercase_and_examples_fit_their_input_type() {
        let c = catalog();
        let mut seen = std::collections::BTreeSet::new();
        for d in &c.commands {
            assert!(
                seen.insert((&d.id, d.version)),
                "{} v{} twice",
                d.id,
                d.version
            );
            assert!(
                d.id.split('.').count() >= 2
                    && d.id
                        .split('.')
                        .all(|p| !p.is_empty() && p.bytes().all(|b| b.is_ascii_lowercase())),
                "{}: alan.nesne.eylem, lowercase ASCII",
                d.id
            );
            assert!(!d.hosts.is_empty(), "{}: listed without a host", d.id);
            assert!(
                d.input.get("$schema").is_some() && d.output.get("$schema").is_some(),
                "{}: schemas",
                d.id
            );
            // Permissions are the fixed names of docs/adr/0015.
            for p in &d.permissions {
                assert!(
                    ProjectPermission::ALL.iter().any(|k| k.name() == p),
                    "{}: unknown permission {p}",
                    d.id
                );
            }
            for e in &d.examples {
                // Each command's example must parse as its own input type.
                let parsed = match d.id.as_str() {
                    crate::PROJECT_CHANGES => {
                        serde_json::from_value::<ProjectChanges>(e.input.clone()).map(|_| ())
                    }
                    crate::PROJECT_SHARE => {
                        serde_json::from_value::<ProjectShare>(e.input.clone()).map(|_| ())
                    }
                    crate::PROJECT_ACCESS_REVOKE => {
                        serde_json::from_value::<ProjectAccessRevoke>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_POLYGON_CREATE => {
                        serde_json::from_value::<PolygonCreate>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_LINE_CREATE => {
                        serde_json::from_value::<LineCreate>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_POLYLINE_CREATE => {
                        serde_json::from_value::<PolylineCreate>(e.input.clone()).map(|_| ())
                    }
                    other => panic!("{other}: add its input type to this test"),
                };
                parsed.unwrap_or_else(|err| panic!("{}: {}: {err}", d.id, e.title));
                // And an example's output its output type.
                if let Some(output) = &e.output {
                    let parsed = match d.id.as_str() {
                        crate::CAD_POLYGON_CREATE => {
                            serde_json::from_value::<PolygonCreated>(output.clone()).map(|_| ())
                        }
                        crate::CAD_LINE_CREATE => {
                            serde_json::from_value::<LineCreated>(output.clone()).map(|_| ())
                        }
                        crate::CAD_POLYLINE_CREATE => {
                            serde_json::from_value::<PolylineCreated>(output.clone()).map(|_| ())
                        }
                        other => panic!("{other}: add its output type to this test"),
                    };
                    parsed.unwrap_or_else(|err| panic!("{}: {}: output: {err}", d.id, e.title));
                }
            }
        }
    }

    #[test]
    fn generated_catalog_file_is_current() {
        let text = format!(
            "{}\n",
            serde_json::to_string_pretty(&catalog()).expect("the catalog serializes")
        );
        let path = catalog_file();
        if std::env::var("KENTOS_WRITE_CATALOG").as_deref() == Ok("1") {
            std::fs::write(&path, &text).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
            return;
        }
        let on_disk = std::fs::read_to_string(&path).unwrap_or_default();
        assert!(
            on_disk == text,
            "{} is out of date: run `KENTOS_WRITE_CATALOG=1 cargo test -p kentos-contracts catalog`, read the diff, commit it",
            path.display()
        );
    }
}

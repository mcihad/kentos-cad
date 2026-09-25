//! The typed settings (docs/adr/0023): the shared cases every host runs
//! (`fixtures/settings/v1/cases.json`; the web runs them in
//! `apps/web/src/core/settings/settings.test.ts`), the schema's own rules, and
//! the generated files the web reads.

use std::collections::BTreeMap;
use std::path::PathBuf;

use kentos_contracts::{
    ResolvedSetting, SettingConstraint, SettingDiagnostic, SettingErrorCode, SettingScope,
    SettingType, SettingsFile, SettingsLayers, SettingsPolicy, check_write, resolve, same_value,
    settings_schema,
};
use serde::Deserialize;
use serde_json::Value;

const CASES: &str = include_str!("../../../../fixtures/settings/v1/cases.json");

#[derive(Deserialize)]
struct Cases {
    format: String,
    version: u32,
    values: Vec<ValueCase>,
    writes: Vec<WriteCase>,
    resolve: Vec<ResolveCase>,
    files: Vec<FileCase>,
    presets: Presets,
}

#[derive(Deserialize)]
struct Outcome {
    #[serde(default)]
    value: Option<Value>,
    #[serde(default)]
    error: Option<SettingErrorCode>,
}

#[derive(Deserialize)]
struct ValueCase {
    name: String,
    key: String,
    value: Value,
    expect: Outcome,
}

#[derive(Deserialize)]
struct WriteCase {
    name: String,
    layer: SettingScope,
    key: String,
    value: Value,
    expect: Outcome,
}

#[derive(Deserialize, Default)]
struct Layers {
    #[serde(default)]
    user: BTreeMap<String, Value>,
    #[serde(default)]
    device: BTreeMap<String, Value>,
    #[serde(default)]
    session: BTreeMap<String, Value>,
    #[serde(default)]
    project: BTreeMap<String, Value>,
}

#[derive(Deserialize)]
struct Expected {
    requested: Value,
    source: SettingScope,
    effective: Value,
    #[serde(default)]
    reason: Option<kentos_contracts::ResolveReason>,
    #[serde(default)]
    locked: bool,
}

#[derive(Deserialize)]
struct ResolveCase {
    name: String,
    layers: Layers,
    #[serde(default)]
    policy: SettingsPolicy,
    #[serde(default)]
    constraints: BTreeMap<String, SettingConstraint>,
    expect: BTreeMap<String, Expected>,
    diagnostics: Vec<SettingDiagnostic>,
}

#[derive(Deserialize)]
struct FileCase {
    name: String,
    #[serde(default)]
    text: Option<String>,
    #[serde(default)]
    document: Option<Value>,
    expect: FileExpect,
}

#[derive(Deserialize)]
struct FileExpect {
    #[serde(default)]
    error: Option<SettingErrorCode>,
    #[serde(default)]
    user: BTreeMap<String, Value>,
    #[serde(default)]
    device: BTreeMap<String, Value>,
    #[serde(default)]
    migrations: usize,
    #[serde(default)]
    diagnostics: Vec<SettingDiagnostic>,
}

#[derive(Deserialize)]
struct Presets {
    matching: Vec<MatchCase>,
    fill: Vec<FillCase>,
}

#[derive(Deserialize)]
struct MatchCase {
    name: String,
    group: String,
    values: BTreeMap<String, Value>,
    expect: Option<String>,
}

#[derive(Deserialize)]
struct FillCase {
    name: String,
    preset: String,
    expect: BTreeMap<String, Value>,
}

fn cases() -> Cases {
    let cases: Cases = serde_json::from_str(CASES).expect("the case file reads");
    assert_eq!(cases.format, "kentos.settings-cases");
    assert_eq!(cases.version, 1);
    cases
}

fn outcome(result: Result<Value, SettingErrorCode>) -> (Option<Value>, Option<SettingErrorCode>) {
    match result {
        Ok(value) => (Some(value), None),
        Err(code) => (None, Some(code)),
    }
}

fn check_outcome(name: &str, got: Result<Value, SettingErrorCode>, expect: &Outcome) {
    let (value, error) = outcome(got);
    assert_eq!(error, expect.error, "{name}: error code");
    match (&value, &expect.value) {
        (Some(v), Some(e)) => assert!(
            same_value(v, e) && v == e,
            "{name}: value {v}, expected {e} (normalised)"
        ),
        (None, None) => {}
        _ => panic!("{name}: value {value:?}, expected {:?}", expect.value),
    }
}

/// Diagnostics compared without order: a layer's keys come in the order its host keeps them.
fn sorted(diagnostics: Vec<SettingDiagnostic>) -> Vec<String> {
    let mut out: Vec<String> = diagnostics
        .iter()
        .map(|d| serde_json::to_string(d).unwrap_or_default())
        .collect();
    out.sort();
    out
}

fn same_layer(a: &BTreeMap<String, Value>, b: &BTreeMap<String, Value>) -> bool {
    a.len() == b.len()
        && a.iter()
            .all(|(k, v)| b.get(k).is_some_and(|w| same_value(v, w) && v == w))
}

#[test]
fn values_validate_with_the_shared_codes() {
    let schema = settings_schema();
    let cases = cases();
    assert!(cases.values.len() >= 30);
    for c in &cases.values {
        let got = schema
            .get(&c.key)
            .ok_or(SettingErrorCode::UnknownKey)
            .and_then(|d| d.validate(&c.value));
        check_outcome(&c.name, got, &c.expect);
    }
}

#[test]
fn writes_follow_the_scope_rules() {
    let schema = settings_schema();
    for c in &cases().writes {
        check_outcome(
            &c.name,
            check_write(&schema, c.layer, &c.key, &c.value),
            &c.expect,
        );
    }
}

#[test]
fn resolution_gives_requested_effective_and_the_reason() {
    let schema = settings_schema();
    for c in cases().resolve {
        let layers = SettingsLayers {
            user: c.layers.user,
            device: c.layers.device,
            session: c.layers.session,
            project: c.layers.project,
        };
        let resolution = resolve(&schema, &layers, &c.policy, &c.constraints);
        for (key, e) in &c.expect {
            let got: &ResolvedSetting = resolution
                .get(key)
                .unwrap_or_else(|| panic!("{}: {key} not resolved", c.name));
            assert!(
                same_value(&got.requested, &e.requested),
                "{}: {key} requested {} ≠ {}",
                c.name,
                got.requested,
                e.requested
            );
            assert_eq!(got.source, e.source, "{}: {key} source", c.name);
            assert!(
                same_value(&got.effective, &e.effective),
                "{}: {key} effective {} ≠ {}",
                c.name,
                got.effective,
                e.effective
            );
            assert_eq!(got.reason, e.reason, "{}: {key} reason", c.name);
            assert_eq!(got.locked, e.locked, "{}: {key} locked", c.name);
            assert_eq!(
                got.detail.is_some(),
                got.reason.is_some(),
                "{}: {key}: a reason comes with its detail",
                c.name
            );
        }
        assert_eq!(
            sorted(resolution.diagnostics),
            sorted(c.diagnostics),
            "{}: diagnostics",
            c.name
        );
    }
}

#[test]
fn files_are_read_by_the_shared_rules() {
    let schema = settings_schema();
    for c in cases().files {
        let text = match (&c.text, &c.document) {
            (Some(text), _) => text.clone(),
            (None, Some(document)) => document.to_string(),
            (None, None) => panic!("{}: text or document", c.name),
        };
        match (SettingsFile::from_json(&text, &schema), c.expect.error) {
            (Err(code), Some(expected)) => assert_eq!(code, expected, "{}", c.name),
            (Err(code), None) => panic!("{}: refused with {code:?}", c.name),
            (Ok(_), Some(expected)) => panic!("{}: read, expected {expected:?}", c.name),
            (Ok((file, diagnostics)), None) => {
                assert!(
                    same_layer(&file.user, &c.expect.user),
                    "{}: user {:?}",
                    c.name,
                    file.user
                );
                assert!(
                    same_layer(&file.device, &c.expect.device),
                    "{}: device {:?}",
                    c.name,
                    file.device
                );
                assert_eq!(
                    file.migrations.len(),
                    c.expect.migrations,
                    "{}: migrations",
                    c.name
                );
                assert_eq!(
                    sorted(diagnostics),
                    sorted(c.expect.diagnostics.clone()),
                    "{}: diagnostics",
                    c.name
                );
            }
        }
    }
}

#[test]
fn presets_are_recognised_and_only_fill_their_values() {
    let schema = settings_schema();
    let cases = cases();
    for c in &cases.presets.matching {
        let found = schema
            .matching_preset(&c.group, |key| c.values.get(key).cloned())
            .map(|p| p.id.clone());
        assert_eq!(found, c.expect, "{}", c.name);
    }
    for c in &cases.presets.fill {
        let preset = schema.preset(&c.preset).expect("the preset exists");
        assert!(same_layer(&preset.values, &c.expect), "{}", c.name);
    }
}

#[test]
fn a_written_file_reads_back_and_keeps_what_another_version_wrote() {
    let schema = settings_schema();
    let mut file = SettingsFile::default();
    file.user.insert("drafting.snapAperture".into(), 14.into());
    file.user.insert("zz.future".into(), "keep".into());
    file.device.insert("graphics.msaa".into(), 8.into());
    let text = file.to_json(&schema);
    let (back, diagnostics) = SettingsFile::from_json(&text, &schema).expect("reads back");
    assert_eq!(back, file);
    assert_eq!(
        diagnostics.len(),
        1,
        "the unknown key is reported, and kept"
    );
}

#[test]
fn sensitive_values_never_reach_a_file() {
    let mut schema = settings_schema();
    let secret = schema
        .settings
        .iter_mut()
        .find(|d| d.key == "drafting.cursorInput")
        .expect("a setting to mark");
    secret.sensitive = true;
    let mut file = SettingsFile::default();
    file.user
        .insert("drafting.cursorInput".into(), false.into());
    file.user.insert("drafting.snapAperture".into(), 9.into());
    let text = file.to_json(&schema);
    assert!(!text.contains("cursorInput"), "{text}");
    let written =
        r#"{"format":"kentos.settings","version":1,"user":{"drafting.cursorInput":false}}"#;
    let (read, diagnostics) = SettingsFile::from_json(written, &schema).expect("reads");
    assert!(read.user.is_empty());
    assert_eq!(diagnostics[0].code, SettingErrorCode::Sensitive);
}

/// SET-01: every setting is complete and consistent.
#[test]
fn every_setting_is_described_completely() {
    let schema = settings_schema();
    let mut keys = std::collections::BTreeSet::new();
    for d in &schema.settings {
        assert!(keys.insert(&d.key), "{} twice", d.key);
        let (group, name) = d.key.split_once('.').expect("group.name");
        assert_eq!(group, d.group, "{}: the group is the key's prefix", d.key);
        assert!(
            !name.is_empty() && name.chars().all(|c| c.is_ascii_alphanumeric()),
            "{}: camelCase ASCII",
            d.key
        );
        assert!(
            schema.groups.iter().any(|g| g.id == d.group),
            "{}: group {} is listed",
            d.key,
            d.group
        );
        assert!(
            !d.title.is_empty() && !d.description.is_empty(),
            "{}: texts",
            d.key
        );
        assert!(!d.hosts.is_empty(), "{}: used by a host", d.key);
        assert!(d.version >= 1);
        assert!(
            d.validate(&d.default).is_ok_and(|v| v == d.default),
            "{}: its default is valid and normalised",
            d.key
        );
        match d.kind {
            SettingType::Enum => assert!(!d.choices.is_empty(), "{}: an enum has choices", d.key),
            SettingType::Boolean => assert!(d.choices.is_empty() && d.min.is_none()),
            SettingType::Integer | SettingType::Number => {}
        }
        for c in &d.choices {
            assert!(!c.label.is_empty(), "{}: every choice is labelled", d.key);
            let mut plain = d.clone();
            plain.choices.clear();
            assert!(
                plain.validate(&c.value).is_ok(),
                "{}: choice {} fits the type",
                d.key,
                c.value
            );
        }
        assert!(
            !matches!(d.scope, SettingScope::Default | SettingScope::Organization),
            "{}: no layer holds a default or organisation setting yet",
            d.key
        );
        assert!(!d.sensitive, "{}: no sensitive setting yet", d.key);
    }
    for p in &schema.presets {
        assert!(schema.groups.iter().any(|g| g.id == p.group));
        for (key, value) in &p.values {
            let d = schema.get(key).expect("a preset fills known settings");
            assert_eq!(d.group, p.group, "{}: a preset fills its own group", p.id);
            assert!(d.validate(value).is_ok(), "{}: {key} = {value}", p.id);
        }
    }
    // The graphics settings are the device's (TODOS.md §7), the project's stay in the project.
    for key in ["graphics.msaa", "graphics.hiDpi", "graphics.backend"] {
        assert_eq!(schema.get(key).map(|d| d.scope), Some(SettingScope::Device));
    }
    for key in ["project.srid", "project.drawingFont"] {
        assert_eq!(
            schema.get(key).map(|d| d.scope),
            Some(SettingScope::Project)
        );
    }
}

fn generated(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../apps/web/src/contracts/generated")
        .join(name)
}

/// The web reads the schema from `settingsSchema.json`; external tools can
/// check a settings file with `settingsFile.schema.json`.
#[test]
fn generated_settings_files_are_current() {
    let schema_text = format!(
        "{}\n",
        serde_json::to_string_pretty(&settings_schema()).expect("the schema serializes")
    );
    let file_schema = format!(
        "{}\n",
        serde_json::to_string_pretty(&schemars::schema_for!(SettingsFile))
            .expect("the file schema serializes")
    );
    let write = std::env::var("KENTOS_WRITE_SETTINGS").as_deref() == Ok("1");
    for (name, text) in [
        ("settingsSchema.json", schema_text),
        ("settingsFile.schema.json", file_schema),
    ] {
        let path = generated(name);
        if write {
            std::fs::write(&path, &text).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
            continue;
        }
        let on_disk = std::fs::read_to_string(&path).unwrap_or_default();
        assert!(
            on_disk == text,
            "{} is out of date: run `KENTOS_WRITE_SETTINGS=1 cargo test -p kentos-contracts settings`, read the diff, commit it",
            path.display()
        );
    }
}

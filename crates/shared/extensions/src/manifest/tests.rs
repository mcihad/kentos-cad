//! The manifest v1's rules (plan/eklenti.md §3.2): the golden manifests pass; every broken rule is one structural
//! Turkish problem at its JSON pointer.

use serde_json::{Value, json};

use super::*;

const MEBIS: &str = include_str!("../../../../../fixtures/extensions/v1/mebis/manifest.json");
const ALTLIK: &str =
    include_str!("../../../../../fixtures/extensions/v1/ornek-altlik/manifest.json");

fn mebis() -> Value {
    serde_json::from_str(MEBIS).unwrap()
}

/// `value` with the field at `pointer` set (added when missing, removed when `Value::Null` is given for an object's
/// key and `remove` is asked).
fn with(mut value: Value, pointer: &str, new: Value) -> Value {
    let (parent, key) = pointer.rsplit_once('/').unwrap();
    match value.pointer_mut(parent).unwrap() {
        Value::Object(map) => {
            map.insert(key.to_owned(), new);
        }
        Value::Array(items) => items[key.parse::<usize>().unwrap()] = new,
        other => panic!("{other}"),
    }
    value
}

fn push(mut value: Value, pointer: &str, new: Value) -> Value {
    value
        .pointer_mut(pointer)
        .unwrap()
        .as_array_mut()
        .unwrap()
        .push(new);
    value
}

fn without(mut value: Value, pointer: &str) -> Value {
    let (parent, key) = pointer.rsplit_once('/').unwrap();
    match value.pointer_mut(parent).unwrap() {
        Value::Object(map) => {
            map.remove(key);
        }
        Value::Array(items) => {
            items.remove(key.parse::<usize>().unwrap());
        }
        other => panic!("{other}"),
    }
    value
}

/// The problems' (code, pointer).
fn found(value: &Value) -> Vec<(&'static str, String)> {
    match Manifest::parse(&serde_json::to_string_pretty(value).unwrap()) {
        Ok(_) => Vec::new(),
        Err(problems) => problems
            .into_iter()
            .map(|p| (p.code, p.at.unwrap_or_default()))
            .collect(),
    }
}

/// Exactly one problem, this one.
#[track_caller]
fn refused(value: &Value, code: &str, at: &str) {
    let found = found(value);
    assert_eq!(found, [(code, at.to_owned())], "{found:#?}");
}

fn warnings(value: &Value) -> Vec<&'static str> {
    let manifest = Manifest::read(&value.to_string()).unwrap();
    manifest.check().1.into_iter().map(|p| p.code).collect()
}

#[test]
fn the_golden_manifests_pass_and_round_trip() {
    for text in [MEBIS, ALTLIK] {
        let manifest = Manifest::parse(text).unwrap_or_else(|p| panic!("{p:#?}"));
        let again = Manifest::parse(&serde_json::to_string(&manifest).unwrap()).unwrap();
        assert_eq!(again, manifest);
        assert_eq!(manifest.check().1, Vec::new(), "no warnings");
    }
    let m = Manifest::parse(MEBIS).unwrap();
    assert_eq!(m.kind, ExtensionKind::Vertical);
    assert_eq!(m.handlers().len(), 5);
    assert!(!m.is_preview());
    assert_eq!(m.capabilities().len(), 10);
    let roles: Vec<FileRole> = m.references().iter().map(|r| r.role).collect();
    for role in [
        FileRole::License,
        FileRole::Changelog,
        FileRole::Icon,
        FileRole::Schema,
        FileRole::Translation,
        FileRole::PythonProject,
        FileRole::PythonLock,
        FileRole::ToolDescriptor,
        FileRole::PanelDefinition,
        FileRole::Form,
    ] {
        assert!(roles.contains(&role), "{role:?}");
    }
    let altlik = Manifest::parse(ALTLIK).unwrap();
    assert!(
        altlik.python.is_none() && altlik.handlers().is_empty(),
        "the smallest package has no Python"
    );
}

#[test]
fn the_format_and_a_newer_schema_are_judged_before_any_field() {
    refused(
        &with(mebis(), "/format", json!("kentos.plugin")),
        "manifest_format",
        "/format",
    );
    refused(&without(mebis(), "/format"), "manifest_format", "/format");
    let newer = with(
        with(mebis(), "/schemaVersion", json!(2)),
        "/futureField",
        json!(true),
    );
    let p = Manifest::parse(&newer.to_string()).unwrap_err();
    assert_eq!(p[0].code, "manifest_newer");
    assert!(
        p[0].message.contains("KentOS'u güncelleyin"),
        "{}",
        p[0].message
    );
    refused(
        &with(mebis(), "/schemaVersion", json!("1")),
        "manifest_schema_version",
        "/schemaVersion",
    );
    let p = Manifest::parse("{\n  \"format\": \"kentos.extension\",\n  \"id\": \n}").unwrap_err();
    assert_eq!(
        (p[0].code, p[0].at.as_deref()),
        ("manifest_json", Some("satır 4, sütun 1"))
    );
    assert!(
        p[0].message.contains("değer bekleniyordu"),
        "{}",
        p[0].message
    );
}

#[test]
fn an_unknown_field_is_refused_where_it_is_written() {
    refused(
        &with(mebis(), "/channel", json!("beta")),
        "manifest_unknown_field",
        "/channel",
    );
    refused(
        &with(
            mebis(),
            "/entrypoints/processing/0/runtime",
            json!("python"),
        ),
        "manifest_unknown_field",
        "/entrypoints/processing/0/runtime",
    );
    refused(
        &with(
            mebis(),
            "/contributions/ribbonTabs/0/panels/2/items/1/tooltip",
            json!("x"),
        ),
        "manifest_unknown_field",
        "/contributions/ribbonTabs/0/panels/2/items/1/tooltip",
    );
    let p = Manifest::parse(&with(mebis(), "/chanel", json!(1)).to_string()).unwrap_err();
    assert!(
        p[0].message
            .starts_with("Bilinmeyen alan “chanel”. Buradaki alanlar: format, schemaVersion"),
        "{}",
        p[0].message
    );
}

#[test]
fn missing_fields_wrong_types_and_unknown_values_say_so_in_turkish() {
    refused(
        &without(mebis(), "/license"),
        "manifest_missing_field",
        "/license",
    );
    refused(
        &without(mebis(), "/entrypoints/audits/0/domain"),
        "manifest_missing_field",
        "/entrypoints/audits/0/domain",
    );
    refused(
        &with(mebis(), "/kind", json!("plugin")),
        "manifest_bad_value",
        "/kind",
    );
    refused(
        &with(mebis(), "/modules/1", json!("kadastro")),
        "manifest_bad_value",
        "/modules/1",
    );
    refused(
        &with(
            mebis(),
            "/requestedCapabilities/0/id",
            json!("filesystem.write"),
        ),
        "manifest_bad_value",
        "/requestedCapabilities/0/id",
    );
    refused(
        &with(mebis(), "/authors", json!("Biri")),
        "manifest_bad_type",
        "/authors",
    );
    refused(
        &with(mebis(), "/compatibility/sdkProtocol/max", json!(-1)),
        "manifest_bad_value",
        "/compatibility/sdkProtocol/max",
    );
    for value in [
        with(mebis(), "/kind", json!("plugin")),
        with(mebis(), "/authors", json!(1)),
    ] {
        let p = Manifest::parse(&value.to_string()).unwrap_err();
        assert!(!p[0].message.contains("expected"), "{}", p[0].message);
    }
    let p = Manifest::parse(&with(mebis(), "/kind", json!("plugin")).to_string()).unwrap_err();
    assert_eq!(
        p[0].message,
        "“plugin” geçerli bir değer değil; geçerli değerler: vertical, utility."
    );
}

#[test]
fn identity_version_and_texts_follow_their_forms() {
    for id in [
        "mebis",
        "Tr.ornek.mebis",
        "tr..mebis",
        "tr.ornek.mebis-",
        "tr.ornek.mezarlık",
        "tr.ornek.1mebis",
    ] {
        refused(&with(mebis(), "/id", json!(id)), "manifest_id", "/id");
    }
    for version in ["1.0", "v1.0.0", "1.0.0+build.1", "01.0.0"] {
        refused(
            &with(mebis(), "/version", json!(version)),
            "manifest_version",
            "/version",
        );
    }
    let preview =
        Manifest::parse(&with(mebis(), "/version", json!("1.1.0-beta.1")).to_string()).unwrap();
    assert!(preview.is_preview());
    refused(&with(mebis(), "/name", json!("")), "manifest_text", "/name");
    refused(
        &with(mebis(), "/summary", json!("iki\nsatır")),
        "manifest_text",
        "/summary",
    );
    refused(
        &with(mebis(), "/name", json!("a".repeat(81))),
        "manifest_text",
        "/name",
    );
    refused(
        &with(mebis(), "/description", json!("<b>Kalın</b> açıklama")),
        "manifest_html",
        "/description",
    );
    assert_eq!(
        found(&with(mebis(), "/description", json!("a < b ve 3 > 2"))),
        []
    );
    refused(
        &with(
            mebis(),
            "/authors/0",
            json!("Ayşe Yılmaz <ayse@ornek.bel.tr>"),
        ),
        "manifest_authors",
        "/authors/0",
    );
    refused(
        &with(mebis(), "/authors", json!([])),
        "manifest_authors",
        "/authors",
    );
    refused(
        &with(mebis(), "/publisherId", json!("Örnek")),
        "manifest_publisher",
        "/publisherId",
    );
}

#[test]
fn the_license_is_an_spdx_expression() {
    for good in [
        "MIT",
        "MIT OR Apache-2.0",
        "(MIT AND BSD-3-Clause) OR GPL-2.0+",
        "GPL-2.0-or-later WITH Classpath-exception-2.0",
        "LicenseRef-Ornek-Belediye",
    ] {
        assert_eq!(found(&with(mebis(), "/license", json!(good))), [], "{good}");
    }
    for bad in [
        "",
        "MIT OR",
        "(MIT",
        "MIT AND AND BSD",
        "GPL v3",
        "NOASSERTION",
        "MIT WITH",
    ] {
        refused(
            &with(mebis(), "/license", json!(bad)),
            "manifest_license",
            "/license",
        );
    }
}

#[test]
fn compatibility_ranges_are_whole() {
    let c = "/compatibility";
    refused(
        &with(
            mebis(),
            &format!("{c}/sdkProtocol"),
            json!({"min": 2, "max": 1}),
        ),
        "manifest_compat",
        "/compatibility/sdkProtocol",
    );
    refused(
        &with(
            mebis(),
            &format!("{c}/sdkProtocol"),
            json!({"min": 0, "max": 1}),
        ),
        "manifest_compat",
        "/compatibility/sdkProtocol",
    );
    refused(
        &with(mebis(), &format!("{c}/kentos"), json!({})),
        "manifest_compat",
        "/compatibility/kentos",
    );
    refused(
        &with(
            mebis(),
            &format!("{c}/kentos"),
            json!({"min": "0.3.0", "maxExclusive": "0.3.0"}),
        ),
        "manifest_compat",
        "/compatibility/kentos",
    );
    refused(
        &with(mebis(), &format!("{c}/kentos/min"), json!("0.2")),
        "manifest_compat",
        "/compatibility/kentos/min",
    );
    refused(
        &with(
            mebis(),
            &format!("{c}/operatingSystems"),
            json!(["linux", "linux"]),
        ),
        "manifest_compat",
        "/compatibility/operatingSystems",
    );
    refused(
        &with(
            mebis(),
            &format!("{c}/operatingSystems/0"),
            json!("android"),
        ),
        "manifest_bad_value",
        "/compatibility/operatingSystems/0",
    );
    assert_eq!(
        found(&without(mebis(), "/compatibility/kentos")),
        [],
        "no KentOS range is every KentOS"
    );
}

#[test]
fn dependencies_name_other_extensions_with_ranges_and_cycles_are_found() {
    let d = "/dependencies/extensions/0";
    refused(
        &with(mebis(), &format!("{d}/id"), json!("tr.ornek.mebis")),
        "manifest_dependency_cycle",
        "/dependencies/extensions/0/id",
    );
    refused(
        &with(mebis(), &format!("{d}/version"), json!("1.x")),
        "manifest_dependency",
        "/dependencies/extensions/0/version",
    );
    refused(
        &with(mebis(), &format!("{d}/id"), json!("kent-temel")),
        "manifest_dependency",
        "/dependencies/extensions/0/id",
    );
    let twice = with(
        mebis(),
        "/dependencies/extensions",
        json!([{"id": "tr.a.b", "version": "^1"}, {"id": "tr.a.b", "version": "^2"}]),
    );
    refused(
        &twice,
        "manifest_dependency",
        "/dependencies/extensions/1/id",
    );
    assert_eq!(
        dependency_cycle(&[
            ("a", vec!["b"]),
            ("b", vec!["c"]),
            ("c", vec!["a"]),
            ("d", vec![])
        ]),
        Some(vec![
            "a".to_owned(),
            "b".to_owned(),
            "c".to_owned(),
            "a".to_owned()
        ])
    );
    assert_eq!(
        dependency_cycle(&[("a", vec!["b"]), ("b", vec!["c"]), ("d", vec!["a", "c"])]),
        None
    );
    assert_eq!(
        dependency_cycle(&[("a", vec!["a"])]),
        Some(vec!["a".to_owned(), "a".to_owned()])
    );
}

#[test]
fn contributions_point_at_what_exists() {
    let item = "/contributions/ribbonTabs/0/panels/2/items/1";
    refused(
        &with(mebis(), &format!("{item}/processing"), json!("nope")),
        "manifest_item_unknown",
        &format!("{item}/processing"),
    );
    refused(
        &with(mebis(), &format!("{item}/command"), json!("init-schema")),
        "manifest_item_target",
        item,
    );
    refused(
        &with(mebis(), item, json!({"size": "large"})),
        "manifest_item_target",
        item,
    );
    refused(
        &with(mebis(), item, json!({"audit": "init-schema"})),
        "manifest_item_unknown",
        &format!("{item}/audit"),
    );
    refused(
        &with(
            mebis(),
            item,
            json!({"builtin": "ext.tr.x.y", "label": "X"}),
        ),
        "manifest_builtin",
        &format!("{item}/builtin"),
    );
    refused(
        &with(mebis(), item, json!({"builtin": "cad.layers.service"})),
        "manifest_item_label",
        item,
    );
    assert_eq!(
        found(&with(
            mebis(),
            item,
            json!({"builtin": "cad.layers.service", "label": "Servis ekle"})
        )),
        []
    );
    let twin = push(
        mebis(),
        "/entrypoints/expressions",
        json!({"id": "init-schema", "handler": "mebis.logic:x"}),
    );
    refused(
        &twin,
        "manifest_duplicate_id",
        "/entrypoints/expressions/1/id",
    );
    refused(
        &with(
            mebis(),
            "/entrypoints/commands/0/handler",
            json!("mebis/logic.py"),
        ),
        "manifest_handler",
        "/entrypoints/commands/0/handler",
    );
    let bad_id = push(
        mebis(),
        "/entrypoints/expressions",
        json!({"id": "Kötü_Kimlik", "handler": "mebis.logic:x"}),
    );
    refused(
        &bad_id,
        "manifest_local_id",
        "/entrypoints/expressions/1/id",
    );
    refused(
        &with(mebis(), "/contributions/ribbonTabs/0/module", json!("yol")),
        "manifest_tab_module",
        "/contributions/ribbonTabs/0/module",
    );
    refused(
        &with(mebis(), "/kind", json!("utility")),
        "manifest_utility_tab",
        "/contributions/ribbonTabs",
    );
    refused(
        &with(
            mebis(),
            "/contributions/ribbonTabs/0/panels/1/items/0/when/capabilityGranted",
            json!("files.userSelected"),
        ),
        "manifest_when",
        "/contributions/ribbonTabs/0/panels/1/items/0/when/capabilityGranted",
    );
    refused(
        &with(
            mebis(),
            "/entrypoints/mapTools/0/templateLengthM",
            json!(-2),
        ),
        "manifest_map_tool",
        "/entrypoints/mapTools/0",
    );
    refused(
        &with(
            mebis(),
            "/entrypoints/forms/0/definition",
            json!("../forms/x.kform"),
        ),
        "manifest_path",
        "/entrypoints/forms/0/definition",
    );
    refused(
        &with(
            mebis(),
            "/entrypoints/forms/0/definition",
            json!("forms/x.json"),
        ),
        "manifest_path",
        "/entrypoints/forms/0/definition",
    );
}

#[test]
fn a_package_that_adds_nothing_is_refused() {
    let empty = without(
        without(without(mebis(), "/entrypoints"), "/contributions"),
        "/python",
    );
    let empty = with(empty, "/requestedCapabilities", json!([]));
    refused(&empty, "manifest_empty", "");
    let handlers_without_python = without(mebis(), "/python");
    refused(
        &handlers_without_python,
        "manifest_python_missing",
        "/entrypoints/commands/0/handler",
    );
}

#[test]
fn capabilities_carry_a_reason_a_scope_and_the_hosts() {
    let r = "/requestedCapabilities";
    refused(
        &with(mebis(), &format!("{r}/0/reason"), json!("  ")),
        "manifest_capability_reason",
        "/requestedCapabilities/0/reason",
    );
    refused(
        &with(mebis(), &format!("{r}/0/reason"), json!("x".repeat(121))),
        "manifest_text",
        "/requestedCapabilities/0/reason",
    );
    refused(
        &with(mebis(), &format!("{r}/1/scope"), json!("current-project")),
        "manifest_capability_scope",
        "/requestedCapabilities/1/scope",
    );
    refused(
        &without(mebis(), &format!("{r}/9/sendsDataToAi")),
        "manifest_capability_network",
        "/requestedCapabilities/9/sendsDataToAi",
    );
    let schemed = json!(["https://harita.ornek.bel.tr", "harita.ornek.bel.tr"]);
    refused(
        &with(mebis(), &format!("{r}/9/hosts"), schemed),
        "manifest_capability_network",
        "/requestedCapabilities/9/hosts/0",
    );
    refused(
        &with(mebis(), &format!("{r}/0/hosts"), json!(["a.b"])),
        "manifest_capability_network",
        "/requestedCapabilities/0",
    );
    let twice = push(
        mebis(),
        r,
        json!({"id": "ui.ribbonTab", "scope": "plugin-owned", "reason": "Bir daha."}),
    );
    refused(
        &twice,
        "manifest_capability_duplicate",
        "/requestedCapabilities/10/id",
    );
    let p = Manifest::parse(&without(mebis(), &format!("{r}/1")).to_string()).unwrap_err();
    assert_eq!((p.len(), p[0].code), (1, "manifest_capability_missing"));
    assert!(
        p[0].message.contains("ui.ribbonTab (şerit sekmesi)"),
        "{}",
        p[0].message
    );
    let elsewhere = with(
        mebis(),
        "/contributions/dataSources/0/url",
        json!("https://tiles.example.org/wmts"),
    );
    refused(
        &elsewhere,
        "manifest_data_source_host",
        "/contributions/dataSources/0/url",
    );
    let secret = with(
        mebis(),
        "/contributions/dataSources/0/url",
        json!("https://kullanici:parola@harita.ornek.bel.tr/wmts"),
    );
    refused(
        &secret,
        "manifest_data_source",
        "/contributions/dataSources/0/url",
    );
    let no_layers = with(mebis(), "/contributions/dataSources/0/layers", json!([]));
    refused(
        &no_layers,
        "manifest_data_source",
        "/contributions/dataSources/0/layers",
    );
    let gpkg_with_url = with(
        mebis(),
        "/contributions/dataSources/0/kind",
        json!("geopackage"),
    );
    refused(
        &gpkg_with_url,
        "manifest_data_source",
        "/contributions/dataSources/0",
    );
    for capability in Capability::ALL {
        let id: Capability = serde_json::from_value(json!(capability.id())).unwrap();
        assert_eq!(id, capability);
        assert!(!capability.title().is_empty() && !capability.scopes().is_empty());
    }
}

#[test]
fn what_works_but_should_be_mended_is_a_warning() {
    let insecure = with(
        mebis(),
        "/links/homepage",
        json!("http://cbs.ornek.bel.tr/mebis"),
    );
    assert_eq!(warnings(&insecure), ["manifest_link_insecure"]);
    let altlik: Value = serde_json::from_str(ALTLIK).unwrap();
    let unused = push(
        altlik,
        "/requestedCapabilities",
        json!({"id": "audit.provider", "scope": "plugin-owned", "reason": "Gerek yok."}),
    );
    assert_eq!(found(&unused), []);
    assert_eq!(warnings(&unused), ["manifest_capability_unused"]);
}

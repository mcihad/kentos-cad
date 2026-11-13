//! The light Python reader: the TOML subset, the names and ranges as uv writes them, and the project against its lock.

use super::*;

const PROJECT: &str = include_str!("../../../../../fixtures/extensions/v1/mebis/pyproject.toml");
const LOCK: &str = include_str!("../../../../../fixtures/extensions/v1/mebis/uv.lock");
const MANIFEST: &str = include_str!("../../../../../fixtures/extensions/v1/mebis/manifest.json");

fn table(text: &str) -> Toml {
    parse_toml(text).unwrap_or_else(|e| panic!("{e:?}"))
}

#[test]
fn the_toml_subset_reads_as_pyproject_and_uv_write_it() {
    let t = table(
        r#"
# a comment
top = "değer" # after a value
[project]
name = "mebis"
dependencies = [
    "shapely>=2.0",   # trailing comma and comments inside
    'literal\no-escape',
]
nested.dotted = 3
[[package]]
name = "a"
source = { registry = "https://pypi.org/simple" }
[package.metadata]
requires-dist = [{ name = "x", specifier = ">=1" }]
[[package]]
name = "b"
flags = [true, false, 1_000, 1.5, 1979-05-27]
text = """
satır \
  devamı "tırnak" \u00e7"""
lit = '''
çok
satır'''
"key with space" = "\t\"kaçış\""
"#,
    );
    assert_eq!(t.get(&["top"]), Some(&Toml::Str("değer".into())));
    assert_eq!(t.get(&["project", "nested", "dotted"]), Some(&Toml::Int(3)));
    let deps = t
        .get(&["project", "dependencies"])
        .and_then(Toml::array)
        .unwrap();
    assert_eq!(deps[1], Toml::Str("literal\\no-escape".into()));
    let packages = t.get(&["package"]).and_then(Toml::array).unwrap();
    assert_eq!(packages.len(), 2);
    assert_eq!(
        packages[0].get(&["source", "registry"]).and_then(Toml::str),
        Some("https://pypi.org/simple")
    );
    assert!(
        packages[0].get(&["metadata", "requires-dist"]).is_some(),
        "[package.metadata] is the last package's"
    );
    assert_eq!(
        packages[1].get(&["flags"]),
        Some(&Toml::Array(vec![
            Toml::Bool(true),
            Toml::Bool(false),
            Toml::Int(1000),
            Toml::Other("1.5".into()),
            Toml::Other("1979-05-27".into()),
        ]))
    );
    assert_eq!(
        packages[1].get(&["text"]).and_then(Toml::str),
        Some("satır devamı \"tırnak\" ç")
    );
    assert_eq!(
        packages[1].get(&["lit"]).and_then(Toml::str),
        Some("çok\nsatır")
    );
    assert_eq!(
        packages[1].get(&["key with space"]).and_then(Toml::str),
        Some("\t\"kaçış\"")
    );
    assert_eq!(
        table("a = \"\"\"x\"\"\"\"\"\n")
            .get(&["a"])
            .and_then(Toml::str),
        Some("x\"\"")
    );
}

#[test]
fn toml_errors_name_their_line() {
    for (text, line) in [
        ("a = 1\na = 2\n", 2),
        ("a = \"open\n", 1),
        ("\n\nb 1\n", 3),
        ("[t]\nx = [1, 2\ny = 3\n", 3),
        ("a = 1 b\n", 1),
        ("[[t]\n", 1),
        ("a = \"\\q\"\n", 1),
        ("a = 1\n[a]\nb = 2\n", 2),
    ] {
        let (at, message) = parse_toml(text).expect_err(text);
        assert_eq!(at, line, "{text:?}: {message}");
    }
    assert!(
        parse_toml("x = \"\\uD800\"").is_err(),
        "a surrogate is not a character"
    );
}

#[test]
fn names_requirements_markers_and_ranges_compare_as_uv_writes_them() {
    assert_eq!(normalized("Kent_Temel.Ext"), "kent-temel-ext");
    assert_eq!(normalized("a--_.b"), "a-b");
    assert_eq!(
        requirement("packaging >= 24.0 , < 30"),
        Some(("packaging".into(), "<30,>=24.0".into()))
    );
    assert_eq!(
        requirement("Shapely[vectorized]>=2.0; python_version < '3.13'"),
        Some(("shapely".into(), ">=2.0".into()))
    );
    assert_eq!(
        requirement("tzdata ; sys_platform == 'win32'"),
        Some(("tzdata".into(), String::new()))
    );
    assert_eq!(
        requirement("pkg @ https://example.org/pkg.whl"),
        Some(("pkg".into(), String::new()))
    );
    assert_eq!(
        requirement("x (>=1,<2)"),
        Some(("x".into(), "<2,>=1".into()))
    );
    assert_eq!(requirement(">=1"), None);
    assert_eq!(specifier(">=24.0,<30"), specifier("< 30, >= 24.0"));
    assert_eq!(
        marker("sys_platform == \"win32\"  and platform_machine == 'AMD64'"),
        "platform_machine == 'AMD64' and sys_platform == 'win32'"
    );
    assert_eq!(
        marker_extra("extra == 'Extra_One'"),
        Some("extra-one".into())
    );
    assert_eq!(marker_extra("sys_platform == 'win32'"), None);
    assert!(
        python_ranges_agree("~=3.12", ">=3.12, <4"),
        "uv writes ~= as a range"
    );
    assert!(python_ranges_agree(">=3.12,<3.15", ">=3.12, <3.15"));
    assert!(python_ranges_agree("==3.13.*", "==3.13.*"));
    assert!(!python_ranges_agree(">=3.12", ">=3.13"));
    assert!(!python_ranges_agree(">=3.12", ">=3.12, <3.14"));
    assert!(
        python_ranges_agree("===3.12", "===3.12"),
        "an unknown operator falls back to the text"
    );
}

/// The problems and warnings of the project against the lock, under the fixture's manifest (changed by `edit`).
fn checked(
    project: &str,
    lock: &str,
    edit: impl Fn(&mut Manifest),
) -> (Vec<&'static str>, Vec<&'static str>) {
    let mut manifest = Manifest::read(MANIFEST).unwrap();
    edit(&mut manifest);
    let (mut problems, mut warnings) = (Vec::new(), Vec::new());
    check(
        &manifest,
        "pyproject.toml",
        project,
        "uv.lock",
        lock,
        &mut problems,
        &mut warnings,
    );
    (
        problems.iter().map(|p| p.code).collect(),
        warnings.iter().map(|p| p.code).collect(),
    )
}

fn plain(project: &str, lock: &str) -> (Vec<&'static str>, Vec<&'static str>) {
    checked(project, lock, |_| {})
}

#[test]
fn the_fixtures_real_lock_agrees_with_its_project() {
    assert_eq!(plain(PROJECT, LOCK), (vec![], vec![]));
}

#[test]
fn a_lock_older_than_its_project_is_stale() {
    let stale = |project: &str| plain(project, LOCK).0;
    assert_eq!(
        stale(&PROJECT.replace("shapely>=2.0", "shapely>=2.1")),
        ["python_lock_stale"]
    );
    assert_eq!(
        stale(&PROJECT.replace("[\"shapely>=2.0\"]", "[\"shapely>=2.0\", \"pyproj>=3.6\"]")),
        ["python_lock_stale", "python_lock_stale"],
        "the list differs and pyproj is not resolved"
    );
    assert_eq!(
        stale(&PROJECT.replace(">=3.12,<3.15", ">=3.13,<3.15")),
        ["python_lock_stale"]
    );
    assert_eq!(
        stale(&PROJECT.replace("pygal>=3.0", "pygal>=3.1")),
        ["python_lock_stale"]
    );
    let fewer_systems = PROJECT.replacen(
        "    \"sys_platform == 'linux' and platform_machine == 'x86_64'\",\n",
        "",
        2,
    );
    let (problems, _) = checked(&fewer_systems, LOCK, |m| {
        m.compatibility.operating_systems.truncate(2)
    });
    assert_eq!(
        problems,
        ["python_lock_stale", "python_lock_stale"],
        "both marker lists moved"
    );
}

#[test]
fn every_system_the_manifest_names_has_its_wheels_required() {
    let no_linux = PROJECT.replacen(
        "    \"sys_platform == 'linux' and platform_machine == 'x86_64'\",\n",
        "",
        2,
    );
    let (problems, _) = plain(&no_linux, LOCK);
    assert_eq!(
        &problems[..2],
        ["python_environments", "python_environments"],
        "{problems:?}"
    );
    let (problems, _) = checked(&no_linux, LOCK, |m| {
        m.compatibility.operating_systems = vec![Os::Windows, Os::Macos]
    });
    assert!(
        !problems.contains(&"python_environments"),
        "the manifest no longer names Linux"
    );
    let (problems, _) = checked(&no_linux, LOCK, |m| {
        m.compatibility.operating_systems.clear()
    });
    assert!(
        problems.contains(&"python_environments"),
        "no systems written is every system"
    );
}

#[test]
fn optional_features_name_an_extra_of_the_project() {
    let (problems, _) = checked(PROJECT, LOCK, |m| {
        m.python
            .as_mut()
            .unwrap()
            .optional_features
            .get_mut("grafik")
            .unwrap()
            .extra = "plots".into();
    });
    assert_eq!(problems, ["python_extra_missing"]);
}

/// The lock with pygal's `source = …` line replaced.
fn pygal_from(source: &str) -> String {
    let at = LOCK
        .find("name = \"pygal\"\nversion")
        .expect("the fixture's lock has pygal");
    let line = at + LOCK[at..].find("source = ").unwrap();
    let end = line + LOCK[line..].find('\n').unwrap();
    format!("{}source = {source}{}", &LOCK[..line], &LOCK[end..])
}

#[test]
fn sources_only_on_the_authors_computer_are_refused_and_others_named() {
    assert_eq!(
        plain(PROJECT, &pygal_from("{ directory = \"../../pygal\" }")).0,
        ["python_local_source"]
    );
    assert_eq!(
        plain(PROJECT, &pygal_from("{ path = \"/tmp/pygal.whl\" }")).0,
        ["python_local_source"]
    );
    assert_eq!(
        plain(PROJECT, &pygal_from("{ editable = \"../pygal\" }")).0,
        ["python_local_source"]
    );
    assert_eq!(
        plain(
            PROJECT,
            &pygal_from("{ git = \"https://example.org/pygal.git?rev=abc#abc\" }")
        ),
        (vec![], vec!["python_vcs_source"])
    );
    let mirror = LOCK.replace(
        "https://pypi.org/simple",
        "https://pypi.ornek.bel.tr/simple",
    );
    let (problems, warnings) = plain(PROJECT, &mirror);
    assert!(
        problems.is_empty()
            && !warnings.is_empty()
            && warnings.iter().all(|w| *w == "python_registry")
    );
}

#[test]
fn an_unknown_lock_version_is_a_warning_and_broken_files_are_problems() {
    assert_eq!(
        plain(PROJECT, &LOCK.replacen("version = 1", "version = 2", 1)),
        (vec![], vec!["python_lock_version"])
    );
    assert_eq!(plain("[project\nname = 1", LOCK).0, ["python_toml"]);
    assert_eq!(
        plain(PROJECT, "version = 1\n[[package]\n").0,
        ["python_toml"]
    );
    assert_eq!(
        plain("[project]\nname = \"x\"\n", LOCK).0,
        ["python_project"]
    );
    assert_eq!(plain("[tool.uv]\n", LOCK).0, ["python_project"]);
    assert_eq!(
        plain(
            &PROJECT.replace("name = \"mebis\"", "name = \"mezar\""),
            LOCK
        )
        .0,
        ["python_lock_stale"],
        "the project itself is not in the lock"
    );
}

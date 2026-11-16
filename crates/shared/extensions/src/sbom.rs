//! A package's software bill of materials (plan/eklenti.md ADIM 18A, PY-UV-29: "sır içermeyen kilit/ortam/SBOM raporu
//! dışa aktarılır"): CycloneDX 1.6 JSON, read from the package's lock. What an organisation's IT keeps for every
//! extension it installs: which Python packages, which versions, from where, with which files' SHA-256. The lock is
//! what the environment is made from (each file checked against its hash), so it is the environment's list too, with
//! no Python run.
//!
//! - **No secret:** an address's user and password (`https://kullanici:parola@ayna/simple`) are cut from every URL.
//! - **The same package, the same report:** the serial number is made from the package's content; only the time a
//!   caller gives changes.
//! - A Python package's licence is not in the lock: not said (an installed environment's metadata would).

use std::collections::BTreeMap;

use serde_json::{Value, json};

use crate::integrity::sha256_hex;
use crate::manifest::Manifest;
use crate::python::{Toml, normalized, parse_toml};

/// `time` as RFC 3339 in UTC (`2026-11-16T12:00:00Z`), for a report's timestamp: no date crate is needed.
pub fn utc_text(time: std::time::SystemTime) -> String {
    let secs = time
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    let (days, rest) = ((secs / 86_400) as i64, secs % 86_400);
    // Howard Hinnant's days-to-civil.
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}Z",
        rest / 3600,
        rest % 3600 / 60,
        rest % 60
    )
}

/// The index most packages come from, as its purl needs no repository.
const PYPI: &str = "https://pypi.org/simple";

/// `url` without a user or a password before its host.
pub fn without_credentials(url: &str) -> String {
    let Some((scheme, rest)) = url.split_once("://") else {
        return url.to_owned();
    };
    let (authority, path) = rest.split_at(rest.find('/').unwrap_or(rest.len()));
    match authority.rsplit_once('@') {
        Some((_, host)) => format!("{scheme}://{host}{path}"),
        None => url.to_owned(),
    }
}

/// A UUID's text made from `digest` (hex), as CycloneDX's serial number asks (RFC 4122, version 5's bits).
fn serial(digest: &str) -> String {
    let hex: Vec<char> = digest.chars().filter(char::is_ascii_hexdigit).collect();
    let at = |i: usize| hex.get(i).copied().unwrap_or('0');
    let mut u: String = (0..32).map(at).collect();
    u.replace_range(12..13, "5");
    let variant = ['8', '9', 'a', 'b'][at(16).to_digit(16).unwrap_or(0) as usize % 4];
    u.replace_range(16..17, &variant.to_string());
    format!(
        "urn:uuid:{}-{}-{}-{}-{}",
        &u[0..8],
        &u[8..12],
        &u[12..16],
        &u[16..20],
        &u[20..32]
    )
}

/// A lock entry's string field.
fn string<'a>(table: &'a BTreeMap<String, Toml>, key: &str) -> Option<&'a str> {
    match table.get(key) {
        Some(Toml::Str(s)) => Some(s.as_str()),
        _ => None,
    }
}

/// A lock entry's package as a CycloneDX component: its purl, its distribution files with their SHA-256, where it
/// comes from.
fn component(p: &BTreeMap<String, Toml>, name: &str) -> Value {
    let version = string(p, "version").unwrap_or_default();
    let (kind, at) = match p.get("source") {
        Some(Toml::Table(s)) => s
            .iter()
            .find_map(|(k, v)| match v {
                Toml::Str(v) => Some((k.clone(), without_credentials(v))),
                _ => None,
            })
            .unwrap_or_default(),
        _ => (String::new(), String::new()),
    };
    let purl = match kind.as_str() {
        "registry" if at.trim_end_matches('/') == PYPI => {
            Some(format!("pkg:pypi/{name}@{version}"))
        }
        "registry" => Some(format!("pkg:pypi/{name}@{version}?repository_url={at}")),
        "git" => Some(format!("pkg:pypi/{name}@{version}?vcs_url=git%2B{at}")),
        _ => None,
    };
    let wheels: &[Toml] = match p.get("wheels") {
        Some(Toml::Array(w)) => w,
        _ => &[],
    };
    let files: Vec<Value> = p
        .get("sdist")
        .into_iter()
        .chain(wheels)
        .filter_map(|f| match f {
            Toml::Table(f) => {
                let url = without_credentials(string(f, "url")?);
                let hash = string(f, "hash")?.strip_prefix("sha256:")?;
                Some(json!({
                    "type": "distribution",
                    "url": url,
                    "hashes": [{ "alg": "SHA-256", "content": hash }]
                }))
            }
            _ => None,
        })
        .collect();
    let mut c = json!({
        "type": "library",
        "bom-ref": format!("{name}@{version}"),
        "name": name,
        "version": version,
    });
    if let Some(purl) = purl {
        c["purl"] = json!(purl);
    }
    if !files.is_empty() {
        c["externalReferences"] = json!(files);
    }
    if !kind.is_empty() {
        c["properties"] = json!([{ "name": "kentos:source", "value": format!("{kind} {at}") }]);
    }
    c
}

/// The package's SBOM: `manifest` and its `files` (the lock among them), made at `timestamp` (RFC 3339) by KentOS
/// `kentos`; `None` when it carries no Python project or its lock does not read.
pub fn cyclonedx(
    manifest: &Manifest,
    files: &BTreeMap<String, Vec<u8>>,
    timestamp: &str,
    kentos: &str,
) -> Option<Value> {
    let project = manifest.python.as_ref()?;
    let lock = std::str::from_utf8(files.get(&project.lock_file)?).ok()?;
    let Toml::Table(root) = parse_toml(lock).ok()? else {
        return None;
    };
    let own = files
        .get(&project.project_file)
        .and_then(|b| std::str::from_utf8(b).ok())
        .and_then(|t| parse_toml(t).ok())
        .and_then(|t| match t {
            Toml::Table(t) => match t.get("project") {
                Some(Toml::Table(p)) => string(p, "name").map(normalized),
                _ => None,
            },
            _ => None,
        });
    let mut components = Vec::new();
    let mut graph = Vec::new();
    if let Some(Toml::Array(list)) = root.get("package") {
        for entry in list {
            let Toml::Table(p) = entry else { continue };
            let Some(name) = string(p, "name").map(normalized) else {
                continue;
            };
            if own.as_deref() == Some(name.as_str()) {
                continue;
            }
            let c = component(p, &name);
            let needs: Vec<String> = match p.get("dependencies") {
                Some(Toml::Array(d)) => d
                    .iter()
                    .filter_map(|d| match d {
                        Toml::Table(d) => string(d, "name").map(normalized),
                        _ => None,
                    })
                    .collect(),
                _ => Vec::new(),
            };
            graph.push((c["bom-ref"].as_str().unwrap_or_default().to_owned(), needs));
            components.push(c);
        }
    }
    // The graph by bom-ref: a dependency's ref is its name's component's.
    let refs: BTreeMap<String, String> = components
        .iter()
        .filter_map(|c| {
            Some((
                c["name"].as_str()?.to_owned(),
                c["bom-ref"].as_str()?.to_owned(),
            ))
        })
        .collect();
    let me = format!("{}@{}", manifest.id, manifest.version);
    let mut dependencies = vec![json!({
        "ref": me,
        "dependsOn": components.iter().filter_map(|c| c["bom-ref"].as_str()).collect::<Vec<_>>(),
    })];
    for (r, needs) in graph {
        let on: Vec<&String> = needs.iter().filter_map(|n| refs.get(n)).collect();
        dependencies.push(json!({ "ref": r, "dependsOn": on }));
    }
    let digest = {
        let mut all = String::new();
        for (path, data) in files {
            all.push_str(path);
            all.push('\n');
            all.push_str(&sha256_hex(data));
            all.push('\n');
        }
        sha256_hex(all.as_bytes())
    };
    let mut component = json!({
        "type": "application",
        "bom-ref": me,
        "name": manifest.id,
        "version": manifest.version.to_string(),
        "description": manifest.name,
    });
    if let Some(publisher) = &manifest.publisher_id {
        component["publisher"] = json!(publisher);
    }
    if !manifest.license.trim().is_empty() {
        component["licenses"] = json!([{ "expression": manifest.license }]);
    }
    Some(json!({
        "bomFormat": "CycloneDX",
        "specVersion": "1.6",
        "serialNumber": serial(&digest),
        "version": 1,
        "metadata": {
            "timestamp": timestamp,
            "tools": { "components": [{ "type": "application", "name": "KentOS kek sbom", "version": kentos }] },
            "component": component,
        },
        "components": components,
        "dependencies": dependencies,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn package(lock: &str) -> (Manifest, BTreeMap<String, Vec<u8>>) {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../../fixtures/extensions/v1/ornek-altlik/manifest.json");
        let mut m: Value = serde_json::from_str(&std::fs::read_to_string(root).unwrap()).unwrap();
        m["python"] = json!({ "projectFile": "pyproject.toml", "lockFile": "uv.lock" });
        m["publisherId"] = json!("ornek");
        let manifest = Manifest::read(&m.to_string()).expect("a manifest");
        let files = BTreeMap::from([
            ("manifest.json".to_owned(), m.to_string().into_bytes()),
            (
                "pyproject.toml".to_owned(),
                b"[project]\nname = \"ornek-altlik\"\nversion = \"1.0.0\"\n".to_vec(),
            ),
            ("uv.lock".to_owned(), lock.as_bytes().to_vec()),
        ]);
        (manifest, files)
    }

    const LOCK: &str = r#"version = 1
revision = 3

[[package]]
name = "ornek-altlik"
version = "1.0.0"
source = { editable = "." }
dependencies = [{ name = "Shapely" }]

[[package]]
name = "numpy"
version = "2.5.4"
source = { registry = "https://pypi.org/simple" }
sdist = { url = "https://files.pythonhosted.org/numpy-2.5.4.tar.gz", hash = "sha256:aa11", size = 1 }
wheels = [
    { url = "https://files.pythonhosted.org/numpy-2.5.4-cp312-win_amd64.whl", hash = "sha256:bb22", size = 2 },
]

[[package]]
name = "Shapely"
version = "2.0.7"
source = { registry = "https://kullanici:parola@ayna.ornek.bel.tr/simple" }
dependencies = [{ name = "numpy" }]
wheels = [
    { url = "https://kullanici:parola@ayna.ornek.bel.tr/shapely-2.0.7.whl", hash = "sha256:cc33", size = 3 },
]
"#;

    /// The lock's packages with their purls, files and hashes; the project itself left out; the graph; no password;
    /// the same package gives the same serial number.
    #[test]
    fn the_lock_reads_as_a_cyclonedx_bom() {
        let (manifest, files) = package(LOCK);
        let bom = cyclonedx(&manifest, &files, "2026-11-16T12:00:00Z", "0.1.0").expect("a bom");
        assert_eq!(bom["bomFormat"], "CycloneDX");
        assert_eq!(bom["specVersion"], "1.6");
        assert_eq!(bom["metadata"]["component"]["name"], "tr.ornek.altlik");
        assert_eq!(bom["metadata"]["component"]["publisher"], "ornek");
        let names: Vec<&str> = bom["components"]
            .as_array()
            .unwrap()
            .iter()
            .map(|c| c["name"].as_str().unwrap())
            .collect();
        assert_eq!(
            names,
            ["numpy", "shapely"],
            "the project itself is not a component"
        );
        let numpy = &bom["components"][0];
        assert_eq!(numpy["purl"], "pkg:pypi/numpy@2.5.4");
        assert_eq!(numpy["externalReferences"].as_array().unwrap().len(), 2);
        assert_eq!(
            numpy["externalReferences"][1]["hashes"][0]["content"],
            "bb22"
        );
        let shapely = &bom["components"][1];
        assert_eq!(
            shapely["purl"],
            "pkg:pypi/shapely@2.0.7?repository_url=https://ayna.ornek.bel.tr/simple"
        );
        let text = bom.to_string();
        assert!(
            !text.contains("parola") && !text.contains("kullanici"),
            "{text}"
        );
        let deps = bom["dependencies"].as_array().unwrap();
        assert_eq!(
            deps[0]["dependsOn"],
            json!(["numpy@2.5.4", "shapely@2.0.7"])
        );
        assert_eq!(
            deps[2],
            json!({ "ref": "shapely@2.0.7", "dependsOn": ["numpy@2.5.4"] })
        );
        let serial = bom["serialNumber"].as_str().unwrap();
        assert!(
            serial.starts_with("urn:uuid:") && serial.len() == 45,
            "{serial}"
        );
        let again = cyclonedx(&manifest, &files, "2026-11-17T08:00:00Z", "0.1.0").unwrap();
        assert_eq!(again["serialNumber"], bom["serialNumber"]);
        // Without Python there is nothing to list.
        let mut bare = manifest.clone();
        bare.python = None;
        assert!(cyclonedx(&bare, &files, "t", "v").is_none());
    }

    #[test]
    fn a_time_reads_as_utc() {
        let at = std::time::UNIX_EPOCH + std::time::Duration::from_secs(1_794_830_400);
        assert_eq!(utc_text(at), "2026-11-16T12:00:00Z");
        assert_eq!(utc_text(std::time::UNIX_EPOCH), "1970-01-01T00:00:00Z");
    }

    #[test]
    fn an_address_loses_its_user_and_password() {
        assert_eq!(
            without_credentials("https://ali:gizli@ayna.ornek.bel.tr/simple"),
            "https://ayna.ornek.bel.tr/simple"
        );
        assert_eq!(
            without_credentials("https://pypi.org/simple"),
            "https://pypi.org/simple"
        );
        assert_eq!(
            without_credentials("git+ssh://git@host/x"),
            "git+ssh://host/x"
        );
        assert_eq!(without_credentials("."), ".");
    }
}

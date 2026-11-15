//! How deep a person reads a version (plan/eklenti.md ADIM 22A, D12): its difference with the version approved before
//! it, read from the two packages themselves. A new capability, a wider one or a new dependency is read whole; changed
//! code with nothing new asked for is read as a difference; text and icons alone take the fast path, and so does a
//! trusted publisher's code, but only when no capability or dependency changed.
//!
//! What every version changes is left out (its version number in the manifest, in `pyproject.toml` and in the project's
//! own entry in `uv.lock`; the lock format's revision), so a version that only says so is not taken for changed code.
//! A file whose part is not known counts as code: when unsure, the slower path.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use super::{LockedPackage, ReleaseFacts};
use crate::manifest::{
    self, Capability, ExtensionDependency, FileRole, Manifest, RequestedCapability, Scope,
};

/// How deep a person reads a version.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ReviewDepth {
    /// A look at the difference: only text and icons changed, or a trusted publisher's code with nothing new asked for.
    Fast,
    /// The difference read: code, the manifest's behaviour, a dependency's version or the lock changed.
    Standard,
    /// The whole package read: the first approved version, a new or wider capability, a new dependency.
    Deep,
}

impl ReviewDepth {
    pub fn title(self) -> &'static str {
        match self {
            ReviewDepth::Fast => "Hızlı yol",
            ReviewDepth::Standard => "Fark incelemesi",
            ReviewDepth::Deep => "Derin inceleme",
        }
    }
}

/// What kind of difference; in the order a reviewer reads them, the riskiest first.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DifferenceKind {
    NewCapability,
    /// A wider scope, a new network host, data newly sent to an outside AI service.
    WiderCapability,
    NewDependency,
    /// A dependency now from another index, repository or commit: another package under the same name.
    DependencySource,
    NarrowerCapability,
    DroppedCapability,
    /// Another version or range, or the same version's files changed.
    ChangedDependency,
    DroppedDependency,
    /// The lock changed and its packages did not.
    Lock,
    /// Code or behaviour: Python, the manifest's entry points and contributions, forms, panels, tools, schemas, data.
    Code,
    /// Text and icons: the change notes, the licence, translations, the listing's words, an icon.
    Text,
}

impl DifferenceKind {
    pub fn title(self) -> &'static str {
        match self {
            DifferenceKind::NewCapability => "Yeni yetenek",
            DifferenceKind::WiderCapability => "Genişleyen yetenek",
            DifferenceKind::NewDependency => "Yeni bağımlılık",
            DifferenceKind::DependencySource => "Kaynağı değişen bağımlılık",
            DifferenceKind::NarrowerCapability => "Daralan yetenek",
            DifferenceKind::DroppedCapability => "Kalkan yetenek",
            DifferenceKind::ChangedDependency => "Değişen bağımlılık",
            DifferenceKind::DroppedDependency => "Kalkan bağımlılık",
            DifferenceKind::Lock => "Kilit",
            DifferenceKind::Code => "Kod",
            DifferenceKind::Text => "Metin ve simge",
        }
    }

    /// A capability's or a dependency's difference: it closes a trusted publisher's fast path (D12).
    pub fn is_capability_or_dependency(self) -> bool {
        !matches!(self, DifferenceKind::Code | DifferenceKind::Text)
    }

    /// The depth it asks for; `trusted_code`: a trusted publisher's version with no capability or dependency
    /// difference, whose code takes the fast path.
    pub fn depth(self, trusted_code: bool) -> ReviewDepth {
        match self {
            DifferenceKind::NewCapability
            | DifferenceKind::WiderCapability
            | DifferenceKind::NewDependency
            | DifferenceKind::DependencySource => ReviewDepth::Deep,
            DifferenceKind::NarrowerCapability
            | DifferenceKind::DroppedCapability
            | DifferenceKind::ChangedDependency
            | DifferenceKind::DroppedDependency
            | DifferenceKind::Lock => ReviewDepth::Standard,
            DifferenceKind::Code if trusted_code => ReviewDepth::Fast,
            DifferenceKind::Code => ReviewDepth::Standard,
            DifferenceKind::Text => ReviewDepth::Fast,
        }
    }
}

/// One difference: what it is about and what changed.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Difference {
    pub kind: DifferenceKind,
    /// A capability's id, a dependency's name, a file's path.
    pub subject: String,
    /// What changed, in Turkish.
    pub detail: String,
}

/// What the reviewer is shown first: the depth and the differences that set it (D12).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ReviewFocus {
    pub depth: ReviewDepth,
    /// The content under review (`sha256:…`).
    pub content: String,
    /// The approved version it is compared with; none for the first.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub previous: Option<String>,
    /// A trusted publisher's code took the fast path.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub trusted_fast_path: bool,
    /// One line for the review queue, in Turkish.
    pub summary: String,
    /// The differences, the deepest first.
    pub differences: Vec<Difference>,
}

/// The focus of `next`'s review against `previous`, the version approved before it
/// ([`Release::baseline`](super::Release::baseline)); none for the first. `trusted`: the catalog trusts the publisher.
/// [`Release::start_review`](super::Release::start_review) computes it so and keeps it; a review queue may compute it
/// ahead to order its work.
pub fn classify(
    previous: Option<&ReleaseFacts>,
    next: &ReleaseFacts,
    trusted: bool,
) -> ReviewFocus {
    let content = next.content().package;
    let mut differences = Vec::new();
    let none = BTreeMap::new();
    let Some(before) = previous else {
        capabilities(&[], &next.manifest.requested_capabilities, &mut differences);
        extensions(
            &[],
            &next.manifest.dependencies.extensions,
            &mut differences,
        );
        python_packages(&none, packages(next, &none), &mut differences);
        sort(&mut differences, false);
        return ReviewFocus {
            depth: ReviewDepth::Deep,
            content,
            previous: None,
            trusted_fast_path: false,
            summary: format!(
                "{} — ilk onaylı sürüm, paketin tamamı okunur",
                ReviewDepth::Deep.title()
            ),
            differences,
        };
    };
    capabilities(
        &before.manifest.requested_capabilities,
        &next.manifest.requested_capabilities,
        &mut differences,
    );
    extensions(
        &before.manifest.dependencies.extensions,
        &next.manifest.dependencies.extensions,
        &mut differences,
    );
    python_packages(
        packages(before, &none),
        packages(next, &none),
        &mut differences,
    );
    python_project(before, next, &mut differences);
    manifest_fields(&before.manifest, &next.manifest, &mut differences);
    files(before, next, &mut differences);
    let trusted_code = trusted
        && !differences
            .iter()
            .any(|d| d.kind.is_capability_or_dependency());
    let depth = differences
        .iter()
        .map(|d| d.kind.depth(trusted_code))
        .max()
        .unwrap_or(ReviewDepth::Fast);
    let trusted_fast_path =
        trusted_code && differences.iter().any(|d| d.kind == DifferenceKind::Code);
    sort(&mut differences, trusted_code);
    let previous = before.manifest.version.clone();
    let summary = summary(
        depth,
        trusted_fast_path,
        trusted_code,
        &previous,
        &differences,
    );
    ReviewFocus {
        depth,
        content,
        previous: Some(previous),
        trusted_fast_path,
        summary,
        differences,
    }
}

/// What the lock installs; `none` for a package without Python.
fn packages<'a>(
    facts: &'a ReleaseFacts,
    none: &'a BTreeMap<String, LockedPackage>,
) -> &'a BTreeMap<String, LockedPackage> {
    facts.python.as_ref().map_or(none, |p| &p.packages)
}

fn push(out: &mut Vec<Difference>, kind: DifferenceKind, subject: &str, detail: impl Into<String>) {
    out.push(Difference {
        kind,
        subject: subject.to_owned(),
        detail: detail.into(),
    });
}

/// How far a scope reaches: what the extension owns, what the user picks, the whole open project.
fn reach(scope: Scope) -> u8 {
    match scope {
        Scope::PluginOwned => 0,
        Scope::UserSelected => 1,
        Scope::CurrentProject => 2,
    }
}

/// A capability as asked, its entries merged (the inspector refuses one asked twice; merging keeps the widest).
struct Asked {
    scope: Scope,
    hosts: BTreeSet<String>,
    ai: bool,
    reasons: BTreeSet<String>,
}

impl Asked {
    fn describe(&self, id: Capability) -> String {
        let mut text = format!("{} · {}", id.title(), self.scope.id());
        if !self.hosts.is_empty() {
            text.push_str(" · ");
            text.push_str(&self.hosts.iter().cloned().collect::<Vec<_>>().join(", "));
        }
        if self.ai {
            text.push_str(" · dış yapay zekâya veri gönderir");
        }
        text
    }
}

fn asked(list: &[RequestedCapability]) -> BTreeMap<Capability, Asked> {
    let mut out: BTreeMap<Capability, Asked> = BTreeMap::new();
    for r in list {
        let a = out.entry(r.id).or_insert_with(|| Asked {
            scope: r.scope,
            hosts: BTreeSet::new(),
            ai: false,
            reasons: BTreeSet::new(),
        });
        if reach(r.scope) > reach(a.scope) {
            a.scope = r.scope;
        }
        a.hosts.extend(r.hosts.iter().cloned());
        a.ai |= r.sends_data_to_ai == Some(true);
        a.reasons.insert(r.reason.clone());
    }
    out
}

fn capabilities(
    before: &[RequestedCapability],
    after: &[RequestedCapability],
    out: &mut Vec<Difference>,
) {
    let (old, new) = (asked(before), asked(after));
    for (id, n) in &new {
        let Some(o) = old.get(id) else {
            push(out, DifferenceKind::NewCapability, id.id(), n.describe(*id));
            continue;
        };
        let (mut wider, mut narrower) = (Vec::new(), Vec::new());
        match reach(n.scope).cmp(&reach(o.scope)) {
            std::cmp::Ordering::Greater => {
                wider.push(format!("kapsam {} → {}", o.scope.id(), n.scope.id()))
            }
            std::cmp::Ordering::Less => {
                narrower.push(format!("kapsam {} → {}", o.scope.id(), n.scope.id()))
            }
            std::cmp::Ordering::Equal => {}
        }
        let added: Vec<&str> = n.hosts.difference(&o.hosts).map(String::as_str).collect();
        if !added.is_empty() {
            wider.push(format!("yeni adres: {}", added.join(", ")));
        }
        let removed: Vec<&str> = o.hosts.difference(&n.hosts).map(String::as_str).collect();
        if !removed.is_empty() {
            narrower.push(format!("kalkan adres: {}", removed.join(", ")));
        }
        if n.ai && !o.ai {
            wider.push("dış yapay zekâya veri göndermeye başlıyor".to_owned());
        } else if o.ai && !n.ai {
            narrower.push("dış yapay zekâya veri göndermeyi bırakıyor".to_owned());
        }
        if !wider.is_empty() {
            wider.extend(narrower);
            push(
                out,
                DifferenceKind::WiderCapability,
                id.id(),
                wider.join("; "),
            );
        } else if !narrower.is_empty() {
            push(
                out,
                DifferenceKind::NarrowerCapability,
                id.id(),
                narrower.join("; "),
            );
        } else if n.reasons != o.reasons {
            push(out, DifferenceKind::Text, id.id(), "gerekçesi değişti");
        }
    }
    for (id, o) in &old {
        if !new.contains_key(id) {
            push(
                out,
                DifferenceKind::DroppedCapability,
                id.id(),
                o.describe(*id),
            );
        }
    }
}

fn extensions(
    before: &[ExtensionDependency],
    after: &[ExtensionDependency],
    out: &mut Vec<Difference>,
) {
    let ranges = |list: &[ExtensionDependency]| -> BTreeMap<String, String> {
        list.iter()
            .map(|d| (d.id.clone(), d.version.clone()))
            .collect()
    };
    let (old, new) = (ranges(before), ranges(after));
    for (id, range) in &new {
        match old.get(id) {
            None => push(
                out,
                DifferenceKind::NewDependency,
                id,
                format!("eklenti, {range}"),
            ),
            Some(was) if was != range => push(
                out,
                DifferenceKind::ChangedDependency,
                id,
                format!("eklenti, {was} → {range}"),
            ),
            Some(_) => {}
        }
    }
    for (id, was) in &old {
        if !new.contains_key(id) {
            push(
                out,
                DifferenceKind::DroppedDependency,
                id,
                format!("eklenti, {was}"),
            );
        }
    }
}

fn python_packages(
    before: &BTreeMap<String, LockedPackage>,
    after: &BTreeMap<String, LockedPackage>,
    out: &mut Vec<Difference>,
) {
    for (name, n) in after {
        match before.get(name) {
            None => push(
                out,
                DifferenceKind::NewDependency,
                name,
                format!("Python, {}", n.describe()),
            ),
            Some(o) if o.source != n.source => push(
                out,
                DifferenceKind::DependencySource,
                name,
                format!("Python, kaynağı {} → {}", o.source, n.source),
            ),
            Some(o) if o.version != n.version => push(
                out,
                DifferenceKind::ChangedDependency,
                name,
                format!("Python, {} → {}", o.version, n.version),
            ),
            Some(o) if o.files != n.files => push(
                out,
                DifferenceKind::ChangedDependency,
                name,
                format!("Python, {}: aynı sürümün dosyaları değişti", n.version),
            ),
            Some(_) => {}
        }
    }
    for (name, o) in before {
        if !after.contains_key(name) {
            push(
                out,
                DifferenceKind::DroppedDependency,
                name,
                format!("Python, {}", o.describe()),
            );
        }
    }
}

/// The project file and the lock, beyond the packages: a Python project added or removed is code, so is its file
/// changed beyond its version; a lock changed with the same packages is the lock's own difference.
fn python_project(before: &ReleaseFacts, after: &ReleaseFacts, out: &mut Vec<Difference>) {
    let path = |f: &ReleaseFacts| f.manifest.python.as_ref().map(|p| p.project_file.clone());
    let lock_path = |f: &ReleaseFacts| f.manifest.python.as_ref().map(|p| p.lock_file.clone());
    match (&before.python, &after.python) {
        (None, Some(_)) => push(
            out,
            DifferenceKind::Code,
            &path(after).unwrap_or_default(),
            "Python projesi eklendi",
        ),
        (Some(_), None) => push(
            out,
            DifferenceKind::Code,
            &path(before).unwrap_or_default(),
            "Python projesi kaldırıldı",
        ),
        (Some(o), Some(n)) => {
            if o.project_compared != n.project_compared {
                push(
                    out,
                    DifferenceKind::Code,
                    &path(after).unwrap_or_default(),
                    "değişti (sürüm numarası dışında)",
                );
            }
            if o.lock_compared != n.lock_compared && o.packages == n.packages {
                push(
                    out,
                    DifferenceKind::Lock,
                    &lock_path(after).unwrap_or_default(),
                    "kilit değişti, paketleri aynı",
                );
            }
        }
        (None, None) => {}
    }
}

/// The manifest's fields: the listing's words are text; capabilities, dependencies and the version are said elsewhere;
/// every other field is behaviour.
fn manifest_fields(before: &Manifest, after: &Manifest, out: &mut Vec<Difference>) {
    const TEXT: [&str; 11] = [
        "name",
        "summary",
        "description",
        "authors",
        "keywords",
        "links",
        "changelogFile",
        "icon",
        "license",
        "licenseFile",
        "l10n",
    ];
    const ELSEWHERE: [&str; 3] = ["version", "requestedCapabilities", "dependencies"];
    let (Ok(serde_json::Value::Object(old)), Ok(serde_json::Value::Object(new))) =
        (serde_json::to_value(before), serde_json::to_value(after))
    else {
        push(
            out,
            DifferenceKind::Code,
            manifest::FILE,
            "karşılaştırılamadı",
        );
        return;
    };
    let keys: BTreeSet<&String> = old.keys().chain(new.keys()).collect();
    let (mut text, mut code) = (Vec::new(), Vec::new());
    for key in keys {
        if ELSEWHERE.contains(&key.as_str()) || old.get(key) == new.get(key) {
            continue;
        }
        if TEXT.contains(&key.as_str()) {
            text.push(key.as_str());
        } else {
            code.push(key.as_str());
        }
    }
    if !code.is_empty() {
        push(
            out,
            DifferenceKind::Code,
            manifest::FILE,
            format!("değişen alanlar: {}", code.join(", ")),
        );
    }
    if !text.is_empty() {
        push(
            out,
            DifferenceKind::Text,
            manifest::FILE,
            format!("değişen alanlar: {}", text.join(", ")),
        );
    }
}

/// The package's other files: text and icons by what the manifest names them (the licence, the change notes, an icon,
/// a translation) or by a text file's extension; anything else is code.
fn files(before: &ReleaseFacts, after: &ReleaseFacts, out: &mut Vec<Difference>) {
    let manifests = [&before.manifest, &after.manifest];
    let mut skip: BTreeSet<&str> = BTreeSet::from([manifest::FILE]);
    let mut roles: BTreeMap<String, BTreeSet<FileRole>> = BTreeMap::new();
    for m in manifests {
        if let Some(python) = &m.python {
            skip.insert(&python.project_file);
            skip.insert(&python.lock_file);
        }
        for reference in m.references() {
            roles
                .entry(reference.path)
                .or_default()
                .insert(reference.role);
        }
    }
    let digests = |f: &ReleaseFacts| -> BTreeMap<String, String> {
        f.files
            .iter()
            .map(|d| (d.path.clone(), d.sha256.clone()))
            .collect()
    };
    let (old, new) = (digests(before), digests(after));
    let paths: BTreeSet<&String> = old.keys().chain(new.keys()).collect();
    for path in paths {
        if skip.contains(path.as_str()) {
            continue;
        }
        let detail = match (old.get(path), new.get(path)) {
            (None, Some(_)) => "eklendi",
            (Some(_), None) => "silindi",
            (Some(a), Some(b)) if a != b => "değişti",
            _ => continue,
        };
        let text = match roles.get(path) {
            Some(roles) => roles.iter().all(|r| {
                matches!(
                    r,
                    FileRole::License
                        | FileRole::Changelog
                        | FileRole::Icon
                        | FileRole::Translation
                )
            }),
            None => {
                let lower = path.to_ascii_lowercase();
                lower.ends_with(".md") || lower.ends_with(".txt")
            }
        };
        let kind = if text {
            DifferenceKind::Text
        } else {
            DifferenceKind::Code
        };
        push(out, kind, path, detail);
    }
}

fn sort(differences: &mut [Difference], trusted_code: bool) {
    differences.sort_by(|a, b| {
        b.kind
            .depth(trusted_code)
            .cmp(&a.kind.depth(trusted_code))
            .then(a.kind.cmp(&b.kind))
            .then_with(|| a.subject.cmp(&b.subject))
    });
}

/// One line for the queue: the depth and the kinds of difference that set it.
fn summary(
    depth: ReviewDepth,
    trusted_fast_path: bool,
    trusted_code: bool,
    previous: &str,
    differences: &[Difference],
) -> String {
    if differences.is_empty() {
        return format!(
            "{} — önceki onaylı sürümden ({previous}) yalnız sürüm numarası farklı",
            depth.title()
        );
    }
    let mut kinds: Vec<DifferenceKind> = differences
        .iter()
        .filter(|d| d.kind.depth(trusted_code) == depth)
        .map(|d| d.kind)
        .collect();
    kinds.dedup();
    let parts: Vec<String> = kinds
        .iter()
        .map(|kind| {
            let subjects: Vec<&str> = differences
                .iter()
                .filter(|d| d.kind == *kind)
                .map(|d| d.subject.as_str())
                .collect();
            let mut list = subjects
                .iter()
                .take(3)
                .copied()
                .collect::<Vec<_>>()
                .join(", ");
            if subjects.len() > 3 {
                list.push_str(&format!(" ve {} tane daha", subjects.len() - 3));
            }
            format!("{}: {list}", kind.title())
        })
        .collect();
    let head = if trusted_fast_path {
        format!(
            "{} (güvenilir yayımcı, yetenek ve bağımlılık farkı yok)",
            depth.title()
        )
    } else {
        depth.title().to_owned()
    };
    format!(
        "{head} — {} (önceki onaylı sürüm {previous})",
        parts.join(" · ")
    )
}

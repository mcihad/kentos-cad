//! The data profiles and profile mappings a project carries inside its own file (docs/adr/0735; plan/gis.md CBS-34's
//! remainder). Beside the `profiller` folder next to the drawing's file (docs/adr/0558), the project's settings keep
//! `profileFiles`: each a profile or a mapping by its name and its document as text, the same text a folder file holds.
//! The text is the institution's document, read by the profile commands as they read a file; the drawing file's
//! schema does not follow the profile's (`.kcad` schema 132 says only that the list is there and holds these rules).

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

/// What a project's profile file is: a data profile or a mapping of one.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum ProfileFileKind {
    Profile,
    Mapping,
}

/// A profile or a mapping the project carries inside its file.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ProjectProfileFile {
    pub kind: ProfileFileKind,
    /// The profile's or the mapping's name, the name of its folder file too: small letters, digits and `-`, at most 64.
    pub name: String,
    /// The document, JSON as the folder's file holds it (`kentos.data-profile` or `kentos.profile-mapping`).
    pub text: String,
}

/// The most profiles and mappings a project carries inside its file.
pub const MAX_PROJECT_PROFILE_FILES: usize = 256;
/// The longest text of one, bytes: a folder file's limit.
pub const MAX_PROFILE_TEXT_BYTES: usize = 4 * 1024 * 1024;

/// Whether `name` is a name a profile or a mapping may carry (small letters, digits and `-`, at most 64): its folder
/// file's name too.
pub fn profile_name_ok(name: &str) -> bool {
    (1..=64).contains(&name.len())
        && name
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
}

/// The order the list is kept in: by name, a profile before a mapping of the same name.
fn key(f: &ProjectProfileFile) -> (&str, ProfileFileKind) {
    (f.name.as_str(), f.kind)
}

fn file_problem(f: &ProjectProfileFile) -> Option<String> {
    if !profile_name_ok(&f.name) {
        return Some(format!(
            "“{}” bir profil ya da eşleme adı olamaz: küçük harf, rakam ve tire, en çok 64",
            f.name
        ));
    }
    if f.text.trim().is_empty() {
        return Some(format!("“{}” boş metinle yazılmaz", f.name));
    }
    if f.text.len() > MAX_PROFILE_TEXT_BYTES {
        return Some(format!(
            "“{}” {} bayt; en çok {MAX_PROFILE_TEXT_BYTES} bayt olur",
            f.name,
            f.text.len()
        ));
    }
    None
}

/// Why a project's profile files do not hold, the first problem: a name the rule refuses, an empty or too long text,
/// more than [`MAX_PROJECT_PROFILE_FILES`], a name of a kind twice, or a list out of its order (by name, a profile before
/// a mapping). The text's document is the profile commands' to judge.
pub fn profile_files_problem(files: &[ProjectProfileFile]) -> Option<String> {
    if files.len() > MAX_PROJECT_PROFILE_FILES {
        return Some(format!(
            "{} profil ve eşleme; en çok {MAX_PROJECT_PROFILE_FILES} olur",
            files.len()
        ));
    }
    if let Some(why) = files.iter().find_map(file_problem) {
        return Some(why);
    }
    for pair in files.windows(2) {
        if key(&pair[0]) == key(&pair[1]) {
            return Some(format!("“{}” iki kez yazılmış", pair[1].name));
        }
        if key(&pair[0]) > key(&pair[1]) {
            return Some(
                "profiller ve eşlemeler adlarının sırasıyla yazılır (aynı adda önce profil)".into(),
            );
        }
    }
    None
}

/// The files kept as a project keeps them: those that hold, in order, the first of a name and kind, at most
/// [`MAX_PROJECT_PROFILE_FILES`].
pub fn sanitized_profile_files(list: Vec<ProjectProfileFile>) -> Vec<ProjectProfileFile> {
    let mut out: Vec<ProjectProfileFile> = Vec::new();
    for f in list {
        if file_problem(&f).is_some() || out.iter().any(|o| key(o) == key(&f)) {
            continue;
        }
        out.push(f);
    }
    out.sort_by(|a, b| key(a).cmp(&key(b)));
    out.truncate(MAX_PROJECT_PROFILE_FILES);
    out
}

/// `files` with `file` put in its place: the one of its name and kind replaced, else added in order.
pub fn with_profile_file(
    mut files: Vec<ProjectProfileFile>,
    file: ProjectProfileFile,
) -> Vec<ProjectProfileFile> {
    match files.binary_search_by(|f| key(f).cmp(&key(&file))) {
        Ok(at) => files[at] = file,
        Err(at) => files.insert(at, file),
    }
    files
}

#[cfg(test)]
mod tests {
    use super::*;

    fn file(kind: ProfileFileKind, name: &str) -> ProjectProfileFile {
        ProjectProfileFile {
            kind,
            name: name.into(),
            text: "{}".into(),
        }
    }

    #[test]
    fn the_list_holds_its_order_and_names() {
        use ProfileFileKind::{Mapping, Profile};
        assert_eq!(profile_files_problem(&[]), None);
        let ok = [
            file(Profile, "park"),
            file(Mapping, "park"),
            file(Mapping, "sivas"),
        ];
        assert_eq!(profile_files_problem(&ok), None);
        assert!(profile_files_problem(&[file(Mapping, "park"), file(Profile, "park")]).is_some());
        assert!(profile_files_problem(&[file(Profile, "b"), file(Profile, "a")]).is_some());
        assert!(profile_files_problem(&[file(Profile, "a"), file(Profile, "a")]).is_some());
        assert!(profile_files_problem(&[file(Profile, "Park")]).is_some());
        let blank = ProjectProfileFile {
            text: "  ".into(),
            ..file(Profile, "a")
        };
        assert!(profile_files_problem(std::slice::from_ref(&blank)).is_some());
        let kept = sanitized_profile_files(vec![
            file(Mapping, "b"),
            blank,
            file(Profile, "b"),
            file(Mapping, "b"),
        ]);
        assert_eq!(kept, [file(Profile, "b"), file(Mapping, "b")]);
        let put = with_profile_file(kept, file(Profile, "a"));
        assert_eq!(put[0], file(Profile, "a"));
        assert_eq!(profile_files_problem(&put), None);
    }
}
